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
