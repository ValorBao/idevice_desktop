//! Real-device check for crash-report transport selection.
//!
//! Usage: cargo run --example verify_crash_transport -- <udid>
//!
//! Mirrors `crash_client` in `commands/crash_reports.rs`: it reports which
//! transport the version and route select, then opens that transport and lists
//! reports through it. The iOS 17.4+ CoreDeviceProxy route had never been
//! exercised on hardware.

use std::time::Duration;

use idevice::{
    IdeviceService, RsdService, core_device_proxy::CoreDeviceProxy, rsd::RsdHandshake,
    services::crashreportcopymobile::CrashReportCopyMobileClient,
};
use idevice_desktop_lib::{
    device_version::{DeveloperGeneration, ios_version},
    provider::routed_provider_for,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let udid = std::env::args()
        .nth(1)
        .expect("usage: verify_crash_transport <udid>");
    let provider = routed_provider_for(&udid, None).await.expect("provider");
    let version = ios_version(&provider).await.expect("version");
    let generation = version.developer_generation();
    println!(
        "iOS {}.{}.{} -> {generation:?}, route: {}",
        version.major,
        version.minor,
        version.patch,
        if provider.is_bonjour() {
            "Bonjour"
        } else {
            "usbmuxd"
        }
    );

    // Over usbmuxd the code stays on direct Lockdown regardless of version, so
    // reaching the CoreDeviceProxy branch means driving it explicitly.
    println!("\n== direct Lockdown (what a USB route selects) ==");
    match tokio::time::timeout(
        Duration::from_secs(30),
        CrashReportCopyMobileClient::connect(&provider),
    )
    .await
    {
        Ok(Ok(mut client)) => match client.ls(None).await {
            Ok(entries) => println!("  connected, {} entries at the root", entries.len()),
            Err(error) => println!("  connected but listing failed: {error}"),
        },
        Ok(Err(error)) => println!("  FAILED: {error}"),
        Err(_) => println!("  timed out"),
    }

    if generation != DeveloperGeneration::CoreDeviceLockdown {
        println!("\nThis device does not use the CoreDeviceProxy route.");
        return;
    }

    println!("\n== CoreDeviceProxy -> RSD shim (the iOS 17.4+ network route) ==");
    let result = tokio::time::timeout(Duration::from_secs(60), async {
        let proxy = CoreDeviceProxy::connect(&provider).await?;
        let rsd_port = proxy.tunnel_info().server_rsd_port;
        println!("  tunnel info: rsd port {rsd_port}");
        let mut adapter = proxy.create_software_tunnel()?.to_async_handle();
        let stream = adapter.connect(rsd_port).await?;
        let mut handshake = RsdHandshake::new(stream).await?;
        println!("  RSD services: {}", handshake.services.len());
        let has_shim = handshake
            .services
            .keys()
            .any(|name| name.contains("crashreportcopymobile"));
        println!("  crash-report shim present: {has_shim}");
        let mut client =
            CrashReportCopyMobileClient::connect_rsd(&mut adapter, &mut handshake).await?;
        let entries = client.ls(None).await?;
        Ok::<_, idevice::IdeviceError>(entries.len())
    })
    .await;

    match result {
        Ok(Ok(count)) => println!("\nRESULT: PASS — listed {count} entries over CoreDeviceProxy"),
        Ok(Err(error)) => println!("\nRESULT: FAIL — {error}"),
        Err(_) => println!("\nRESULT: FAIL — timed out"),
    }
}
