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
    pub session_id: String,
    pub udid: String,
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
pub struct PersonalSigningIdentity {
    pub hash: String,
    pub name: String,
    pub team_identifier: Option<String>,
    pub matches_profile: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalSigningPreflight {
    pub ipa_name: String,
    pub app_name: String,
    pub bundle_id: String,
    pub version: String,
    pub profile_name: String,
    pub profile_uuid: Option<String>,
    pub team_identifier: Option<String>,
    pub application_identifier: Option<String>,
    pub expires_at: Option<String>,
    pub device_count: u64,
    pub device_included: bool,
    pub identities: Vec<PersonalSigningIdentity>,
    pub selected_identity_hash: Option<String>,
    pub ready: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalSigningRequest {
    pub ipa_path: String,
    pub profile_path: String,
    pub identity_hash: String,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalSigningResult {
    pub output_path: String,
    pub app_name: String,
    pub bundle_id: String,
    pub profile_name: String,
    pub identity_name: String,
    pub size_bytes: u64,
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
pub struct LogStatus {
    pub session_id: String,
    pub udid: String,
    pub state: String,
    pub message: Option<String>,
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

#[cfg(test)]
mod contract;
