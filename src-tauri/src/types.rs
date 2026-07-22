use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSummary {
    pub udid: String,
    pub device_id: u32,
    pub connection: String,
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
