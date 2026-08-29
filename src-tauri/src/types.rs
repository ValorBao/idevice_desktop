use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSummary {
    pub id: String,
    pub udid: String,
    pub device_id: u32,
    pub connection: String,
    pub transports: Vec<String>,
    pub connectable: bool,
    pub paired: bool,
    pub name: Option<String>,
    pub model: Option<String>,
    pub ios: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceChangeEvent {
    pub kind: String,
    pub device: Option<DeviceSummary>,
    pub device_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatterySummary {
    pub level: Option<u64>,
    pub health_percent: Option<u64>,
    pub cycle_count: Option<u64>,
    pub temperature_celsius: Option<f64>,
    pub voltage_volts: Option<f64>,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSummary {
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
    pub block_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceOverview {
    pub udid: String,
    pub name: Option<String>,
    pub product_type: Option<String>,
    pub product_version: Option<String>,
    pub build_version: Option<String>,
    pub serial_number: Option<String>,
    pub unique_chip_id: Option<String>,
    pub hardware_model: Option<String>,
    pub hardware_platform: Option<String>,
    pub wifi_address: Option<String>,
    pub connection: String,
    pub paired: bool,
    pub battery: BatterySummary,
    pub storage: Option<StorageSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFileEntry {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified: String,
    /// True when the entry is present in the directory but its metadata could
    /// not be read. The name is real; the size, kind, and timestamp are not.
    pub unreadable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSharingApp {
    pub bundle_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApp {
    pub bundle_id: String,
    pub name: String,
    pub version: String,
    pub size_bytes: u64,
    pub system: bool,
    pub icon_data_url: Option<String>,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReportSummary {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub process: String,
    pub size_bytes: Option<u64>,
    pub modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReportContent {
    pub path: String,
    pub content: String,
    pub truncated: bool,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationProgress {
    pub operation: String,
    pub item: String,
    pub percent: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLog {
    pub timestamp: String,
    pub level: String,
    pub process: String,
    pub pid: u32,
    pub message: String,
    pub subsystem: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSummary {
    pub pid: u32,
    pub name: String,
    pub executable_path: Option<String>,
    pub is_application: bool,
    pub can_stop: bool,
    /// Opaque process identity used to protect stop requests from stale rows
    /// and PID reuse. The interface must return this value unchanged.
    pub identity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSnapshot {
    pub processes: Vec<ProcessSummary>,
    pub transport: String,
    pub available: bool,
    pub supports_launch: bool,
    pub supports_stop: bool,
    pub limitation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessLaunch {
    pub pid: u32,
    pub bundle_id: String,
    pub transport: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceProcessSample {
    pub pid: u32,
    pub name: String,
    /// Prefers sysmontap's unique process identifier or start time so a reused
    /// PID does not inherit the previous process's chart history.
    pub identity: String,
    pub cpu_percent: Option<f64>,
    pub memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceSample {
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub interval_ms: u32,
    pub transport: String,
    pub system_cpu_percent: Option<f64>,
    pub processes: Vec<PerformanceProcessSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceStatus {
    pub state: String,
    pub message: Option<String>,
    pub transport: Option<String>,
    pub interval_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceExportRow {
    pub timestamp_ms: u64,
    pub pid: u32,
    pub name: String,
    pub identity: String,
    pub cpu_percent: Option<f64>,
    pub memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkCaptureFilter {
    pub pid: Option<u32>,
    pub interface_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkCaptureProgress {
    pub packets: u64,
    pub bytes: u64,
    pub output_bytes: u64,
    pub elapsed_ms: u64,
    pub last_process: Option<String>,
    pub last_interface: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkCaptureStatus {
    pub state: String,
    pub message: Option<String>,
    pub destination: String,
    pub transport: Option<String>,
    pub filter: NetworkCaptureFilter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationObservationEvent {
    pub session_id: String,
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationObservationStatus {
    pub session_id: String,
    pub state: String,
    pub message: Option<String>,
    pub transport: Option<String>,
    pub subscriptions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveScreenFrame {
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub fps: f64,
    pub data_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveScreenStatus {
    pub state: String,
    pub message: Option<String>,
    pub transport: Option<String>,
    pub target_fps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisioningProfileSummary {
    pub id: String,
    pub uuid: Option<String>,
    pub name: String,
    pub team_name: Option<String>,
    pub team_identifier: Option<String>,
    pub application_identifier: Option<String>,
    pub created_at: Option<String>,
    pub expires_at: Option<String>,
    pub days_remaining: Option<i64>,
    pub expiration_state: String,
    pub profile_type: String,
    pub platforms: Vec<String>,
    pub device_count: u64,
    pub provisions_all_devices: bool,
    pub get_task_allow: Option<bool>,
    pub size_bytes: u64,
    pub parse_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisioningProfileSnapshot {
    pub profiles: Vec<ProvisioningProfileSummary>,
    pub transport: String,
    pub total_count: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteboardTextSnapshot {
    pub text: Option<String>,
    pub byte_length: u64,
    pub character_count: u64,
    pub change_count: Option<i64>,
    pub item_count: u64,
    pub state: String,
    pub message: Option<String>,
    pub transport: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteboardWriteResult {
    pub byte_length: u64,
    pub character_count: u64,
    pub transport: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteboardImageSnapshot {
    pub data_url: Option<String>,
    pub mime_type: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub byte_length: u64,
    pub change_count: Option<i64>,
    pub item_count: u64,
    pub state: String,
    pub message: Option<String>,
    pub transport: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteboardImagePreparation {
    pub preparation_id: String,
    pub data_url: String,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    pub byte_length: u64,
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteboardImageWriteResult {
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    pub byte_length: u64,
    pub transport: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XCTestRunnerCandidate {
    pub bundle_id: String,
    pub name: String,
    pub version: String,
    pub executable: Option<String>,
    pub debuggable: bool,
    pub is_webdriver_agent: bool,
    pub configuration_ready: bool,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XCTestTargetApp {
    pub bundle_id: String,
    pub name: String,
    pub version: String,
    pub debuggable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XCTestPreflightSnapshot {
    pub ios_version: String,
    pub transport: String,
    pub execution_supported: bool,
    pub limitation: Option<String>,
    pub runner_total: u64,
    pub target_total: u64,
    pub truncated: bool,
    pub runners: Vec<XCTestRunnerCandidate>,
    pub targets: Vec<XCTestTargetApp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XCTestPlanRequest {
    pub runner_bundle_id: String,
    pub target_bundle_id: Option<String>,
    pub mode: String,
    pub tests_to_run: Vec<String>,
    pub tests_to_skip: Vec<String>,
    pub timeout_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XCTestRunPlan {
    pub runner_bundle_id: String,
    pub runner_name: String,
    pub target_bundle_id: Option<String>,
    pub target_name: Option<String>,
    pub mode: String,
    pub tests_to_run: Vec<String>,
    pub tests_to_skip: Vec<String>,
    pub timeout_seconds: u32,
    pub wda_bridge: bool,
    pub transport: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamStatus {
    pub stream: String,
    pub state: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperStatus {
    pub developer_mode: Option<bool>,
    pub ddi_mounted: bool,
    pub ddi_images: serde_json::Value,
    pub rsd_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JitSession {
    pub bundle_id: String,
    pub pid: u64,
    pub response: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationSession {
    pub latitude: f64,
    pub longitude: f64,
    pub transport: String,
}

/// Contract tests between these types and their TypeScript counterparts in
/// `src/api.ts`.
///
/// `src/api.ts` is the frontend-backend contract, but nothing enforced it: a
/// renamed or added Rust field still compiled and still built on the frontend,
/// and the mismatch only surfaced as a silently undefined value at runtime.
/// These tests read the TypeScript declarations and compare them against what
/// serde actually emits.
#[cfg(test)]
mod contract {
    use super::*;
    use std::collections::BTreeSet;

    const API_TS: &str = include_str!("../../src/api.ts");

    /// Field names serde emits for a value, which is what the frontend receives.
    fn rust_fields<T: Serialize>(value: &T) -> BTreeSet<String> {
        serde_json::to_value(value)
            .expect("serializes")
            .as_object()
            .expect("object")
            .keys()
            .cloned()
            .collect()
    }

    /// Top-level field names of `export type <name> = { ... }` in `src/api.ts`.
    ///
    /// Only depth one is collected, so a nested object literal such as
    /// `battery` contributes its own name and not its inner fields; those are
    /// covered by the test for the corresponding Rust type.
    fn typescript_fields(name: &str) -> BTreeSet<String> {
        let declaration = format!("export type {name} = ");
        let start = API_TS
            .find(&declaration)
            .unwrap_or_else(|| panic!("{name} is not declared in src/api.ts"));
        let body = &API_TS[start + declaration.len()..];
        let open = body
            .find('{')
            .unwrap_or_else(|| panic!("{name} is not an object type"));

        let mut fields = BTreeSet::new();
        let mut depth = 0usize;
        let mut token = String::new();
        for character in body[open..].chars() {
            match character {
                '{' => {
                    depth += 1;
                    token.clear();
                }
                '}' => {
                    depth -= 1;
                    token.clear();
                    if depth == 0 {
                        break;
                    }
                }
                ':' if depth == 1 => {
                    if let Some(field) = token
                        .split(|character: char| character == ';' || character.is_whitespace())
                        .next_back()
                        .filter(|field| !field.is_empty())
                    {
                        fields.insert(field.trim_end_matches('?').to_owned());
                    }
                    token.clear();
                }
                _ => token.push(character),
            }
        }
        assert!(!fields.is_empty(), "{name} parsed to no fields");
        fields
    }

    fn assert_matches<T: Serialize>(name: &str, value: &T) {
        let rust = rust_fields(value);
        let typescript = typescript_fields(name);
        assert_eq!(
            rust,
            typescript,
            "\n{name} has drifted.\n  only in Rust:       {:?}\n  only in TypeScript: {:?}\n",
            rust.difference(&typescript).collect::<Vec<_>>(),
            typescript.difference(&rust).collect::<Vec<_>>(),
        );
    }

    fn device_summary() -> DeviceSummary {
        DeviceSummary {
            id: String::new(),
            udid: String::new(),
            device_id: 0,
            connection: String::new(),
            transports: Vec::new(),
            connectable: false,
            paired: false,
            name: None,
            model: None,
            ios: None,
        }
    }

    fn battery_summary() -> BatterySummary {
        BatterySummary {
            level: None,
            health_percent: None,
            cycle_count: None,
            temperature_celsius: None,
            voltage_volts: None,
            raw: serde_json::Value::Null,
        }
    }

    fn storage_summary() -> StorageSummary {
        StorageSummary {
            total_bytes: 0,
            free_bytes: 0,
            used_bytes: 0,
            block_size: 0,
        }
    }

    /// `CommandError` lives in `error.rs` rather than here, which is how it was
    /// missed when the other cross-boundary types were covered. Every failed
    /// command returns it, so a drift here breaks error reporting everywhere.
    #[test]
    fn command_error_matches_typescript() {
        assert_matches(
            "CommandError",
            &crate::error::CommandError::new("", "", false),
        );
    }

    #[test]
    fn device_summary_matches_typescript() {
        assert_matches("DeviceSummary", &device_summary());
    }

    #[test]
    fn device_change_event_matches_typescript() {
        assert_matches(
            "DeviceChangeEvent",
            &DeviceChangeEvent {
                kind: String::new(),
                device: None,
                device_id: None,
            },
        );
    }

    #[test]
    fn device_overview_matches_typescript() {
        assert_matches(
            "DeviceOverview",
            &DeviceOverview {
                udid: String::new(),
                name: None,
                product_type: None,
                product_version: None,
                build_version: None,
                serial_number: None,
                unique_chip_id: None,
                hardware_model: None,
                hardware_platform: None,
                wifi_address: None,
                connection: String::new(),
                paired: false,
                battery: battery_summary(),
                storage: None,
            },
        );
    }

    /// `battery` and `storage` are inline object literals on the TypeScript
    /// side, so they are compared against the nested declarations directly.
    #[test]
    fn nested_overview_types_match_typescript() {
        let overview = typescript_fields("DeviceOverview");
        assert!(overview.contains("battery") && overview.contains("storage"));

        let declaration = API_TS
            .find("battery: {")
            .expect("battery literal in DeviceOverview");
        let battery: BTreeSet<String> = API_TS[declaration..]
            .lines()
            .skip(1)
            .take_while(|line| !line.trim_start().starts_with('}'))
            .filter_map(|line| line.split(':').next())
            .map(|field| field.trim().to_owned())
            .filter(|field| !field.is_empty())
            .collect();
        assert_eq!(rust_fields(&battery_summary()), battery);

        let declaration = API_TS
            .find("storage: null | {")
            .expect("storage literal in DeviceOverview");
        let storage: BTreeSet<String> = API_TS[declaration..]
            .lines()
            .skip(1)
            .take_while(|line| !line.trim_start().starts_with('}'))
            .filter_map(|line| line.split(':').next())
            .map(|field| field.trim().to_owned())
            .filter(|field| !field.is_empty())
            .collect();
        assert_eq!(rust_fields(&storage_summary()), storage);
    }

    #[test]
    fn remote_file_entry_matches_typescript() {
        assert_matches(
            "RemoteFileEntry",
            &RemoteFileEntry {
                name: String::new(),
                path: String::new(),
                kind: String::new(),
                is_directory: false,
                size: 0,
                modified: String::new(),
                unreadable: false,
            },
        );
    }

    #[test]
    fn file_sharing_app_matches_typescript() {
        assert_matches(
            "FileSharingApp",
            &FileSharingApp {
                bundle_id: String::new(),
                name: String::new(),
            },
        );
    }

    #[test]
    fn installed_app_matches_typescript() {
        assert_matches(
            "InstalledApp",
            &InstalledApp {
                bundle_id: String::new(),
                name: String::new(),
                version: String::new(),
                size_bytes: 0,
                system: false,
                icon_data_url: None,
                raw: serde_json::Value::Null,
            },
        );
    }

    #[test]
    fn crash_report_types_match_typescript() {
        assert_matches(
            "CrashReportSummary",
            &CrashReportSummary {
                name: String::new(),
                path: String::new(),
                kind: String::new(),
                process: String::new(),
                size_bytes: None,
                modified: String::new(),
            },
        );
        assert_matches(
            "CrashReportContent",
            &CrashReportContent {
                path: String::new(),
                content: String::new(),
                truncated: false,
                size_bytes: 0,
            },
        );
    }

    #[test]
    fn operation_progress_matches_typescript() {
        assert_matches(
            "OperationProgress",
            &OperationProgress {
                operation: String::new(),
                item: String::new(),
                percent: 0,
            },
        );
    }

    #[test]
    fn device_log_matches_typescript() {
        assert_matches(
            "DeviceLog",
            &DeviceLog {
                timestamp: String::new(),
                level: String::new(),
                process: String::new(),
                pid: 0,
                message: String::new(),
                subsystem: None,
                category: None,
            },
        );
    }

    #[test]
    fn process_types_match_typescript() {
        assert_matches(
            "ProcessSummary",
            &ProcessSummary {
                pid: 0,
                name: String::new(),
                executable_path: None,
                is_application: false,
                can_stop: false,
                identity: String::new(),
            },
        );
        assert_matches(
            "ProcessSnapshot",
            &ProcessSnapshot {
                processes: Vec::new(),
                transport: String::new(),
                available: false,
                supports_launch: false,
                supports_stop: false,
                limitation: None,
            },
        );
        assert_matches(
            "ProcessLaunch",
            &ProcessLaunch {
                pid: 0,
                bundle_id: String::new(),
                transport: String::new(),
            },
        );
    }

    #[test]
    fn performance_types_match_typescript() {
        assert_matches(
            "PerformanceProcessSample",
            &PerformanceProcessSample {
                pid: 0,
                name: String::new(),
                identity: String::new(),
                cpu_percent: None,
                memory_bytes: None,
            },
        );
        assert_matches(
            "PerformanceSample",
            &PerformanceSample {
                sequence: 0,
                timestamp_ms: 0,
                interval_ms: 0,
                transport: String::new(),
                system_cpu_percent: None,
                processes: Vec::new(),
            },
        );
        assert_matches(
            "PerformanceStatus",
            &PerformanceStatus {
                state: String::new(),
                message: None,
                transport: None,
                interval_ms: 0,
            },
        );
        assert_matches(
            "PerformanceExportRow",
            &PerformanceExportRow {
                timestamp_ms: 0,
                pid: 0,
                name: String::new(),
                identity: String::new(),
                cpu_percent: None,
                memory_bytes: None,
            },
        );
    }

    #[test]
    fn network_capture_types_match_typescript() {
        let filter = NetworkCaptureFilter {
            pid: None,
            interface_name: None,
        };
        assert_matches("NetworkCaptureFilter", &filter);
        assert_matches(
            "NetworkCaptureProgress",
            &NetworkCaptureProgress {
                packets: 0,
                bytes: 0,
                output_bytes: 0,
                elapsed_ms: 0,
                last_process: None,
                last_interface: None,
            },
        );
        assert_matches(
            "NetworkCaptureStatus",
            &NetworkCaptureStatus {
                state: String::new(),
                message: None,
                destination: String::new(),
                transport: None,
                filter,
            },
        );
    }

    #[test]
    fn notification_observation_types_match_typescript() {
        assert_matches(
            "NotificationObservationEvent",
            &NotificationObservationEvent {
                session_id: String::new(),
                sequence: 0,
                timestamp_ms: 0,
                name: String::new(),
            },
        );
        assert_matches(
            "NotificationObservationStatus",
            &NotificationObservationStatus {
                session_id: String::new(),
                state: String::new(),
                message: None,
                transport: None,
                subscriptions: Vec::new(),
            },
        );
    }

    #[test]
    fn live_screen_types_match_typescript() {
        assert_matches(
            "LiveScreenFrame",
            &LiveScreenFrame {
                sequence: 0,
                timestamp_ms: 0,
                width: 0,
                height: 0,
                bytes: 0,
                fps: 0.0,
                data_url: String::new(),
            },
        );
        assert_matches(
            "LiveScreenStatus",
            &LiveScreenStatus {
                state: String::new(),
                message: None,
                transport: None,
                target_fps: 0,
            },
        );
    }

    #[test]
    fn provisioning_profile_types_match_typescript() {
        let profile = ProvisioningProfileSummary {
            id: String::new(),
            uuid: None,
            name: String::new(),
            team_name: None,
            team_identifier: None,
            application_identifier: None,
            created_at: None,
            expires_at: None,
            days_remaining: None,
            expiration_state: String::new(),
            profile_type: String::new(),
            platforms: Vec::new(),
            device_count: 0,
            provisions_all_devices: false,
            get_task_allow: None,
            size_bytes: 0,
            parse_error: None,
        };
        assert_matches("ProvisioningProfileSummary", &profile);
        assert_matches(
            "ProvisioningProfileSnapshot",
            &ProvisioningProfileSnapshot {
                profiles: vec![profile],
                transport: String::new(),
                total_count: 1,
                truncated: false,
            },
        );
    }

    #[test]
    fn pasteboard_types_match_typescript() {
        assert_matches(
            "PasteboardTextSnapshot",
            &PasteboardTextSnapshot {
                text: None,
                byte_length: 0,
                character_count: 0,
                change_count: None,
                item_count: 0,
                state: String::new(),
                message: None,
                transport: String::new(),
            },
        );
        assert_matches(
            "PasteboardWriteResult",
            &PasteboardWriteResult {
                byte_length: 0,
                character_count: 0,
                transport: String::new(),
            },
        );
        assert_matches(
            "PasteboardImageSnapshot",
            &PasteboardImageSnapshot {
                data_url: None,
                mime_type: None,
                width: None,
                height: None,
                byte_length: 0,
                change_count: None,
                item_count: 0,
                state: String::new(),
                message: None,
                transport: String::new(),
            },
        );
        assert_matches(
            "PasteboardImagePreparation",
            &PasteboardImagePreparation {
                preparation_id: String::new(),
                data_url: String::new(),
                mime_type: String::new(),
                width: 0,
                height: 0,
                byte_length: 0,
                file_name: String::new(),
            },
        );
        assert_matches(
            "PasteboardImageWriteResult",
            &PasteboardImageWriteResult {
                mime_type: String::new(),
                width: 0,
                height: 0,
                byte_length: 0,
                transport: String::new(),
            },
        );
    }

    #[test]
    fn xctest_preflight_types_match_typescript() {
        let runner = XCTestRunnerCandidate {
            bundle_id: String::new(),
            name: String::new(),
            version: String::new(),
            executable: None,
            debuggable: false,
            is_webdriver_agent: false,
            configuration_ready: false,
            issues: Vec::new(),
        };
        let target = XCTestTargetApp {
            bundle_id: String::new(),
            name: String::new(),
            version: String::new(),
            debuggable: false,
        };
        assert_matches("XCTestRunnerCandidate", &runner);
        assert_matches("XCTestTargetApp", &target);
        assert_matches(
            "XCTestPreflightSnapshot",
            &XCTestPreflightSnapshot {
                ios_version: String::new(),
                transport: String::new(),
                execution_supported: false,
                limitation: None,
                runner_total: 0,
                target_total: 0,
                truncated: false,
                runners: vec![runner],
                targets: vec![target],
            },
        );
        assert_matches(
            "XCTestPlanRequest",
            &XCTestPlanRequest {
                runner_bundle_id: String::new(),
                target_bundle_id: None,
                mode: String::new(),
                tests_to_run: Vec::new(),
                tests_to_skip: Vec::new(),
                timeout_seconds: 0,
            },
        );
        assert_matches(
            "XCTestRunPlan",
            &XCTestRunPlan {
                runner_bundle_id: String::new(),
                runner_name: String::new(),
                target_bundle_id: None,
                target_name: None,
                mode: String::new(),
                tests_to_run: Vec::new(),
                tests_to_skip: Vec::new(),
                timeout_seconds: 0,
                wda_bridge: false,
                transport: String::new(),
            },
        );
    }

    #[test]
    fn developer_status_matches_typescript() {
        assert_matches(
            "DeveloperStatus",
            &DeveloperStatus {
                developer_mode: None,
                ddi_mounted: false,
                ddi_images: serde_json::Value::Null,
                rsd_available: false,
            },
        );
    }

    #[test]
    fn jit_session_matches_typescript() {
        assert_matches(
            "JitSession",
            &JitSession {
                bundle_id: String::new(),
                pid: 0,
                response: None,
            },
        );
    }

    #[test]
    fn location_session_matches_typescript() {
        assert_matches(
            "LocationSession",
            &LocationSession {
                latitude: 0.0,
                longitude: 0.0,
                transport: String::new(),
            },
        );
    }
}
