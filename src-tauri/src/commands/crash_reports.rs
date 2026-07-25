use std::{
    ops::{Deref, DerefMut},
    path::Path,
};

use idevice::{
    IdeviceService, RsdService,
    core_device_proxy::CoreDeviceProxy,
    rsd::RsdHandshake,
    services::crashreportcopymobile::{CrashReportCopyMobileClient, flush_reports},
    tcp::handle::AdapterHandle,
};
use tauri::{AppHandle, State};

use crate::{
    device_version::{DeveloperGeneration, ios_version},
    error::{CommandError, CommandResult},
    provider::selected_provider,
    state::AppState,
    tunnel::{open_remote_pairing_tunnel, remote_pairing_path},
    types::{CrashReportContent, CrashReportSummary},
};

const MAX_PREVIEW_BYTES: usize = 4 * 1024 * 1024;
const MAX_REPORTS: usize = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CrashTransport {
    Lockdown,
    RemoteRsd,
    CoreDeviceRsd,
    UsbRequired,
}

fn crash_transport(is_bonjour: bool, generation: DeveloperGeneration) -> CrashTransport {
    match (is_bonjour, generation) {
        (false, _) => CrashTransport::Lockdown,
        (true, DeveloperGeneration::Legacy) => CrashTransport::UsbRequired,
        (true, DeveloperGeneration::CoreDeviceRemote) => CrashTransport::RemoteRsd,
        (true, DeveloperGeneration::CoreDeviceLockdown) => CrashTransport::CoreDeviceRsd,
    }
}

struct CrashClient {
    inner: CrashReportCopyMobileClient,
    _adapter: Option<AdapterHandle>,
}

impl Deref for CrashClient {
    type Target = CrashReportCopyMobileClient;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for CrashClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

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
    app: &AppHandle,
    state: &AppState,
    udid: Option<String>,
    flush_pending: bool,
) -> CommandResult<CrashClient> {
    let (udid, provider) = selected_provider(state, udid).await?;
    let generation = ios_version(&provider).await?.developer_generation();
    match crash_transport(provider.is_bonjour(), generation) {
        CrashTransport::Lockdown => {
            if flush_pending && let Err(error) = flush_reports(&provider).await {
                tracing::warn!(?error, "unable to flush pending crash reports");
            }
            let inner = CrashReportCopyMobileClient::connect(&provider)
                .await
                .map_err(CommandError::from)?;
            Ok(CrashClient {
                inner,
                _adapter: None,
            })
        }
        CrashTransport::UsbRequired => Err(CommandError::new(
            "crash_reports",
            "Crash reports over the network require iOS 17 or later. Connect the device by USB.",
            false,
        )),
        CrashTransport::RemoteRsd | CrashTransport::CoreDeviceRsd => {
            let (mut adapter, mut handshake) = match generation {
                DeveloperGeneration::CoreDeviceRemote => {
                    let pairing_path = remote_pairing_path(app, &udid)?;
                    let remote_target = state.discovery.read().await.remote_pairing_target(&udid);
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
                        CommandError::new(
                            "crash_reports",
                            format!("Unable to create the CoreDevice tunnel: {error}"),
                            true,
                        )
                    })?;
                    let mut adapter = adapter.to_async_handle();
                    let stream = adapter.connect(rsd_port).await.map_err(|error| {
                        CommandError::new(
                            "crash_reports",
                            format!("Unable to connect to tunneled RSD: {error}"),
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
            let inner =
                CrashReportCopyMobileClient::connect_rsd(&mut adapter, &mut handshake)
                    .await
                    .map_err(|error| {
                        CommandError::new(
                            "crash_reports",
                            format!(
                                "The network crash-report service is unavailable: {error}. Connect the device by USB and retry."
                            ),
                            true,
                        )
                    })?;
            Ok(CrashClient {
                inner,
                _adapter: Some(adapter),
            })
        }
    }
}

#[tauri::command]
pub async fn crash_reports_list(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<Vec<CrashReportSummary>> {
    let mut client = match crash_client(&app, &state, udid, true).await {
        Ok(client) => client,
        Err(error) => {
            tracing::error!(?error, "unable to connect to crash report service");
            return Err(error);
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
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
    path: String,
) -> CommandResult<CrashReportContent> {
    let normalized = validate_report_path(&path)?;
    let mut client = crash_client(&app, &state, udid, false).await?;
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
    app: AppHandle,
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
    let mut client = crash_client(&app, &state, udid, false).await?;
    let bytes = client.pull(normalized).await.map_err(CommandError::from)?;
    tokio::fs::write(local_path, bytes).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_version::DeveloperGeneration;

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

    #[test]
    fn selects_transport_by_connection_and_ios_generation() {
        assert_eq!(
            crash_transport(false, DeveloperGeneration::Legacy),
            CrashTransport::Lockdown
        );
        assert_eq!(
            crash_transport(false, DeveloperGeneration::CoreDeviceRemote),
            CrashTransport::Lockdown
        );
        assert_eq!(
            crash_transport(true, DeveloperGeneration::Legacy),
            CrashTransport::UsbRequired
        );
        assert_eq!(
            crash_transport(true, DeveloperGeneration::CoreDeviceRemote),
            CrashTransport::RemoteRsd
        );
        assert_eq!(
            crash_transport(true, DeveloperGeneration::CoreDeviceLockdown),
            CrashTransport::CoreDeviceRsd
        );
    }
}
