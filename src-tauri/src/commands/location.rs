use std::time::Duration;

use idevice::{
    IdeviceService, RsdService,
    core_device_proxy::CoreDeviceProxy,
    dvt::{location_simulation::LocationSimulationClient, remote_server::RemoteServerClient},
    rsd::RsdHandshake,
    services::simulate_location::LocationSimulationService,
};
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use crate::{
    device_version::{DeveloperGeneration, ios_version},
    error::{CommandError, CommandResult},
    state::AppState,
    tunnel::{open_remote_pairing_tunnel, remote_pairing_path},
    types::{LocationSession, StreamStatus},
};

#[tauri::command]
pub async fn location_start(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
    latitude: f64,
    longitude: f64,
) -> CommandResult<LocationSession> {
    if !latitude.is_finite() || !(-90.0..=90.0).contains(&latitude) {
        return Err(CommandError::new(
            "location",
            "Latitude must be between -90 and 90",
            false,
        ));
    }
    if !longitude.is_finite() || !(-180.0..=180.0).contains(&longitude) {
        return Err(CommandError::new(
            "location",
            "Longitude must be between -180 and 180",
            false,
        ));
    }

    let udid = state
        .selected(udid)
        .await
        .ok_or_else(|| CommandError::new("device", "No device selected", true))?;
    let remote_target = state.discovery.read().await.remote_pairing_target(&udid);
    let lockdown_target = state.discovery.read().await.lockdown_target(&udid);
    let pairing_path = remote_pairing_path(&app, &udid)?;
    let token = CancellationToken::new();
    state.replace_task("location", token.clone()).await;
    let (sender, receiver) = tokio::sync::oneshot::channel();

    std::thread::Builder::new()
        .name("idevice-location".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = sender.send(Err(CommandError::new(
                        "runtime",
                        error.to_string(),
                        true,
                    )));
                    return;
                }
            };

            runtime.block_on(async move {
                let mut sender = Some(sender);
                let result: CommandResult<()> = async {
                    let provider =
                        crate::provider::routed_provider_for(&udid, lockdown_target.as_ref())
                            .await?;
                    let generation = ios_version(&provider).await?.developer_generation();

                    if generation != DeveloperGeneration::Legacy {
                        let (mut adapter, mut handshake) = match generation {
                            DeveloperGeneration::CoreDeviceRemote => {
                                let tunnel = open_remote_pairing_tunnel(
                                    &provider,
                                    &pairing_path,
                                    "idevice-desktop",
                                    remote_target.as_ref(),
                                )
                                .await?;
                                (tunnel.adapter, tunnel.handshake)
                            }
                            DeveloperGeneration::CoreDeviceLockdown => {
                                let proxy = CoreDeviceProxy::connect(&provider)
                                    .await
                                    .map_err(CommandError::from)?;
                                let rsd_port = proxy.tunnel_info().server_rsd_port;
                                let adapter = proxy.create_software_tunnel().map_err(|error| {
                                    CommandError::new("tunnel", error.to_string(), true)
                                })?;
                                let mut adapter = adapter.to_async_handle();
                                let stream = adapter.connect(rsd_port).await.map_err(|error| {
                                    CommandError::new("tunnel", error.to_string(), true)
                                })?;
                                let handshake = RsdHandshake::new(stream)
                                    .await
                                    .map_err(CommandError::from)?;
                                (adapter, handshake)
                            }
                            DeveloperGeneration::Legacy => unreachable!(),
                        };
                        let mut remote_server =
                            RemoteServerClient::connect_rsd(&mut adapter, &mut handshake)
                                .await
                                .map_err(CommandError::from)?;
                        remote_server
                            .read_message(0)
                            .await
                            .map_err(CommandError::from)?;
                        let mut client = LocationSimulationClient::new(&mut remote_server)
                            .await
                            .map_err(CommandError::from)?;
                        client
                            .set(latitude, longitude)
                            .await
                            .map_err(CommandError::from)?;
                        if let Some(sender) = sender.take() {
                            let _ = sender.send(Ok(LocationSession {
                                latitude,
                                longitude,
                                transport: "DVT/RSD".into(),
                            }));
                        }
                        let _ = app.emit(
                            "location://status",
                            StreamStatus {
                                stream: "location".into(),
                                state: "active".into(),
                                message: Some(format!("{latitude:.6}, {longitude:.6}")),
                            },
                        );

                        loop {
                            tokio::select! {
                                _ = token.cancelled() => break,
                                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                                    client.set(latitude, longitude).await.map_err(CommandError::from)?;
                                }
                            }
                        }
                        client.clear().await.map_err(CommandError::from)?;
                    } else {
                        let mut client = LocationSimulationService::connect(&provider)
                            .await
                            .map_err(CommandError::from)?;
                        client
                            .set(&latitude.to_string(), &longitude.to_string())
                            .await
                            .map_err(CommandError::from)?;
                        if let Some(sender) = sender.take() {
                            let _ = sender.send(Ok(LocationSession {
                                latitude,
                                longitude,
                                transport: "Lockdown".into(),
                            }));
                        }
                        let _ = app.emit(
                            "location://status",
                            StreamStatus {
                                stream: "location".into(),
                                state: "active".into(),
                                message: Some(format!("{latitude:.6}, {longitude:.6}")),
                            },
                        );
                        token.cancelled().await;
                        client.clear().await.map_err(CommandError::from)?;
                    }

                    let _ = app.emit(
                        "location://status",
                        StreamStatus {
                            stream: "location".into(),
                            state: "cleared".into(),
                            message: None,
                        },
                    );
                    Ok(())
                }
                .await;

                if let Err(error) = result {
                    if let Some(sender) = sender.take() {
                        let _ = sender.send(Err(error.clone()));
                    }
                    let _ = app.emit(
                        "location://status",
                        StreamStatus {
                            stream: "location".into(),
                            state: "error".into(),
                            message: Some(error.message),
                        },
                    );
                }
            });
        })
        .map_err(|error| CommandError::new("runtime", error.to_string(), true))?;

    receiver.await.map_err(|_| {
        CommandError::new(
            "location",
            "Location worker stopped before applying the coordinates",
            true,
        )
    })?
}

#[tauri::command]
pub async fn location_stop(state: State<'_, AppState>) -> CommandResult<()> {
    state.cancel_task("location").await;
    Ok(())
}
