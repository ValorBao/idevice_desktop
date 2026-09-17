//! Contract tests between these types and their TypeScript counterparts in
//! `src/api.ts`.
//!
//! `src/api.ts` is the frontend-backend contract, but nothing enforced it: a
//! renamed or added Rust field still compiled and still built on the frontend,
//! and the mismatch only surfaced as a silently undefined value at runtime.
//! These tests read the TypeScript declarations and compare them against what
//! serde actually emits.

use super::*;
use std::collections::BTreeSet;

const API_TS: &str = include_str!("../../../src/api.ts");

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
        "LogStatus",
        &LogStatus {
            session_id: String::new(),
            udid: String::new(),
            state: String::new(),
            message: None,
        },
    );
    assert_matches(
        "DeviceLog",
        &DeviceLog {
            session_id: String::new(),
            udid: String::new(),
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
fn personal_signing_types_match_typescript() {
    let identity = PersonalSigningIdentity {
        hash: String::new(),
        name: String::new(),
        team_identifier: None,
        matches_profile: false,
    };
    assert_matches("PersonalSigningIdentity", &identity);
    assert_matches(
        "PersonalSigningPreflight",
        &PersonalSigningPreflight {
            ipa_name: String::new(),
            app_name: String::new(),
            bundle_id: String::new(),
            version: String::new(),
            profile_name: String::new(),
            profile_uuid: None,
            team_identifier: None,
            application_identifier: None,
            expires_at: None,
            device_count: 0,
            device_included: false,
            identities: vec![identity],
            selected_identity_hash: None,
            ready: false,
            blockers: Vec::new(),
            warnings: Vec::new(),
        },
    );
    assert_matches(
        "PersonalSigningRequest",
        &PersonalSigningRequest {
            ipa_path: String::new(),
            profile_path: String::new(),
            identity_hash: String::new(),
            output_path: String::new(),
        },
    );
    assert_matches(
        "PersonalSigningResult",
        &PersonalSigningResult {
            output_path: String::new(),
            app_name: String::new(),
            bundle_id: String::new(),
            profile_name: String::new(),
            identity_name: String::new(),
            size_bytes: 0,
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
