use base64::{Engine as _, engine::general_purpose::STANDARD};
use idevice::{
    IdeviceService, RsdService,
    dvt::{remote_server::RemoteServerClient, screenshot::ScreenshotClient},
    screenshotr::ScreenshotService,
};
use tauri::{AppHandle, State};

use crate::{
    device_version::{DeveloperGeneration, developer_generation},
    error::{CommandError, CommandResult},
    state::AppState,
    transport::DeviceContext,
};

#[tauri::command]
pub async fn device_screenshot(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<String> {
    let context = DeviceContext::resolve(&app, &state, udid).await?;
    tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| CommandError::new("runtime", error.to_string(), true))?;
        runtime.block_on(capture_screenshot(context))
    })
    .await
    .map_err(|error| CommandError::new("runtime", error.to_string(), true))?
}

async fn capture_screenshot(context: DeviceContext) -> CommandResult<String> {
    let provider = context.provider().await?;
    let generation = developer_generation(&provider).await?;

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
            let Some((_, mut tunnel)) = context.open_rsd_tunnel(&provider, generation, 1).await?
            else {
                unreachable!("only Legacy devices have no RSD tunnel");
            };
            let mut remote =
                RemoteServerClient::connect_rsd(&mut tunnel.adapter, &mut tunnel.handshake)
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
