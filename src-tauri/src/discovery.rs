use std::collections::HashMap;

use mdns_sd::ResolvedService;

use crate::types::DeviceSummary;

pub const MOBDEV2_SERVICE: &str = "_apple-mobdev2._tcp.local.";
pub const REMOTE_PAIRING_SERVICE: &str = "_remotepairing._tcp.local.";
pub const REMOTE_PAIRING_MANUAL_SERVICE: &str = "_remotepairing-manual-pairing._tcp.local.";
pub const NETWORK_SERVICE_TYPES: &[&str] = &[
    MOBDEV2_SERVICE,
    REMOTE_PAIRING_SERVICE,
    REMOTE_PAIRING_MANUAL_SERVICE,
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemotePairingTarget {
    pub hostname: String,
    pub port: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockdownTarget {
    pub hostname: String,
    pub addresses: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsbDiscovery {
    pub udid: String,
    pub device_id: u32,
    pub connection: String,
    pub paired: bool,
    pub name: Option<String>,
    pub model: Option<String>,
    pub ios: Option<String>,
    pub wifi_address: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BonjourEndpoint {
    pub fullname: String,
    pub service_type: String,
    pub hostname: String,
    pub port: u16,
    pub addresses: Vec<String>,
    pub udid: Option<String>,
    pub wifi_address: Option<String>,
    pub name: Option<String>,
}

impl BonjourEndpoint {
    pub fn from_resolved(service: &ResolvedService) -> Self {
        let service_type = service.ty_domain.clone();
        let fullname = service.get_fullname().to_string();
        let instance = service_instance(&fullname, &service_type);
        let udid = ["udid", "identifier", "UniqueDeviceID", "SerialNumber"]
            .into_iter()
            .find_map(|key| service.get_property_val_str(key))
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let (wifi_address, name) = if service_type == MOBDEV2_SERVICE {
            instance
                .split_once('@')
                .map(|(wifi, name)| (normalize_mac(wifi), nonempty(name)))
                .unwrap_or((None, nonempty(&instance)))
        } else {
            (None, None)
        };
        let mut addresses = service
            .get_addresses()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        addresses.sort();
        Self {
            fullname,
            service_type,
            hostname: service.get_hostname().trim_end_matches('.').to_string(),
            port: service.get_port(),
            addresses,
            udid,
            wifi_address,
            name,
        }
    }

    fn transport_label(&self) -> &'static str {
        match self.service_type.as_str() {
            MOBDEV2_SERVICE => "Wi-Fi",
            REMOTE_PAIRING_MANUAL_SERVICE => "RemotePairing Setup",
            _ => "RemotePairing",
        }
    }
}

#[derive(Clone, Debug, Default)]
struct DiscoveryRecord {
    id: String,
    udid: Option<String>,
    usb: Option<UsbDiscovery>,
    bonjour: HashMap<String, BonjourEndpoint>,
    wifi_address: Option<String>,
    name: Option<String>,
    model: Option<String>,
    ios: Option<String>,
    paired: bool,
}

impl DiscoveryRecord {
    fn from_id(id: String) -> Self {
        Self {
            id,
            ..Self::default()
        }
    }

    fn summary(&self) -> DeviceSummary {
        let mut transports = Vec::new();
        if let Some(usb) = &self.usb {
            transports.push(
                if usb.connection.starts_with("USB") {
                    "USB"
                } else {
                    "usbmuxd Network"
                }
                .to_string(),
            );
        }
        for endpoint in self.bonjour.values() {
            let label = endpoint.transport_label().to_string();
            if !transports.contains(&label) {
                transports.push(label);
            }
        }
        transports.sort_by_key(|value| match value.as_str() {
            "USB" => 0,
            "usbmuxd Network" => 1,
            "Wi-Fi" => 2,
            "RemotePairing" => 3,
            _ => 4,
        });
        let connection = self
            .usb
            .as_ref()
            .map(|usb| usb.connection.clone())
            .unwrap_or_else(|| {
                let labels = transports.join(" + ");
                if labels.is_empty() {
                    "Network · Bonjour".to_string()
                } else {
                    format!("Network · {labels}")
                }
            });
        DeviceSummary {
            id: self.id.clone(),
            udid: self.udid.clone().unwrap_or_default(),
            device_id: self.usb.as_ref().map(|usb| usb.device_id).unwrap_or(0),
            connection,
            transports,
            connectable: self.usb.is_some()
                || (self.udid.is_some()
                    && self
                        .bonjour
                        .values()
                        .any(|endpoint| endpoint.service_type == MOBDEV2_SERVICE)),
            paired: self
                .usb
                .as_ref()
                .map(|usb| usb.paired)
                .unwrap_or(self.paired),
            name: self.name.clone(),
            model: self.model.clone(),
            ios: self.ios.clone(),
        }
    }

    fn absorb(&mut self, other: DiscoveryRecord) {
        self.usb = self.usb.take().or(other.usb);
        self.udid = self.udid.take().or(other.udid);
        self.wifi_address = self.wifi_address.take().or(other.wifi_address);
        self.name = self.name.take().or(other.name);
        self.model = self.model.take().or(other.model);
        self.ios = self.ios.take().or(other.ios);
        self.paired |= other.paired;
        self.bonjour.extend(other.bonjour);
    }

    fn is_empty(&self) -> bool {
        self.usb.is_none() && self.bonjour.is_empty()
    }

    fn is_visible(&self) -> bool {
        self.usb.is_some()
            || self.bonjour.values().any(|endpoint| {
                endpoint.service_type == MOBDEV2_SERVICE
                    || endpoint.service_type == REMOTE_PAIRING_MANUAL_SERVICE
            })
    }
}

#[derive(Debug, Default)]
pub struct DiscoveryCatalog {
    records: HashMap<String, DiscoveryRecord>,
}

impl DiscoveryCatalog {
    pub fn replace_usbmuxd(&mut self, devices: Vec<UsbDiscovery>) {
        for record in self.records.values_mut() {
            record.usb = None;
        }
        for device in devices {
            self.upsert_usb(device);
        }
        self.records.retain(|_, record| !record.is_empty());
    }

    pub fn upsert_usb(&mut self, device: UsbDiscovery) -> DeviceSummary {
        let id = device.udid.clone();
        let wifi_address = device.wifi_address.as_deref().and_then(normalize_mac);
        let merge_keys = self.matching_keys(Some(&device.udid), wifi_address.as_deref(), None, &[]);
        let mut record = self
            .records
            .remove(&id)
            .unwrap_or_else(|| DiscoveryRecord::from_id(id.clone()));
        for merge_key in merge_keys {
            if merge_key != id
                && let Some(other) = self.records.remove(&merge_key)
            {
                record.absorb(other);
            }
        }
        record.id = id.clone();
        record.udid = Some(device.udid.clone());
        record.wifi_address = wifi_address;
        record.name = device.name.clone().or(record.name);
        record.model = device.model.clone().or(record.model);
        record.ios = device.ios.clone().or(record.ios);
        record.paired = device.paired;
        record.usb = Some(device);
        let summary = record.summary();
        self.records.insert(id, record);
        summary
    }

    pub fn remove_usbmuxd(&mut self, device_id: u32) {
        let key = self.records.iter().find_map(|(key, record)| {
            (record.usb.as_ref().map(|usb| usb.device_id) == Some(device_id)).then(|| key.clone())
        });
        if let Some(key) = key {
            let remove = if let Some(record) = self.records.get_mut(&key) {
                record.usb = None;
                record.is_empty()
            } else {
                false
            };
            if remove {
                self.records.remove(&key);
            }
        }
    }

    pub fn upsert_bonjour(&mut self, endpoint: BonjourEndpoint) -> DeviceSummary {
        let wifi_address = endpoint.wifi_address.as_deref().and_then(normalize_mac);
        let merge_keys = self.matching_keys(
            endpoint.udid.as_deref(),
            wifi_address.as_deref(),
            Some(&endpoint.hostname),
            &endpoint.addresses,
        );
        let fallback_key = endpoint.udid.clone().unwrap_or_else(|| {
            format!(
                "bonjour:{}",
                endpoint
                    .wifi_address
                    .clone()
                    .unwrap_or_else(|| endpoint.fullname.clone())
            )
        });
        let mut record = DiscoveryRecord::from_id(fallback_key.clone());
        for merge_key in merge_keys {
            if let Some(other) = self.records.remove(&merge_key) {
                record.absorb(other);
            }
        }
        record.udid = endpoint.udid.clone().or(record.udid);
        record.wifi_address = wifi_address.or(record.wifi_address);
        record.name = endpoint.name.clone().or(record.name);
        record.bonjour.insert(endpoint.fullname.clone(), endpoint);
        let key = record.udid.clone().unwrap_or(fallback_key);
        record.id = key.clone();
        let summary = record.summary();
        self.records.insert(key, record);
        summary
    }

    pub fn remove_bonjour(&mut self, fullname: &str) {
        let key = self
            .records
            .iter()
            .find_map(|(key, record)| record.bonjour.contains_key(fullname).then(|| key.clone()));
        if let Some(key) = key {
            let remove = if let Some(record) = self.records.get_mut(&key) {
                record.bonjour.remove(fullname);
                record.is_empty()
            } else {
                false
            };
            if remove {
                self.records.remove(&key);
            }
        }
    }

    pub fn summaries(&self) -> Vec<DeviceSummary> {
        let mut devices = self
            .records
            .values()
            .filter(|record| record.is_visible())
            .map(DiscoveryRecord::summary)
            .collect::<Vec<_>>();
        devices.sort_by(|left, right| {
            right.connectable.cmp(&left.connectable).then_with(|| {
                left.name
                    .as_deref()
                    .unwrap_or(&left.id)
                    .to_lowercase()
                    .cmp(&right.name.as_deref().unwrap_or(&right.id).to_lowercase())
            })
        });
        devices
    }

    pub fn connectable_udid(&self, id: &str) -> Option<String> {
        self.records
            .get(id)
            .filter(|record| {
                record.usb.is_some()
                    || record
                        .bonjour
                        .values()
                        .any(|endpoint| endpoint.service_type == MOBDEV2_SERVICE)
            })
            .and_then(|record| record.udid.clone())
    }

    pub fn lockdown_target(&self, id: &str) -> Option<LockdownTarget> {
        let record = self.record(id)?;
        record
            .bonjour
            .values()
            .find(|endpoint| endpoint.service_type == MOBDEV2_SERVICE)
            .map(|endpoint| LockdownTarget {
                hostname: endpoint.hostname.clone(),
                addresses: endpoint.addresses.clone(),
            })
    }

    pub fn remote_pairing_target(&self, id: &str) -> Option<RemotePairingTarget> {
        let record = self.record(id)?;
        record
            .bonjour
            .values()
            .find(|endpoint| endpoint.service_type == REMOTE_PAIRING_SERVICE)
            .map(|endpoint| RemotePairingTarget {
                hostname: endpoint.hostname.clone(),
                port: endpoint.port,
            })
    }

    pub fn contains(&self, id: &str) -> bool {
        self.records.contains_key(id)
    }

    fn record(&self, id: &str) -> Option<&DiscoveryRecord> {
        self.records.get(id).or_else(|| {
            self.records
                .values()
                .find(|record| record.udid.as_deref() == Some(id))
        })
    }

    fn matching_keys(
        &self,
        udid: Option<&str>,
        wifi_address: Option<&str>,
        hostname: Option<&str>,
        addresses: &[String],
    ) -> Vec<String> {
        self.records
            .iter()
            .filter(|(_, record)| record_matches(record, udid, wifi_address, hostname, addresses))
            .map(|(key, _)| key.clone())
            .collect()
    }
}

fn record_matches(
    record: &DiscoveryRecord,
    udid: Option<&str>,
    wifi_address: Option<&str>,
    hostname: Option<&str>,
    addresses: &[String],
) -> bool {
    udid.is_some_and(|udid| record.id == udid || record.udid.as_deref() == Some(udid))
        || (wifi_address.is_some() && record.wifi_address.as_deref() == wifi_address)
        || record.bonjour.values().any(|known| {
            hostname.is_some_and(|hostname| known.hostname == hostname)
                || known
                    .addresses
                    .iter()
                    .any(|address| addresses.contains(address))
        })
}

fn nonempty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn normalize_mac(value: &str) -> Option<String> {
    let normalized = value
        .chars()
        .filter(|character| character.is_ascii_hexdigit())
        .map(|character| character.to_ascii_uppercase())
        .collect::<String>();
    (normalized.len() == 12).then_some(normalized)
}

fn service_instance(fullname: &str, service_type: &str) -> String {
    fullname
        .trim_end_matches('.')
        .strip_suffix(service_type.trim_end_matches('.'))
        .unwrap_or(fullname)
        .trim_end_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usb(udid: &str, wifi: &str) -> UsbDiscovery {
        UsbDiscovery {
            udid: udid.into(),
            device_id: 7,
            connection: "USB".into(),
            paired: true,
            name: Some("Test iPhone".into()),
            model: Some("iPhone11,8".into()),
            ios: Some("17.0".into()),
            wifi_address: Some(wifi.into()),
        }
    }

    fn mobdev(wifi: &str) -> BonjourEndpoint {
        BonjourEndpoint {
            fullname: format!("{wifi}@Test iPhone.{MOBDEV2_SERVICE}"),
            service_type: MOBDEV2_SERVICE.into(),
            hostname: "test.local".into(),
            port: 62078,
            addresses: vec!["192.168.1.20".into()],
            udid: None,
            wifi_address: normalize_mac(wifi),
            name: Some("Test iPhone".into()),
        }
    }

    fn remote_pairing() -> BonjourEndpoint {
        BonjourEndpoint {
            fullname: "pairing._remotepairing._tcp.local.".into(),
            service_type: REMOTE_PAIRING_SERVICE.into(),
            hostname: "test.local".into(),
            port: 49_152,
            addresses: vec!["192.168.1.20".into()],
            udid: None,
            wifi_address: None,
            name: None,
        }
    }

    #[test]
    fn merges_bonjour_and_usb_by_wifi_address() {
        let mut catalog = DiscoveryCatalog::default();
        let network = catalog.upsert_bonjour(mobdev("AA:BB:CC:DD:EE:FF"));
        assert!(!network.connectable);
        let merged = catalog.upsert_usb(usb("device-udid", "aa-bb-cc-dd-ee-ff"));
        assert!(merged.connectable);
        assert_eq!(merged.id, "device-udid");
        assert_eq!(merged.transports, vec!["USB", "Wi-Fi"]);
        assert_eq!(catalog.summaries().len(), 1);
    }

    #[test]
    fn keeps_network_record_after_usb_disconnect() {
        let mut catalog = DiscoveryCatalog::default();
        catalog.upsert_bonjour(mobdev("AA:BB:CC:DD:EE:FF"));
        catalog.upsert_usb(usb("device-udid", "AA:BB:CC:DD:EE:FF"));
        catalog.remove_usbmuxd(7);
        let devices = catalog.summaries();
        assert_eq!(devices.len(), 1);
        assert!(devices[0].connectable);
        assert!(devices[0].paired);
        assert_eq!(devices[0].connection, "Network · Wi-Fi");
    }

    #[test]
    fn replaces_stale_usbmuxd_snapshot_without_losing_bonjour() {
        let mut catalog = DiscoveryCatalog::default();
        catalog.upsert_bonjour(mobdev("AA:BB:CC:DD:EE:FF"));
        catalog.upsert_usb(usb("device-udid", "AA:BB:CC:DD:EE:FF"));
        catalog.replace_usbmuxd(Vec::new());
        assert_eq!(catalog.summaries().len(), 1);
        assert!(catalog.summaries()[0].connectable);
        assert!(catalog.summaries()[0].paired);
    }

    #[test]
    fn resolves_remote_pairing_target_for_merged_device() {
        let mut catalog = DiscoveryCatalog::default();
        catalog.upsert_usb(usb("device-udid", "AA:BB:CC:DD:EE:FF"));
        catalog.upsert_bonjour(mobdev("AA:BB:CC:DD:EE:FF"));
        catalog.upsert_bonjour(remote_pairing());

        assert_eq!(
            catalog.remote_pairing_target("device-udid"),
            Some(RemotePairingTarget {
                hostname: "test.local".into(),
                port: 49_152,
            })
        );
        assert_eq!(
            catalog.lockdown_target("device-udid"),
            Some(LockdownTarget {
                hostname: "test.local".into(),
                addresses: vec!["192.168.1.20".into()],
            })
        );
    }

    #[test]
    fn coalesces_remote_first_observations_when_mobdev_bridges_usb() {
        let mut catalog = DiscoveryCatalog::default();
        catalog.upsert_bonjour(remote_pairing());
        assert!(catalog.summaries().is_empty());

        catalog.upsert_usb(usb("device-udid", "AA:BB:CC:DD:EE:FF"));
        assert_eq!(catalog.summaries().len(), 1);

        catalog.upsert_bonjour(mobdev("AA:BB:CC:DD:EE:FF"));
        let devices = catalog.summaries();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, "device-udid");
        assert_eq!(devices[0].transports, vec!["USB", "Wi-Fi", "RemotePairing"]);
        assert_eq!(
            catalog.remote_pairing_target("device-udid"),
            Some(RemotePairingTarget {
                hostname: "test.local".into(),
                port: 49_152,
            })
        );
    }
}
