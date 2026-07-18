use futures_util::StreamExt;
use idevice::{
    IdeviceService,
    lockdown::LockdownClient,
    provider::IdeviceProvider,
    usbmuxd::{Connection, UsbmuxdAddr, UsbmuxdConnection, UsbmuxdDevice, UsbmuxdListenEvent},
};
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use crate::{
    error::{CommandError, CommandResult},
    provider::provider_for,
    state::AppState,
    types::{DeviceChangeEvent, DeviceSummary},
};

fn connection_label(connection: &Connection) -> String {
    match connection {
        Connection::Usb => "USB".to_string(),
        Connection::Network(address) => format!("Network · {address}"),
        Connection::Unknown(value) => value.clone(),
    }
}

async fn enrich_device(device: UsbmuxdDevice) -> DeviceSummary {
    let mut paired = false;
    let mut name = None;
    let mut model = None;
    let mut ios = None;

    if let Ok(mut mux) = UsbmuxdConnection::default().await {
        paired = mux.get_pair_record(&device.udid).await.is_ok();
    }

    if let Ok(provider) = provider_for(&device.udid).await
        && let Ok(mut lockdown) = LockdownClient::connect(&provider).await
    {
        if paired && let Ok(pairing_file) = provider.get_pairing_file().await {
            let _ = lockdown.start_session(&pairing_file).await;
        }
        name = lockdown
            .get_value(Some("DeviceName"), None)
            .await
            .ok()
            .and_then(|value| value.into_string());
        model = lockdown
            .get_value(Some("ProductType"), None)
            .await
            .ok()
            .and_then(|value| value.into_string());
        ios = lockdown
            .get_value(Some("ProductVersion"), None)
            .await
            .ok()
            .and_then(|value| value.into_string());
    }

    DeviceSummary {
        udid: device.udid,
        device_id: device.device_id,
        connection: connection_label(&device.connection_type),
        paired,
        name,
        model,
        ios,
    }
}

#[tauri::command]
pub async fn device_list() -> CommandResult<Vec<DeviceSummary>> {
    let mut mux = UsbmuxdConnection::default()
        .await
        .map_err(CommandError::from)?;
    let devices = mux.get_devices().await.map_err(CommandError::from)?;
    let mut result = Vec::with_capacity(devices.len());
    for device in devices {
        result.push(enrich_device(device).await);
    }
    Ok(result)
}

#[tauri::command]
pub async fn device_select(state: State<'_, AppState>, udid: String) -> CommandResult<()> {
    let _ = provider_for(&udid).await?;
    state.cancel_device_tasks().await;
    *state.selected_udid.write().await = Some(udid);
    Ok(())
}

#[tauri::command]
pub async fn device_disconnect(state: State<'_, AppState>) -> CommandResult<()> {
    state.cancel_device_tasks().await;
    *state.selected_udid.write().await = None;
    Ok(())
}

#[tauri::command]
pub async fn device_pair(udid: String, host_name: Option<String>) -> CommandResult<DeviceSummary> {
    let mut mux = UsbmuxdConnection::default()
        .await
        .map_err(CommandError::from)?;
    let device = mux.get_device(&udid).await.map_err(CommandError::from)?;
    if device.connection_type != Connection::Usb {
        return Err(CommandError::new(
            "pairing",
            "Initial pairing requires a USB connection",
            false,
        ));
    }

    let provider = device.to_provider(
        UsbmuxdAddr::from_env_var().map_err(CommandError::from)?,
        "idevice-desktop-pair",
    );
    let mut lockdown = LockdownClient::connect(&provider)
        .await
        .map_err(CommandError::from)?;
    let host_id = uuid::Uuid::new_v4().to_string().to_uppercase();
    let buid = mux.get_buid().await.map_err(CommandError::from)?;
    let mut pairing_file = lockdown
        .pair(host_id, buid, host_name.as_deref())
        .await
        .map_err(CommandError::from)?;

    lockdown
        .start_session(&pairing_file)
        .await
        .map_err(CommandError::from)?;
    pairing_file.udid = Some(udid.clone());
    let bytes = pairing_file.serialize().map_err(CommandError::from)?;
    mux.save_pair_record(&udid, bytes)
        .await
        .map_err(CommandError::from)?;

    Ok(enrich_device(device).await)
}

#[tauri::command]
pub async fn device_forget(udid: String, state: State<'_, AppState>) -> CommandResult<()> {
    let provider = provider_for(&udid).await?;
    let pairing_file = provider
        .get_pairing_file()
        .await
        .map_err(CommandError::from)?;
    if let Ok(mut lockdown) = LockdownClient::connect(&provider).await {
        let _ = lockdown.unpair(pairing_file.host_id).await;
    }
    let mut mux = UsbmuxdConnection::default()
        .await
        .map_err(CommandError::from)?;
    mux.delete_pair_record(&udid)
        .await
        .map_err(CommandError::from)?;
    if state.selected_udid.read().await.as_deref() == Some(udid.as_str()) {
        state.cancel_device_tasks().await;
        *state.selected_udid.write().await = None;
    }
    Ok(())
}

#[tauri::command]
pub async fn device_monitor_start(app: AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    let token = CancellationToken::new();
    state.replace_task("device-monitor", token.clone()).await;

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("device monitor runtime");
        runtime.block_on(async move {
            let Ok(mut mux) = UsbmuxdConnection::default().await else {
                return;
            };
            let Ok(mut stream) = mux.listen().await else {
                return;
            };

            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    event = stream.next() => {
                        let Some(Ok(event)) = event else { break };
                        let payload = match event {
                            UsbmuxdListenEvent::Connected(device) => DeviceChangeEvent {
                                kind: "connected".into(),
                                device: Some(enrich_device(device).await),
                                device_id: None,
                            },
                            UsbmuxdListenEvent::Disconnected(device_id) => DeviceChangeEvent {
                                kind: "disconnected".into(),
                                device: None,
                                device_id: Some(device_id),
                            },
                        };
                        let _ = app.emit("device://changed", payload);
                    }
                }
            }
        });
    });
    Ok(())
}

#[tauri::command]
pub async fn device_monitor_stop(state: State<'_, AppState>) -> CommandResult<()> {
    state.cancel_task("device-monitor").await;
    Ok(())
}
