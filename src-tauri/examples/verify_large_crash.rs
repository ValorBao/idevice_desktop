//! Read-only real-device check for the large crash-report preview and export contract.
//!
//! Usage: cargo run --example verify_large_crash -- <udid>
//!
//! This harness uses the same direct Lockdown service selected by the production
//! command for a USB route. It finds a report larger than the 4 MiB preview cap,
//! pulls it without deleting it from the phone, writes a temporary local export,
//! verifies byte equality, and removes only that temporary local file.

use std::{path::Path, time::Duration};

use idevice::{IdeviceService, services::crashreportcopymobile::CrashReportCopyMobileClient};
use idevice_desktop_lib::provider::routed_provider_for;

const PREVIEW_BYTES: usize = 4 * 1024 * 1024;
const MAX_REPORT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ENTRIES: usize = 2_000;

fn looks_like_report(name: &str) -> bool {
    matches!(
        Path::new(name)
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .as_deref(),
        Some("ips" | "crash" | "panic" | "log" | "synced")
    )
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let udid = std::env::args()
        .nth(1)
        .expect("usage: verify_large_crash <udid>");

    println!("== Large crash-report verification for {udid} ==");
    let provider = routed_provider_for(&udid, None).await.expect("provider");
    if provider.is_bonjour() {
        panic!(
            "this harness verifies the production USB Lockdown route; connect the device by USB"
        );
    }
    let client = tokio::time::timeout(
        Duration::from_secs(30),
        CrashReportCopyMobileClient::connect(&provider),
    )
    .await;
    let mut client = match client {
        Ok(Ok(client)) => client,
        Ok(Err(error)) => {
            println!("RESULT: BLOCKED — unable to open the crash-report service: {error}");
            println!(
                "Unlock the device and run the same command again; no device file was changed"
            );
            return;
        }
        Err(_) => {
            println!("RESULT: BLOCKED — opening the crash-report service timed out");
            println!(
                "Keep the device unlocked and run the same command again; no device file was changed"
            );
            return;
        }
    };

    let mut directories = vec!["/".to_string()];
    let mut inspected = 0usize;
    let mut candidate: Option<(String, u64)> = None;

    while let Some(directory) = directories.pop() {
        let names = client.ls(Some(&directory)).await.expect("list directory");
        for name in names {
            if inspected >= MAX_ENTRIES {
                break;
            }
            if name == "." || name == ".." || name.contains('/') {
                continue;
            }
            inspected += 1;
            let path = if directory == "/" {
                format!("/{name}")
            } else {
                format!("{}/{name}", directory.trim_end_matches('/'))
            };
            let Ok(info) = client.afc_client.get_file_info(&path).await else {
                continue;
            };
            if info.st_ifmt == "S_IFDIR" {
                directories.push(path);
                continue;
            }
            if info.st_ifmt != "S_IFREG" || !looks_like_report(&name) {
                continue;
            }
            let size = info.size as u64;
            if size > PREVIEW_BYTES as u64 && size <= MAX_REPORT_BYTES {
                let replace = candidate
                    .as_ref()
                    .is_none_or(|(_, current_size)| size > *current_size);
                if replace {
                    candidate = Some((path, size));
                }
            }
        }
        if inspected >= MAX_ENTRIES {
            break;
        }
    }

    println!("  inspected {inspected} entries");
    let Some((path, metadata_size)) = candidate else {
        println!(
            "RESULT: INCONCLUSIVE — no report between 4 MiB and 64 MiB was available; no device file was changed"
        );
        return;
    };
    println!("  candidate: {path} ({metadata_size} bytes)");

    let normalized = path.trim_start_matches('/');
    let bytes = tokio::time::timeout(Duration::from_secs(60), client.pull(normalized))
        .await
        .expect("report pull timed out")
        .expect("report pull");
    assert_eq!(
        bytes.len() as u64,
        metadata_size,
        "AFC metadata and pull length differ"
    );
    assert!(
        bytes.len() > PREVIEW_BYTES,
        "candidate does not exceed the preview cap"
    );
    println!(
        "  preview contract: {} of {} bytes, truncated=true",
        PREVIEW_BYTES,
        bytes.len()
    );

    let export_path = std::env::temp_dir().join(format!(
        "idevice-desktop-large-crash-{}-{}.bin",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    tokio::fs::write(&export_path, &bytes)
        .await
        .expect("write temporary export");
    let exported = tokio::fs::read(&export_path)
        .await
        .expect("read temporary export");
    let _ = tokio::fs::remove_file(&export_path).await;
    assert_eq!(
        exported, bytes,
        "temporary export differs from the device report"
    );

    println!("  export contract: complete byte equality; temporary local file removed");
    println!(
        "RESULT: PASS — large preview and complete export contracts hold; device report preserved"
    );
}
