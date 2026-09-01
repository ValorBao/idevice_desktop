//! Read-only real-device verification of the production Processes list path.
//!
//! Usage:
//!   cargo run --example verify_processes_command -- <udid>
//!   cargo run --example verify_processes_command -- <udid> <bundle_id>
//!
//! Unlike the protocol exploration harness, this calls the same service-layer
//! function as the Tauri command, including generation selection, AppService
//! versus DVT fallback, user-application classification, and result mapping.

use idevice_desktop_lib::commands::{
    process_launch_for_device, process_stop_for_device, processes_snapshot_for_device,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut args = std::env::args().skip(1);
    let udid = args
        .next()
        .expect("usage: verify_processes_command <udid> [bundle_id]");
    let bundle_id = args.next();
    if args.next().is_some() {
        panic!("usage: verify_processes_command <udid> [bundle_id]");
    }
    let pairing_path = std::path::PathBuf::from(std::env::var("HOME").expect("HOME"))
        .join("Library/Application Support/dev.idevice.desktop")
        .join(format!("remote-pairing-{udid}.plist"));

    let snapshot =
        match processes_snapshot_for_device(udid.clone(), pairing_path.clone(), None, None).await {
            Ok(snapshot) => snapshot,
            Err(error) => {
                println!("RESULT: BLOCKED — {error:?}");
                println!(
                    "Wake or reconnect the selected device, then run the same read-only check again"
                );
                return;
            }
        };
    println!("transport: {}", snapshot.transport);
    println!("available: {}", snapshot.available);
    println!(
        "controls: launch={} · stop={}",
        snapshot.supports_launch, snapshot.supports_stop
    );
    if let Some(limitation) = &snapshot.limitation {
        println!("limitation: {limitation}");
    }
    let applications = snapshot
        .processes
        .iter()
        .filter(|process| process.is_application)
        .count();
    let stoppable = snapshot
        .processes
        .iter()
        .filter(|process| process.can_stop)
        .count();
    println!(
        "processes: {} · applications: {applications} · installed user applications eligible to stop: {stoppable}",
        snapshot.processes.len()
    );
    for process in snapshot.processes.iter().filter(|process| process.can_stop) {
        println!("  safe user app: {} · pid {}", process.name, process.pid);
    }

    assert!(snapshot.available, "modern device returned unavailable");
    assert!(
        !snapshot.processes.is_empty(),
        "device returned no processes"
    );
    assert!(
        snapshot
            .processes
            .iter()
            .all(|process| !process.can_stop || process.is_application),
        "a non-application process was marked stoppable"
    );
    let Some(bundle_id) = bundle_id else {
        println!("RESULT: PASS — production read-only Processes path returned a safe snapshot");
        return;
    };

    println!("launching dedicated test app: {bundle_id}");
    let mut launch = process_launch_for_device(
        udid.clone(),
        pairing_path.clone(),
        None,
        None,
        bundle_id.clone(),
    )
    .await
    .unwrap_or_else(|error| panic!("production launch failed: {error:?}"));
    assert_eq!(launch.bundle_id, bundle_id);
    if let Some(existing) = snapshot
        .processes
        .iter()
        .find(|process| process.pid == launch.pid)
    {
        println!(
            "pre-existing pid details: name={} path={:?} identity={}",
            existing.name, existing.executable_path, existing.identity
        );
        assert!(
            existing.can_stop,
            "launch returned a pre-existing pid that is not classified as safe to stop"
        );
        println!(
            "dedicated test app was already running as pid {}; cleaning the verified baseline",
            launch.pid
        );
        process_stop_for_device(
            udid.clone(),
            pairing_path.clone(),
            None,
            None,
            launch.pid,
            existing.identity.clone(),
        )
        .await
        .unwrap_or_else(|error| panic!("production baseline cleanup failed: {error:?}"));
        let cleaned = processes_snapshot_for_device(udid.clone(), pairing_path.clone(), None, None)
            .await
            .unwrap_or_else(|error| panic!("baseline cleanup list failed: {error:?}"));
        assert!(
            cleaned
                .processes
                .iter()
                .all(|process| process.pid != launch.pid),
            "pre-existing test pid {} remained after cleanup",
            launch.pid
        );
        launch = process_launch_for_device(
            udid.clone(),
            pairing_path.clone(),
            None,
            None,
            bundle_id.clone(),
        )
        .await
        .unwrap_or_else(|error| panic!("production relaunch failed: {error:?}"));
        assert!(
            cleaned
                .processes
                .iter()
                .all(|process| process.pid != launch.pid),
            "relaunch reused a pid that remained in the cleaned snapshot"
        );
    }
    println!("launched pid {} via {}", launch.pid, launch.transport);

    let launched_snapshot =
        processes_snapshot_for_device(udid.clone(), pairing_path.clone(), None, None)
            .await
            .unwrap_or_else(|error| panic!("post-launch production list failed: {error:?}"));
    let launched = launched_snapshot
        .processes
        .iter()
        .find(|process| process.pid == launch.pid)
        .unwrap_or_else(|| panic!("launched pid {} was not listed", launch.pid));
    assert!(
        launched.can_stop,
        "dedicated test app was not classified as safe to stop"
    );
    println!("confirmed pid {} with safe identity", launch.pid);

    process_stop_for_device(
        udid.clone(),
        pairing_path.clone(),
        None,
        None,
        launch.pid,
        launched.identity.clone(),
    )
    .await
    .unwrap_or_else(|error| panic!("production stop failed: {error:?}"));

    let final_snapshot = processes_snapshot_for_device(udid, pairing_path, None, None)
        .await
        .unwrap_or_else(|error| panic!("post-stop production list failed: {error:?}"));
    assert!(
        final_snapshot
            .processes
            .iter()
            .all(|process| process.pid != launch.pid),
        "pid {} remained listed after production stop",
        launch.pid
    );
    println!(
        "RESULT: PASS — production path listed, launched, identity-checked, stopped, and verified {bundle_id}"
    );
}
