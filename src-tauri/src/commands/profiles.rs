use std::time::{Duration, SystemTime};

use idevice::{
    IdeviceService, RsdService, core_device_proxy::CoreDeviceProxy, misagent::MisagentClient,
    rsd::RsdHandshake,
};
use plist::{Dictionary, Value};
use tauri::{AppHandle, State};

use crate::{
    device_version::{DeveloperGeneration, ios_version},
    error::{CommandError, CommandResult},
    provider::selected_provider,
    state::AppState,
    tunnel::{open_remote_pairing_tunnel, remote_pairing_path},
    types::{ProvisioningProfileSnapshot, ProvisioningProfileSummary},
};

const OPERATION_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_PROFILES: usize = 512;
const MAX_PROFILE_BYTES: usize = 8 * 1024 * 1024;
const EXPIRING_DAYS: i64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProvisioningTransport {
    Lockdown,
    RemoteRsd,
    CoreDeviceRsd,
    UsbRequired,
}

fn provisioning_transport(
    is_bonjour: bool,
    generation: DeveloperGeneration,
) -> ProvisioningTransport {
    match (is_bonjour, generation) {
        (false, _) => ProvisioningTransport::Lockdown,
        (true, DeveloperGeneration::Legacy) => ProvisioningTransport::UsbRequired,
        (true, DeveloperGeneration::CoreDeviceRemote) => ProvisioningTransport::RemoteRsd,
        (true, DeveloperGeneration::CoreDeviceLockdown) => ProvisioningTransport::CoreDeviceRsd,
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8], offset: usize) -> Option<usize> {
    haystack
        .get(offset..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|index| offset + index)
}

fn embedded_plist(profile: &[u8]) -> CommandResult<&[u8]> {
    if profile.len() > MAX_PROFILE_BYTES {
        return Err(CommandError::new(
            "provisioning_profiles",
            "Profile exceeds the 8 MB inspection limit",
            false,
        ));
    }
    let xml = find_bytes(profile, b"<?xml", 0);
    let plist = find_bytes(profile, b"<plist", 0);
    let start = match (xml, plist) {
        (Some(xml), Some(plist)) => xml.min(plist),
        (Some(xml), None) => xml,
        (None, Some(plist)) => plist,
        (None, None) => {
            return Err(CommandError::new(
                "provisioning_profiles",
                "Signed profile contains no XML plist",
                false,
            ));
        }
    };
    let close = b"</plist>";
    let end = find_bytes(profile, close, start)
        .map(|index| index + close.len())
        .ok_or_else(|| {
            CommandError::new(
                "provisioning_profiles",
                "Signed profile contains a truncated XML plist",
                false,
            )
        })?;
    Ok(&profile[start..end])
}

fn string(dictionary: &Dictionary, key: &str) -> Option<String> {
    dictionary
        .get(key)
        .and_then(Value::as_string)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn string_array(dictionary: &Dictionary, key: &str) -> Vec<String> {
    dictionary
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_string)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn expiration(date: Option<plist::Date>, now: SystemTime) -> (Option<String>, Option<i64>, String) {
    let Some(date) = date else {
        return (None, None, "unknown".into());
    };
    let system_time = SystemTime::from(date);
    let (days, state) = if system_time <= now {
        let seconds = now
            .duration_since(system_time)
            .unwrap_or_default()
            .as_secs();
        let days = -i64::try_from(seconds.div_ceil(86_400)).unwrap_or(i64::MAX);
        (days, "expired")
    } else {
        let seconds = system_time
            .duration_since(now)
            .unwrap_or_default()
            .as_secs();
        let days = i64::try_from(seconds.div_ceil(86_400)).unwrap_or(i64::MAX);
        (
            days,
            if days <= EXPIRING_DAYS {
                "expiring"
            } else {
                "valid"
            },
        )
    };
    (Some(date.to_xml_format()), Some(days), state.into())
}

fn unreadable_profile(
    index: usize,
    size_bytes: u64,
    error: CommandError,
) -> ProvisioningProfileSummary {
    ProvisioningProfileSummary {
        id: format!("unreadable-{}", index + 1),
        uuid: None,
        name: format!("Unreadable profile {}", index + 1),
        team_name: None,
        team_identifier: None,
        application_identifier: None,
        created_at: None,
        expires_at: None,
        days_remaining: None,
        expiration_state: "unknown".into(),
        profile_type: "unknown".into(),
        platforms: Vec::new(),
        device_count: 0,
        provisions_all_devices: false,
        get_task_allow: None,
        size_bytes,
        parse_error: Some(error.message),
    }
}

fn parse_profile(profile: &[u8], index: usize, now: SystemTime) -> ProvisioningProfileSummary {
    let size_bytes = u64::try_from(profile.len()).unwrap_or(u64::MAX);
    let parsed = embedded_plist(profile).and_then(|bytes| {
        plist::from_bytes::<Value>(bytes).map_err(|error| {
            CommandError::new(
                "provisioning_profiles",
                format!("Unable to parse the embedded plist: {error}"),
                false,
            )
        })
    });
    let dictionary = match parsed.and_then(|value| {
        value.into_dictionary().ok_or_else(|| {
            CommandError::new(
                "provisioning_profiles",
                "Embedded profile plist is not a dictionary",
                false,
            )
        })
    }) {
        Ok(dictionary) => dictionary,
        Err(error) => return unreadable_profile(index, size_bytes, error),
    };

    let uuid = string(&dictionary, "UUID");
    let team_identifier = string_array(&dictionary, "TeamIdentifier")
        .into_iter()
        .next()
        .or_else(|| {
            string_array(&dictionary, "ApplicationIdentifierPrefix")
                .into_iter()
                .next()
        });
    let entitlements = dictionary
        .get("Entitlements")
        .and_then(Value::as_dictionary);
    let application_identifier = entitlements.and_then(|values| {
        string(values, "application-identifier")
            .or_else(|| string(values, "com.apple.application-identifier"))
    });
    let get_task_allow = entitlements
        .and_then(|values| values.get("get-task-allow"))
        .and_then(Value::as_boolean);
    let provisioned_devices = dictionary
        .get("ProvisionedDevices")
        .and_then(Value::as_array);
    let device_count = provisioned_devices
        .map(|devices| u64::try_from(devices.len()).unwrap_or(u64::MAX))
        .unwrap_or(0);
    let provisions_all_devices = dictionary
        .get("ProvisionsAllDevices")
        .and_then(Value::as_boolean)
        .unwrap_or(false);
    let profile_type = if provisions_all_devices {
        "enterprise"
    } else if provisioned_devices.is_some() && get_task_allow == Some(true) {
        "development"
    } else if provisioned_devices.is_some() {
        "ad-hoc"
    } else {
        "app-store"
    };
    let created_at = dictionary
        .get("CreationDate")
        .and_then(Value::as_date)
        .map(|date| date.to_xml_format());
    let (expires_at, days_remaining, expiration_state) = expiration(
        dictionary.get("ExpirationDate").and_then(Value::as_date),
        now,
    );

    ProvisioningProfileSummary {
        id: uuid
            .clone()
            .unwrap_or_else(|| format!("profile-{}", index + 1)),
        uuid,
        name: string(&dictionary, "Name")
            .unwrap_or_else(|| format!("Unnamed profile {}", index + 1)),
        team_name: string(&dictionary, "TeamName"),
        team_identifier,
        application_identifier,
        created_at,
        expires_at,
        days_remaining,
        expiration_state,
        profile_type: profile_type.into(),
        platforms: string_array(&dictionary, "Platform"),
        device_count,
        provisions_all_devices,
        get_task_allow,
        size_bytes,
        parse_error: None,
    }
}

async fn load_raw_profiles(
    app: &AppHandle,
    state: &AppState,
    udid: Option<String>,
) -> CommandResult<(Vec<Vec<u8>>, String)> {
    let (udid, provider) = selected_provider(state, udid).await?;
    let generation = ios_version(&provider).await?.developer_generation();
    match provisioning_transport(provider.is_bonjour(), generation) {
        ProvisioningTransport::Lockdown => {
            let mut client = MisagentClient::connect(&provider)
                .await
                .map_err(CommandError::from)?;
            let profiles = client.copy_all().await.map_err(CommandError::from)?;
            Ok((profiles, "Misagent · Lockdown".into()))
        }
        ProvisioningTransport::UsbRequired => Err(CommandError::new(
            "provisioning_profiles",
            "Provisioning profiles over the network require iOS 17 or later. Connect the device by USB.",
            false,
        )),
        ProvisioningTransport::RemoteRsd | ProvisioningTransport::CoreDeviceRsd => {
            let (route, mut adapter, mut handshake) = match generation {
                DeveloperGeneration::CoreDeviceRemote => {
                    let pairing_path = remote_pairing_path(app, &udid)?;
                    let target = state.discovery.read().await.remote_pairing_target(&udid);
                    let tunnel = open_remote_pairing_tunnel(
                        &provider,
                        &pairing_path,
                        "idevice-desktop",
                        target.as_ref(),
                    )
                    .await?;
                    ("RemotePairing/RSD", tunnel.adapter, tunnel.handshake)
                }
                DeveloperGeneration::CoreDeviceLockdown => {
                    let proxy = CoreDeviceProxy::connect(&provider)
                        .await
                        .map_err(CommandError::from)?;
                    let rsd_port = proxy.tunnel_info().server_rsd_port;
                    let mut adapter = proxy
                        .create_software_tunnel()
                        .map_err(|error| {
                            CommandError::new(
                                "provisioning_profiles",
                                format!("Unable to create the CoreDevice tunnel: {error}"),
                                true,
                            )
                        })?
                        .to_async_handle();
                    let stream = adapter.connect(rsd_port).await.map_err(|error| {
                        CommandError::new(
                            "provisioning_profiles",
                            format!("Unable to connect to tunneled RSD: {error}"),
                            true,
                        )
                    })?;
                    let handshake = RsdHandshake::new(stream)
                        .await
                        .map_err(CommandError::from)?;
                    ("CoreDeviceProxy/RSD", adapter, handshake)
                }
                DeveloperGeneration::Legacy => unreachable!(),
            };
            let mut client = MisagentClient::connect_rsd(&mut adapter, &mut handshake)
                .await
                .map_err(|error| {
                    CommandError::new(
                        "provisioning_profiles",
                        format!("The network Misagent service is unavailable: {error}"),
                        true,
                    )
                })?;
            let profiles = client.copy_all().await.map_err(CommandError::from)?;
            Ok((profiles, format!("Misagent shim · {route}")))
        }
    }
}

#[tauri::command]
pub async fn provisioning_profiles_list(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<ProvisioningProfileSnapshot> {
    let (raw, transport) =
        tokio::time::timeout(OPERATION_TIMEOUT, load_raw_profiles(&app, &state, udid))
            .await
            .map_err(|_| {
                CommandError::new(
                    "provisioning_profiles",
                    "Timed out reading provisioning profiles. Keep the device unlocked and retry.",
                    true,
                )
            })??;
    let total_count = u64::try_from(raw.len()).unwrap_or(u64::MAX);
    let truncated = raw.len() > MAX_PROFILES;
    let now = SystemTime::now();
    let profiles = tokio::task::spawn_blocking(move || {
        raw.into_iter()
            .take(MAX_PROFILES)
            .enumerate()
            .map(|(index, profile)| parse_profile(&profile, index, now))
            .collect()
    })
    .await
    .map_err(|error| CommandError::new("runtime", error.to_string(), true))?;
    Ok(ProvisioningProfileSnapshot {
        profiles,
        transport,
        total_count,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signed_profile(dictionary: Dictionary) -> Vec<u8> {
        let mut xml = Vec::new();
        plist::to_writer_xml(&mut xml, &Value::Dictionary(dictionary)).unwrap();
        let mut signed = b"signed-prefix".to_vec();
        signed.extend(xml);
        signed.extend_from_slice(b"signed-suffix");
        signed
    }

    fn date(value: &str) -> plist::Date {
        plist::Date::from_xml_format(value).unwrap()
    }

    fn base_profile(expiration: &str) -> Dictionary {
        let mut entitlements = Dictionary::new();
        entitlements.insert(
            "application-identifier".into(),
            Value::String("TEAM123.com.example.app".into()),
        );
        entitlements.insert("get-task-allow".into(), Value::Boolean(true));
        let mut dictionary = Dictionary::new();
        dictionary.insert("UUID".into(), Value::String("PROFILE-UUID".into()));
        dictionary.insert("Name".into(), Value::String("Example Development".into()));
        dictionary.insert("TeamName".into(), Value::String("Example Team".into()));
        dictionary.insert(
            "TeamIdentifier".into(),
            Value::Array(vec![Value::String("TEAM123".into())]),
        );
        dictionary.insert(
            "Platform".into(),
            Value::Array(vec![Value::String("iOS".into())]),
        );
        dictionary.insert(
            "CreationDate".into(),
            Value::Date(date("2026-01-01T00:00:00Z")),
        );
        dictionary.insert("ExpirationDate".into(), Value::Date(date(expiration)));
        dictionary.insert(
            "ProvisionedDevices".into(),
            Value::Array(vec![Value::String("device-1".into())]),
        );
        dictionary.insert("Entitlements".into(), Value::Dictionary(entitlements));
        dictionary
    }

    #[test]
    fn extracts_and_normalizes_a_development_profile() {
        let raw = signed_profile(base_profile("2026-09-01T00:00:00Z"));
        let now = SystemTime::from(date("2026-08-09T00:00:00Z"));
        let profile = parse_profile(&raw, 0, now);

        assert_eq!(profile.id, "PROFILE-UUID");
        assert_eq!(profile.name, "Example Development");
        assert_eq!(profile.team_identifier.as_deref(), Some("TEAM123"));
        assert_eq!(
            profile.application_identifier.as_deref(),
            Some("TEAM123.com.example.app")
        );
        assert_eq!(profile.profile_type, "development");
        assert_eq!(profile.device_count, 1);
        assert_eq!(profile.expiration_state, "expiring");
        assert_eq!(profile.days_remaining, Some(23));
        assert_eq!(profile.parse_error, None);
    }

    #[test]
    fn classifies_app_store_enterprise_and_expired_profiles() {
        let now = SystemTime::from(date("2026-08-09T00:00:00Z"));
        let mut app_store = base_profile("2027-01-01T00:00:00Z");
        app_store.remove("ProvisionedDevices");
        assert_eq!(
            parse_profile(&signed_profile(app_store), 0, now).profile_type,
            "app-store"
        );

        let mut enterprise = base_profile("2026-08-01T00:00:00Z");
        enterprise.insert("ProvisionsAllDevices".into(), Value::Boolean(true));
        enterprise.remove("ProvisionedDevices");
        let enterprise = parse_profile(&signed_profile(enterprise), 1, now);
        assert_eq!(enterprise.profile_type, "enterprise");
        assert_eq!(enterprise.expiration_state, "expired");
        assert_eq!(enterprise.days_remaining, Some(-8));
    }

    #[test]
    fn preserves_an_unreadable_profile_as_a_visible_row() {
        let profile = parse_profile(b"not a signed profile", 2, SystemTime::now());
        assert_eq!(profile.id, "unreadable-3");
        assert_eq!(profile.expiration_state, "unknown");
        assert!(
            profile
                .parse_error
                .as_deref()
                .unwrap()
                .contains("no XML plist")
        );
    }

    #[test]
    fn rejects_truncated_and_oversized_profile_payloads() {
        assert!(embedded_plist(b"prefix<plist><dict></dict>").is_err());
        assert!(embedded_plist(&vec![0; MAX_PROFILE_BYTES + 1]).is_err());
    }

    #[test]
    fn selects_safe_transports_for_each_generation() {
        assert_eq!(
            provisioning_transport(false, DeveloperGeneration::Legacy),
            ProvisioningTransport::Lockdown
        );
        assert_eq!(
            provisioning_transport(true, DeveloperGeneration::Legacy),
            ProvisioningTransport::UsbRequired
        );
        assert_eq!(
            provisioning_transport(true, DeveloperGeneration::CoreDeviceRemote),
            ProvisioningTransport::RemoteRsd
        );
        assert_eq!(
            provisioning_transport(true, DeveloperGeneration::CoreDeviceLockdown),
            ProvisioningTransport::CoreDeviceRsd
        );
    }
}
