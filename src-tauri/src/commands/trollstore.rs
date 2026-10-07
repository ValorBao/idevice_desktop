use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
    time::Duration,
};

use futures_util::StreamExt;
use idevice::{
    IdeviceService,
    diagnostics_relay::DiagnosticsRelayClient,
    installation_proxy::InstallationProxyClient,
    lockdown::LockdownClient,
    mobilebackup2::{FsBackupDelegate, MobileBackup2Client, RestoreOptions},
    usbmuxd::{Connection, UsbmuxdConnection},
};
use tauri::{AppHandle, Emitter, State};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tokio_util::sync::CancellationToken;

use crate::{
    error::{CommandError, CommandResult},
    provider::selected_provider,
    state::AppState,
    trollstore::{
        HELPER_BYTES, HELPER_SHA256, HELPER_URL, TROLLSTORE_BUNDLE_ID, helper_bytes_match,
        helper_install_caution, helper_install_refusal, is_replaceable_apple_app,
        removable_app_location, removable_bundle_name, write_helper_backup,
    },
    types::{
        OperationProgress, TrollStoreHelperResult, TrollStoreIpaResult, TrollStoreRemovableApp,
        TrollStoreStatus,
    },
    utils::dict_string,
};

const MAX_REMOVABLE_APPS: usize = 64;
const HELPER_TASK: &str = "trollstore-helper";
const IPA_TASK: &str = "trollstore-ipa";
const IPA_LIMIT: u64 = 2 * 1024 * 1024 * 1024;
const IPA_WAIT: Duration = Duration::from_secs(90);

#[tauri::command]
pub async fn trollstore_status(
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<TrollStoreStatus> {
    let (_, provider) = selected_provider(&state, udid).await?;
    let (product_version, build_version) = product_identity(&provider).await?;
    let refusal = helper_install_refusal(&product_version, &build_version);
    let helper_supported = refusal.is_none();
    let helper_detail = refusal.unwrap_or_else(|| {
        format!("iOS {product_version} ({build_version}) is inside the helper-install window.")
    });
    let helper_caution = helper_supported
        .then(|| helper_install_caution(&product_version).map(str::to_owned))
        .flatten();

    let mut client = InstallationProxyClient::connect(&provider)
        .await
        .map_err(CommandError::from)?;
    let apps = client
        .get_apps(None, None)
        .await
        .map_err(CommandError::from)?;

    let trollstore_installed = apps.contains_key(TROLLSTORE_BUNDLE_ID);
    let mut removable_apps = apps
        .iter()
        .filter_map(|(bundle_id, value)| {
            let dict = value.as_dictionary()?;
            let kind = dict_string(dict, &["ApplicationType"]);
            if !is_replaceable_apple_app(bundle_id, kind.as_deref()) {
                return None;
            }
            let path = dict_string(dict, &["Path"])?;
            let bundle_name = removable_bundle_name(&path)?;
            let name = dict_string(dict, &["CFBundleDisplayName", "CFBundleName"])
                .unwrap_or_else(|| bundle_name.trim_end_matches(".app").to_owned());
            Some(TrollStoreRemovableApp {
                bundle_id: bundle_id.clone(),
                name,
                bundle_name,
            })
        })
        .collect::<Vec<_>>();
    removable_apps.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    removable_apps.truncate(MAX_REMOVABLE_APPS);

    let ipa_install_detail = if trollstore_installed {
        "TrollStore is installed. Choosing an IPA serves it on this Mac's local network and asks TrollStore to download it. The URL scheme in TrollStore's settings has to be on, and the phone still shows its own install confirmation.".to_owned()
    } else {
        "TrollStore is not installed, so an IPA cannot be handed to it.".to_owned()
    };

    Ok(TrollStoreStatus {
        product_version,
        build_version,
        helper_supported,
        helper_detail,
        helper_caution,
        trollstore_installed,
        removable_apps,
        ipa_install_available: trollstore_installed,
        ipa_install_detail,
    })
}

#[tauri::command]
pub async fn trollstore_helper_install(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
    bundle_id: String,
) -> CommandResult<TrollStoreHelperResult> {
    let (udid, provider) = selected_provider(&state, udid).await?;
    require_usb(&udid).await?;
    let (product_version, build_version) = product_identity(&provider).await?;
    if let Some(refusal) = helper_install_refusal(&product_version, &build_version) {
        return Err(CommandError::new("trollstore", refusal, false));
    }

    let mut client = InstallationProxyClient::connect(&provider)
        .await
        .map_err(CommandError::from)?;
    let apps = client
        .get_apps(None, None)
        .await
        .map_err(CommandError::from)?;
    let dict = apps
        .get(&bundle_id)
        .and_then(plist::Value::as_dictionary)
        .ok_or_else(|| {
            CommandError::new(
                "trollstore",
                "That app is not installed on the selected device.",
                false,
            )
        })?;
    let kind = dict_string(dict, &["ApplicationType"]);
    if !is_replaceable_apple_app(&bundle_id, kind.as_deref()) {
        return Err(CommandError::new(
            "trollstore",
            "Only an Apple app that can be deleted and downloaded again from the App Store can be replaced.",
            false,
        ));
    }
    let path = dict_string(dict, &["Path"]).ok_or_else(|| {
        CommandError::new("trollstore", "The selected app has no install path.", false)
    })?;
    let location = removable_app_location(&path).ok_or_else(|| {
        CommandError::new(
            "trollstore",
            "The selected app is not a removable system app. Choose one that can be deleted and downloaded again from the App Store, such as Tips.",
            false,
        )
    })?;
    let app_name = dict_string(dict, &["CFBundleDisplayName", "CFBundleName"])
        .unwrap_or_else(|| location.bundle_name.trim_end_matches(".app").to_owned());

    let token = CancellationToken::new();
    state.replace_task(HELPER_TASK, token.clone()).await;
    emit_progress(&app, "Downloading the TrollStore helper", 5);
    let helper = download_helper(&token).await?;
    if token.is_cancelled() {
        return Err(cancelled());
    }

    let root = std::env::temp_dir().join(format!("idevice-trollstore-{}", uuid::Uuid::new_v4()));
    let backup_dir = root.join(&udid);
    let written = write_helper_backup(
        &backup_dir,
        &helper,
        &location.container_id,
        &location.bundle_name,
    );
    if let Err(error) = written {
        let _ = std::fs::remove_dir_all(&root);
        return Err(CommandError::new("trollstore", error, false));
    }

    emit_progress(&app, "Restoring the helper", 40);
    let restore = restore_helper(&provider, &root, &udid).await;
    let _ = std::fs::remove_dir_all(&root);
    let (restore_failed, restore_text) = match restore {
        Ok(value) => (false, value.unwrap_or_default()),
        Err(error) => (true, error.to_string()),
    };
    if find_my_blocked(&restore_text) {
        return Err(CommandError::new(
            "trollstore",
            "Find My has to be off before the helper can be restored. Turn it off in Settings, under your name, then Find My, and try again.",
            true,
        ));
    }
    let reported_failure = restore_failed
        || restore_text.contains("ErrorCode")
        || restore_text.contains("ErrorDescription");
    if reported_failure && !expected_restore_stop(&restore_text) {
        return Err(CommandError::new(
            "trollstore",
            if restore_text.is_empty() {
                "The helper restore failed.".to_owned()
            } else {
                restore_text
            },
            true,
        ));
    }

    emit_progress(&app, "Rebooting the device", 90);
    let mut diagnostics = DiagnosticsRelayClient::connect(&provider)
        .await
        .map_err(CommandError::from)?;
    let message = if let Err(error) = diagnostics.restart().await {
        format!(
            "The helper restore finished, but the reboot command failed ({error}). Restart the device, then open {app_name} and install TrollStore from there."
        )
    } else {
        format!(
            "The helper replaced {app_name} and the device is rebooting. After it starts, open that app and install TrollStore. {app_name} stays replaced until you delete it and download it again from the App Store. Then install a persistence helper into an app you can reinstall. Turn Find My back on if you use it."
        )
    };
    Ok(TrollStoreHelperResult { app_name, message })
}

#[tauri::command]
pub async fn trollstore_ipa_install(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
    local_path: String,
) -> CommandResult<TrollStoreIpaResult> {
    let (udid, provider) = selected_provider(&state, udid).await?;
    let path = Path::new(&local_path);
    let metadata = tokio::fs::symlink_metadata(path).await?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CommandError::new(
            "trollstore",
            "Choose one IPA file, not a link or a folder.",
            false,
        ));
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("app.ipa")
        .to_owned();
    if !file_name.to_ascii_lowercase().ends_with(".ipa") {
        return Err(CommandError::new(
            "trollstore",
            "The selected file is not an IPA.",
            false,
        ));
    }
    if metadata.len() == 0 || metadata.len() > IPA_LIMIT {
        return Err(CommandError::new(
            "trollstore",
            "The IPA is empty or larger than 2 GB.",
            false,
        ));
    }

    let mut client = InstallationProxyClient::connect(&provider)
        .await
        .map_err(CommandError::from)?;
    let apps = client
        .get_apps(None, Some(vec![TROLLSTORE_BUNDLE_ID.to_owned()]))
        .await
        .map_err(CommandError::from)?;
    if !apps.contains_key(TROLLSTORE_BUNDLE_ID) {
        return Err(CommandError::new(
            "trollstore",
            "TrollStore is not installed on this device.",
            false,
        ));
    }

    let address = lan_ipv4()?;
    let listener = TcpListener::bind(SocketAddr::from((address, 0)))
        .await
        .map_err(|error| {
            CommandError::new(
                "trollstore",
                format!("Unable to offer the IPA on the local network: {error}"),
                true,
            )
        })?;
    let port = listener.local_addr().map_err(CommandError::from)?.port();
    let token_path = uuid::Uuid::new_v4().simple().to_string();
    let route = format!("/{token_path}/app.ipa");
    let ipa_url = format!("http://{address}:{port}{route}");
    let payload = format!("apple-magnifier://install?url={}", percent_encode(&ipa_url));

    let cancel = CancellationToken::new();
    state.replace_task(IPA_TASK, cancel.clone()).await;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let serve_path = local_path.clone();
    let serve_route = route.clone();
    let serve_cancel = cancel.clone();
    tokio::spawn(async move {
        let result = serve_ipa(listener, &serve_route, &serve_path, &serve_cancel).await;
        let _ = sender.send(result);
    });

    emit_progress(&app, "Asking TrollStore to download the IPA", 30);
    let launched = launch_trollstore(&udid, &payload).await;
    if let Err(error) = launched {
        cancel.cancel();
        return Err(error);
    }

    let downloaded = tokio::time::timeout(IPA_WAIT, receiver).await;
    cancel.cancel();
    match downloaded {
        Ok(Ok(Ok(()))) => Ok(TrollStoreIpaResult {
            file_name,
            message: "TrollStore downloaded the IPA. Confirm the install on the phone. This Mac cannot see whether that confirmation succeeded.".to_owned(),
        }),
        Ok(Ok(Err(error))) => Err(error),
        _ => Err(CommandError::new(
            "trollstore",
            "TrollStore did not download the IPA. Turn on URL Scheme in TrollStore settings, keep the phone on the same Wi-Fi, and try again. A plain http address can also be blocked by the phone.",
            true,
        )),
    }
}

async fn product_identity(
    provider: &impl idevice::provider::IdeviceProvider,
) -> CommandResult<(String, String)> {
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
    let product_version = lockdown
        .get_value(Some("ProductVersion"), None)
        .await
        .map_err(CommandError::from)?
        .into_string()
        .unwrap_or_default();
    let build_version = lockdown
        .get_value(Some("BuildVersion"), None)
        .await
        .map_err(CommandError::from)?
        .into_string()
        .unwrap_or_default();
    if product_version.trim().is_empty() {
        return Err(CommandError::new(
            "device",
            "Device did not return an iOS version",
            false,
        ));
    }
    Ok((product_version, build_version))
}

async fn require_usb(udid: &str) -> CommandResult<()> {
    let mut mux = UsbmuxdConnection::default()
        .await
        .map_err(CommandError::from)?;
    let device = mux.get_device(udid).await.map_err(CommandError::from)?;
    match device.connection_type {
        Connection::Usb => Ok(()),
        _ => Err(CommandError::new(
            "trollstore",
            "Helper install requires a USB connection.",
            true,
        )),
    }
}

async fn download_helper(cancel: &CancellationToken) -> CommandResult<Vec<u8>> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(4))
        .build()
        .map_err(|error| CommandError::new("trollstore", error.to_string(), true))?;
    let response = client.get(HELPER_URL).send().await.map_err(|error| {
        CommandError::new(
            "trollstore",
            format!("The TrollStore helper could not be downloaded: {error}"),
            true,
        )
    })?;
    if !response.status().is_success() {
        return Err(CommandError::new(
            "trollstore",
            format!(
                "The TrollStore helper download failed with HTTP {}.",
                response.status()
            ),
            true,
        ));
    }
    let mut received = Vec::with_capacity(HELPER_BYTES as usize);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        if cancel.is_cancelled() {
            return Err(cancelled());
        }
        let chunk = chunk.map_err(|error| {
            CommandError::new(
                "trollstore",
                format!("The TrollStore helper download stopped: {error}"),
                true,
            )
        })?;
        received.extend_from_slice(&chunk);
        if received.len() as u64 > HELPER_BYTES {
            return Err(CommandError::new(
                "trollstore",
                "The TrollStore helper download was larger than the pinned file and was discarded.",
                false,
            ));
        }
    }
    if !helper_bytes_match(&received) {
        return Err(CommandError::new(
            "trollstore",
            format!(
                "The downloaded helper does not match the pinned TrollStore 2.1.1 file ({HELPER_SHA256}). Nothing was sent to the device."
            ),
            false,
        ));
    }
    Ok(received)
}

async fn restore_helper(
    provider: &impl idevice::provider::IdeviceProvider,
    root: &Path,
    udid: &str,
) -> Result<Option<String>, CommandError> {
    let mut backup = MobileBackup2Client::connect(provider)
        .await
        .map_err(CommandError::from)?;
    let options = RestoreOptions::new()
        .with_reboot(false)
        .with_copy(false)
        .with_system_files(true);
    match backup
        .restore_from_path(root, Some(udid), Some(options), &FsBackupDelegate)
        .await
    {
        Ok(Some(dictionary)) => Ok(Some(format!("{dictionary:?}"))),
        Ok(None) => Ok(None),
        Err(error) => Err(CommandError::from(error)),
    }
}

fn expected_restore_stop(text: &str) -> bool {
    text.to_ascii_lowercase().contains("crash_on_purpose")
}

fn find_my_blocked(text: &str) -> bool {
    text.to_ascii_lowercase().contains("find my")
}

fn lan_ipv4() -> CommandResult<Ipv4Addr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("1.1.1.1:80").map_err(|_| {
        CommandError::new(
            "trollstore",
            "This Mac has no local network address the phone can reach. Join the same Wi-Fi and try again.",
            true,
        )
    })?;
    match socket.local_addr()?.ip() {
        IpAddr::V4(address) if !address.is_loopback() && !address.is_unspecified() => Ok(address),
        _ => Err(CommandError::new(
            "trollstore",
            "This Mac has no local network address the phone can reach. Join the same Wi-Fi and try again.",
            true,
        )),
    }
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

async fn launch_trollstore(udid: &str, payload: &str) -> CommandResult<()> {
    let output = tokio::process::Command::new("/usr/bin/xcrun")
        .arg("devicectl")
        .args([
            "device",
            "process",
            "launch",
            "--device",
            udid,
            "--terminate-existing",
            "--payload-url",
            payload,
            TROLLSTORE_BUNDLE_ID,
        ])
        .output()
        .await
        .map_err(|error| {
            CommandError::new(
                "trollstore",
                format!("Unable to run devicectl to open TrollStore: {error}"),
                true,
            )
        })?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if !stderr.trim().is_empty() {
        stderr.trim()
    } else {
        stdout.trim()
    };
    Err(CommandError::new(
        "trollstore",
        format!("TrollStore did not open. {detail}"),
        true,
    ))
}

async fn serve_ipa(
    listener: TcpListener,
    route: &str,
    file_path: &str,
    cancel: &CancellationToken,
) -> CommandResult<()> {
    loop {
        let accepted = tokio::select! {
            _ = cancel.cancelled() => return Err(cancelled()),
            accepted = listener.accept() => accepted,
        };
        let (mut stream, _) = accepted.map_err(CommandError::from)?;
        let mut header = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            if header.len() > 8 * 1024 {
                break;
            }
            match stream.read(&mut byte).await {
                Ok(0) => break,
                Ok(_) => {
                    header.push(byte[0]);
                    if header.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let request = String::from_utf8_lossy(&header);
        let request_line = request.lines().next().unwrap_or("");
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or("");
        let path = parts.next().unwrap_or("");
        if path != route || (method != "GET" && method != "HEAD") {
            let _ = stream
                .write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await;
            continue;
        }
        let file = match tokio::fs::File::open(file_path).await {
            Ok(file) => file,
            Err(error) => {
                let _ = stream
                    .write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .await;
                return Err(CommandError::from(error));
            }
        };
        let length = file
            .metadata()
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(response.as_bytes()).await?;
        if method == "HEAD" {
            continue;
        }
        let mut file = file;
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            if cancel.is_cancelled() {
                return Err(cancelled());
            }
            let read = file.read(&mut buffer).await?;
            if read == 0 {
                let _ = stream.shutdown().await;
                return Ok(());
            }
            if stream.write_all(&buffer[..read]).await.is_err() {
                return Err(CommandError::new(
                    "trollstore",
                    "The phone stopped reading the IPA before the download finished.",
                    true,
                ));
            }
        }
    }
}

fn emit_progress(app: &AppHandle, item: &str, percent: u64) {
    let _ = app.emit(
        "trollstore://progress",
        OperationProgress {
            operation: "trollstore".into(),
            item: item.into(),
            percent,
        },
    );
}

fn cancelled() -> CommandError {
    CommandError::new("cancelled", "TrollStore operation cancelled", false)
}
