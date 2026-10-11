use std::{future::Future, path::PathBuf, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use idevice::{
    IdeviceService, ReadWrite, RsdService,
    dvt::{remote_server::RemoteServerClient, screenshot::ScreenshotClient},
    screenshotr::ScreenshotService,
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::{
    device_version::{DeveloperGeneration, ios_version},
    error::{CommandError, CommandResult},
    state::AppState,
    transport::{
        DeviceContext, ROUTE_CORE_DEVICE_PROXY, ROUTE_REMOTE_PAIRING,
        open_core_device_proxy_bounded,
    },
    types::{LiveScreenFrame, LiveScreenStatus},
};

const LIVE_SCREEN_TASK: &str = "live-screen";
const TARGET_FPS: u32 = 2;
const FRAME_INTERVAL: Duration = Duration::from_millis(1_000 / TARGET_FPS as u64);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(45);
const FRAME_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_FRAME_BYTES: usize = 12 * 1024 * 1024;
const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

#[derive(Clone)]
struct StoredFrame {
    bytes: Vec<u8>,
}

#[derive(Default)]
pub struct LiveScreenState {
    latest: Mutex<Option<StoredFrame>>,
}

trait FrameSource {
    async fn take_frame(&mut self) -> Result<Vec<u8>, idevice::IdeviceError>;
}

impl FrameSource for ScreenshotService {
    async fn take_frame(&mut self) -> Result<Vec<u8>, idevice::IdeviceError> {
        self.take_screenshot().await
    }
}

impl<R: ReadWrite> FrameSource for ScreenshotClient<'_, R> {
    async fn take_frame(&mut self) -> Result<Vec<u8>, idevice::IdeviceError> {
        self.take_screenshot().await
    }
}

fn emit_status(app: &AppHandle, state: &str, message: Option<String>, transport: Option<String>) {
    let _ = app.emit(
        "live-screen://status",
        LiveScreenStatus {
            state: state.into(),
            message,
            transport,
            target_fps: TARGET_FPS,
        },
    );
}

fn validate_png(bytes: &[u8]) -> CommandResult<(u32, u32)> {
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(CommandError::new(
            "live_screen",
            "The device returned a frame larger than the 12 MB safety limit",
            false,
        ));
    }
    if bytes.len() < 24
        || bytes.get(..8) != Some(PNG_SIGNATURE.as_slice())
        || bytes.get(8..12) != Some(13u32.to_be_bytes().as_slice())
        || bytes.get(12..16) != Some(b"IHDR".as_slice())
    {
        return Err(CommandError::new(
            "live_screen",
            "The device returned an invalid PNG frame",
            true,
        ));
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().expect("validated PNG width"));
    let height = u32::from_be_bytes(bytes[20..24].try_into().expect("validated PNG height"));
    if width == 0 || height == 0 {
        return Err(CommandError::new(
            "live_screen",
            "The device returned a PNG frame with no visible area",
            true,
        ));
    }
    Ok((width, height))
}

fn frame_destination(value: &str) -> CommandResult<PathBuf> {
    if value.trim().is_empty() {
        return Err(CommandError::new(
            "live_screen",
            "Choose a destination for the PNG frame",
            false,
        ));
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() || path.file_name().is_none() {
        return Err(CommandError::new(
            "live_screen",
            "Choose an absolute PNG destination",
            false,
        ));
    }
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    {
        return Err(CommandError::new(
            "live_screen",
            "The frame destination must end in .png",
            false,
        ));
    }
    Ok(path)
}

async fn wait_for<T, E, F>(
    label: &str,
    duration: Duration,
    token: &CancellationToken,
    future: F,
) -> CommandResult<T>
where
    F: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    tokio::select! {
        biased;
        _ = token.cancelled() => Err(CommandError::new(
            "cancelled",
            "Live Screen stopped",
            false,
        )),
        result = tokio::time::timeout(duration, future) => result
            .map_err(|_| CommandError::new(
                "live_screen",
                format!("Timed out while {label}. Keep the device unlocked and retry."),
                true,
            ))?
            .map_err(|error| CommandError::new("live_screen", error.to_string(), true)),
    }
}

async fn stream_frames<S: FrameSource>(
    app: &AppHandle,
    source: &mut S,
    transport: String,
    token: &CancellationToken,
) -> CommandResult<()> {
    emit_status(app, "running", None, Some(transport));
    let screen_state = app.state::<LiveScreenState>();
    let mut sequence = 0u64;
    let mut previous_frame_at = None;

    loop {
        let cycle_started = tokio::time::Instant::now();
        let bytes = wait_for(
            "waiting for a screen frame",
            FRAME_TIMEOUT,
            token,
            source.take_frame(),
        )
        .await?;
        if token.is_cancelled() {
            return Ok(());
        }
        let (width, height) = validate_png(&bytes)?;
        let frame_at = tokio::time::Instant::now();
        let fps = previous_frame_at
            .map(|previous: tokio::time::Instant| {
                let seconds = frame_at.duration_since(previous).as_secs_f64();
                if seconds > 0.0 { 1.0 / seconds } else { 0.0 }
            })
            .unwrap_or(0.0);
        previous_frame_at = Some(frame_at);
        sequence = sequence.saturating_add(1);
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        let frame = LiveScreenFrame {
            sequence,
            timestamp_ms,
            width,
            height,
            bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            fps,
            data_url: format!("data:image/png;base64,{}", STANDARD.encode(&bytes)),
        };
        *screen_state.latest.lock().await = Some(StoredFrame { bytes });
        let _ = app.emit("live-screen://frame", frame);

        let remaining = FRAME_INTERVAL.saturating_sub(cycle_started.elapsed());
        if !remaining.is_zero() {
            tokio::select! {
                biased;
                _ = token.cancelled() => return Ok(()),
                _ = tokio::time::sleep(remaining) => {}
            }
        }
    }
}

async fn run_stream(
    app: AppHandle,
    context: DeviceContext,
    token: CancellationToken,
) -> CommandResult<()> {
    let provider = wait_for(
        "opening the selected device",
        CONNECT_TIMEOUT,
        &token,
        context.provider(),
    )
    .await?;
    let version = wait_for(
        "reading the iOS version",
        CONNECT_TIMEOUT,
        &token,
        ios_version(&provider),
    )
    .await?;

    match version.developer_generation() {
        DeveloperGeneration::Legacy => {
            let mut source = wait_for(
                "opening the screenshot service",
                CONNECT_TIMEOUT,
                &token,
                ScreenshotService::connect(&provider),
            )
            .await
            .map_err(|error| {
                CommandError::new(
                    "live_screen",
                    format!(
                        "Legacy screenshot service unavailable. Mount the matching DeveloperDiskImage first: {}",
                        error.message
                    ),
                    true,
                )
            })?;
            stream_frames(&app, &mut source, "Screenshotr · Lockdown".into(), &token).await
        }
        DeveloperGeneration::CoreDeviceRemote | DeveloperGeneration::CoreDeviceLockdown => {
            let (route, mut tunnel) = match version.developer_generation() {
                DeveloperGeneration::CoreDeviceRemote => {
                    let tunnel = wait_for(
                        "opening the RemotePairing tunnel",
                        CONNECT_TIMEOUT,
                        &token,
                        context.open_remote_pairing_tunnel(&provider, 1),
                    )
                    .await?;
                    (ROUTE_REMOTE_PAIRING, tunnel)
                }
                DeveloperGeneration::CoreDeviceLockdown => (
                    ROUTE_CORE_DEVICE_PROXY,
                    open_core_device_proxy_bounded(&provider, &token, CONNECT_TIMEOUT).await?,
                ),
                DeveloperGeneration::Legacy => unreachable!(),
            };
            let mut remote = wait_for(
                "opening the DVT remote server",
                CONNECT_TIMEOUT,
                &token,
                RemoteServerClient::connect_rsd(&mut tunnel.adapter, &mut tunnel.handshake),
            )
            .await
            .map_err(|error| {
                CommandError::new(
                    "live_screen",
                    format!(
                        "DVT screenshot service unavailable. Mount the Developer Disk Image first: {}",
                        error.message
                    ),
                    true,
                )
            })?;
            wait_for(
                "reading the DVT handshake",
                CONNECT_TIMEOUT,
                &token,
                remote.read_message(0),
            )
            .await?;
            let mut source = wait_for(
                "opening the DVT screenshot channel",
                CONNECT_TIMEOUT,
                &token,
                ScreenshotClient::new(&mut remote),
            )
            .await?;
            stream_frames(
                &app,
                &mut source,
                format!("DVT Screenshot · {route}"),
                &token,
            )
            .await
        }
    }
}

#[tauri::command]
pub async fn live_screen_start(
    app: AppHandle,
    app_state: State<'_, AppState>,
    screen_state: State<'_, LiveScreenState>,
    udid: Option<String>,
) -> CommandResult<()> {
    let context = DeviceContext::resolve(&app, &app_state, udid).await?;
    let token = CancellationToken::new();
    app_state
        .replace_task(LIVE_SCREEN_TASK, token.clone())
        .await;
    *screen_state.latest.lock().await = None;
    emit_status(&app, "connecting", None, None);

    tauri::async_runtime::spawn_blocking(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                emit_status(&app, "error", Some(error.to_string()), None);
                return;
            }
        };
        let completion_token = token.clone();
        if let Err(error) = runtime.block_on(run_stream(app.clone(), context, token))
            && !completion_token.is_cancelled()
        {
            emit_status(&app, "error", Some(error.message), None);
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn live_screen_stop(app_state: State<'_, AppState>) -> CommandResult<()> {
    app_state.cancel_task(LIVE_SCREEN_TASK).await;
    Ok(())
}

#[tauri::command]
pub async fn live_screen_export_frame(
    local_path: String,
    screen_state: State<'_, LiveScreenState>,
) -> CommandResult<()> {
    let destination = frame_destination(&local_path)?;
    let bytes = screen_state
        .latest
        .lock()
        .await
        .as_ref()
        .map(|frame| frame.bytes.clone())
        .ok_or_else(|| {
            CommandError::new(
                "live_screen",
                "Wait for the first screen frame before saving",
                false,
            )
        })?;
    tokio::fs::write(destination, bytes).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::from(PNG_SIGNATURE.as_slice());
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes
    }

    #[test]
    fn reads_dimensions_from_a_png_ihdr() {
        assert_eq!(
            validate_png(&png_header(1_290, 2_796)).unwrap(),
            (1_290, 2_796)
        );
    }

    #[test]
    fn rejects_truncated_or_zero_sized_frames() {
        assert!(validate_png(&[]).is_err());
        assert!(validate_png(&png_header(0, 2_796)).is_err());
        assert!(validate_png(&png_header(1_290, 0)).is_err());
    }

    #[test]
    fn rejects_non_png_and_oversized_frames() {
        let mut invalid = png_header(10, 10);
        invalid[0] = 0;
        assert!(validate_png(&invalid).is_err());
        assert!(validate_png(&vec![0; MAX_FRAME_BYTES + 1]).is_err());
    }

    #[test]
    fn frame_exports_require_an_absolute_png_path() {
        assert!(frame_destination("").is_err());
        assert!(frame_destination("frame.png").is_err());
        assert!(frame_destination("/tmp/frame.jpg").is_err());
        assert_eq!(
            frame_destination("/tmp/frame.PNG").unwrap(),
            PathBuf::from("/tmp/frame.PNG")
        );
    }
}
