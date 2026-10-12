use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{Mutex, oneshot};
use tokio_util::sync::CancellationToken;

use crate::{
    error::{CommandError, CommandResult},
    screen_capture::{self, CaptureError, RecorderMessage},
    state::AppState,
    types::{
        ScreenRecording, ScreenRecordingExportProgress, ScreenRecordingSegment,
        ScreenRecordingStatus,
    },
};

const RECORDING_TASK: &str = "screen-recording";
const RECORDINGS_DIRECTORY: &str = "screen-recordings";

struct ActiveRecording {
    stop: mpsc::Sender<RecorderMessage>,
    done: oneshot::Receiver<CommandResult<ScreenRecording>>,
}

/// One recording at a time. The finished recording stays in the app cache as
/// the editor's working copy until it is discarded or a new recording starts.
#[derive(Default)]
pub struct ScreenRecordingState {
    active: Mutex<Option<ActiveRecording>>,
    current: Mutex<Option<ScreenRecording>>,
    export: Mutex<Option<(uuid::Uuid, CancellationToken)>>,
}

struct StatusEmitter {
    app: AppHandle,
    session_id: String,
}

impl StatusEmitter {
    fn emit(
        &self,
        state: &str,
        message: Option<String>,
        device_name: Option<String>,
        recording: Option<ScreenRecording>,
    ) {
        let _ = self.app.emit(
            "screen-recording://status",
            ScreenRecordingStatus {
                session_id: self.session_id.clone(),
                state: state.into(),
                message,
                device_name,
                recording,
            },
        );
    }
}

fn recordings_directory(app: &AppHandle) -> CommandResult<PathBuf> {
    let directory = app
        .path()
        .app_cache_dir()
        .map_err(|error| CommandError::new("screen_recording", error.to_string(), false))?
        .join(RECORDINGS_DIRECTORY);
    std::fs::create_dir_all(&directory)?;
    Ok(directory)
}

/// Removes every working copy. Only the current recording is ever kept, and a
/// crash can leave earlier ones behind.
fn clear_working_copies(directory: &Path) {
    if let Ok(entries) = std::fs::read_dir(directory) {
        for entry in entries.flatten() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Session IDs become file names, so only a UUID's alphabet is accepted.
fn valid_session_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

fn capture_error(error: CaptureError) -> CommandError {
    match error {
        CaptureError::Cancelled => CommandError::new("cancelled", "Recording cancelled", false),
        CaptureError::Failed(message) => CommandError::new("screen_recording", message, true),
    }
}

fn export_destination(value: &str) -> CommandResult<PathBuf> {
    let path = PathBuf::from(value);
    let is_mp4 = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("mp4"));
    if !path.is_absolute() || path.file_name().is_none() || !is_mp4 {
        return Err(CommandError::new(
            "screen_recording",
            "Choose an absolute destination ending in .mp4",
            false,
        ));
    }
    Ok(path)
}

#[tauri::command]
pub async fn screen_recording_start(
    app: AppHandle,
    app_state: State<'_, AppState>,
    state: State<'_, ScreenRecordingState>,
    session_id: String,
    device_name: Option<String>,
) -> CommandResult<()> {
    if !valid_session_id(&session_id) {
        return Err(CommandError::new(
            "screen_recording",
            "Invalid recording session",
            false,
        ));
    }
    let directory = recordings_directory(&app)?;
    let token = CancellationToken::new();
    app_state.replace_task(RECORDING_TASK, token.clone()).await;
    clear_working_copies(&directory);
    *state.current.lock().await = None;

    let path = directory.join(format!("{session_id}.mov"));
    let (sender, receiver) = mpsc::channel();
    let (done_sender, done) = oneshot::channel();
    *state.active.lock().await = Some(ActiveRecording {
        stop: sender.clone(),
        done,
    });
    let status = StatusEmitter {
        app: app.clone(),
        session_id: session_id.clone(),
    };
    status.emit("connecting", None, None, None);

    std::thread::spawn(move || {
        let started = |device: &str| status.emit("recording", None, Some(device.into()), None);
        let result = screen_capture::record(
            device_name.as_deref(),
            &path,
            sender,
            &receiver,
            &started,
            &|| token.is_cancelled(),
        )
        .map(|movie| ScreenRecording {
            id: session_id,
            path: path.to_string_lossy().into_owned(),
            device_name: movie.device_name,
            duration_ms: movie.duration_ms,
            width: movie.width,
            height: movie.height,
            bytes: movie.bytes,
        });

        match &result {
            Ok(recording) => {
                *app.state::<ScreenRecordingState>().current.blocking_lock() =
                    Some(recording.clone());
                status.emit(
                    "finished",
                    None,
                    Some(recording.device_name.clone()),
                    Some(recording.clone()),
                );
            }
            Err(CaptureError::Cancelled) => status.emit("cancelled", None, None, None),
            Err(CaptureError::Failed(message)) => {
                status.emit("error", Some(message.clone()), None, None);
            }
        }
        let _ = done_sender.send(result.map_err(capture_error));
    });
    Ok(())
}

/// Finishes the recording and keeps it as the working copy.
#[tauri::command]
pub async fn screen_recording_stop(
    app_state: State<'_, AppState>,
    state: State<'_, ScreenRecordingState>,
) -> CommandResult<ScreenRecording> {
    let active = state.active.lock().await.take().ok_or_else(|| {
        CommandError::new("screen_recording", "No recording is in progress", false)
    })?;
    let _ = active.stop.send(RecorderMessage::Stop);
    let result = active.done.await.unwrap_or_else(|_| {
        Err(CommandError::new(
            "screen_recording",
            "The recording stopped unexpectedly",
            true,
        ))
    });
    app_state.cancel_task(RECORDING_TASK).await;
    result
}

/// Abandons a recording that is starting or running and deletes it.
#[tauri::command]
pub async fn screen_recording_cancel(
    app_state: State<'_, AppState>,
    state: State<'_, ScreenRecordingState>,
) -> CommandResult<()> {
    state.active.lock().await.take();
    app_state.cancel_task(RECORDING_TASK).await;
    Ok(())
}

/// The newest `.mov` left in the cache. Restarting the app drops the in-memory
/// handle, but the file is still the recording the editor should reopen.
fn newest_working_copy(directory: &Path) -> Option<PathBuf> {
    let mut copies = std::fs::read_dir(directory)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("mov"))
        })
        .collect::<Vec<_>>();
    copies.sort_by_key(|path| {
        std::fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok()
    });
    copies.pop()
}

fn recover_working_copy(path: &Path) -> Option<ScreenRecording> {
    let movie = screen_capture::inspect(path).ok()?;
    Some(ScreenRecording {
        id: path.file_stem()?.to_str()?.to_string(),
        path: path.to_string_lossy().into_owned(),
        device_name: movie.device_name,
        duration_ms: movie.duration_ms,
        width: movie.width,
        height: movie.height,
        bytes: movie.bytes,
    })
}

#[tauri::command]
pub async fn screen_recording_current(
    app: AppHandle,
    state: State<'_, ScreenRecordingState>,
) -> CommandResult<Option<ScreenRecording>> {
    if let Some(current) = state.current.lock().await.clone() {
        return Ok(Some(current));
    }
    let Some(path) = newest_working_copy(&recordings_directory(&app)?) else {
        return Ok(None);
    };
    let recovered = tauri::async_runtime::spawn_blocking(move || recover_working_copy(&path))
        .await
        .map_err(|error| CommandError::new("screen_recording", error.to_string(), true))?;
    if let Some(recording) = &recovered {
        *state.current.lock().await = Some(recording.clone());
    }
    Ok(recovered)
}

#[tauri::command]
pub async fn screen_recording_discard(state: State<'_, ScreenRecordingState>) -> CommandResult<()> {
    if let Some(recording) = state.current.lock().await.take() {
        let _ = std::fs::remove_file(recording.path);
    }
    Ok(())
}

#[tauri::command]
pub async fn screen_recording_export(
    app: AppHandle,
    state: State<'_, ScreenRecordingState>,
    destination: String,
    segments: Vec<ScreenRecordingSegment>,
) -> CommandResult<()> {
    let destination = export_destination(&destination)?;
    let recording = state.current.lock().await.clone().ok_or_else(|| {
        CommandError::new(
            "screen_recording",
            "Record something before exporting",
            false,
        )
    })?;
    let segments: Vec<(u64, u64)> = segments
        .iter()
        .map(|segment| (segment.start_ms, segment.end_ms))
        .collect();
    screen_capture::validate_segments(recording.duration_ms, &segments)
        .map_err(|message| CommandError::new("screen_recording", message, false))?;

    let token = CancellationToken::new();
    let export_id = uuid::Uuid::new_v4();
    if let Some((_, previous)) = state
        .export
        .lock()
        .await
        .replace((export_id, token.clone()))
    {
        previous.cancel();
    }
    let source = PathBuf::from(recording.path);
    let progress_app = app.clone();
    let export_token = token.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        screen_capture::export(
            &source,
            &destination,
            &segments,
            &|progress| {
                let _ = progress_app.emit(
                    "screen-recording://export-progress",
                    ScreenRecordingExportProgress { progress },
                );
            },
            &|| export_token.is_cancelled(),
        )
    })
    .await
    .map_err(|error| CommandError::new("screen_recording", error.to_string(), true))?;

    let mut export = state.export.lock().await;
    if export
        .as_ref()
        .is_some_and(|(current, _)| *current == export_id)
    {
        *export = None;
    }
    result.map_err(capture_error)
}

#[tauri::command]
pub async fn screen_recording_export_cancel(
    state: State<'_, ScreenRecordingState>,
) -> CommandResult<()> {
    if let Some((_, token)) = state.export.lock().await.take() {
        token.cancel();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_require_an_absolute_mp4_path() {
        assert!(export_destination("").is_err());
        assert!(export_destination("clip.mp4").is_err());
        assert!(export_destination("/tmp/clip.mov").is_err());
        assert_eq!(
            export_destination("/tmp/clip.MP4").unwrap(),
            PathBuf::from("/tmp/clip.MP4")
        );
    }

    #[test]
    fn session_ids_cannot_escape_the_recordings_directory() {
        assert!(valid_session_id("3f2b8c1e-9a4d-4e6f-8b7a-1c2d3e4f5a6b"));
        assert!(!valid_session_id(""));
        assert!(!valid_session_id("../escape"));
        assert!(!valid_session_id("a/b"));
        assert!(!valid_session_id(&"a".repeat(65)));
    }

    #[test]
    fn the_newest_working_copy_is_reopened() {
        let directory = std::env::temp_dir().join(format!("recordings-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let older = directory.join("older.mov");
        let newer = directory.join("newer.mov");
        std::fs::write(&older, b"x").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&newer, b"y").unwrap();
        assert_eq!(newest_working_copy(&directory).as_ref(), Some(&newer));
        assert!(newest_working_copy(&directory.join("missing")).is_none());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn clearing_working_copies_empties_the_directory() {
        let directory = std::env::temp_dir().join(format!("recordings-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("old.mov"), b"x").unwrap();
        clear_working_copies(&directory);
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
        std::fs::remove_dir(directory).unwrap();
    }
}
