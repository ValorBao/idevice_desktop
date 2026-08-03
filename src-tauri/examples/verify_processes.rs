//! Real-device protocol proof for process listing, launch, and stop.
//!
//! Usage:
//!   cargo run --example verify_processes -- <udid>
//!   cargo run --example verify_processes -- <udid> <bundle_id>
//!
//! The default run is read-only. Supplying a bundle ID explicitly exercises the
//! iOS 17+ launch and SIGTERM path. The harness refuses to terminate a PID that
//! was already present before launch; a newly returned PID is cleaned up.

use std::{future::Future, io::Write, time::Duration};

use idevice::{
    IdeviceService, RsdService,
    core_device::{AppServiceClient, ProcessToken},
    core_device_proxy::CoreDeviceProxy,
    dvt::{
        device_info::{DeviceInfoClient, RunningProcess},
        process_control::ProcessControlClient,
        remote_server::RemoteServerClient,
    },
    rsd::RsdHandshake,
    tcp::handle::AdapterHandle,
};
use idevice_desktop_lib::{
    device_version::{DeveloperGeneration, ios_version},
    error::{CommandError, CommandResult},
    provider::{lockdown_service_socket, routed_provider_for},
    tunnel::open_remote_pairing_tunnel,
};

const STEP_TIMEOUT: Duration = Duration::from_secs(30);
const STOP_WAIT: Duration = Duration::from_secs(10);
const SIGKILL: u32 = 9;
const SIGTERM: u32 = 15;

const LEGACY_DVT_SERVICES: [&str; 2] = [
    "com.apple.instruments.remoteserver.DVTSecureSocketProxy",
    "com.apple.instruments.remoteserver",
];

struct RsdTransport {
    adapter: AdapterHandle,
    handshake: RsdHandshake,
}

async fn step<T, F>(label: &str, future: F) -> CommandResult<T>
where
    F: Future<Output = CommandResult<T>>,
{
    let started = std::time::Instant::now();
    print!("  {label} ... ");
    let _ = std::io::stdout().flush();
    match tokio::time::timeout(STEP_TIMEOUT, future).await {
        Ok(Ok(value)) => {
            println!("ok ({} ms)", started.elapsed().as_millis());
            Ok(value)
        }
        Ok(Err(error)) => {
            println!("FAILED: {error:?}");
            Err(error)
        }
        Err(_) => {
            println!("TIMED OUT after {}s", STEP_TIMEOUT.as_secs());
            Err(CommandError::new(
                "processes",
                format!("Timed out while {label}"),
                true,
            ))
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .init();

    let mut args = std::env::args().skip(1);
    let udid = args
        .next()
        .expect("usage: verify_processes <udid> [bundle_id]");
    let bundle_id = args.next();
    if args.next().is_some() {
        eprintln!("usage: verify_processes <udid> [bundle_id]");
        std::process::exit(2);
    }

    let result = run(udid, bundle_id).await;
    match result {
        Ok(()) => {}
        Err(error) => {
            eprintln!("\nRESULT: FAIL — {error:?}");
            std::process::exit(1);
        }
    }
}

async fn run(udid: String, bundle_id: Option<String>) -> CommandResult<()> {
    println!("== Processes verification for {udid} ==");
    if let Some(bundle_id) = &bundle_id {
        println!(
            "  MUTATING RUN: {bundle_id} will be launched and the returned pid will receive SIGTERM"
        );
    } else {
        println!("  read-only run");
    }

    let provider = step("connecting to the device", routed_provider_for(&udid, None)).await?;
    println!(
        "     route: {}",
        if provider.is_bonjour() {
            "Bonjour TCP"
        } else {
            "usbmuxd"
        }
    );

    let version = step("reading the iOS version", ios_version(&provider)).await?;
    let generation = version.developer_generation();
    println!(
        "     iOS {}.{}.{} -> {generation:?}",
        version.major, version.minor, version.patch
    );

    match generation {
        DeveloperGeneration::Legacy => {
            if bundle_id.is_some() {
                println!(
                    "     Legacy launch and stop are intentionally not exercised: \
                     this generation's DVT instruments service is not yet proven reliable"
                );
            }
            verify_legacy(&provider).await
        }
        DeveloperGeneration::CoreDeviceRemote => {
            let pairing_path = app_data_dir().join(format!("remote-pairing-{udid}.plist"));
            println!("     pairing file: {}", pairing_path.display());
            let tunnel = step("opening the RemotePairing developer tunnel", async {
                open_remote_pairing_tunnel(&provider, &pairing_path, "idevice-desktop", None).await
            })
            .await?;
            verify_core_device(
                RsdTransport {
                    adapter: tunnel.adapter,
                    handshake: tunnel.handshake,
                },
                bundle_id,
            )
            .await
        }
        DeveloperGeneration::CoreDeviceLockdown => {
            let transport = step("opening the CoreDeviceProxy developer tunnel", async {
                let proxy = CoreDeviceProxy::connect(&provider).await?;
                let rsd_port = proxy.tunnel_info().server_rsd_port;
                let mut adapter = proxy.create_software_tunnel()?.to_async_handle();
                let stream = adapter.connect(rsd_port).await?;
                let handshake = RsdHandshake::new(stream).await?;
                Ok(RsdTransport { adapter, handshake })
            })
            .await?;
            verify_core_device(transport, bundle_id).await
        }
    }
}

async fn verify_core_device(
    mut transport: RsdTransport,
    bundle_id: Option<String>,
) -> CommandResult<()> {
    println!("     RSD services: {}", transport.handshake.services.len());
    print_process_services(&transport.handshake);
    if transport
        .handshake
        .services
        .contains_key("com.apple.coredevice.appservice")
    {
        println!("     process API: CoreDevice AppService");
        verify_app_service(transport, bundle_id).await
    } else {
        println!(
            "     CoreDevice AppService is not advertised; falling back to DVT DeviceInfo/ProcessControl"
        );
        verify_dvt(&mut transport, bundle_id).await
    }
}

async fn verify_app_service(
    mut transport: RsdTransport,
    bundle_id: Option<String>,
) -> CommandResult<()> {
    let mut client = step("connecting to CoreDevice AppService", async {
        Ok(AppServiceClient::connect_rsd(&mut transport.adapter, &mut transport.handshake).await?)
    })
    .await?;

    let processes = step("listing running processes", async {
        Ok(client.list_processes().await?)
    })
    .await?;
    print_core_processes(&processes);
    if processes.is_empty() {
        return Err(CommandError::new(
            "processes",
            "CoreDevice returned an empty process list",
            true,
        ));
    }

    let Some(bundle_id) = bundle_id else {
        println!("\nRESULT: PASS — CoreDevice returned a non-empty process list");
        return Ok(());
    };

    let launch = step("launching the requested application", async {
        Ok(client
            .launch_application(bundle_id.clone(), &[], false, false, None, None, None)
            .await?)
    })
    .await?;
    println!(
        "     pid {} · {}",
        launch.pid, launch.executable_url.relative
    );
    if processes.iter().any(|process| process.pid == launch.pid) {
        return Err(CommandError::new(
            "processes",
            format!(
                "Launch returned pre-existing pid {}; refusing to terminate a process the harness did not create",
                launch.pid
            ),
            false,
        ));
    }

    let visible = step("confirming the launched pid is listed", async {
        let processes = client.list_processes().await?;
        Ok(processes.iter().any(|process| process.pid == launch.pid))
    })
    .await;

    // Cleanup runs even if the post-launch listing exposed a defect.
    let stop = step("sending SIGTERM to the launched pid", async {
        Ok(client.send_signal(launch.pid, SIGTERM).await?)
    })
    .await;

    let visible_result = visible;
    if let Err(error) = stop {
        let _ = step("forcing cleanup with SIGKILL", async {
            Ok(client.send_signal(launch.pid, SIGKILL).await?)
        })
        .await;
        return Err(error);
    }
    let visible = visible_result?;
    if !visible {
        return Err(CommandError::new(
            "processes",
            format!(
                "Launched pid {} was missing from the next process list",
                launch.pid
            ),
            true,
        ));
    }

    let exit = step("waiting for the pid to exit", async {
        let deadline = tokio::time::Instant::now() + STOP_WAIT;
        loop {
            let processes = client.list_processes().await?;
            if processes.iter().all(|process| process.pid != launch.pid) {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(CommandError::new(
                    "processes",
                    format!("Pid {} remained listed after SIGTERM", launch.pid),
                    true,
                ));
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    })
    .await;
    if let Err(error) = exit {
        let _ = step("forcing cleanup with SIGKILL", async {
            Ok(client.send_signal(launch.pid, SIGKILL).await?)
        })
        .await;
        return Err(error);
    }

    println!(
        "\nRESULT: PASS — listed processes, launched {bundle_id}, and stopped pid {}",
        launch.pid
    );
    Ok(())
}

async fn verify_dvt(transport: &mut RsdTransport, bundle_id: Option<String>) -> CommandResult<()> {
    let mut remote = step("connecting to the DVT service hub", async {
        Ok(
            RemoteServerClient::connect_rsd(&mut transport.adapter, &mut transport.handshake)
                .await?,
        )
    })
    .await?;
    step("reading the DVT handshake", async {
        Ok(remote.read_message(0).await?)
    })
    .await?;

    let processes = {
        let mut client = step("opening the DVT DeviceInfo channel", async {
            Ok(DeviceInfoClient::new(&mut remote).await?)
        })
        .await?;
        step("listing running processes through DVT", async {
            Ok(client.running_processes().await?)
        })
        .await?
    };
    print_dvt_processes(&processes);
    if processes.is_empty() {
        return Err(CommandError::new(
            "processes",
            "DVT returned an empty process list",
            true,
        ));
    }

    let Some(bundle_id) = bundle_id else {
        println!("\nRESULT: PASS — DVT returned a non-empty process list");
        return Ok(());
    };

    let pid = {
        let mut client = step("opening the DVT ProcessControl channel", async {
            Ok(ProcessControlClient::new(&mut remote).await?)
        })
        .await?;
        step("launching the requested application", async {
            Ok(client
                .launch_app(bundle_id.clone(), None, None, false, false)
                .await?)
        })
        .await?
    };
    println!("     pid {pid}");
    if processes
        .iter()
        .any(|process| u64::from(process.pid) == pid)
    {
        return Err(CommandError::new(
            "processes",
            format!(
                "Launch returned pre-existing pid {pid}; refusing to terminate a process the harness did not create"
            ),
            false,
        ));
    }

    let visible = {
        let mut client = step("reopening the DVT DeviceInfo channel", async {
            Ok(DeviceInfoClient::new(&mut remote).await?)
        })
        .await?;
        step("confirming the launched pid is listed", async {
            let processes = client.running_processes().await?;
            Ok(processes
                .iter()
                .any(|process| u64::from(process.pid) == pid))
        })
        .await
    };

    // Cleanup runs even if the post-launch listing exposed a defect.
    let stop = {
        let mut client = step("reopening the DVT ProcessControl channel", async {
            Ok(ProcessControlClient::new(&mut remote).await?)
        })
        .await?;
        step("terminating the launched pid", async {
            Ok(client.kill_app(pid).await?)
        })
        .await
    };

    let visible = visible?;
    stop?;
    if !visible {
        return Err(CommandError::new(
            "processes",
            format!("Launched pid {pid} was missing from the next DVT process list"),
            true,
        ));
    }

    let exit = {
        let mut client = step("reopening DVT DeviceInfo for cleanup verification", async {
            Ok(DeviceInfoClient::new(&mut remote).await?)
        })
        .await?;
        step("waiting for the pid to exit", async {
            let deadline = tokio::time::Instant::now() + STOP_WAIT;
            loop {
                let processes = client.running_processes().await?;
                if processes
                    .iter()
                    .all(|process| u64::from(process.pid) != pid)
                {
                    return Ok(());
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(CommandError::new(
                        "processes",
                        format!("Pid {pid} remained listed after termination"),
                        true,
                    ));
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        })
        .await
    };
    if let Err(error) = exit {
        let _ = step("retrying forced DVT cleanup", async {
            let mut client = ProcessControlClient::new(&mut remote).await?;
            Ok(client.kill_app(pid).await?)
        })
        .await;
        return Err(error);
    }

    println!("\nRESULT: PASS — DVT listed processes, launched {bundle_id}, and stopped pid {pid}");
    Ok(())
}

async fn verify_legacy(provider: &impl idevice::provider::IdeviceProvider) -> CommandResult<()> {
    let socket = step("opening the Legacy DVT instruments service", async {
        let mut last_error = None;
        for service in LEGACY_DVT_SERVICES {
            match lockdown_service_socket(provider, service).await {
                Ok(socket) => {
                    println!("\n     lockdown service: {service}");
                    return Ok(socket);
                }
                Err(error) => last_error = Some(error),
            }
        }
        Err(last_error.unwrap_or_else(|| {
            CommandError::new("processes", "No Legacy DVT service was tried", false)
        }))
    })
    .await?;

    let mut remote = RemoteServerClient::new(socket);
    step("reading the Legacy DVT handshake", async {
        Ok(remote.read_message(0).await?)
    })
    .await?;
    let mut client = step("opening the Legacy DeviceInfo channel", async {
        Ok(DeviceInfoClient::new(&mut remote).await?)
    })
    .await?;
    let processes = step("listing Legacy running processes", async {
        Ok(client.running_processes().await?)
    })
    .await?;

    print_dvt_processes(&processes);
    if processes.is_empty() {
        return Err(CommandError::new(
            "processes",
            "Legacy DVT returned an empty process list",
            true,
        ));
    }

    println!("\nRESULT: PASS — Legacy DVT returned a non-empty process list");
    Ok(())
}

fn print_core_processes(processes: &[ProcessToken]) {
    println!("\n  {:>6}  executable", "PID");
    for process in processes.iter().take(100) {
        let path = process
            .executable_url
            .as_ref()
            .map(|url| url.relative.as_str())
            .unwrap_or("—");
        println!("  {:>6}  {}", process.pid, display_text(path, 100));
    }
    if processes.len() > 100 {
        println!("  ... {} more", processes.len() - 100);
    }
}

fn print_dvt_processes(processes: &[RunningProcess]) {
    println!("\n  {:>6}  {:<40}  app", "PID", "name");
    for process in processes.iter().take(100) {
        println!(
            "  {:>6}  {:<40}  {}",
            process.pid,
            display_text(&process.name, 40),
            process.is_application
        );
    }
    if processes.len() > 100 {
        println!("  ... {} more", processes.len() - 100);
    }
}

fn print_process_services(handshake: &RsdHandshake) {
    let mut services: Vec<_> = handshake
        .services
        .keys()
        .filter(|name| {
            let name = name.to_ascii_lowercase();
            name.contains("appservice")
                || name.contains("dtservice")
                || name.contains("instrument")
                || name.contains("process")
        })
        .collect();
    services.sort();
    println!("     relevant services:");
    for service in services {
        println!("       {service}");
    }
}

fn display_text(value: &str, width: usize) -> String {
    let mut chars = value.chars();
    let visible: String = chars.by_ref().take(width).collect();
    if chars.next().is_some() {
        format!("{visible}…")
    } else {
        visible
    }
}

fn app_data_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").expect("HOME"))
        .join("Library/Application Support/dev.idevice.desktop")
}
