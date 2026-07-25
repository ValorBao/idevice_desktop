//! Real-device verification harness for the JIT path.
//!
//! Usage:
//!   cargo run --example verify_jit -- <udid>            # list debuggable candidates
//!   cargo run --example verify_jit -- <udid> <bundle>   # run the full JIT sequence
//!
//! This mirrors the transport selection and step order used by `jit_start` in
//! `commands/developer.rs`, calling the project's real provider and tunnel code.

use std::time::Duration;

use idevice::{
    IdeviceService, ReadWrite, RsdService,
    core_device_proxy::CoreDeviceProxy,
    debug_proxy::DebugProxyClient,
    dvt::{process_control::ProcessControlClient, remote_server::RemoteServerClient},
    installation_proxy::InstallationProxyClient,
    rsd::RsdHandshake,
    tcp::handle::AdapterHandle,
};
use idevice_desktop_lib::{
    device_version::{DeveloperGeneration, ios_version},
    error::{CommandError, CommandResult},
    provider::{lockdown_service_socket, routed_provider_for},
    tunnel::open_remote_pairing_tunnel,
};

/// Mirrors `LEGACY_DVT_SERVICES` in `commands/developer.rs`.
const LEGACY_DVT_SERVICES: [&str; 2] = [
    "com.apple.instruments.remoteserver.DVTSecureSocketProxy",
    "com.apple.instruments.remoteserver",
];

/// Mirrors `LEGACY_DEBUGSERVER_SERVICES` in `commands/developer.rs`.
const LEGACY_DEBUGSERVER_SERVICES: [&str; 2] = [
    "com.apple.debugserver.DVTSecureSocketProxy",
    "com.apple.debugserver",
];

/// Mirrors `JitTransport` in `commands/developer.rs`.
enum Transport {
    Rsd {
        adapter: AdapterHandle,
        handshake: RsdHandshake,
    },
    Lockdown,
}

async fn first_available(
    provider: &impl idevice::provider::IdeviceProvider,
    candidates: &[&str],
) -> CommandResult<Box<dyn ReadWrite>> {
    let mut last = None;
    for service in candidates {
        match lockdown_service_socket(provider, service).await {
            Ok(socket) => {
                println!("     lockdown service: {service}");
                return Ok(socket);
            }
            Err(error) => last = Some(error),
        }
    }
    Err(last.unwrap_or_else(|| CommandError::new("jit", "no candidates", false)))
}

impl Transport {
    async fn remote_server(
        &mut self,
        provider: &impl idevice::provider::IdeviceProvider,
    ) -> CommandResult<RemoteServerClient<Box<dyn ReadWrite>>> {
        match self {
            Self::Rsd { adapter, handshake } => {
                Ok(RemoteServerClient::connect_rsd(adapter, handshake).await?)
            }
            Self::Lockdown => Ok(RemoteServerClient::new(
                first_available(provider, &LEGACY_DVT_SERVICES).await?,
            )),
        }
    }

    async fn debug_proxy(
        &mut self,
        provider: &impl idevice::provider::IdeviceProvider,
    ) -> CommandResult<DebugProxyClient<Box<dyn ReadWrite>>> {
        match self {
            Self::Rsd { adapter, handshake } => {
                Ok(DebugProxyClient::connect_rsd(adapter, handshake).await?)
            }
            Self::Lockdown => Ok(DebugProxyClient::new(
                first_available(provider, &LEGACY_DEBUGSERVER_SERVICES).await?,
            )),
        }
    }
}

const STEP_TIMEOUT: Duration = Duration::from_secs(30);

async fn step<T, F>(label: &str, future: F) -> CommandResult<T>
where
    F: std::future::Future<Output = CommandResult<T>>,
{
    let started = std::time::Instant::now();
    print!("  {label} ... ");
    use std::io::Write;
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
            Err(idevice_desktop_lib::error::CommandError::new(
                "jit",
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
    let udid = args.next().expect("usage: verify_jit <udid> [bundle_id]");
    let bundle_id = args.next();

    // The JIT worker runs on a dedicated 8 MB stack in production; match that here.
    let handle = std::thread::Builder::new()
        .name("verify-jit".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            runtime.block_on(run(udid, bundle_id))
        })
        .expect("spawn");
    let code = match handle.join().expect("join") {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("\nRESULT: FAIL — {error:?}");
            1
        }
    };
    std::process::exit(code);
}

async fn run(udid: String, bundle_id: Option<String>) -> CommandResult<()> {
    println!("== JIT verification for {udid} ==");

    // Discovery targets come from the running app's catalog; the harness lets the
    // tunnel fall back to its own Bonjour discovery by passing None.
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

    let pairing_path = dirs_app_data().join(format!("remote-pairing-{udid}.plist"));
    if generation != DeveloperGeneration::Legacy {
        println!("     pairing file: {}", pairing_path.display());
    }

    let mut transport = step("opening the developer transport", async {
        match generation {
            // iOS 16 and earlier publish each developer service over lockdown.
            DeveloperGeneration::Legacy => Ok(Transport::Lockdown),
            DeveloperGeneration::CoreDeviceRemote => {
                let tunnel =
                    open_remote_pairing_tunnel(&provider, &pairing_path, "idevice-desktop", None)
                        .await?;
                Ok(Transport::Rsd {
                    adapter: tunnel.adapter,
                    handshake: tunnel.handshake,
                })
            }
            DeveloperGeneration::CoreDeviceLockdown => {
                let proxy = CoreDeviceProxy::connect(&provider).await?;
                let rsd_port = proxy.tunnel_info().server_rsd_port;
                let mut adapter = proxy.create_software_tunnel()?.to_async_handle();
                let stream = adapter.connect(rsd_port).await?;
                let handshake = RsdHandshake::new(stream).await?;
                Ok(Transport::Rsd { adapter, handshake })
            }
        }
    })
    .await?;
    if let Transport::Rsd { handshake, .. } = &transport {
        println!("     RSD services: {}", handshake.services.len());
    }

    let Some(bundle_id) = bundle_id else {
        list_candidates(&provider).await?;
        println!("\nRESULT: PASS (tunnel only) — re-run with a bundle id to attach");
        return Ok(());
    };

    let mut remote_server = step(
        "connecting to the remote server",
        transport.remote_server(&provider),
    )
    .await?;
    step("reading the remote-server handshake", async {
        Ok(remote_server.read_message(0).await?)
    })
    .await?;

    let pid = step("launching the app", async {
        let mut process_control = ProcessControlClient::new(&mut remote_server).await?;
        let pid = process_control
            .launch_app(bundle_id.clone(), None, None, false, false)
            .await?;
        Ok(pid)
    })
    .await?;
    println!("     pid: {pid}");

    let mut process_control = ProcessControlClient::new(&mut remote_server).await?;
    let _ = step("disabling the app memory limit", async {
        Ok(process_control.disable_memory_limit(pid).await?)
    })
    .await;
    drop(remote_server);

    let mut debug = match step(
        "connecting to the debug server",
        transport.debug_proxy(&provider),
    )
    .await
    {
        Ok(debug) => debug,
        Err(error) => {
            // Production `jit_start` terminates the launched app before returning.
            println!("  -- exercising the failure cleanup path --");
            cleanup(&mut transport, &provider, pid).await;
            return Err(error);
        }
    };

    let response = step("attaching to the app", async {
        Ok(debug
            .send_command(format!("vAttach;{pid:x}").into())
            .await?)
    })
    .await?;
    println!("     vAttach response: {response:?}");

    // Mirrors `attach_failure` in `commands/developer.rs`: debugserver rejects an
    // attach with a protocol error packet, which the send itself reports as Ok.
    if let Some(detail) = attach_failure(response.as_deref()) {
        println!("     attach REJECTED: {detail}");
        drop(debug);
        cleanup(&mut transport, &provider, pid).await;
        return Err(idevice_desktop_lib::error::CommandError::new(
            "jit",
            format!("Unable to attach to {bundle_id}: {detail}"),
            false,
        ));
    }

    let _ = step("detaching", async {
        Ok(debug.send_command("D".into()).await?)
    })
    .await;
    drop(debug);

    cleanup(&mut transport, &provider, pid).await;

    println!("\nRESULT: PASS — launched, attached, detached, and cleaned up {bundle_id}");
    Ok(())
}

/// Mirrors `attach_failure` in `commands/developer.rs`.
fn attach_failure(response: Option<&str>) -> Option<String> {
    let rest = response?.trim().strip_prefix('E')?;
    let (code, message) = match rest.split_once(';') {
        Some((code, message)) => (code, Some(message)),
        None => (rest, None),
    };
    if code.is_empty() || !code.chars().all(|value| value.is_ascii_hexdigit()) {
        return None;
    }
    let detail = message
        .filter(|message| !message.is_empty() && message.len() % 2 == 0)
        .and_then(|message| {
            let bytes = (0..message.len())
                .step_by(2)
                .map(|index| u8::from_str_radix(&message[index..index + 2], 16))
                .collect::<Result<Vec<u8>, _>>()
                .ok()?;
            String::from_utf8(bytes).ok()
        })
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| format!("debugserver rejected the attach with error {code}"));
    Some(detail)
}

/// Mirrors `cleanup_jit_process` in `commands/developer.rs`.
async fn cleanup(
    transport: &mut Transport,
    provider: &impl idevice::provider::IdeviceProvider,
    pid: u64,
) {
    let result = step("terminating the launched app", async {
        let mut remote_server = transport.remote_server(provider).await?;
        remote_server.read_message(0).await?;
        let mut process_control = ProcessControlClient::new(&mut remote_server).await?;
        Ok(process_control.kill_app(pid).await?)
    })
    .await;
    if result.is_ok() {
        println!("     pid {pid} terminated");
    }
}

async fn list_candidates(provider: &impl idevice::provider::IdeviceProvider) -> CommandResult<()> {
    let mut client = InstallationProxyClient::connect(provider).await?;
    let apps = client.get_apps(Some("User"), None).await?;
    println!("\n  user applications ({}):", apps.len());
    let mut rows: Vec<(String, bool)> = apps
        .into_iter()
        .filter_map(|(bundle_id, value)| {
            let dict = value.as_dictionary()?;
            // get-task-allow is what makes an app attachable by the debug server.
            let debuggable = dict
                .get("Entitlements")
                .and_then(|value| value.as_dictionary())
                .and_then(|entitlements| entitlements.get("get-task-allow"))
                .and_then(|value| value.as_boolean())
                .unwrap_or(false);
            Some((bundle_id, debuggable))
        })
        .collect();
    rows.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    for (bundle_id, debuggable) in &rows {
        println!(
            "    {} {bundle_id}",
            if *debuggable {
                "[debuggable]"
            } else {
                "[          ]"
            }
        );
    }
    if !rows.iter().any(|(_, debuggable)| *debuggable) {
        println!(
            "\n  NOTE: no installed app carries get-task-allow, so vAttach cannot succeed \
             on this device without installing a development-signed build."
        );
    }
    Ok(())
}

fn dirs_app_data() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").expect("HOME"))
        .join("Library/Application Support/dev.idevice.desktop")
}
