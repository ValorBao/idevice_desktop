//! Read-only Personal Signing Assistant preflight harness.
//!
//! Usage:
//! cargo run --manifest-path src-tauri/Cargo.toml --example verify_personal_signing -- \
//!   /path/to/App.ipa /path/to/Profile.mobileprovision DEVICE-UDID

use idevice_desktop_lib::commands::personal_signing_preflight;

#[tokio::main]
async fn main() {
    let arguments = std::env::args().collect::<Vec<_>>();
    let (ipa, profile, udid) = match arguments.len() {
        4 => (
            arguments[1].clone(),
            arguments[2].clone(),
            arguments[3].clone(),
        ),
        3 => (
            arguments[1].clone(),
            arguments[2].clone(),
            idevice_desktop_lib::device_rig::udid_or_exit(None),
        ),
        _ => {
            eprintln!(
                "usage: verify_personal_signing <app.ipa> <profile.mobileprovision> [device-udid]"
            );
            std::process::exit(2);
        }
    };

    match personal_signing_preflight(ipa, profile, udid).await {
        Ok(preflight) => println!(
            "{}",
            serde_json::to_string_pretty(&preflight).expect("serialize preflight")
        ),
        Err(error) => {
            eprintln!("preflight error [{}]: {}", error.kind, error.message);
            std::process::exit(1);
        }
    }
}
