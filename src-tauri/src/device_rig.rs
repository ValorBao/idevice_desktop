//! Shared helpers for unattended real-device verification.
//!
//! The Mac mini CI job and the `verify_*` examples resolve a device through
//! [`IDEVICE_TEST_UDID`] and never prompt. Every usbmuxd / Lockdown call in
//! [`inspect_attached_devices`] is bounded so a wedged phone cannot hang the
//! runner. Transport for developer services stays in [`crate::transport`].

use std::{collections::BTreeMap, future::Future, time::Duration};

use idevice::{
    IdeviceService,
    amfi::AmfiClient,
    lockdown::LockdownClient,
    provider::IdeviceProvider,
    usbmuxd::{Connection, UsbmuxdConnection},
};
use serde::Serialize;

use crate::{
    device_version::{DeveloperGeneration, ios_version},
    error::{CommandError, CommandResult},
    provider::routed_provider_for,
    transport::DeviceContext,
    utils::{dict_string, dict_u64},
};

/// Environment variable that selects which attached phone a harness talks to.
pub const IDEVICE_TEST_UDID: &str = "IDEVICE_TEST_UDID";
/// Optional generation slug (`legacy`, `core-device-remote`, `core-device-lockdown`).
pub const IDEVICE_TEST_GENERATION: &str = "IDEVICE_TEST_GENERATION";

const OP_TIMEOUT: Duration = Duration::from_secs(20);

/// One USB (or usbmuxd-visible) phone as preflight sees it.
#[derive(Debug, Clone, Serialize)]
pub struct InspectedDevice {
    pub udid: String,
    pub connection: String,
    pub usb: bool,
    pub name: Option<String>,
    pub product_type: Option<String>,
    pub ios: Option<String>,
    pub generation: Option<String>,
    pub paired: bool,
    pub trusted: bool,
    pub locked: Option<bool>,
    pub developer_mode: Option<bool>,
    pub battery_percent: Option<u64>,
    pub charging: Option<bool>,
    pub issues: Vec<String>,
}

impl InspectedDevice {
    pub fn generation_slug(&self) -> Option<DeveloperGeneration> {
        self.generation
            .as_deref()
            .and_then(DeveloperGeneration::from_slug)
    }

    pub fn issue_summary(&self) -> String {
        if self.issues.is_empty() {
            format!("{}: ready", self.udid)
        } else {
            format!("{}: {}", self.udid, self.issues.join("; "))
        }
    }

    pub fn blocking(&self) -> bool {
        self.issues.iter().any(|issue| {
            issue.contains("locked")
                || issue.contains("untrusted")
                || issue.contains("not paired")
                || issue.contains("no pair record")
                || issue.contains("Developer Mode is off")
        })
    }
}

/// Preflight snapshot of every usbmuxd record plus per-generation assignment.
#[derive(Debug, Clone, Serialize)]
pub struct PreflightReport {
    pub ok: bool,
    pub devices: Vec<InspectedDevice>,
    pub by_generation: BTreeMap<String, Option<String>>,
    pub failures: Vec<String>,
}

impl PreflightReport {
    pub fn udid_for(&self, generation: DeveloperGeneration) -> Option<&str> {
        self.by_generation
            .get(generation.slug())
            .and_then(|value| value.as_deref())
    }

    pub fn github_matrix(&self) -> serde_json::Value {
        let include: Vec<serde_json::Value> = DeveloperGeneration::ALL
            .into_iter()
            .map(|generation| {
                let udid = self.udid_for(generation).unwrap_or("");
                serde_json::json!({
                    "generation": generation.slug(),
                    "udid": udid,
                    "skip": udid.is_empty(),
                })
            })
            .collect();
        serde_json::json!({ "include": include })
    }

    pub fn write_github_output(&self, output: &mut impl std::io::Write) -> std::io::Result<()> {
        for generation in DeveloperGeneration::ALL {
            let key = generation.slug().replace('-', "_");
            let udid = self.udid_for(generation).unwrap_or("");
            writeln!(output, "{key}={udid}")?;
        }
        let matrix = serde_json::to_string(&self.github_matrix()).unwrap_or_else(|_| "{}".into());
        writeln!(output, "matrix<<DEVICE_RIG_MATRIX")?;
        writeln!(output, "{matrix}")?;
        writeln!(output, "DEVICE_RIG_MATRIX")?;
        writeln!(output, "ok={}", self.ok)?;
        Ok(())
    }
}

/// True when `value` looks like an iOS UDID rather than a bundle ID or flag.
pub fn looks_like_udid(value: &str) -> bool {
    if value.is_empty() || value.starts_with('-') || value.contains('.') {
        return false;
    }
    let compact: String = value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    compact.len() >= 16 && compact.chars().all(|c| c.is_ascii_hexdigit())
}

/// Resolves the device UDID from an optional CLI token or [`IDEVICE_TEST_UDID`].
pub fn resolve_test_udid(cli: Option<String>) -> Result<String, String> {
    resolve_test_udid_from(
        cli,
        std::env::var(IDEVICE_TEST_UDID)
            .ok()
            .filter(|value| !value.is_empty()),
    )
}

pub fn resolve_test_udid_from(cli: Option<String>, env: Option<String>) -> Result<String, String> {
    if let Some(udid) = cli.filter(|value| !value.is_empty()) {
        return Ok(udid);
    }
    if let Some(udid) = env.filter(|value| !value.is_empty()) {
        return Ok(udid);
    }
    Err(format!(
        "Pass a device UDID as an argument or set {IDEVICE_TEST_UDID}. Unattended runs never prompt."
    ))
}

/// Pops a leading UDID from `args` when present, otherwise uses the env var.
pub fn take_udid(args: &mut Vec<String>) -> String {
    let cli = args.first().cloned().filter(|value| looks_like_udid(value));
    if cli.is_some() {
        args.remove(0);
    }
    udid_or_exit(cli)
}

pub fn udid_or_exit(cli: Option<String>) -> String {
    match resolve_test_udid(cli) {
        Ok(udid) => udid,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}

pub fn requested_generation() -> Option<DeveloperGeneration> {
    std::env::var(IDEVICE_TEST_GENERATION)
        .ok()
        .filter(|value| !value.is_empty())
        .and_then(|value| DeveloperGeneration::from_slug(&value))
}

/// Bound a device operation so a wedged service cannot hang the runner.
pub async fn bounded<T, E>(
    label: &str,
    future: impl Future<Output = Result<T, E>>,
) -> CommandResult<T>
where
    E: Into<CommandError>,
{
    bounded_for(label, OP_TIMEOUT, future).await
}

pub async fn bounded_for<T, E>(
    label: &str,
    timeout: Duration,
    future: impl Future<Output = Result<T, E>>,
) -> CommandResult<T>
where
    E: Into<CommandError>,
{
    tokio::time::timeout(timeout, future)
        .await
        .map_err(|_| CommandError::new("timeout", format!("{label} timed out"), true))?
        .map_err(Into::into)
}

/// App-support path the desktop session uses for RemotePairing records.
pub fn remote_pairing_path(udid: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
        .join("Library/Application Support/dev.idevice.desktop")
        .join(format!("remote-pairing-{udid}.plist"))
}

/// Standalone [`DeviceContext`] for harnesses; production still uses the catalog.
pub fn standalone_context(udid: impl Into<String>) -> DeviceContext {
    let udid = udid.into();
    DeviceContext::standalone(udid.clone(), remote_pairing_path(&udid), None, None)
}

fn dict_bool(dict: &plist::Dictionary, keys: &[&str]) -> Option<bool> {
    keys.iter().find_map(|key| {
        let value = dict.get(key)?;
        value.as_boolean().or_else(|| match value {
            plist::Value::Integer(integer) => integer.as_signed().map(|value| value != 0),
            plist::Value::String(text) => match text.to_ascii_lowercase().as_str() {
                "true" | "yes" | "1" => Some(true),
                "false" | "no" | "0" => Some(false),
                _ => None,
            },
            _ => None,
        })
    })
}

fn locked_from_error(error: &CommandError) -> bool {
    error.kind == "idevice" && error.message.to_ascii_lowercase().contains("lock")
}

async fn inspect_one(udid: &str, connection: &Connection) -> InspectedDevice {
    let usb = matches!(connection, Connection::Usb);
    let connection_label = match connection {
        Connection::Usb => "USB".to_string(),
        Connection::Network(address) => format!("Network · {address}"),
        Connection::Unknown(value) => value.clone(),
    };
    let mut device = InspectedDevice {
        udid: udid.to_owned(),
        connection: connection_label,
        usb,
        name: None,
        product_type: None,
        ios: None,
        generation: None,
        paired: false,
        trusted: false,
        locked: None,
        developer_mode: None,
        battery_percent: None,
        charging: None,
        issues: Vec::new(),
    };
    if !usb {
        device
            .issues
            .push("not on USB; the unattended rig requires a USB cable".into());
    }

    let mut mux = match bounded("usbmuxd reconnect", UsbmuxdConnection::default()).await {
        Ok(mux) => mux,
        Err(error) => {
            device
                .issues
                .push(format!("usbmuxd reconnect failed: {error}"));
            return device;
        }
    };
    match bounded("pair record", mux.get_pair_record(udid)).await {
        Ok(_) => device.paired = true,
        Err(error) => {
            device.issues.push(format!("no pair record ({error})"));
            return device;
        }
    }

    let provider = match bounded("provider", routed_provider_for(udid, None)).await {
        Ok(provider) => provider,
        Err(error) => {
            device.issues.push(format!("no Lockdown route: {error}"));
            return device;
        }
    };

    let pairing = match bounded("pairing file", provider.get_pairing_file()).await {
        Ok(pairing) => pairing,
        Err(error) => {
            device
                .issues
                .push(format!("pairing file unreadable: {error}"));
            return device;
        }
    };

    let mut lockdown = match bounded("Lockdown connect", LockdownClient::connect(&provider)).await {
        Ok(client) => client,
        Err(error) => {
            device
                .issues
                .push(format!("Lockdown connect failed: {error}"));
            return device;
        }
    };

    match bounded("Lockdown session", lockdown.start_session(&pairing)).await {
        Ok(_) => {
            device.trusted = true;
            device.locked = Some(false);
        }
        Err(error) => {
            if locked_from_error(&error) {
                device.locked = Some(true);
                device
                    .issues
                    .push("locked; wake the phone and disable Auto-Lock".into());
            } else {
                device
                    .issues
                    .push(format!("untrusted or session failed: {error}"));
            }
            return device;
        }
    }

    if let Ok(values) = bounded("Lockdown values", lockdown.get_value(None, None)).await
        && let Some(dict) = values.as_dictionary()
    {
        device.name = dict_string(dict, &["DeviceName"]);
        device.product_type = dict_string(dict, &["ProductType"]);
        if dict_bool(dict, &["PasswordProtected"]) == Some(true) {
            device.locked = Some(true);
            device
                .issues
                .push("PasswordProtected=true; unlock the device".into());
        }
    }

    match bounded("iOS version", ios_version(&provider)).await {
        Ok(version) => {
            device.ios = Some(format!(
                "{}.{}.{}",
                version.major, version.minor, version.patch
            ));
            device.generation = Some(version.developer_generation().slug().to_owned());
            if version.major >= 16 {
                device.developer_mode = developer_mode_status(&provider).await;
                if device.developer_mode == Some(false) {
                    device.issues.push(
                        "Developer Mode is off; turn it on in Settings › Privacy & Security".into(),
                    );
                }
            }
        }
        Err(error) => device
            .issues
            .push(format!("could not read iOS version: {error}")),
    }

    let battery = bounded(
        "battery domain",
        lockdown.get_value(None, Some("com.apple.mobile.battery")),
    )
    .await
    .ok()
    .and_then(plist::Value::into_dictionary)
    .unwrap_or_default();
    device.battery_percent = dict_u64(
        &battery,
        &[
            "BatteryCurrentCapacity",
            "CurrentCapacity",
            "AbsoluteCapacity",
        ],
    );
    device.charging = dict_bool(
        &battery,
        &[
            "BatteryIsCharging",
            "IsCharging",
            "ExternalConnected",
            "ExternalChargeCapable",
        ],
    );

    device
}

async fn developer_mode_status(provider: &impl IdeviceProvider) -> Option<bool> {
    let mut client = bounded("AMFI connect", AmfiClient::connect(provider))
        .await
        .ok()?;
    bounded("Developer Mode status", client.get_developer_mode_status())
        .await
        .ok()
}

/// Enumerate usbmuxd, inspect each record, and assign one USB phone per generation.
pub async fn inspect_attached_devices() -> CommandResult<PreflightReport> {
    let mut mux = bounded("usbmuxd connect", UsbmuxdConnection::default()).await?;
    let records = bounded("usbmuxd list", mux.get_devices()).await?;

    let mut devices = Vec::new();
    for record in records {
        devices.push(inspect_one(&record.udid, &record.connection_type).await);
    }

    let mut by_generation = BTreeMap::new();
    for generation in DeveloperGeneration::ALL {
        let udid = devices
            .iter()
            .filter(|device| device.usb && !device.blocking())
            .find(|device| device.generation_slug() == Some(generation))
            .map(|device| device.udid.clone());
        by_generation.insert(generation.slug().to_owned(), udid);
    }

    let mut failures: Vec<String> = devices
        .iter()
        .filter(|device| device.blocking() || device.usb && device.generation.is_none())
        .map(InspectedDevice::issue_summary)
        .collect();
    if devices.iter().filter(|device| device.usb).count() == 0 {
        failures
            .push("no USB devices reported by usbmuxd; check cables and the powered hub".into());
    }

    let ok = failures.is_empty();
    Ok(PreflightReport {
        ok,
        devices,
        by_generation,
        failures,
    })
}

/// Fail when a required generation has no healthy USB phone.
pub fn require_generations(
    report: &PreflightReport,
    required: &[DeveloperGeneration],
) -> Result<(), String> {
    let mut missing = Vec::new();
    for generation in required {
        if report.udid_for(*generation).is_none() {
            missing.push(format!(
                "missing {}: no paired, unlocked USB phone in {}",
                generation.slug(),
                generation.label()
            ));
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing.join("\n"))
    }
}

pub fn format_human_report(report: &PreflightReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "device-rig preflight: {} usbmuxd record(s)",
        report.devices.len()
    ));
    if report.devices.is_empty() {
        lines.push("  (none)".into());
    }
    for device in &report.devices {
        lines.push(format!(
            "  {udid}  {connection}  {name}  {product}  iOS {ios}  {generation}",
            udid = device.udid,
            connection = device.connection,
            name = device.name.as_deref().unwrap_or("-"),
            product = device.product_type.as_deref().unwrap_or("-"),
            ios = device.ios.as_deref().unwrap_or("?"),
            generation = device.generation.as_deref().unwrap_or("unknown"),
        ));
        lines.push(format!(
            "    paired={} trusted={} locked={} developer_mode={} battery={} charging={}",
            device.paired,
            device.trusted,
            display_option_bool(device.locked),
            display_option_bool(device.developer_mode),
            device
                .battery_percent
                .map(|value| format!("{value}%"))
                .unwrap_or_else(|| "-".into()),
            display_option_bool(device.charging),
        ));
        for issue in &device.issues {
            lines.push(format!("    ISSUE: {issue}"));
        }
    }
    lines.push(String::new());
    lines.push("by generation:".into());
    for generation in DeveloperGeneration::ALL {
        let assigned = report.udid_for(generation).unwrap_or("(none)");
        lines.push(format!("  {}: {assigned}", generation.slug()));
    }
    if !report.failures.is_empty() {
        lines.push(String::new());
        lines.push("failures:".into());
        for failure in &report.failures {
            lines.push(format!("  - {failure}"));
        }
    }
    lines.join("\n")
}

fn display_option_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "n/a",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn udid_shape_rejects_bundle_ids_and_flags() {
        assert!(looks_like_udid("00008101-001C25D40A01001E"));
        assert!(looks_like_udid("00008020aa00112233445566778899aabbcc"));
        assert!(!looks_like_udid("com.example.app"));
        assert!(!looks_like_udid("--unmount"));
        assert!(!looks_like_udid(""));
    }

    #[test]
    fn cli_udid_wins_over_env() {
        assert_eq!(
            resolve_test_udid_from(Some("CLI".into()), Some("ENV".into())).unwrap(),
            "CLI"
        );
        assert_eq!(
            resolve_test_udid_from(None, Some("ENV".into())).unwrap(),
            "ENV"
        );
        assert!(
            resolve_test_udid_from(None, None)
                .unwrap_err()
                .contains(IDEVICE_TEST_UDID)
        );
    }

    #[test]
    fn empty_cli_falls_through_to_env() {
        assert_eq!(
            resolve_test_udid_from(Some(String::new()), Some("ENV".into())).unwrap(),
            "ENV"
        );
    }

    #[test]
    fn github_matrix_skips_missing_generations() {
        let mut by_generation = BTreeMap::new();
        by_generation.insert("legacy".into(), Some("AAA".into()));
        by_generation.insert("core-device-remote".into(), None);
        by_generation.insert("core-device-lockdown".into(), Some("CCC".into()));
        let report = PreflightReport {
            ok: true,
            devices: Vec::new(),
            by_generation,
            failures: Vec::new(),
        };
        let matrix = report.github_matrix();
        let include = matrix
            .get("include")
            .and_then(|value| value.as_array())
            .unwrap();
        assert_eq!(include.len(), 3);
        assert_eq!(include[0]["udid"], "AAA");
        assert_eq!(include[0]["skip"], false);
        assert_eq!(include[1]["skip"], true);
        assert_eq!(include[1]["udid"], "");
        assert_eq!(include[2]["udid"], "CCC");
    }

    #[test]
    fn require_generations_lists_every_gap() {
        let mut by_generation = BTreeMap::new();
        by_generation.insert("legacy".into(), Some("AAA".into()));
        by_generation.insert("core-device-remote".into(), None);
        by_generation.insert("core-device-lockdown".into(), None);
        let report = PreflightReport {
            ok: true,
            devices: Vec::new(),
            by_generation,
            failures: Vec::new(),
        };
        let error = require_generations(&report, &DeveloperGeneration::ALL).unwrap_err();
        assert!(error.contains("core-device-remote"));
        assert!(error.contains("core-device-lockdown"));
        assert!(!error.contains("missing legacy"));
        assert!(require_generations(&report, &[DeveloperGeneration::Legacy]).is_ok());
    }

    #[test]
    fn github_output_uses_underscore_keys() {
        let mut by_generation = BTreeMap::new();
        by_generation.insert("legacy".into(), Some("AAA".into()));
        by_generation.insert("core-device-remote".into(), None);
        by_generation.insert("core-device-lockdown".into(), Some("CCC".into()));
        let report = PreflightReport {
            ok: false,
            devices: Vec::new(),
            by_generation,
            failures: Vec::new(),
        };
        let mut buffer = Vec::new();
        report.write_github_output(&mut buffer).unwrap();
        let text = String::from_utf8(buffer).unwrap();
        assert!(text.contains("legacy=AAA"));
        assert!(text.contains("core_device_remote="));
        assert!(text.contains("core_device_lockdown=CCC"));
        assert!(text.contains("ok=false"));
        assert!(text.contains(r#""generation":"legacy""#));
    }

    #[test]
    fn blocking_detects_lock_and_trust_issues() {
        let mut device = InspectedDevice {
            udid: "AAA".into(),
            connection: "USB".into(),
            usb: true,
            name: None,
            product_type: None,
            ios: None,
            generation: None,
            paired: false,
            trusted: false,
            locked: Some(true),
            developer_mode: None,
            battery_percent: None,
            charging: None,
            issues: vec!["locked; wake the phone".into()],
        };
        assert!(device.blocking());
        device.issues = vec!["battery percent unknown".into()];
        assert!(!device.blocking());
        device.issues = vec!["Developer Mode is off".into()];
        assert!(device.blocking());
    }

    #[tokio::test]
    #[ignore = "requires a USB iOS device; set IDEVICE_TEST_UDID"]
    async fn hardware_selected_device_is_unlocked_and_trusted() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let udid = resolve_test_udid(None).expect("IDEVICE_TEST_UDID must be set");
        let report = inspect_attached_devices()
            .await
            .expect("usbmuxd must be reachable");
        let device = report
            .devices
            .iter()
            .find(|device| device.udid == udid)
            .unwrap_or_else(|| panic!("device {udid} is not attached via usbmuxd"));
        assert!(
            !device.blocking(),
            "selected device is not ready: {}",
            device.issue_summary()
        );
        assert!(device.trusted, "{}", device.issue_summary());
        assert_eq!(device.locked, Some(false), "{}", device.issue_summary());
        if let Some(expected) = requested_generation() {
            assert_eq!(
                device.generation_slug(),
                Some(expected),
                "IDEVICE_TEST_GENERATION={} but the phone is {:?}",
                expected.slug(),
                device.generation
            );
        }
    }
}
