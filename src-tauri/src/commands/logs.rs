use std::time::Duration;

use idevice::{
    IdeviceService,
    os_trace_relay::{LogLevel, OsTraceRelayClient},
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

use crate::{
    error::{CommandError, CommandResult},
    provider::selected_provider,
    state::AppState,
    task::stage,
    types::{DeviceLog, LogStatus},
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
    udid: String,
    session_id: String,
    pid: Option<u32>,
) -> CommandResult<()> {
    uuid::Uuid::parse_str(&session_id)
        .map_err(|_| CommandError::new("logs", "Invalid log session ID", false))?;
    let token = CancellationToken::new();
    state
        .replace_session_task("logs:", &session_id, token.clone())
        .await;
    let key = format!("logs:{session_id}");
    let receiver = stage(&token, "Starting logs", Duration::from_secs(30), async {
        let (_, provider) = selected_provider(&state, Some(udid.clone())).await?;
        let client = OsTraceRelayClient::connect(&provider)
            .await
            .map_err(CommandError::from)?;
        client.start_trace(pid).await.map_err(CommandError::from)
    })
    .await;
    let mut receiver = match receiver {
        Ok(receiver) => receiver,
        Err(error) => {
            state.cancel_task(&key).await;
            return Err(error);
        }
    };
    let _ = app.emit(
        "logs://status",
        LogStatus {
            session_id: session_id.clone(),
            udid: udid.clone(),
            state: "running".into(),
            message: None,
        },
    );

    tauri::async_runtime::spawn(async move {
        let mut consecutive_errors = 0u8;
        let mut failure = None;
        loop {
            tokio::select! {
                biased;
                _ = token.cancelled() => break,
                result = receiver.next() => match result {
                    Ok(log) => {
                        consecutive_errors = 0;
                        let label = log.label;
                        let _ = app.emit("logs://line", DeviceLog {
                            session_id: session_id.clone(), udid: udid.clone(),
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
                            failure = Some(error.to_string());
                            break;
                        }
                    }
                }
            }
        }
        let _ = app.emit(
            "logs://status",
            LogStatus {
                session_id,
                udid,
                state: if failure.is_some() {
                    "error"
                } else {
                    "stopped"
                }
                .into(),
                message: failure,
            },
        );
        app.state::<AppState>().cancel_task(&key).await;
    });
    Ok(())
}

#[tauri::command]
pub async fn logs_stop(state: State<'_, AppState>, session_id: String) -> CommandResult<()> {
    state.cancel_task(&format!("logs:{session_id}")).await;
    Ok(())
}
