//! Real-device verification harness for the pairing path.
//!
//! Usage:
//!   cargo run --example verify_pairing                    # non-destructive checks
//!   cargo run --example verify_pairing -- --repair <udid> # DESTRUCTIVE: unpair then re-pair
//!
//! The non-destructive run exercises `device_pair`'s transport guard, the saved
//! pair-record lookup, and `routed_provider_for` route selection against every
//! device usbmuxd currently reports. The `--repair` run additionally calls the
//! real `device_forget` and `device_pair`, which drops the trust relationship and
//! requires tapping "Trust" on the device.

use idevice::{
    IdeviceService,
    lockdown::LockdownClient,
    provider::IdeviceProvider,
    usbmuxd::{Connection, UsbmuxdConnection},
};
use idevice_desktop_lib::{commands, provider::routed_provider_for};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--repair") {
        let udid = args.get(1).expect("usage: verify_pairing --repair <udid>");
        repair(udid).await;
        return;
    }

    let mut mux = UsbmuxdConnection::default().await.expect("usbmuxd");
    let devices = mux.get_devices().await.expect("get_devices");
    println!("== pairing verification: {} device(s) ==\n", devices.len());

    for device in devices {
        let udid = device.udid.clone();
        let usb = device.connection_type == Connection::Usb;
        println!("-- {udid}");
        println!(
            "   usbmuxd connection: {:?} (device_id {})",
            device.connection_type, device.device_id
        );

        // 1. Saved pair record.
        match mux.get_pair_record(&udid).await {
            Ok(record) => println!("   pair record: present (host_id {})", record.host_id),
            Err(error) => println!("   pair record: MISSING ({error})"),
        }

        // 2. Route selection, then a real paired Lockdown session.
        match routed_provider_for(&udid, None).await {
            Ok(provider) => {
                println!(
                    "   routed_provider_for: {}",
                    if provider.is_bonjour() {
                        "Bonjour TCP"
                    } else {
                        "usbmuxd"
                    }
                );
                match LockdownClient::connect(&provider).await {
                    Ok(mut lockdown) => match provider.get_pairing_file().await {
                        Ok(pairing) => match lockdown.start_session(&pairing).await {
                            Ok(_) => {
                                let version = lockdown
                                    .get_value(Some("ProductVersion"), None)
                                    .await
                                    .ok()
                                    .and_then(|value| {
                                        value.as_string().map(std::string::ToString::to_string)
                                    })
                                    .unwrap_or_else(|| "<unreadable>".into());
                                println!(
                                    "   paired Lockdown session: ok (ProductVersion {version})"
                                );
                            }
                            Err(error) => println!("   paired Lockdown session: FAILED ({error})"),
                        },
                        Err(error) => println!("   pairing file: FAILED ({error})"),
                    },
                    Err(error) => println!("   Lockdown connect: FAILED ({error})"),
                }
            }
            Err(error) => println!("   routed_provider_for: FAILED ({error:?})"),
        }

        // 3. device_pair's transport guard. Only safe to call on a non-USB
        //    record, where it must refuse before touching the trust relationship.
        if usb {
            println!("   device_pair guard: skipped (USB record; calling it would re-pair)");
        } else {
            match commands::device_pair(udid.clone(), Some("verify-harness".into())).await {
                Ok(_) => println!("   device_pair guard: UNEXPECTED SUCCESS on a non-USB record"),
                Err(error) => println!("   device_pair guard: refused as expected ({error:?})"),
            }
        }
        println!();
    }
}

async fn repair(udid: &str) {
    use idevice_desktop_lib::commands::device_pair;
    println!("== DESTRUCTIVE re-pair for {udid} ==");
    println!("This drops the existing trust relationship.");
    println!("Unlock the device and tap \"Trust\" when prompted.\n");

    // device_forget needs AppState, so drive the same two steps it performs.
    let provider = routed_provider_for(udid, None).await.expect("provider");
    let pairing_file = provider.get_pairing_file().await.expect("pairing file");
    if let Ok(mut lockdown) = LockdownClient::connect(&provider).await {
        match lockdown.unpair(pairing_file.host_id).await {
            Ok(()) => println!("unpair: ok"),
            Err(error) => println!("unpair: {error}"),
        }
    }
    let mut mux = UsbmuxdConnection::default().await.expect("usbmuxd");
    match mux.delete_pair_record(udid).await {
        Ok(()) => println!("delete_pair_record: ok"),
        Err(error) => println!("delete_pair_record: {error}"),
    }

    println!("\nnow calling the real device_pair ...");
    println!("(tap Trust on the device within 30 seconds)");
    match device_pair(udid.to_string(), Some("idevice-desktop-verify".into())).await {
        Ok(summary) => println!(
            "device_pair: ok — {} {} iOS {} paired={} connectable={}",
            summary.name.as_deref().unwrap_or("<none>"),
            summary.model.as_deref().unwrap_or("<none>"),
            summary.ios.as_deref().unwrap_or("<none>"),
            summary.paired,
            summary.connectable
        ),
        Err(error) => println!("device_pair: FAILED {error:?}"),
    }
}
