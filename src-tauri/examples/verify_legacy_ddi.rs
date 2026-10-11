//! Host and device harness for the Legacy Developer Disk Image path.
//!
//! Usage:
//!   cargo run --example verify_legacy_ddi                     # report install state only
//!   cargo run --example verify_legacy_ddi -- --install        # download and install if missing
//!   cargo run --example verify_legacy_ddi -- --mount <udid>   # full ensure path against a device
//!   cargo run --example verify_legacy_ddi -- --developer-mode <udid> [--accept]
//!   cargo run --example verify_legacy_ddi -- --mount-file <udid> <dmg> <signature>
//!
//! Every iOS 16 or earlier device mounts one pinned image, which is downloaded
//! on request rather than shipped. The report and download modes exercise the
//! production download, checksum, and install path with no device attached.
//!
//! `--mount` mirrors `ddi_ensure` and `ddi_download` in `commands/developer.rs`:
//! it reads the version, checks whether an image is already mounted, downloads
//! one if this Mac has none, mounts it, and re-reads every mounted-image signal
//! afterwards. It refuses anything that is not a Legacy device, so it cannot be
//! pointed at the iOS 17 path by accident.

use std::time::Duration;

use idevice::{IdeviceService, amfi::AmfiClient, mobile_image_mounter::ImageMounter};
use idevice_desktop_lib::{
    device_version::{DeveloperGeneration, ios_version},
    legacy_ddi,
    provider::routed_provider_for,
};

const STEP_TIMEOUT: Duration = Duration::from_secs(120);

fn report_install_state() -> Option<legacy_ddi::LegacyDdi> {
    println!("Pinned image: iOS {}", legacy_ddi::IMAGE_VERSION);
    println!("Source:       {}", legacy_ddi::source_description());
    println!("Expected:     {} bytes", legacy_ddi::IMAGE_BYTES);

    println!("\nSearched, in order:");
    for directory in legacy_ddi::search_directories() {
        let state = if directory.is_dir() {
            "present"
        } else {
            "missing"
        };
        println!("  [{state}] {}", directory.display());
    }

    match legacy_ddi::installed() {
        Some(found) => {
            let size = std::fs::metadata(&found.image)
                .map(|data| data.len())
                .unwrap_or(0);
            println!("\nInstalled: {} ({size} bytes)", found.image.display());
            println!("Signature: {}", found.signature.display());
            Some(found)
        }
        None => {
            println!("\nInstalled: no");
            None
        }
    }
}

async fn install() -> Option<legacy_ddi::LegacyDdi> {
    println!("\nDownloading…");
    // Reporting every chunk would flood the terminal; report each tenth once.
    let mut reported = 0;
    let result = legacy_ddi::download(|percent| {
        if percent / 10 > reported {
            reported = percent / 10;
            println!("  {}%", reported * 10);
        }
    })
    .await;

    match result {
        Ok(found) => {
            let size = std::fs::metadata(&found.image)
                .map(|data| data.len())
                .unwrap_or(0);
            println!("Installed {} ({size} bytes)", found.image.display());
            Some(found)
        }
        Err(error) => {
            println!("Failed [{}]: {}", error.kind, error.message);
            None
        }
    }
}

/// Drives the same AMFI calls as the Developer Mode buttons on the Developer
/// page. Enabling reboots the device; `--accept` is the confirmation that has to
/// follow the reboot.
async fn developer_mode(udid: &str, accept: bool) {
    let provider = routed_provider_for(udid, None).await.expect("provider");

    // AMFI closes the connection after each request, so every call opens its
    // own client. Production does the same: `amfi_client` in
    // `commands/developer.rs` connects per command. Reusing one client here
    // makes the second request fail with `device socket io failed`.
    macro_rules! amfi {
        ($label:expr, $call:ident) => {
            match AmfiClient::connect(&provider).await {
                Ok(mut client) => match client.$call().await {
                    Ok(value) => Some(value),
                    Err(error) => {
                        println!("{}: FAILED — {error}", $label);
                        None
                    }
                },
                Err(error) => {
                    println!("{}: AMFI unavailable — {error}", $label);
                    None
                }
            }
        };
    }

    let report = |label: &str, status: Option<bool>| match status {
        Some(true) => println!("{label}: enabled"),
        Some(false) => println!("{label}: disabled"),
        None => {}
    };

    report(
        "Developer Mode",
        amfi!("Developer Mode", get_developer_mode_status),
    );

    if accept {
        if amfi!("accept_developer_mode", accept_developer_mode).is_some() {
            println!("accept_developer_mode: ok");
        }
    } else {
        if amfi!(
            "reveal_developer_mode_option_in_ui",
            reveal_developer_mode_option_in_ui
        )
        .is_some()
        {
            println!("reveal_developer_mode_option_in_ui: ok");
        }
        if amfi!("enable_developer_mode", enable_developer_mode).is_some() {
            println!("enable_developer_mode: ok — the device reboots to apply it");
        }
    }

    report(
        "Developer Mode now",
        amfi!("Developer Mode now", get_developer_mode_status),
    );
}

async fn mount_against(udid: &str, override_files: Option<legacy_ddi::LegacyDdi>) {
    let provider = routed_provider_for(udid, None).await.expect("provider");
    let version = ios_version(&provider).await.expect("version");
    let generation = version.developer_generation();
    println!(
        "\nDevice: iOS {}.{}.{} -> {generation:?}",
        version.major, version.minor, version.patch
    );
    if generation != DeveloperGeneration::Legacy {
        println!("Not a Legacy device; this path does not apply. Nothing was changed.");
        return;
    }

    // Developer Mode arrived in iOS 16 and gates the developer services. A
    // failure to mount with it disabled is that, not a bad image.
    let developer_mode = match AmfiClient::connect(&provider).await {
        Ok(mut client) => client.get_developer_mode_status().await.ok(),
        Err(_) => None,
    };
    println!(
        "Developer Mode: {}",
        match developer_mode {
            Some(true) => "enabled",
            Some(false) => "DISABLED — mounting is expected to fail",
            None => "unknown (absent before iOS 16)",
        }
    );

    let before = match ImageMounter::connect(&provider).await {
        Ok(mut client) => client.copy_devices().await.unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    println!("Mounted images before: {}", before.len());
    if !before.is_empty() {
        println!("Already mounted; ddi_ensure would return without acting.");
        return;
    }

    let installed = match override_files {
        Some(files) => files,
        None => match legacy_ddi::installed() {
            Some(found) => found,
            None => match install().await {
                Some(found) => found,
                None => return,
            },
        },
    };

    println!("\nMounting {}…", installed.image.display());
    let image = std::fs::read(&installed.image).expect("image");
    let signature = std::fs::read(&installed.signature).expect("signature");
    let outcome = tokio::time::timeout(STEP_TIMEOUT, async {
        let mut mounter = ImageMounter::connect(&provider).await?;
        mounter.mount_developer(&image, signature).await
    })
    .await;
    match outcome {
        Ok(Ok(())) => println!("  mount_developer: ok"),
        Ok(Err(error)) => {
            println!("  mount_developer: FAILED — {error}");
            return;
        }
        Err(_) => {
            println!("  mount_developer: timed out");
            return;
        }
    }

    println!("\nAfter mounting:");
    match ImageMounter::connect(&provider).await {
        Ok(mut client) => {
            let images = client.copy_devices().await.unwrap_or_default();
            println!("  copy_devices: {} image(s)", images.len());
            match client.lookup_image("Developer").await {
                Ok(signature) => println!("  lookup_image(Developer): {} bytes", signature.len()),
                Err(error) => println!("  lookup_image(Developer): {error}"),
            }
        }
        Err(error) => println!("  mounter unavailable: {error}"),
    }

    // The services the image provides are the point of mounting it, and on a
    // cross-version image they are what may still be wrong.
    for service in [
        "com.apple.debugserver.DVTSecureSocketProxy",
        "com.apple.instruments.remoteserver.DVTSecureSocketProxy",
        "com.apple.mobile.screenshotr",
    ] {
        match idevice_desktop_lib::provider::lockdown_service_socket(&provider, service).await {
            Ok(_) => println!("  {service}: available"),
            Err(error) => println!("  {service}: {}", error.message),
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    if let Some(index) = arguments
        .iter()
        .position(|value| value == "--developer-mode")
    {
        let udid = arguments
            .get(index + 1)
            .expect("usage: verify_legacy_ddi -- --developer-mode <udid> [--accept]");
        developer_mode(udid, arguments.iter().any(|value| value == "--accept")).await;
        return;
    }

    if let Some(index) = arguments.iter().position(|value| value == "--mount-file") {
        // Mounts an arbitrary pair instead of the pinned image. This exists to
        // separate "the device refuses this image" from "the device refuses to
        // mount at all": pointing it at a version-matched image answers which
        // one a failure was.
        let udid = arguments.get(index + 1).expect("udid");
        let image = arguments.get(index + 2).expect("image path");
        let signature = arguments.get(index + 3).expect("signature path");
        mount_against(
            udid,
            Some(legacy_ddi::LegacyDdi {
                image: image.into(),
                signature: signature.into(),
            }),
        )
        .await;
        return;
    }

    if let Some(index) = arguments.iter().position(|value| value == "--mount") {
        let udid = arguments
            .get(index + 1)
            .expect("usage: verify_legacy_ddi -- --mount <udid>");
        report_install_state();
        mount_against(udid, None).await;
        return;
    }

    if report_install_state().is_some() {
        return;
    }
    if arguments.iter().any(|value| value == "--install") {
        install().await;
    } else {
        println!("\n{}", legacy_ddi::missing_image_message());
        println!("Re-run with --install to download it.");
    }
}
