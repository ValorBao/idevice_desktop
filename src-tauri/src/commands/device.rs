use futures_util::StreamExt;
use idevice::{
    IdeviceService,
    lockdown::LockdownClient,
    provider::IdeviceProvider,
    usbmuxd::{Connection, UsbmuxdAddr, UsbmuxdConnection, UsbmuxdDevice, UsbmuxdListenEvent},
};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

use crate::{
    discovery::{BonjourEndpoint, NETWORK_SERVICE_TYPES, UsbDiscovery},
    error::{CommandError, CommandResult},
    provider::{provider_for, routed_provider_for},
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

/// Decides which UDID a selection request should route to.
///
/// A catalog entry that carries a usable Lockdown route wins. An entry the
/// catalog knows but cannot route is refused rather than attempted, because the
/// attempt would fail later with a transport error that does not explain what
/// the user has to do. An id the catalog has never seen is passed through, so a
/// device that appeared between the last discovery pass and this call is still
/// reachable.
fn resolve_selection(
    requested: &str,
    connectable: Option<String>,
    known: bool,
) -> CommandResult<String> {
    if let Some(connectable) = connectable {
        return Ok(connectable);
    }
    if known || requested.starts_with("bonjour:") {
        return Err(CommandError::new(
            "network_discovery",
            "This iPhone is visible through Bonjour but has no usable Lockdown route. Connect it by USB once or configure paired Wi-Fi access.",
            true,
        ));
    }
    Ok(requested.to_string())
}

async fn enrich_device(device: UsbmuxdDevice) -> UsbDiscovery {
    let mut paired = false;
    let mut name = None;
    let mut model = None;
    let mut ios = None;
    let mut wifi_address = None;

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
        wifi_address = lockdown
            .get_value(Some("WiFiAddress"), None)
            .await
            .ok()
            .and_then(|value| value.into_string());
    }

    UsbDiscovery {
        udid: device.udid,
        device_id: device.device_id,
        connection: connection_label(&device.connection_type),
        paired,
        name,
        model,
        ios,
        wifi_address,
    }
}

#[tauri::command]
pub async fn device_list(state: State<'_, AppState>) -> CommandResult<Vec<DeviceSummary>> {
    match UsbmuxdConnection::default().await {
        Ok(mut mux) => match mux.get_devices().await {
            Ok(devices) => {
                let mut enriched = Vec::with_capacity(devices.len());
                for device in devices {
                    enriched.push(enrich_device(device).await);
                }
                state.discovery.write().await.replace_usbmuxd(enriched);
            }
            Err(error) => {
                if state.discovery.read().await.summaries().is_empty() {
                    return Err(CommandError::from(error));
                }
                tracing::warn!(?error, "unable to refresh usbmuxd discovery");
            }
        },
        Err(error) => {
            if state.discovery.read().await.summaries().is_empty() {
                return Err(CommandError::from(error));
            }
            tracing::warn!(?error, "unable to connect to usbmuxd during discovery");
        }
    }
    Ok(state.discovery.read().await.summaries())
}

#[tauri::command]
pub async fn device_select(state: State<'_, AppState>, udid: String) -> CommandResult<()> {
    let discovery = state.discovery.read().await;
    let selected_udid = resolve_selection(
        &udid,
        discovery.connectable_udid(&udid),
        discovery.contains(&udid),
    )?;
    let lockdown_target = discovery.lockdown_target(&udid);
    drop(discovery);
    let _ = routed_provider_for(&selected_udid, lockdown_target.as_ref()).await?;
    state.cancel_device_tasks().await;
    *state.selected_udid.write().await = Some(selected_udid);
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

    let discovered = enrich_device(device).await;
    let transport = if discovered.connection.starts_with("USB") {
        "USB"
    } else {
        "usbmuxd Network"
    };
    Ok(DeviceSummary {
        id: discovered.udid.clone(),
        udid: discovered.udid,
        device_id: discovered.device_id,
        connection: discovered.connection,
        transports: vec![transport.into()],
        connectable: true,
        paired: discovered.paired,
        name: discovered.name,
        model: discovered.model,
        ios: discovered.ios,
    })
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

    let usb_app = app.clone();
    let usb_token = token.clone();
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
                    _ = usb_token.cancelled() => break,
                    event = stream.next() => {
                        let Some(Ok(event)) = event else { break };
                        let payload = match event {
                            UsbmuxdListenEvent::Connected(device) => {
                                let discovered = enrich_device(device).await;
                                let summary = usb_app
                                    .state::<AppState>()
                                    .discovery
                                    .write()
                                    .await
                                    .upsert_usb(discovered);
                                DeviceChangeEvent {
                                    kind: "connected".into(),
                                    device: Some(summary),
                                    device_id: None,
                                }
                            }
                            UsbmuxdListenEvent::Disconnected(device_id) => {
                                usb_app
                                    .state::<AppState>()
                                    .discovery
                                    .write()
                                    .await
                                    .remove_usbmuxd(device_id);
                                DeviceChangeEvent {
                                    kind: "disconnected".into(),
                                    device: None,
                                    device_id: Some(device_id),
                                }
                            }
                        };
                        let _ = usb_app.emit("device://changed", payload);
                    }
                }
            }
        });
    });

    spawn_bonjour_monitor(app, token);
    Ok(())
}

fn spawn_bonjour_monitor(app: AppHandle, token: CancellationToken) {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Bonjour monitor runtime");
        runtime.block_on(async move {
            let Ok(daemon) = ServiceDaemon::new() else {
                return;
            };
            let Ok(mobdev) = daemon.browse(NETWORK_SERVICE_TYPES[0]) else {
                let _ = daemon.shutdown();
                return;
            };
            let Ok(remote_pairing) = daemon.browse(NETWORK_SERVICE_TYPES[1]) else {
                let _ = daemon.shutdown();
                return;
            };
            let Ok(manual_pairing) = daemon.browse(NETWORK_SERVICE_TYPES[2]) else {
                let _ = daemon.shutdown();
                return;
            };
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    event = mobdev.recv_async() => {
                        let Ok(event) = event else { break };
                        handle_bonjour_event(&app, event).await;
                    },
                    event = remote_pairing.recv_async() => {
                        let Ok(event) = event else { break };
                        handle_bonjour_event(&app, event).await;
                    },
                    event = manual_pairing.recv_async() => {
                        let Ok(event) = event else { break };
                        handle_bonjour_event(&app, event).await;
                    },
                }
            }
            for service_type in NETWORK_SERVICE_TYPES {
                let _ = daemon.stop_browse(service_type);
            }
            let _ = daemon.shutdown();
        });
    });
}

async fn handle_bonjour_event(app: &AppHandle, event: ServiceEvent) {
    let payload = match event {
        ServiceEvent::ServiceResolved(service) => {
            let endpoint = BonjourEndpoint::from_resolved(&service);
            let service_type = endpoint.service_type.clone();
            let summary = app
                .state::<AppState>()
                .discovery
                .write()
                .await
                .upsert_bonjour(endpoint);
            tracing::info!(
                service_type,
                connectable = summary.connectable,
                transports = ?summary.transports,
                "network device discovered"
            );
            Some(DeviceChangeEvent {
                kind: "updated".into(),
                device: Some(summary),
                device_id: None,
            })
        }
        ServiceEvent::ServiceRemoved(_, fullname) => {
            app.state::<AppState>()
                .discovery
                .write()
                .await
                .remove_bonjour(&fullname);
            Some(DeviceChangeEvent {
                kind: "disconnected".into(),
                device: None,
                device_id: None,
            })
        }
        _ => None,
    };
    if let Some(payload) = payload {
        let _ = app.emit("device://changed", payload);
    }
}

#[tauri::command]
pub async fn device_monitor_stop(state: State<'_, AppState>) -> CommandResult<()> {
    state.cancel_task("device-monitor").await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{connection_label, resolve_selection};
    use idevice::usbmuxd::Connection;

    #[test]
    fn labels_every_connection_kind() {
        assert_eq!(connection_label(&Connection::Usb), "USB");
        assert_eq!(
            connection_label(&Connection::Network("192.168.1.20".parse().unwrap())),
            "Network · 192.168.1.20"
        );
        assert_eq!(
            connection_label(&Connection::Unknown("Carrier".into())),
            "Carrier",
            "an unrecognised kind is shown as reported rather than replaced"
        );
    }

    #[test]
    fn prefers_the_routable_udid_over_the_requested_id() {
        let resolved = resolve_selection("bonjour:abc", Some("REAL-UDID".into()), true)
            .expect("a connectable record routes");
        assert_eq!(
            resolved, "REAL-UDID",
            "the catalog key was requested, but the device is reached by its UDID"
        );
    }

    /// The failure this guards against is a Bonjour-only device: visible in the
    /// list, but with no Lockdown route. Attempting it surfaces a transport
    /// error that does not tell the user to plug in a cable.
    #[test]
    fn refuses_a_known_record_with_no_route() {
        let error = resolve_selection("KNOWN-UDID", None, true)
            .expect_err("a known but unroutable record must be refused up front");
        assert_eq!(error.kind, "network_discovery");
        assert!(error.retryable, "plugging in USB makes this succeed");
        assert!(error.message.contains("USB"));
    }

    #[test]
    fn refuses_a_bonjour_key_even_when_the_catalog_has_dropped_it() {
        assert!(
            resolve_selection("bonjour:stale", None, false).is_err(),
            "a bonjour: key never names something usbmuxd can open"
        );
    }

    /// Discovery runs on its own schedule, so a device can be selected before
    /// the catalog has caught up. That has to stay reachable.
    #[test]
    fn passes_through_an_unknown_udid() {
        let resolved = resolve_selection("FRESH-UDID", None, false)
            .expect("an id the catalog has not seen yet is still attempted");
        assert_eq!(resolved, "FRESH-UDID");
    }
}
