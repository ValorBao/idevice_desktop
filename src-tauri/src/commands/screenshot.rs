use base64::{Engine as _, engine::general_purpose::STANDARD};
use idevice::{
    IdeviceService, RsdService,
    core_device_proxy::CoreDeviceProxy,
    dvt::{remote_server::RemoteServerClient, screenshot::ScreenshotClient},
    rsd::RsdHandshake,
    screenshotr::ScreenshotService,
};
use tauri::{AppHandle, State};

use crate::{
    device_version::{DeveloperGeneration, ios_version},
    error::{CommandError, CommandResult},
    provider::provider_for,
    state::AppState,
    tunnel::{open_remote_pairing_tunnel, remote_pairing_path},
};

#[tauri::command]
pub async fn device_screenshot(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<String> {
    let udid = state
        .selected(udid)
        .await
        .ok_or_else(|| CommandError::new("device", "No device selected", true))?;
    let pairing_path = remote_pairing_path(&app, &udid)?;
    tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| CommandError::new("runtime", error.to_string(), true))?;
        runtime.block_on(capture_screenshot(udid, pairing_path))
    })
    .await
    .map_err(|error| CommandError::new("runtime", error.to_string(), true))?
}

async fn capture_screenshot(
    udid: String,
    pairing_path: std::path::PathBuf,
) -> CommandResult<String> {
    let provider = provider_for(&udid).await?;
    let generation = ios_version(&provider).await?.developer_generation();

    let bytes = match generation {
        DeveloperGeneration::Legacy => {
            let mut client = ScreenshotService::connect(&provider).await.map_err(|error| {
                CommandError::new(
                    "screenshot",
                    format!("Legacy screenshot service unavailable. Mount the matching DeveloperDiskImage first: {error}"),
                    true,
                )
            })?;
            client.take_screenshot().await.map_err(CommandError::from)?
        }
        DeveloperGeneration::CoreDeviceRemote | DeveloperGeneration::CoreDeviceLockdown => {
            let (mut adapter, mut handshake) = match generation {
                DeveloperGeneration::CoreDeviceRemote => {
                    let tunnel =
                        open_remote_pairing_tunnel(&provider, &pairing_path, "idevice-desktop")
                            .await?;
                    (tunnel.adapter, tunnel.handshake)
                }
                DeveloperGeneration::CoreDeviceLockdown => {
                    let proxy = CoreDeviceProxy::connect(&provider)
                        .await
                        .map_err(CommandError::from)?;
                    let rsd_port = proxy.tunnel_info().server_rsd_port;
                    let adapter = proxy.create_software_tunnel().map_err(|error| {
                        CommandError::new(
                            "screenshot",
                            format!("Unable to create device tunnel: {error}"),
                            true,
                        )
                    })?;
                    let mut adapter = adapter.to_async_handle();
                    let stream = adapter.connect(rsd_port).await.map_err(|error| {
                        CommandError::new(
                            "screenshot",
                            format!("Unable to connect to RSD: {error}"),
                            true,
                        )
                    })?;
                    let handshake = RsdHandshake::new(stream)
                        .await
                        .map_err(CommandError::from)?;
                    (adapter, handshake)
                }
                DeveloperGeneration::Legacy => unreachable!(),
            };
            let mut remote = RemoteServerClient::connect_rsd(&mut adapter, &mut handshake)
                .await
                .map_err(|error| {
                    CommandError::new(
                        "screenshot",
                        format!("DVT screenshot service unavailable. Mount the Developer Disk Image first: {error}"),
                        true,
                    )
                })?;
            remote.read_message(0).await.map_err(CommandError::from)?;
            let mut client = ScreenshotClient::new(&mut remote)
                .await
                .map_err(CommandError::from)?;
            client.take_screenshot().await.map_err(CommandError::from)?
        }
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
