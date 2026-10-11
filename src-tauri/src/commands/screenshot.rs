use base64::{Engine as _, engine::general_purpose::STANDARD};
use idevice::{
    IdeviceService, RsdService,
    dvt::{remote_server::RemoteServerClient, screenshot::ScreenshotClient},
    screenshotr::ScreenshotService,
};
use tauri::{AppHandle, State};

use crate::{
    error::{CommandError, CommandResult},
    state::AppState,
    task::run_on_tunnel_runtime,
    transport::{DeviceContext, DeviceService, OpenOptions},
};

#[tauri::command]
pub async fn device_screenshot(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<String> {
    let context = DeviceContext::resolve(&app, &state, udid).await?;
    run_on_tunnel_runtime(move || capture_screenshot(context)).await
}

async fn capture_screenshot(context: DeviceContext) -> CommandResult<String> {
    let provider = context.provider().await?;
    let bytes = match context.open_service(&provider, &OpenOptions::new()).await? {
        DeviceService::Legacy { .. } => {
            let mut client = ScreenshotService::connect(&provider).await.map_err(|error| {
                CommandError::new(
                    "screenshot",
                    format!("Legacy screenshot service unavailable. Mount the matching DeveloperDiskImage first: {error}"),
                    true,
                )
            })?;
            client.take_screenshot().await.map_err(CommandError::from)?
        }
        DeviceService::Core { mut tunnel, .. } => {
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
