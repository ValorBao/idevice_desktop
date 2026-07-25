//! Real-device check that the catalog keeps several devices apart.
//!
//! Usage: cargo run --example verify_discovery
//!
//! Feeds every usbmuxd record through `DiscoveryCatalog` the way `device_list`
//! does, then reports what the interface would show. The risk this covers is
//! two devices collapsing into one entry, or one device appearing twice when it
//! is reachable over both USB and the network.

use idevice::usbmuxd::{Connection, UsbmuxdConnection};
use idevice_desktop_lib::discovery::{DiscoveryCatalog, UsbDiscovery};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut mux = UsbmuxdConnection::default().await.expect("usbmuxd");
    let devices = mux.get_devices().await.expect("get_devices");
    println!("usbmuxd records: {}\n", devices.len());

    let mut catalog = DiscoveryCatalog::default();
    let mut enriched = Vec::new();
    for device in &devices {
        let paired = UsbmuxdConnection::default()
            .await
            .ok()
            .map(|mut mux| async move { mux.get_pair_record(&device.udid).await.is_ok() })
            .unwrap()
            .await;
        println!(
            "  {}  device_id {}  {:?}  paired={paired}",
            device.udid, device.device_id, device.connection_type
        );
        enriched.push(UsbDiscovery {
            udid: device.udid.clone(),
            device_id: device.device_id,
            // Mirrors `connection_label` in `commands/device.rs`; the catalog
            // keys the USB transport off this exact prefix.
            connection: match &device.connection_type {
                Connection::Usb => "USB".to_string(),
                Connection::Network(address) => format!("Network · {address}"),
                Connection::Unknown(value) => value.clone(),
            },
            paired,
            name: None,
            model: None,
            ios: None,
            wifi_address: None,
        });
    }
    catalog.replace_usbmuxd(enriched);

    let summaries = catalog.summaries();
    println!("\ncatalog entries: {}", summaries.len());
    for summary in &summaries {
        println!(
            "  id={}\n     udid={}\n     transports={:?}  connectable={}  paired={}",
            summary.id, summary.udid, summary.transports, summary.connectable, summary.paired
        );
    }

    let distinct: std::collections::BTreeSet<_> =
        summaries.iter().map(|summary| &summary.udid).collect();
    println!("\ndistinct udids in the catalog: {}", distinct.len());
    if summaries.len() == distinct.len() {
        println!("RESULT: PASS — every device has exactly one entry");
    } else {
        println!("RESULT: FAIL — a device appears more than once");
    }
}
