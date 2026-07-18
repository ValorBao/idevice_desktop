use base64::{Engine as _, engine::general_purpose::STANDARD};
use idevice::{
    IdeviceService, RsdService,
    core_device_proxy::CoreDeviceProxy,
    dvt::{remote_server::RemoteServerClient, screenshot::ScreenshotClient},
    rsd::RsdHandshake,
    screenshotr::ScreenshotService,
};
use tauri::State;

use crate::{
    error::{CommandError, CommandResult},
    provider::provider_for,
    state::AppState,
};

#[tauri::command]
pub async fn device_screenshot(
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<String> {
    let udid = state
        .selected(udid)
        .await
        .ok_or_else(|| CommandError::new("device", "No device selected", true))?;
    tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| CommandError::new("runtime", error.to_string(), true))?;
        runtime.block_on(capture_screenshot(udid))
    })
    .await
    .map_err(|error| CommandError::new("runtime", error.to_string(), true))?
}

async fn capture_screenshot(udid: String) -> CommandResult<String> {
    let provider = provider_for(&udid).await?;

    let bytes = if let Ok(proxy) = CoreDeviceProxy::connect(&provider).await {
        let rsd_port = proxy.tunnel_info().server_rsd_port;
        let adapter = proxy.create_software_tunnel().map_err(|error| {
            CommandError::new("screenshot", format!("Unable to create device tunnel: {error}"), true)
        })?;
        let mut adapter = adapter.to_async_handle();
        let stream = adapter.connect(rsd_port).await.map_err(|error| {
            CommandError::new("screenshot", format!("Unable to connect to RSD: {error}"), true)
        })?;
        let mut handshake = RsdHandshake::new(stream)
            .await
            .map_err(CommandError::from)?;
        let mut remote = RemoteServerClient::connect_rsd(&mut adapter, &mut handshake)
            .await
            .map_err(|error| {
                CommandError::new(
                    "screenshot",
                    format!("Screenshot service unavailable. Mount the Developer Disk Image first: {error}"),
                    true,
                )
            })?;
        remote.read_message(0).await.map_err(CommandError::from)?;
        let mut client = ScreenshotClient::new(&mut remote)
            .await
            .map_err(CommandError::from)?;
        client.take_screenshot().await.map_err(CommandError::from)?
    } else {
        let mut client = ScreenshotService::connect(&provider).await.map_err(|error| {
            CommandError::new(
                "screenshot",
                format!("Screenshot service unavailable. Mount the Developer Disk Image first: {error}"),
                true,
            )
        })?;
        client.take_screenshot().await.map_err(CommandError::from)?
    };

    if bytes.is_empty() {
        return Err(CommandError::new(
            "screenshot",
            "The device returned an empty screenshot",
            true,
        ));
    }

    Ok(format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
}
