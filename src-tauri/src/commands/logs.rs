use idevice::{
    IdeviceService,
    os_trace_relay::{LogLevel, OsTraceRelayClient},
};
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use crate::{
    error::{CommandError, CommandResult},
    provider::selected_provider,
    state::AppState,
    types::{DeviceLog, StreamStatus},
};

fn level_name(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Notice => "NOTICE",
        LogLevel::Info => "INFO",
        LogLevel::Debug => "DEBUG",
        LogLevel::Error => "ERROR",
        LogLevel::Fault => "FAULT",
    }
}

#[tauri::command]
pub async fn logs_start(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
    pid: Option<u32>,
) -> CommandResult<()> {
    let (_, provider) = selected_provider(&state, udid).await?;
    let client = OsTraceRelayClient::connect(&provider)
        .await
        .map_err(CommandError::from)?;
    let mut receiver = client.start_trace(pid).await.map_err(CommandError::from)?;
    let token = CancellationToken::new();
    state.replace_task("logs", token.clone()).await;
    let _ = app.emit(
        "logs://status",
        StreamStatus {
            stream: "logs".into(),
            state: "running".into(),
            message: None,
        },
    );

    tauri::async_runtime::spawn(async move {
        let mut consecutive_errors = 0u8;
        loop {
            tokio::select! {
                _ = token.cancelled() => break,
                result = receiver.next() => match result {
                    Ok(log) => {
                        consecutive_errors = 0;
                        let label = log.label;
                        let _ = app.emit("logs://line", DeviceLog {
                            timestamp: log.timestamp.format("%H:%M:%S%.3f").to_string(),
                            level: level_name(log.level).into(),
                            process: log.image_name,
                            pid: log.pid,
                            message: log.message,
                            subsystem: label.as_ref().map(|value| value.subsystem.clone()),
                            category: label.map(|value| value.category),
                        });
                    }
                    Err(error) => {
                        consecutive_errors = consecutive_errors.saturating_add(1);
                        if consecutive_errors >= 12 {
                            let _ = app.emit("logs://status", StreamStatus {
                                stream: "logs".into(),
                                state: "error".into(),
                                message: Some(error.to_string()),
                            });
                            break;
                        }
                    }
                }
            }
        }
        let _ = app.emit(
            "logs://status",
            StreamStatus {
                stream: "logs".into(),
                state: "stopped".into(),
                message: None,
            },
        );
    });
    Ok(())
}

#[tauri::command]
pub async fn logs_stop(state: State<'_, AppState>) -> CommandResult<()> {
    state.cancel_task("logs").await;
    Ok(())
}
