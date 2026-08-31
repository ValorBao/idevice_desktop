use std::{collections::HashSet, time::Duration};

use idevice::{
    IdeviceService, RsdService, core_device_proxy::CoreDeviceProxy,
    notification_proxy::NotificationProxyClient, rsd::RsdHandshake, tcp::handle::AdapterHandle,
};
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use crate::{
    device_version::{DeveloperGeneration, ios_version},
    discovery::{LockdownTarget, RemotePairingTarget},
    error::{CommandError, CommandResult},
    provider::{RoutedProvider, routed_provider_for},
    state::AppState,
    tunnel::{open_remote_pairing_tunnel, remote_pairing_path},
    types::{NotificationObservationEvent, NotificationObservationStatus},
};

const TASK_KEY: &str = "notification-observation";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(45);
const MAX_SUBSCRIPTIONS: usize = 32;
const MAX_NAME_BYTES: usize = 200;
const MAX_SESSION_ID_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NotificationTransport {
    Lockdown,
    RemoteRsd,
    CoreDeviceRsd,
    UsbRequired,
}

#[derive(Clone)]
struct NotificationContext {
    udid: String,
    pairing_path: std::path::PathBuf,
    lockdown_target: Option<LockdownTarget>,
    remote_target: Option<RemotePairingTarget>,
}

struct NotificationConnection {
    client: NotificationProxyClient,
    transport: String,
    _adapter: Option<AdapterHandle>,
}

fn notification_transport(
    is_bonjour: bool,
    generation: DeveloperGeneration,
) -> NotificationTransport {
    match (is_bonjour, generation) {
        (false, _) => NotificationTransport::Lockdown,
        (true, DeveloperGeneration::Legacy) => NotificationTransport::UsbRequired,
        (true, DeveloperGeneration::CoreDeviceRemote) => NotificationTransport::RemoteRsd,
        (true, DeveloperGeneration::CoreDeviceLockdown) => NotificationTransport::CoreDeviceRsd,
    }
}

fn validate_session_id(value: String) -> CommandResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() || value.len() > MAX_SESSION_ID_BYTES || value.chars().any(char::is_control)
    {
        return Err(CommandError::new(
            "notifications",
            "The notification session identifier is invalid",
            false,
        ));
    }
    Ok(value)
}

fn validate_subscriptions(values: Vec<String>) -> CommandResult<Vec<String>> {
    let mut seen = HashSet::new();
    let mut subscriptions = Vec::new();
    for value in values {
        let value = value.trim().to_string();
        if value.is_empty() {
            continue;
        }
        if value.len() > MAX_NAME_BYTES || value.chars().any(char::is_control) {
            return Err(CommandError::new(
                "notifications",
                "Notification names must be at most 200 visible bytes",
                false,
            ));
        }
        if seen.insert(value.clone()) {
            subscriptions.push(value);
        }
    }
    if subscriptions.is_empty() {
        return Err(CommandError::new(
            "notifications",
            "Select at least one notification before listening",
            false,
        ));
    }
    if subscriptions.len() > MAX_SUBSCRIPTIONS {
        return Err(CommandError::new(
            "notifications",
            "Listen to at most 32 notification names at once",
            false,
        ));
    }
    Ok(subscriptions)
}

async fn context(
    app: &AppHandle,
    state: &AppState,
    override_udid: Option<String>,
) -> CommandResult<NotificationContext> {
    let udid = state
        .selected(override_udid)
        .await
        .ok_or_else(|| CommandError::new("device", "No device selected", true))?;
    let catalog = state.discovery.read().await;
    Ok(NotificationContext {
        pairing_path: remote_pairing_path(app, &udid)?,
        lockdown_target: catalog.lockdown_target(&udid),
        remote_target: catalog.remote_pairing_target(&udid),
        udid,
    })
}

async fn open_core_device_proxy(
    provider: &RoutedProvider,
) -> CommandResult<(AdapterHandle, RsdHandshake)> {
    let proxy = CoreDeviceProxy::connect(provider)
        .await
        .map_err(CommandError::from)?;
    let rsd_port = proxy.tunnel_info().server_rsd_port;
    let mut adapter = proxy
        .create_software_tunnel()
        .map_err(|error| CommandError::new("notifications", error.to_string(), true))?
        .to_async_handle();
    let stream = adapter
        .connect(rsd_port)
        .await
        .map_err(|error| CommandError::new("notifications", error.to_string(), true))?;
    let handshake = RsdHandshake::new(stream)
        .await
        .map_err(CommandError::from)?;
    Ok((adapter, handshake))
}

async fn connect(context: &NotificationContext) -> CommandResult<NotificationConnection> {
    let provider = routed_provider_for(&context.udid, context.lockdown_target.as_ref()).await?;
    let generation = ios_version(&provider).await?.developer_generation();
    match notification_transport(provider.is_bonjour(), generation) {
        NotificationTransport::Lockdown => Ok(NotificationConnection {
            client: NotificationProxyClient::connect(&provider)
                .await
                .map_err(CommandError::from)?,
            transport: "Notification Proxy · Lockdown".into(),
            _adapter: None,
        }),
        NotificationTransport::UsbRequired => Err(CommandError::new(
            "notifications",
            "Notification observation over the network requires iOS 17 or later. Connect this device by USB.",
            false,
        )),
        NotificationTransport::RemoteRsd | NotificationTransport::CoreDeviceRsd => {
            let (route, mut adapter, mut handshake) = match generation {
                DeveloperGeneration::CoreDeviceRemote => {
                    let tunnel = open_remote_pairing_tunnel(
                        &provider,
                        &context.pairing_path,
                        "idevice-desktop",
                        context.remote_target.as_ref(),
                    )
                    .await?;
                    ("RemotePairing/RSD", tunnel.adapter, tunnel.handshake)
                }
                DeveloperGeneration::CoreDeviceLockdown => {
                    let (adapter, handshake) = open_core_device_proxy(&provider).await?;
                    ("CoreDeviceProxy/RSD", adapter, handshake)
                }
                DeveloperGeneration::Legacy => unreachable!(),
            };
            let client = NotificationProxyClient::connect_rsd(&mut adapter, &mut handshake)
                .await
                .map_err(|error| {
                    CommandError::new(
                        "notifications",
                        format!("The network notification proxy is unavailable: {error}"),
                        true,
                    )
                })?;
            Ok(NotificationConnection {
                client,
                transport: format!("Notification Proxy shim · {route}"),
                _adapter: Some(adapter),
            })
        }
    }
}

fn emit_status(
    app: &AppHandle,
    session_id: &str,
    state: &str,
    message: Option<String>,
    transport: Option<String>,
    subscriptions: &[String],
) {
    let _ = app.emit(
        "notifications://status",
        NotificationObservationStatus {
            session_id: session_id.into(),
            state: state.into(),
            message,
            transport,
            subscriptions: subscriptions.to_vec(),
        },
    );
}

async fn run_observation(
    app: AppHandle,
    context: NotificationContext,
    session_id: String,
    subscriptions: Vec<String>,
    token: CancellationToken,
) -> CommandResult<()> {
    let mut connection = tokio::select! {
        _ = token.cancelled() => return Ok(()),
        result = tokio::time::timeout(CONNECT_TIMEOUT, connect(&context)) => result
            .map_err(|_| CommandError::new(
                "notifications",
                "Timed out opening the notification proxy. Keep the device unlocked and retry.",
                true,
            ))??,
    };

    for name in &subscriptions {
        tokio::select! {
            _ = token.cancelled() => return Ok(()),
            result = tokio::time::timeout(
                CONNECT_TIMEOUT,
                connection.client.observe_notification(name.clone()),
            ) => result
                .map_err(|_| CommandError::new(
                    "notifications",
                    format!("Timed out subscribing to {name}"),
                    true,
                ))?
                .map_err(CommandError::from)?,
        }
    }

    emit_status(
        &app,
        &session_id,
        "running",
        None,
        Some(connection.transport.clone()),
        &subscriptions,
    );
    let mut sequence = 0u64;
    loop {
        tokio::select! {
            _ = token.cancelled() => {
                emit_status(
                    &app,
                    &session_id,
                    "stopped",
                    None,
                    Some(connection.transport.clone()),
                    &subscriptions,
                );
                return Ok(());
            }
            result = connection.client.receive_notification() => {
                let name = result.map_err(CommandError::from)?;
                sequence = sequence.saturating_add(1);
                let _ = app.emit("notifications://event", NotificationObservationEvent {
                    session_id: session_id.clone(),
                    sequence,
                    timestamp_ms: chrono::Utc::now().timestamp_millis().max(0) as u64,
                    name,
                });
            }
        }
    }
}

#[tauri::command]
pub async fn notification_observation_start(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
    session_id: String,
    subscriptions: Vec<String>,
) -> CommandResult<()> {
    let session_id = validate_session_id(session_id)?;
    let subscriptions = validate_subscriptions(subscriptions)?;
    let context = context(&app, &state, udid).await?;
    let token = CancellationToken::new();
    state.replace_task(TASK_KEY, token.clone()).await;
    emit_status(&app, &session_id, "connecting", None, None, &subscriptions);

    tauri::async_runtime::spawn(async move {
        let completion_token = token.clone();
        if let Err(error) = run_observation(
            app.clone(),
            context,
            session_id.clone(),
            subscriptions.clone(),
            token,
        )
        .await
            && !completion_token.is_cancelled()
        {
            emit_status(
                &app,
                &session_id,
                "error",
                Some(error.message),
                None,
                &subscriptions,
            );
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn notification_observation_stop(state: State<'_, AppState>) -> CommandResult<()> {
    state.cancel_task(TASK_KEY).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_deduplicates_and_preserves_subscription_order() {
        let values = validate_subscriptions(vec![
            "  com.apple.first  ".into(),
            "com.apple.second".into(),
            "com.apple.first".into(),
            "".into(),
        ])
        .unwrap();
        assert_eq!(values, ["com.apple.first", "com.apple.second"]);
    }

    #[test]
    fn requires_an_explicit_subscription() {
        let error = validate_subscriptions(vec!["  ".into()]).unwrap_err();
        assert_eq!(error.kind, "notifications");
        assert!(error.message.contains("at least one"));
    }

    #[test]
    fn bounds_subscription_names_and_count() {
        assert!(validate_subscriptions(vec!["x".repeat(MAX_NAME_BYTES + 1)]).is_err());
        let too_many = (0..=MAX_SUBSCRIPTIONS)
            .map(|index| format!("com.apple.{index}"))
            .collect();
        assert!(validate_subscriptions(too_many).is_err());
    }

    #[test]
    fn rejects_control_characters_in_names_and_session_ids() {
        assert!(validate_subscriptions(vec!["com.apple.\ninvalid".into()]).is_err());
        assert!(validate_session_id("session\u{0000}".into()).is_err());
    }

    #[test]
    fn selects_generation_aware_notification_transports() {
        assert_eq!(
            notification_transport(false, DeveloperGeneration::Legacy),
            NotificationTransport::Lockdown,
        );
        assert_eq!(
            notification_transport(true, DeveloperGeneration::Legacy),
            NotificationTransport::UsbRequired,
        );
        assert_eq!(
            notification_transport(true, DeveloperGeneration::CoreDeviceRemote),
            NotificationTransport::RemoteRsd,
        );
        assert_eq!(
            notification_transport(true, DeveloperGeneration::CoreDeviceLockdown),
            NotificationTransport::CoreDeviceRsd,
        );
    }
}
