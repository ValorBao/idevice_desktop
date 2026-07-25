use std::future::Future;

use idevice::{
    IdeviceService, RsdService,
    amfi::AmfiClient,
    core_device_proxy::CoreDeviceProxy,
    debug_proxy::DebugProxyClient,
    dvt::{process_control::ProcessControlClient, remote_server::RemoteServerClient},
    lockdown::LockdownClient,
    mobile_image_mounter::ImageMounter,
    provider::IdeviceProvider,
    rsd::RsdHandshake,
    tcp::handle::AdapterHandle,
};
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use crate::{
    device_version::{DeveloperGeneration, IosVersion, ios_version},
    error::{CommandError, CommandResult},
    provider::selected_provider,
    state::AppState,
    tunnel::{open_remote_pairing_tunnel, remote_pairing_path},
    types::{DeveloperStatus, JitSession, OperationProgress, StreamStatus},
    utils::plist_to_json,
};

const JIT_STEP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

async fn jit_step<T, F>(label: &str, future: F) -> CommandResult<T>
where
    F: Future<Output = CommandResult<T>>,
{
    tokio::time::timeout(JIT_STEP_TIMEOUT, future)
        .await
        .map_err(|_| CommandError::new("jit", format!("Timed out while {label}"), true))?
}

/// Decodes a debugserver reply to `vAttach`, returning a human-readable reason
/// when the reply is a GDB remote protocol error packet.
///
/// Errors take the form `E<hex code>` and may carry a hex-encoded description
/// after a semicolon. Successful replies are stop packets, which never start
/// with `E`.
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
        .and_then(decode_hex_message)
        .unwrap_or_else(|| format!("debugserver rejected the attach with error {code}"));
    Some(detail)
}

fn decode_hex_message(message: &str) -> Option<String> {
    if message.is_empty() || message.len() % 2 != 0 {
        return None;
    }
    let bytes = (0..message.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&message[index..index + 2], 16))
        .collect::<Result<Vec<u8>, _>>()
        .ok()?;
    let text = String::from_utf8(bytes).ok()?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

async fn cleanup_jit_process(
    adapter: &mut AdapterHandle,
    handshake: &mut RsdHandshake,
    pid: u64,
) -> CommandResult<()> {
    let mut remote_server = jit_step("connecting to the remote server for cleanup", async {
        RemoteServerClient::connect_rsd(adapter, handshake)
            .await
            .map_err(CommandError::from)
    })
    .await?;
    jit_step("reading the cleanup handshake", async {
        remote_server
            .read_message(0)
            .await
            .map_err(CommandError::from)
    })
    .await?;
    let mut process_control = jit_step("connecting to process control for cleanup", async {
        ProcessControlClient::new(&mut remote_server)
            .await
            .map_err(CommandError::from)
    })
    .await?;
    jit_step("terminating the launched app", async {
        process_control
            .kill_app(pid)
            .await
            .map_err(CommandError::from)
    })
    .await
}

#[cfg(target_os = "macos")]
async fn run_devicectl(arguments: &[&str]) -> std::io::Result<std::process::Output> {
    tokio::process::Command::new("/usr/bin/xcrun")
        .arg("devicectl")
        .args(arguments)
        .output()
        .await
}

#[cfg(target_os = "macos")]
fn devicectl_output(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    match (stdout.is_empty(), stderr.is_empty()) {
        (false, false) => format!("{stdout}\n{stderr}"),
        (false, true) => stdout,
        (true, false) => stderr,
        (true, true) => "devicectl did not return an error description".to_owned(),
    }
}

#[cfg(target_os = "macos")]
fn emit_ddi_progress(app: &AppHandle, item: &str, percent: u64) {
    let _ = app.emit(
        "developer://ddi-progress",
        OperationProgress {
            operation: "mount-ddi".into(),
            item: item.into(),
            percent,
        },
    );
}

#[cfg(target_os = "macos")]
async fn mount_ddi_with_devicectl(app: &AppHandle, udid: &str) -> CommandResult<()> {
    let arguments = [
        "device",
        "info",
        "ddiServices",
        "--device",
        udid,
        "--auto-mount-ddis",
    ];

    emit_ddi_progress(app, "Connecting with Apple CoreDevice", 15);
    let first = run_devicectl(&arguments).await.map_err(|error| {
        CommandError::new(
            "ddi",
            format!("Unable to start Apple's DDI manager: {error}"),
            false,
        )
    })?;
    if first.status.success() {
        emit_ddi_progress(app, "Developer Disk Image services ready", 100);
        return Ok(());
    }

    let first_error = devicectl_output(&first);
    let pairing_required = first_error.contains("must be paired")
        || first_error.contains("RemotePairingError")
        || first_error.contains("not paired");
    if !pairing_required {
        return Err(CommandError::new(
            "ddi",
            format!("Apple DDI manager failed: {first_error}"),
            true,
        ));
    }

    emit_ddi_progress(app, "Pairing with Apple CoreDevice", 40);
    let pair = run_devicectl(&["manage", "pair", "--device", udid])
        .await
        .map_err(|error| {
            CommandError::new(
                "pairing",
                format!("Unable to start CoreDevice pairing: {error}"),
                true,
            )
        })?;
    if !pair.status.success() {
        return Err(CommandError::new(
            "pairing",
            format!(
                "CoreDevice pairing failed. Keep the iPhone unlocked and accept the trust prompt, then retry. {}",
                devicectl_output(&pair)
            ),
            true,
        ));
    }

    emit_ddi_progress(app, "Mounting Developer Disk Image", 70);
    let retry = run_devicectl(&arguments).await.map_err(|error| {
        CommandError::new(
            "ddi",
            format!("Unable to restart Apple's DDI manager: {error}"),
            true,
        )
    })?;
    if !retry.status.success() {
        return Err(CommandError::new(
            "ddi",
            format!(
                "CoreDevice paired successfully, but DDI mounting failed: {}",
                devicectl_output(&retry)
            ),
            true,
        ));
    }

    emit_ddi_progress(app, "Developer Disk Image services ready", 100);
    Ok(())
}

#[cfg(target_os = "macos")]
async fn devicectl_ddi_is_usable(udid: &str) -> bool {
    let output = run_devicectl(&[
        "device",
        "info",
        "ddiServices",
        "--device",
        udid,
        "--no-auto-mount-ddis",
    ])
    .await;
    match output {
        Ok(output) if output.status.success() => {
            devicectl_output(&output).contains("isUsable: true")
        }
        _ => false,
    }
}

async fn product_details(provider: &impl IdeviceProvider) -> CommandResult<(IosVersion, u64)> {
    let mut lockdown = LockdownClient::connect(provider)
        .await
        .map_err(CommandError::from)?;
    let pairing = provider
        .get_pairing_file()
        .await
        .map_err(CommandError::from)?;
    lockdown
        .start_session(&pairing)
        .await
        .map_err(CommandError::from)?;
    let version = lockdown
        .get_value(Some("ProductVersion"), None)
        .await
        .map_err(CommandError::from)?
        .into_string()
        .unwrap_or_default();
    let version = IosVersion::parse(&version).ok_or_else(|| {
        CommandError::new("device", "Device returned an invalid iOS version", false)
    })?;
    let chip_id = lockdown
        .get_value(Some("UniqueChipID"), None)
        .await
        .map_err(CommandError::from)?
        .as_unsigned_integer()
        .unwrap_or(0);
    Ok((version, chip_id))
}

#[cfg(not(target_os = "macos"))]
async fn product_type(provider: &impl IdeviceProvider) -> CommandResult<String> {
    let mut lockdown = LockdownClient::connect(provider)
        .await
        .map_err(CommandError::from)?;
    let pairing = provider
        .get_pairing_file()
        .await
        .map_err(CommandError::from)?;
    lockdown
        .start_session(&pairing)
        .await
        .map_err(CommandError::from)?;
    lockdown
        .get_value(Some("ProductType"), None)
        .await
        .map_err(CommandError::from)?
        .into_string()
        .ok_or_else(|| CommandError::new("ddi", "Device did not return ProductType", false))
}

#[cfg(not(target_os = "macos"))]
fn manifest_component_path(identity: &plist::Dictionary, component: &str) -> Option<String> {
    identity
        .get("Manifest")?
        .as_dictionary()?
        .get(component)?
        .as_dictionary()?
        .get("Info")?
        .as_dictionary()?
        .get("Path")?
        .as_string()
        .map(str::to_owned)
}

#[cfg(not(target_os = "macos"))]
async fn automatic_ddi_files(product_type: &str) -> CommandResult<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    let restore = std::path::Path::new("/Library/Developer/DeveloperDiskImages/iOS_DDI/Restore");
    let manifest_path = restore.join("BuildManifest.plist");
    let manifest = tokio::fs::read(&manifest_path).await.map_err(|error| {
        CommandError::new(
            "ddi",
            format!(
                "Automatic DDI not found at {}: {error}",
                manifest_path.display()
            ),
            false,
        )
    })?;
    let manifest_value = plist::Value::from_reader(std::io::Cursor::new(&manifest))
        .map_err(|error| CommandError::new("ddi", error.to_string(), false))?;
    let identities = manifest_value
        .as_dictionary()
        .and_then(|dict| dict.get("BuildIdentities"))
        .and_then(plist::Value::as_array)
        .ok_or_else(|| CommandError::new("ddi", "BuildManifest has no identities", false))?;
    let identity = identities
        .iter()
        .filter_map(plist::Value::as_dictionary)
        .find(|identity| {
            identity
                .get("Ap,ProductType")
                .and_then(plist::Value::as_string)
                == Some(product_type)
        })
        .ok_or_else(|| {
            CommandError::new(
                "ddi",
                format!("No compatible DDI found for {product_type}"),
                false,
            )
        })?;
    let image_path = manifest_component_path(identity, "PersonalizedDMG")
        .ok_or_else(|| CommandError::new("ddi", "DDI image path is missing", false))?;
    let trust_path = manifest_component_path(identity, "LoadableTrustCache")
        .ok_or_else(|| CommandError::new("ddi", "DDI trust cache path is missing", false))?;
    let image = tokio::fs::read(restore.join(image_path)).await?;
    let trust_cache = tokio::fs::read(restore.join(trust_path)).await?;
    Ok((image, trust_cache, manifest))
}

#[tauri::command]
pub async fn developer_status(
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<DeveloperStatus> {
    let (selected_udid, provider) = selected_provider(&state, udid).await?;
    let developer_mode = match AmfiClient::connect(&provider).await {
        Ok(mut client) => client.get_developer_mode_status().await.ok(),
        Err(_) => None,
    };
    let images = match ImageMounter::connect(&provider).await {
        Ok(mut client) => client.copy_devices().await.unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    let mut ddi_mounted = !images.is_empty();
    #[cfg(target_os = "macos")]
    if !ddi_mounted
        && matches!(
            product_details(&provider).await,
            Ok((version, _)) if version.developer_generation() != DeveloperGeneration::Legacy
        )
    {
        ddi_mounted = devicectl_ddi_is_usable(&selected_udid).await;
    }
    let ddi_images = plist_to_json(&plist::Value::Array(images));
    let generation = product_details(&provider)
        .await
        .map(|(version, _)| version.developer_generation())
        .unwrap_or(DeveloperGeneration::Legacy);
    let rsd_available = match generation {
        DeveloperGeneration::Legacy => false,
        DeveloperGeneration::CoreDeviceRemote => ddi_mounted,
        DeveloperGeneration::CoreDeviceLockdown => {
            CoreDeviceProxy::connect(&provider).await.is_ok()
        }
    };
    Ok(DeveloperStatus {
        developer_mode,
        ddi_mounted,
        ddi_images,
        rsd_available,
    })
}

async fn amfi_client(state: &AppState, udid: Option<String>) -> CommandResult<AmfiClient> {
    let (_, provider) = selected_provider(state, udid).await?;
    AmfiClient::connect(&provider)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn developer_mode_reveal(
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<()> {
    let mut client = amfi_client(&state, udid).await?;
    client
        .reveal_developer_mode_option_in_ui()
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn developer_mode_enable(
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<()> {
    let mut client = amfi_client(&state, udid).await?;
    client
        .enable_developer_mode()
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn developer_mode_accept(
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<()> {
    let mut client = amfi_client(&state, udid).await?;
    client
        .accept_developer_mode()
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn ddi_mount(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
    image_path: String,
    signature_path: Option<String>,
    manifest_path: Option<String>,
    trust_cache_path: Option<String>,
) -> CommandResult<()> {
    let (_, provider) = selected_provider(&state, udid).await?;
    let (version, chip_id) = product_details(&provider).await?;
    let image = tokio::fs::read(image_path).await?;
    let mut mounter = ImageMounter::connect(&provider)
        .await
        .map_err(CommandError::from)?;

    if version.developer_generation() == DeveloperGeneration::Legacy {
        let signature_path = signature_path.ok_or_else(|| {
            CommandError::new("ddi", "A signature file is required before iOS 17", false)
        })?;
        let signature = tokio::fs::read(signature_path).await?;
        mounter
            .mount_developer(&image, signature)
            .await
            .map_err(CommandError::from)
    } else {
        let manifest_path = manifest_path.ok_or_else(|| {
            CommandError::new("ddi", "BuildManifest.plist is required on iOS 17+", false)
        })?;
        let trust_cache_path = trust_cache_path.ok_or_else(|| {
            CommandError::new("ddi", "A trust cache is required on iOS 17+", false)
        })?;
        let manifest = tokio::fs::read(manifest_path).await?;
        let trust_cache = tokio::fs::read(trust_cache_path).await?;
        let event_app = app.clone();
        mounter
            .mount_personalized_with_callback(
                &provider,
                image,
                trust_cache,
                &manifest,
                None,
                chip_id,
                move |((current, total), ())| {
                    let app = event_app.clone();
                    async move {
                        let percent = if total == 0 {
                            0
                        } else {
                            ((current as f64 / total as f64) * 100.0).round() as u64
                        };
                        let _ = app.emit(
                            "developer://ddi-progress",
                            OperationProgress {
                                operation: "mount-ddi".into(),
                                item: "Personalized Developer Disk Image".into(),
                                percent,
                            },
                        );
                    }
                },
                (),
            )
            .await
            .map_err(CommandError::from)
    }
}

#[tauri::command]
pub async fn ddi_mount_auto(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<()> {
    #[cfg(target_os = "macos")]
    {
        let (udid, provider) = selected_provider(&state, udid).await?;
        let (version, _) = product_details(&provider).await?;
        if version.developer_generation() == DeveloperGeneration::Legacy {
            return Err(CommandError::new(
                "ddi",
                "Automatic CoreDevice DDI mounting requires iOS 17 or later. Use Choose files with a matching DeveloperDiskImage.dmg and signature for this iOS version.",
                false,
            ));
        }
        return mount_ddi_with_devicectl(&app, &udid).await;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let (_, provider) = selected_provider(&state, udid).await?;
        let (version, chip_id) = product_details(&provider).await?;
        if version.developer_generation() == DeveloperGeneration::Legacy {
            return Err(CommandError::new(
                "ddi",
                "Automatic DDI selection currently requires iOS 17 or later",
                false,
            ));
        }
        let product_type = product_type(&provider).await?;
        let (image, trust_cache, manifest) = automatic_ddi_files(&product_type).await?;
        let mut mounter = ImageMounter::connect(&provider)
            .await
            .map_err(CommandError::from)?;
        let event_app = app.clone();
        mounter
            .mount_personalized_with_callback(
                &provider,
                image,
                trust_cache,
                &manifest,
                None,
                chip_id,
                move |((current, total), ())| {
                    let app = event_app.clone();
                    async move {
                        let percent = if total == 0 {
                            0
                        } else {
                            ((current as f64 / total as f64) * 100.0).round() as u64
                        };
                        let _ = app.emit(
                            "developer://ddi-progress",
                            OperationProgress {
                                operation: "mount-ddi".into(),
                                item: "Automatic Developer Disk Image".into(),
                                percent,
                            },
                        );
                    }
                },
                (),
            )
            .await
            .map_err(CommandError::from)
    }
}

#[tauri::command]
pub async fn ddi_unmount(state: State<'_, AppState>, udid: Option<String>) -> CommandResult<()> {
    let (_, provider) = selected_provider(&state, udid).await?;
    let (version, _) = product_details(&provider).await?;
    let mut mounter = ImageMounter::connect(&provider)
        .await
        .map_err(CommandError::from)?;
    mounter
        .unmount_image(
            if version.developer_generation() == DeveloperGeneration::Legacy {
                "/Developer"
            } else {
                "/System/Developer"
            },
        )
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn jit_start(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
    bundle_id: String,
) -> CommandResult<JitSession> {
    let udid = state
        .selected(udid)
        .await
        .ok_or_else(|| CommandError::new("device", "No device selected", true))?;
    let remote_target = state.discovery.read().await.remote_pairing_target(&udid);
    let lockdown_target = state.discovery.read().await.lockdown_target(&udid);
    let pairing_path = remote_pairing_path(&app, &udid)?;
    let token = CancellationToken::new();
    state.replace_task("jit", token.clone()).await;
    let (sender, receiver) = tokio::sync::oneshot::channel();

    std::thread::Builder::new()
        .name("idevice-jit".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = sender.send(Err(CommandError::new("runtime", error.to_string(), true)));
                    return;
                }
            };
            runtime.block_on(async move {
                let mut sender = Some(sender);
                let result: CommandResult<()> = async {
                    let provider = jit_step(
                        "connecting to the device",
                        crate::provider::routed_provider_for(&udid, lockdown_target.as_ref()),
                    )
                    .await?;
                    let generation = jit_step("reading the iOS version", ios_version(&provider))
                        .await?
                        .developer_generation();
                    if generation == DeveloperGeneration::Legacy {
                        return Err(CommandError::new(
                            "jit",
                            "JIT on iOS 16 and earlier requires the legacy debugserver transport",
                            false,
                        ));
                    }
                    let (mut adapter, mut handshake) =
                        jit_step("opening the developer tunnel", async {
                            let result = match generation {
                                DeveloperGeneration::CoreDeviceRemote => {
                                    let tunnel = open_remote_pairing_tunnel(
                                        &provider,
                                        &pairing_path,
                                        "idevice-desktop",
                                        remote_target.as_ref(),
                                    )
                                    .await?;
                                    (tunnel.adapter, tunnel.handshake)
                                }
                                DeveloperGeneration::CoreDeviceLockdown => {
                                    let proxy = CoreDeviceProxy::connect(&provider)
                                        .await
                                        .map_err(CommandError::from)?;
                                    let rsd_port = proxy.tunnel_info().server_rsd_port;
                                    let adapter =
                                        proxy.create_software_tunnel().map_err(|error| {
                                            CommandError::new("tunnel", error.to_string(), true)
                                        })?;
                                    let mut adapter = adapter.to_async_handle();
                                    let stream =
                                        adapter.connect(rsd_port).await.map_err(|error| {
                                            CommandError::new("tunnel", error.to_string(), true)
                                        })?;
                                    let handshake = RsdHandshake::new(stream)
                                        .await
                                        .map_err(CommandError::from)?;
                                    (adapter, handshake)
                                }
                                DeveloperGeneration::Legacy => unreachable!(),
                            };
                            Ok(result)
                        })
                        .await?;

                    let mut remote_server = jit_step("connecting to the remote server", async {
                        RemoteServerClient::connect_rsd(&mut adapter, &mut handshake)
                            .await
                            .map_err(CommandError::from)
                    })
                    .await?;
                    jit_step("reading the remote-server handshake", async {
                        remote_server
                            .read_message(0)
                            .await
                            .map_err(CommandError::from)
                    })
                    .await?;
                    let pid = jit_step("launching the app", async {
                        let mut process_control = ProcessControlClient::new(&mut remote_server)
                            .await
                            .map_err(CommandError::from)?;
                        let pid = process_control
                            .launch_app(bundle_id.clone(), None, None, false, false)
                            .await
                            .map_err(CommandError::from)?;
                        let _ = jit_step("disabling the app memory limit", async {
                            process_control
                                .disable_memory_limit(pid)
                                .await
                                .map_err(CommandError::from)
                        })
                        .await;
                        Ok(pid)
                    })
                    .await?;
                    drop(remote_server);

                    let mut debug = match jit_step("connecting to the debug server", async {
                        DebugProxyClient::connect_rsd(&mut adapter, &mut handshake)
                            .await
                            .map_err(CommandError::from)
                    })
                    .await
                    {
                        Ok(debug) => debug,
                        Err(error) => {
                            let _ = cleanup_jit_process(&mut adapter, &mut handshake, pid).await;
                            return Err(error);
                        }
                    };
                    let response = match jit_step("attaching to the app", async {
                        debug
                            .send_command(format!("vAttach;{pid:x}").into())
                            .await
                            .map_err(CommandError::from)
                    })
                    .await
                    {
                        Ok(response) => response,
                        Err(error) => {
                            drop(debug);
                            let _ = cleanup_jit_process(&mut adapter, &mut handshake, pid).await;
                            return Err(error);
                        }
                    };
                    // debugserver answers a rejected vAttach with a protocol-level
                    // error packet, not a transport error, so the send above still
                    // succeeds. Reporting that as an attached session would leave the
                    // user with a JIT badge over a process that was never attached.
                    if let Some(detail) = attach_failure(response.as_deref()) {
                        drop(debug);
                        let _ = cleanup_jit_process(&mut adapter, &mut handshake, pid).await;
                        return Err(CommandError::new(
                            "jit",
                            format!("Unable to attach to {bundle_id}: {detail}"),
                            false,
                        ));
                    }
                    let _ = app.emit(
                        "jit://status",
                        StreamStatus {
                            stream: "jit".into(),
                            state: "attached".into(),
                            message: Some(format!("{bundle_id} · pid {pid}")),
                        },
                    );
                    if let Some(sender) = sender.take() {
                        let _ = sender.send(Ok(JitSession {
                            bundle_id,
                            pid,
                            response,
                        }));
                    }

                    token.cancelled().await;
                    let _ = debug.send_command("D".into()).await;
                    let _ = app.emit(
                        "jit://status",
                        StreamStatus {
                            stream: "jit".into(),
                            state: "detached".into(),
                            message: None,
                        },
                    );
                    Ok(())
                }
                .await;

                if let Err(error) = result
                    && let Some(sender) = sender.take()
                {
                    let _ = sender.send(Err(error));
                }
            });
        })
        .map_err(|error| CommandError::new("runtime", error.to_string(), true))?;

    receiver
        .await
        .map_err(|_| CommandError::new("jit", "JIT worker stopped before attaching", true))?
}

#[tauri::command]
pub async fn jit_stop(state: State<'_, AppState>) -> CommandResult<()> {
    state.cancel_task("jit").await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn treats_stop_packets_as_a_successful_attach() {
        assert_eq!(attach_failure(Some("T11thread:1234;")), None);
        assert_eq!(attach_failure(Some("S05")), None);
        assert_eq!(attach_failure(None), None);
        assert_eq!(attach_failure(Some("")), None);
    }

    #[test]
    fn reports_the_decoded_reason_for_a_rejected_attach() {
        // Observed on iPhone11,8 / iOS 17.0 when attaching to an App Store build.
        let response = "E96;617474616368206661696c6564";
        assert_eq!(
            attach_failure(Some(response)).as_deref(),
            Some("attach failed")
        );
    }

    #[test]
    fn falls_back_to_the_error_code_without_a_usable_message() {
        assert_eq!(
            attach_failure(Some("E96")).as_deref(),
            Some("debugserver rejected the attach with error 96")
        );
        assert_eq!(
            attach_failure(Some("E08;zz")).as_deref(),
            Some("debugserver rejected the attach with error 08")
        );
    }

    #[test]
    fn ignores_replies_that_only_look_like_an_error() {
        // A stop packet whose payload happens to start with E is not an error.
        assert_eq!(attach_failure(Some("Exception")), None);
    }
}
