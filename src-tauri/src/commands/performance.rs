use std::{cmp::Ordering, future::Future, time::Duration};

use idevice::{
    IdeviceService, RsdService,
    core_device_proxy::CoreDeviceProxy,
    dvt::{
        device_info::DeviceInfoClient,
        remote_server::RemoteServerClient,
        sysmontap::{SysmontapClient, SysmontapConfig, SysmontapSample},
    },
    rsd::RsdHandshake,
};
use plist::{Dictionary, Value};
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use crate::{
    device_version::{DeveloperGeneration, ios_version},
    discovery::{LockdownTarget, RemotePairingTarget},
    error::{CommandError, CommandResult},
    provider::{RoutedProvider, routed_provider_for},
    state::AppState,
    tunnel::{RsdTunnel, open_remote_pairing_tunnel, remote_pairing_path},
    types::{PerformanceExportRow, PerformanceProcessSample, PerformanceSample, PerformanceStatus},
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(45);
const REMOTE_PAIRING_ATTEMPTS: usize = 3;
const DEFAULT_INTERVAL_MS: u32 = 1_000;
const VALID_INTERVALS: [u32; 3] = [500, 1_000, 2_000];
const MAX_PROCESSES_PER_SAMPLE: usize = 80;
const MAX_EXPORT_ROWS: usize = 10_000;
const LEGACY_LIMITATION: &str = "Performance sampling is unavailable on iOS 16 and earlier because the verified Legacy instruments service does not provide a reliable sysmontap stream.";

#[derive(Clone)]
struct PerformanceContext {
    udid: String,
    pairing_path: std::path::PathBuf,
    lockdown_target: Option<LockdownTarget>,
    remote_target: Option<RemotePairingTarget>,
}

enum PerformanceEnd {
    Stopped,
    Unavailable,
}

#[derive(Debug)]
struct ProcessAttributeIndexes {
    pid: Option<usize>,
    name: Option<usize>,
    unique_id: Option<usize>,
    start_abs_time: Option<usize>,
    cpu_percent: Option<usize>,
    memory_bytes: Option<usize>,
}

impl ProcessAttributeIndexes {
    fn new(attributes: &[String]) -> Self {
        Self {
            pid: attribute_index(attributes, &["pid", "processIdentifier"]),
            name: attribute_index(attributes, &["name", "comm", "processName"]),
            unique_id: attribute_index(attributes, &["uniqueID", "uniqueId"]),
            start_abs_time: attribute_index(attributes, &["startAbsTime", "startTime"]),
            cpu_percent: attribute_index(attributes, &["cpuUsage", "cpuPercent", "CPU"]),
            memory_bytes: attribute_index(
                attributes,
                &["physFootprint", "memResidentSize", "residentSize", "Memory"],
            ),
        }
    }
}

fn attribute_index(attributes: &[String], candidates: &[&str]) -> Option<usize> {
    attributes.iter().position(|attribute| {
        candidates
            .iter()
            .any(|candidate| attribute.eq_ignore_ascii_case(candidate))
    })
}

fn validated_interval(interval_ms: Option<u32>) -> CommandResult<u32> {
    let interval_ms = interval_ms.unwrap_or(DEFAULT_INTERVAL_MS);
    if VALID_INTERVALS.contains(&interval_ms) {
        Ok(interval_ms)
    } else {
        Err(CommandError::new(
            "performance",
            "Choose a 500 ms, 1 second, or 2 second sample interval",
            false,
        ))
    }
}

async fn context(
    app: &AppHandle,
    state: &AppState,
    override_udid: Option<String>,
) -> CommandResult<PerformanceContext> {
    let udid = state
        .selected(override_udid)
        .await
        .ok_or_else(|| CommandError::new("device", "No device selected", true))?;
    let catalog = state.discovery.read().await;
    Ok(PerformanceContext {
        pairing_path: remote_pairing_path(app, &udid)?,
        lockdown_target: catalog.lockdown_target(&udid),
        remote_target: catalog.remote_pairing_target(&udid),
        udid,
    })
}

async fn open_core_device_proxy(provider: &RoutedProvider) -> CommandResult<RsdTunnel> {
    let proxy = CoreDeviceProxy::connect(provider)
        .await
        .map_err(CommandError::from)?;
    let rsd_port = proxy.tunnel_info().server_rsd_port;
    let mut adapter = proxy
        .create_software_tunnel()
        .map_err(|error| CommandError::new("tunnel", error.to_string(), true))?
        .to_async_handle();
    let stream = adapter
        .connect(rsd_port)
        .await
        .map_err(|error| CommandError::new("tunnel", error.to_string(), true))?;
    let handshake = RsdHandshake::new(stream)
        .await
        .map_err(CommandError::from)?;
    Ok(RsdTunnel { adapter, handshake })
}

async fn connect(context: &PerformanceContext) -> CommandResult<Option<(&'static str, RsdTunnel)>> {
    let provider = routed_provider_for(&context.udid, context.lockdown_target.as_ref()).await?;
    match ios_version(&provider).await?.developer_generation() {
        DeveloperGeneration::Legacy => Ok(None),
        DeveloperGeneration::CoreDeviceRemote => {
            let mut attempt = 1;
            let tunnel = loop {
                match open_remote_pairing_tunnel(
                    &provider,
                    &context.pairing_path,
                    "idevice-desktop",
                    context.remote_target.as_ref(),
                )
                .await
                {
                    Ok(tunnel) => break tunnel,
                    Err(error) if error.retryable && attempt < REMOTE_PAIRING_ATTEMPTS => {
                        tracing::warn!(
                            attempt,
                            error = %error.message,
                            "retrying the Performance RemotePairing tunnel"
                        );
                        tokio::time::sleep(Duration::from_millis(250 * attempt as u64)).await;
                        attempt += 1;
                    }
                    Err(error) => return Err(error),
                }
            };
            Ok(Some(("RemotePairing/RSD", tunnel)))
        }
        DeveloperGeneration::CoreDeviceLockdown => Ok(Some((
            "CoreDeviceProxy/RSD",
            open_core_device_proxy(&provider).await?,
        ))),
    }
}

fn emit_status(
    app: &AppHandle,
    state: &str,
    message: Option<String>,
    transport: Option<String>,
    interval_ms: u32,
) {
    let _ = app.emit(
        "performance://status",
        PerformanceStatus {
            state: state.into(),
            message,
            transport,
            interval_ms,
        },
    );
}

async fn timeout<T, E, F>(
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
        _ = token.cancelled() => Err(CommandError::new(
            "cancelled",
            "Performance sampling stopped",
            false,
        )),
        result = tokio::time::timeout(duration, future) => result
            .map_err(|_| CommandError::new(
                "performance",
                format!("Timed out while {label}. Keep the device unlocked and retry."),
                true,
            ))?
            .map_err(|error| CommandError::new("performance", error.to_string(), true)),
    }
}

fn value_at(values: &[Value], index: Option<usize>) -> Option<&Value> {
    index.and_then(|index| values.get(index))
}

fn value_f64(value: &Value) -> Option<f64> {
    let value = match value {
        Value::Real(value) => *value,
        Value::Integer(value) => value
            .as_signed()
            .map(|value| value as f64)
            .or_else(|| value.as_unsigned().map(|value| value as f64))?,
        Value::String(value) => value.parse().ok()?,
        _ => return None,
    };
    value.is_finite().then_some(value)
}

fn value_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Integer(value) => value.as_unsigned(),
        Value::Real(value) if value.is_finite() && *value >= 0.0 => Some(*value as u64),
        Value::String(value) => value.parse().ok(),
        Value::Uid(value) => Some(value.get()),
        _ => None,
    }
}

fn value_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if !value.is_empty() => Some(value.clone()),
        _ => None,
    }
}

fn identity_value(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if !value.is_empty() => Some(value.clone()),
        Value::Integer(value) => Some(value.to_string()),
        Value::Uid(value) => Some(value.get().to_string()),
        Value::Real(value) if value.is_finite() => Some(value.to_string()),
        _ => None,
    }
}

fn parse_process(
    pid_key: &str,
    raw: &Value,
    indexes: &ProcessAttributeIndexes,
) -> Option<PerformanceProcessSample> {
    let values = raw.as_array()?;
    let pid = value_at(values, indexes.pid)
        .and_then(value_u64)
        .and_then(|value| u32::try_from(value).ok())
        .or_else(|| pid_key.parse().ok())?;
    if pid == 0 {
        return None;
    }

    let name = value_at(values, indexes.name)
        .and_then(value_string)
        .unwrap_or_else(|| format!("Process {pid}"));
    let identity = value_at(values, indexes.unique_id)
        .and_then(identity_value)
        .map(|value| format!("unique:{value}"))
        .or_else(|| {
            value_at(values, indexes.start_abs_time)
                .and_then(identity_value)
                .map(|value| format!("start:{value}"))
        })
        .unwrap_or_else(|| format!("pid:{pid}:{name}"));
    let cpu_percent = value_at(values, indexes.cpu_percent)
        .and_then(value_f64)
        .filter(|value| *value >= 0.0);
    let memory_bytes = value_at(values, indexes.memory_bytes).and_then(value_u64);

    Some(PerformanceProcessSample {
        pid,
        name,
        identity,
        cpu_percent,
        memory_bytes,
    })
}

fn system_cpu_percent(system_cpu_usage: Option<&Dictionary>) -> Option<f64> {
    let system_cpu_usage = system_cpu_usage?;
    let value = system_cpu_usage
        .get("CPU_TotalLoad")
        .or_else(|| system_cpu_usage.get("CPU_TotalUsage"))
        .and_then(value_f64)?;
    if value < 0.0 {
        None
    } else if value <= 1.0 {
        Some(value * 100.0)
    } else {
        Some(value)
    }
}

fn process_order(left: &PerformanceProcessSample, right: &PerformanceProcessSample) -> Ordering {
    right
        .cpu_percent
        .unwrap_or(-1.0)
        .total_cmp(&left.cpu_percent.unwrap_or(-1.0))
        .then_with(|| {
            right
                .memory_bytes
                .unwrap_or(0)
                .cmp(&left.memory_bytes.unwrap_or(0))
        })
        .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        .then_with(|| left.pid.cmp(&right.pid))
}

fn normalize_sample(
    raw: SysmontapSample,
    indexes: &ProcessAttributeIndexes,
    sequence: u64,
    interval_ms: u32,
    transport: &str,
) -> PerformanceSample {
    let mut processes = raw
        .processes
        .as_ref()
        .into_iter()
        .flat_map(|processes| processes.iter())
        .filter_map(|(pid, value)| parse_process(pid, value, indexes))
        .collect::<Vec<_>>();
    processes.sort_by(process_order);
    processes.truncate(MAX_PROCESSES_PER_SAMPLE);

    PerformanceSample {
        sequence,
        timestamp_ms: chrono::Utc::now().timestamp_millis().max(0) as u64,
        interval_ms,
        transport: transport.into(),
        system_cpu_percent: system_cpu_percent(raw.system_cpu_usage.as_ref()),
        processes,
    }
}

async fn run_stream(
    app: AppHandle,
    context: PerformanceContext,
    interval_ms: u32,
    token: CancellationToken,
) -> CommandResult<PerformanceEnd> {
    let connection = tokio::select! {
        _ = token.cancelled() => return Ok(PerformanceEnd::Stopped),
        result = tokio::time::timeout(CONNECT_TIMEOUT, connect(&context)) => result
            .map_err(|_| CommandError::new(
                "performance",
                "Timed out opening the Performance developer transport",
                true,
            ))??,
    };
    let Some((route, mut tunnel)) = connection else {
        if token.is_cancelled() {
            return Ok(PerformanceEnd::Stopped);
        }
        emit_status(
            &app,
            "unavailable",
            Some(LEGACY_LIMITATION.into()),
            Some("Legacy".into()),
            interval_ms,
        );
        return Ok(PerformanceEnd::Unavailable);
    };

    let mut remote = timeout(
        "opening the DVT remote server",
        CONNECT_TIMEOUT,
        &token,
        RemoteServerClient::connect_rsd(&mut tunnel.adapter, &mut tunnel.handshake),
    )
    .await?;
    timeout(
        "reading the DVT handshake",
        CONNECT_TIMEOUT,
        &token,
        remote.read_message(0),
    )
    .await?;

    let (process_attributes, system_attributes) = {
        let mut device_info = timeout(
            "opening DVT DeviceInfo",
            CONNECT_TIMEOUT,
            &token,
            DeviceInfoClient::new(&mut remote),
        )
        .await?;
        let process_attributes = timeout(
            "reading sysmontap process attributes",
            CONNECT_TIMEOUT,
            &token,
            device_info.sysmon_process_attributes(),
        )
        .await?;
        let system_attributes = timeout(
            "reading sysmontap system attributes",
            CONNECT_TIMEOUT,
            &token,
            device_info.sysmon_system_attributes(),
        )
        .await?;
        (process_attributes, system_attributes)
    };
    let indexes = ProcessAttributeIndexes::new(&process_attributes);
    if indexes.cpu_percent.is_none() && indexes.memory_bytes.is_none() {
        return Err(CommandError::new(
            "performance",
            "The device's sysmontap schema contains neither CPU nor memory metrics",
            false,
        ));
    }

    let mut sysmontap = timeout(
        "opening sysmontap",
        CONNECT_TIMEOUT,
        &token,
        SysmontapClient::new(&mut remote),
    )
    .await?;
    timeout(
        "configuring sysmontap",
        CONNECT_TIMEOUT,
        &token,
        sysmontap.set_config(&SysmontapConfig {
            interval_ms,
            process_attributes,
            system_attributes,
        }),
    )
    .await?;
    timeout(
        "starting sysmontap",
        CONNECT_TIMEOUT,
        &token,
        sysmontap.start(),
    )
    .await?;

    let transport = format!("DVT Sysmontap · {route}");
    emit_status(&app, "running", None, Some(transport.clone()), interval_ms);
    let sample_timeout = Duration::from_millis((interval_ms as u64 * 8).max(10_000));
    let mut sequence = 0u64;
    loop {
        tokio::select! {
            _ = token.cancelled() => {
                let _ = tokio::time::timeout(Duration::from_secs(5), sysmontap.stop()).await;
                return Ok(PerformanceEnd::Stopped);
            }
            result = tokio::time::timeout(sample_timeout, sysmontap.next_sample()) => {
                let raw = result
                    .map_err(|_| CommandError::new(
                        "performance",
                        "Timed out waiting for a sysmontap sample",
                        true,
                    ))?
                    .map_err(CommandError::from)?;
                sequence = sequence.saturating_add(1);
                let sample = normalize_sample(raw, &indexes, sequence, interval_ms, &transport);
                let _ = app.emit("performance://sample", sample);
            }
        }
    }
}

#[tauri::command]
pub async fn performance_start(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
    interval_ms: Option<u32>,
) -> CommandResult<()> {
    let interval_ms = validated_interval(interval_ms)?;
    let context = context(&app, &state, udid).await?;
    let token = CancellationToken::new();
    state.replace_task("performance", token.clone()).await;
    emit_status(&app, "connecting", None, None, interval_ms);

    tauri::async_runtime::spawn_blocking(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                emit_status(&app, "error", Some(error.to_string()), None, interval_ms);
                return;
            }
        };
        let completion_token = token.clone();
        match runtime.block_on(run_stream(app.clone(), context, interval_ms, token)) {
            Ok(PerformanceEnd::Stopped) => {}
            Ok(PerformanceEnd::Unavailable) => {}
            Err(error) if !completion_token.is_cancelled() => {
                emit_status(&app, "error", Some(error.message), None, interval_ms)
            }
            Err(_) => {}
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn performance_stop(state: State<'_, AppState>) -> CommandResult<()> {
    state.cancel_task("performance").await;
    Ok(())
}

fn csv_field(value: &str) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.into()
    }
}

fn export_csv(rows: &[PerformanceExportRow]) -> String {
    let mut output = String::from("timestamp_ms,pid,name,identity,cpu_percent,memory_bytes\n");
    for row in rows {
        let cpu = row
            .cpu_percent
            .filter(|value| value.is_finite())
            .map(|value| value.to_string())
            .unwrap_or_default();
        let memory = row
            .memory_bytes
            .map(|value| value.to_string())
            .unwrap_or_default();
        output.push_str(&format!(
            "{},{},{},{},{},{}\n",
            row.timestamp_ms,
            row.pid,
            csv_field(&row.name),
            csv_field(&row.identity),
            cpu,
            memory,
        ));
    }
    output
}

#[tauri::command]
pub async fn performance_export_csv(
    local_path: String,
    rows: Vec<PerformanceExportRow>,
) -> CommandResult<()> {
    if local_path.trim().is_empty() {
        return Err(CommandError::new(
            "performance",
            "Choose a CSV destination",
            false,
        ));
    }
    if rows.is_empty() {
        return Err(CommandError::new(
            "performance",
            "Collect at least one sample before exporting",
            false,
        ));
    }
    if rows.len() > MAX_EXPORT_ROWS {
        return Err(CommandError::new(
            "performance",
            "The CSV export exceeds the 10,000-row safety limit",
            false,
        ));
    }
    tokio::fs::write(local_path, export_csv(&rows)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_values(values: Vec<Value>) -> Value {
        Value::Array(values)
    }

    #[test]
    fn accepts_only_the_visible_sample_intervals() {
        assert_eq!(validated_interval(None).unwrap(), 1_000);
        for interval in VALID_INTERVALS {
            assert_eq!(validated_interval(Some(interval)).unwrap(), interval);
        }
        assert!(validated_interval(Some(250)).is_err());
        assert!(validated_interval(Some(10_000)).is_err());
    }

    #[test]
    fn maps_dynamic_attribute_order_and_prefers_unique_identity() {
        let attributes = vec![
            "physFootprint".into(),
            "name".into(),
            "uniqueID".into(),
            "cpuUsage".into(),
            "pid".into(),
        ];
        let indexes = ProcessAttributeIndexes::new(&attributes);
        let process = parse_process(
            "ignored",
            &sample_values(vec![
                Value::Integer(64_000_000u64.into()),
                Value::String("Example".into()),
                Value::Uid(plist::Uid::new(77)),
                Value::Real(12.5),
                Value::Integer(4242u64.into()),
            ]),
            &indexes,
        )
        .unwrap();

        assert_eq!(process.pid, 4242);
        assert_eq!(process.name, "Example");
        assert_eq!(process.identity, "unique:77");
        assert_eq!(process.cpu_percent, Some(12.5));
        assert_eq!(process.memory_bytes, Some(64_000_000));
    }

    #[test]
    fn missing_metrics_remain_unavailable_instead_of_becoming_zero() {
        let attributes = vec!["pid".into(), "name".into(), "startAbsTime".into()];
        let indexes = ProcessAttributeIndexes::new(&attributes);
        let process = parse_process(
            "99",
            &sample_values(vec![
                Value::Integer(99u64.into()),
                Value::String("Short lived".into()),
                Value::Integer(123_456u64.into()),
            ]),
            &indexes,
        )
        .unwrap();

        assert_eq!(process.identity, "start:123456");
        assert_eq!(process.cpu_percent, None);
        assert_eq!(process.memory_bytes, None);
    }

    #[test]
    fn normalizes_fractional_system_cpu_without_clamping_percent_values() {
        let mut fractional = Dictionary::new();
        fractional.insert("CPU_TotalLoad".into(), Value::Real(0.42));
        assert_eq!(system_cpu_percent(Some(&fractional)), Some(42.0));

        let mut percent = Dictionary::new();
        percent.insert("CPU_TotalLoad".into(), Value::Real(134.5));
        assert_eq!(system_cpu_percent(Some(&percent)), Some(134.5));
    }

    #[test]
    fn csv_quotes_text_and_preserves_missing_values() {
        let csv = export_csv(&[
            PerformanceExportRow {
                timestamp_ms: 1,
                pid: 42,
                name: "Example, \"Debug\"".into(),
                identity: "start:9".into(),
                cpu_percent: Some(7.5),
                memory_bytes: Some(2048),
            },
            PerformanceExportRow {
                timestamp_ms: 2,
                pid: 42,
                name: "Example".into(),
                identity: "start:9".into(),
                cpu_percent: None,
                memory_bytes: None,
            },
        ]);

        assert!(csv.contains("\"Example, \"\"Debug\"\"\""));
        assert!(csv.contains("1,42,\"Example, \"\"Debug\"\"\",start:9,7.5,2048"));
        assert!(csv.contains("2,42,Example,start:9,,"));
    }
}
