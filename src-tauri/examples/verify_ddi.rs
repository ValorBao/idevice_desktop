//! Real-device harness for the Developer Disk Image state and unmount path.
//!
//! Usage:
//!   cargo run --example verify_ddi -- <udid>             # report state only
//!   cargo run --example verify_ddi -- <udid> --unmount   # report, unmount, re-report
//!
//! Reports every signal the project uses to decide whether a DDI is mounted, so
//! they can be compared against each other. `developer_status` treats a
//! non-empty `copy_devices` as mounted and falls back to devicectl on macOS;
//! this harness shows what each source actually returns.
//!
//! The unmount step mirrors `ddi_unmount` in `commands/developer.rs`. Every step
//! is bounded so a stalled service cannot hang the run.

use std::time::Duration;

use idevice::{IdeviceService, mobile_image_mounter::ImageMounter};
use idevice_desktop_lib::{
    device_version::{DeveloperGeneration, IosVersion, ios_version},
    provider::{RoutedProvider, routed_provider_for},
    tunnel::open_remote_pairing_tunnel,
};

const STEP_TIMEOUT: Duration = Duration::from_secs(20);
/// Opening a tunnel right after an unmount has been observed to stall well past
/// the normal handshake time, so it gets a longer bound of its own.
const TUNNEL_TIMEOUT: Duration = Duration::from_secs(45);

fn mount_point(version: IosVersion) -> &'static str {
    if version.developer_generation() == DeveloperGeneration::Legacy {
        "/Developer"
    } else {
        "/System/Developer"
    }
}

async fn report_state(provider: &RoutedProvider, udid: &str, label: &str) {
    println!("\n== {label} ==");

    match tokio::time::timeout(STEP_TIMEOUT, async {
        let mut mounter = ImageMounter::connect(provider).await?;
        mounter.copy_devices().await
    })
    .await
    {
        Ok(Ok(images)) => println!("  copy_devices:              {} image(s)", images.len()),
        Ok(Err(error)) => println!("  copy_devices:              error ({error})"),
        Err(_) => println!("  copy_devices:              timed out"),
    }

    for kind in ["Developer", "Personalized"] {
        match tokio::time::timeout(STEP_TIMEOUT, async {
            let mut mounter = ImageMounter::connect(provider).await?;
            mounter.lookup_image(kind).await
        })
        .await
        {
            Ok(Ok(signature)) => {
                println!("  lookup_image({kind}): {} byte(s)", signature.len())
            }
            Ok(Err(error)) => println!("  lookup_image({kind}): {error}"),
            Err(_) => println!("  lookup_image({kind}): timed out"),
        }
    }

    // The RSD service list is the ground truth: the debug proxy only exists once
    // a DDI is mounted.
    let pairing_path = std::path::PathBuf::from(std::env::var("HOME").expect("HOME")).join(
        format!("Library/Application Support/dev.idevice.desktop/remote-pairing-{udid}.plist"),
    );
    match tokio::time::timeout(
        TUNNEL_TIMEOUT,
        open_remote_pairing_tunnel(provider, &pairing_path, "idevice-desktop", None),
    )
    .await
    {
        Ok(Ok(tunnel)) => {
            let total = tunnel.handshake.services.len();
            let debug = tunnel
                .handshake
                .services
                .keys()
                .filter(|name| name.contains("debugproxy") || name.contains("debugserverproxy"))
                .count();
            println!("  RSD services:              {total} ({debug} debug-proxy)");
        }
        Ok(Err(error)) => println!("  RSD services:              unavailable ({error:?})"),
        Err(_) => println!("  RSD services:              tunnel timed out"),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut args = std::env::args().skip(1);
    let udid = args.next().expect("usage: verify_ddi <udid> [--unmount]");
    let unmount = args.next().as_deref() == Some("--unmount");

    let provider = routed_provider_for(&udid, None).await.expect("provider");
    let version = ios_version(&provider).await.expect("version");
    println!(
        "iOS {}.{}.{} -> {:?}",
        version.major,
        version.minor,
        version.patch,
        version.developer_generation()
    );

    report_state(&provider, &udid, "current state").await;

    if !unmount {
        return;
    }

    let target = mount_point(version);
    println!("\n== unmounting {target} ==");
    match tokio::time::timeout(STEP_TIMEOUT, async {
        let mut mounter = ImageMounter::connect(&provider).await?;
        mounter.unmount_image(target).await
    })
    .await
    {
        Ok(Ok(())) => println!("  ok"),
        Ok(Err(error)) => println!("  FAILED: {error}"),
        Err(_) => println!("  timed out"),
    }

    report_state(&provider, &udid, "after unmount").await;
}
