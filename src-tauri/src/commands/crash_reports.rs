use std::path::Path;

use idevice::{
    IdeviceService,
    services::crashreportcopymobile::{CrashReportCopyMobileClient, flush_reports},
};
use tauri::State;

use crate::{
    error::{CommandError, CommandResult},
    provider::selected_provider,
    state::AppState,
    types::{CrashReportContent, CrashReportSummary},
};

const MAX_PREVIEW_BYTES: usize = 4 * 1024 * 1024;
const MAX_REPORTS: usize = 2_000;

fn validate_report_path(path: &str) -> CommandResult<String> {
    let normalized = path.trim_start_matches('/');
    if normalized.is_empty()
        || normalized.contains('\0')
        || normalized
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(CommandError::new(
            "crash_reports",
            "Invalid crash report path",
            false,
        ));
    }
    Ok(normalized.to_string())
}

fn report_process(name: &str) -> String {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(name);
    let timestamp_start = stem.find("-20").or_else(|| stem.find("_20"));
    timestamp_start
        .map(|index| &stem[..index])
        .unwrap_or(stem)
        .to_string()
}

fn report_kind(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_uppercase())
        .unwrap_or_else(|| "REPORT".to_string())
}

fn looks_like_report(name: &str) -> bool {
    matches!(
        Path::new(name)
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .as_deref(),
        Some("ips" | "crash" | "panic" | "log" | "synced")
    )
}

fn report_modified(name: &str) -> String {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(name);
    let parts = stem.split('-').collect::<Vec<_>>();
    for window in parts.windows(4).rev() {
        let year = window[0];
        let month = window[1];
        let day = window[2];
        let time = window[3].get(..6).unwrap_or("");
        if year.len() == 4
            && year.starts_with("20")
            && month.len() == 2
            && day.len() == 2
            && time.len() == 6
            && year.chars().all(|value| value.is_ascii_digit())
            && month.chars().all(|value| value.is_ascii_digit())
            && day.chars().all(|value| value.is_ascii_digit())
            && time.chars().all(|value| value.is_ascii_digit())
        {
            return format!(
                "{year}-{month}-{day} {}:{}:{}",
                &time[0..2],
                &time[2..4],
                &time[4..6]
            );
        }
    }
    String::new()
}

async fn crash_client(
    state: &AppState,
    udid: Option<String>,
) -> CommandResult<CrashReportCopyMobileClient> {
    let (_, provider) = selected_provider(state, udid).await?;
    CrashReportCopyMobileClient::connect(&provider)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn crash_reports_list(
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<Vec<CrashReportSummary>> {
    let (_, provider) = selected_provider(&state, udid).await?;
    if let Err(error) = flush_reports(&provider).await {
        tracing::warn!(?error, "unable to flush pending crash reports");
    }

    let mut client = match CrashReportCopyMobileClient::connect(&provider).await {
        Ok(client) => client,
        Err(error) => {
            tracing::error!(?error, "unable to connect to crash report service");
            return Err(CommandError::from(error));
        }
    };
    let mut directories = vec!["/".to_string()];
    let mut reports = Vec::new();

    while let Some(directory) = directories.pop() {
        let names = client
            .ls(Some(&directory))
            .await
            .map_err(CommandError::from)?;
        for name in names {
            if name == "." || name == ".." || name.contains('/') {
                continue;
            }
            let path = if directory == "/" {
                format!("/{name}")
            } else {
                format!("{}/{name}", directory.trim_end_matches('/'))
            };
            if looks_like_report(&name) {
                reports.push(CrashReportSummary {
                    process: report_process(&name),
                    kind: report_kind(&name),
                    modified: report_modified(&name),
                    name,
                    path,
                    size_bytes: None,
                });
                if reports.len() >= MAX_REPORTS {
                    break;
                }
                continue;
            }
            let info = match client.afc_client.get_file_info(&path).await {
                Ok(info) => info,
                Err(error) => {
                    tracing::warn!(%path, ?error, "unable to inspect crash report entry");
                    continue;
                }
            };
            if info.st_ifmt == "S_IFDIR" {
                directories.push(path);
                continue;
            }
            if info.st_ifmt != "S_IFREG" {
                continue;
            }
            reports.push(CrashReportSummary {
                process: report_process(&name),
                kind: report_kind(&name),
                modified: info.modified.format("%Y-%m-%d %H:%M:%S").to_string(),
                name,
                path,
                size_bytes: Some(info.size as u64),
            });
            if reports.len() >= MAX_REPORTS {
                break;
            }
        }
        if reports.len() >= MAX_REPORTS {
            break;
        }
    }

    reports.sort_by(|left, right| {
        right
            .modified
            .cmp(&left.modified)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    tracing::info!(report_count = reports.len(), "loaded crash reports");
    Ok(reports)
}

#[tauri::command]
pub async fn crash_report_read(
    state: State<'_, AppState>,
    udid: Option<String>,
    path: String,
) -> CommandResult<CrashReportContent> {
    let normalized = validate_report_path(&path)?;
    let mut client = crash_client(&state, udid).await?;
    let bytes = client.pull(normalized).await.map_err(CommandError::from)?;
    let truncated = bytes.len() > MAX_PREVIEW_BYTES;
    let preview_len = bytes.len().min(MAX_PREVIEW_BYTES);
    tracing::info!(
        size_bytes = bytes.len(),
        truncated,
        "loaded crash report preview"
    );
    Ok(CrashReportContent {
        path,
        content: String::from_utf8_lossy(&bytes[..preview_len]).into_owned(),
        truncated,
        size_bytes: bytes.len() as u64,
    })
}

#[tauri::command]
pub async fn crash_report_export(
    state: State<'_, AppState>,
    udid: Option<String>,
    path: String,
    local_path: String,
) -> CommandResult<()> {
    let normalized = validate_report_path(&path)?;
    if local_path.trim().is_empty() {
        return Err(CommandError::new(
            "crash_reports",
            "No export destination selected",
            false,
        ));
    }
    let mut client = crash_client(&state, udid).await?;
    let bytes = client.pull(normalized).await.map_err(CommandError::from)?;
    tokio::fs::write(local_path, bytes).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_report_paths() {
        assert_eq!(
            validate_report_path("/DiagnosticLogs/App.ips").unwrap(),
            "DiagnosticLogs/App.ips"
        );
        assert!(validate_report_path("../App.ips").is_err());
        assert!(validate_report_path("/DiagnosticLogs//App.ips").is_err());
        assert!(validate_report_path("/").is_err());
    }

    #[test]
    fn derives_report_metadata_from_filename() {
        assert_eq!(
            report_process("ExampleApp-2026-07-24-120000.ips"),
            "ExampleApp"
        );
        assert_eq!(report_process("JetsamEvent_2026-07-24.ips"), "JetsamEvent");
        assert_eq!(report_kind("ExampleApp.crash"), "CRASH");
        assert!(looks_like_report("ExampleApp.ips"));
        assert!(!looks_like_report("DiagnosticLogs"));
        assert_eq!(
            report_modified("ExampleApp-2026-07-24-120000.ips"),
            "2026-07-24 12:00:00"
        );
    }
}
