use std::{
    ffi::CString,
    io,
    mem::MaybeUninit,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    time::{Duration, Instant},
};

use idevice::{
    IdeviceService, RsdService,
    core_device_proxy::CoreDeviceProxy,
    pcapd::{DevicePacket, PcapdClient},
    rsd::RsdHandshake,
    tcp::handle::AdapterHandle,
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncWriteExt,
    sync::Mutex,
    time::MissedTickBehavior,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    device_version::{DeveloperGeneration, ios_version},
    discovery::{LockdownTarget, RemotePairingTarget},
    error::{CommandError, CommandResult},
    provider::{RoutedProvider, routed_provider_for},
    state::AppState,
    tunnel::{open_remote_pairing_tunnel, remote_pairing_path},
    types::{NetworkCaptureFilter, NetworkCaptureProgress, NetworkCaptureStatus},
};

const CAPTURE_TASK: &str = "network-capture";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(45);
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);
const SPACE_CHECK_INTERVAL: Duration = Duration::from_secs(2);
const MIN_FREE_SPACE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_OUTPUT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const ACTION_DELETE: u8 = 0;
const ACTION_SAVE: u8 = 1;
const PCAP_GLOBAL_HEADER: [u8; 24] = [
    0xa1, 0xb2, 0xc3, 0xd4, // big-endian magic number
    0x00, 0x02, // major version
    0x00, 0x04, // minor version
    0x00, 0x00, 0x00, 0x00, // GMT offset
    0x00, 0x00, 0x00, 0x00, // timestamp accuracy
    0x00, 0x04, 0x00, 0x00, // 262,144-byte snapshot limit
    0x00, 0x00, 0x00, 0x01, // LINKTYPE_ETHERNET
];

#[derive(Clone)]
struct CaptureControl {
    id: Uuid,
    token: CancellationToken,
    action: Arc<AtomicU8>,
    destination: String,
    filter: NetworkCaptureFilter,
}

#[derive(Default)]
pub struct NetworkCaptureState {
    current: Mutex<Option<CaptureControl>>,
}

#[derive(Clone)]
struct CaptureContext {
    udid: String,
    lockdown_target: Option<LockdownTarget>,
    remote_target: Option<RemotePairingTarget>,
}

struct CaptureClient {
    client: PcapdClient,
    transport: String,
    _adapter: Option<AdapterHandle>,
}

struct CaptureOutcome {
    saved: bool,
    message: Option<String>,
    transport: Option<String>,
    progress: NetworkCaptureProgress,
}

fn validate_destination(value: &str) -> CommandResult<PathBuf> {
    if value.trim().is_empty() {
        return Err(CommandError::new(
            "network_capture",
            "Choose a destination for the PCAP file",
            false,
        ));
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() || path.file_name().is_none() {
        return Err(CommandError::new(
            "network_capture",
            "Choose an absolute PCAP destination",
            false,
        ));
    }
    let is_pcap = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pcap"));
    if !is_pcap {
        return Err(CommandError::new(
            "network_capture",
            "The capture destination must end in .pcap",
            false,
        ));
    }
    Ok(path)
}

fn capture_filter(
    pid: Option<u32>,
    interface_name: Option<String>,
) -> CommandResult<NetworkCaptureFilter> {
    if pid == Some(0) {
        return Err(CommandError::new(
            "network_capture",
            "A process filter must use a PID greater than zero",
            false,
        ));
    }
    let interface_name = interface_name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if interface_name
        .as_ref()
        .is_some_and(|value| value.len() > 64 || value.chars().any(char::is_control))
    {
        return Err(CommandError::new(
            "network_capture",
            "The interface filter must be at most 64 visible characters",
            false,
        ));
    }
    Ok(NetworkCaptureFilter {
        pid,
        interface_name,
    })
}

fn temporary_path(destination: &Path, id: Uuid) -> CommandResult<PathBuf> {
    let mut name = destination
        .file_name()
        .ok_or_else(|| CommandError::new("network_capture", "Invalid PCAP destination", false))?
        .to_os_string();
    name.push(format!(".{id}.partial"));
    Ok(destination.with_file_name(name))
}

fn packet_matches(filter: &NetworkCaptureFilter, packet: &DevicePacket) -> bool {
    filter
        .pid
        .is_none_or(|pid| packet.pid == pid || packet.epid == pid)
        && filter
            .interface_name
            .as_ref()
            .is_none_or(|interface| packet.interface_name == *interface)
}

fn packet_process(packet: &DevicePacket) -> Option<String> {
    [&packet.comm, &packet.ecomm]
        .into_iter()
        .find(|value| !value.trim().is_empty())
        .cloned()
}

fn pcap_record_header(packet: &DevicePacket) -> CommandResult<[u8; 16]> {
    let length = u32::try_from(packet.data.len()).map_err(|_| {
        CommandError::new(
            "network_capture",
            "A captured packet exceeds the PCAP record limit",
            false,
        )
    })?;
    let mut header = [0u8; 16];
    header[0..4].copy_from_slice(&packet.seconds.to_be_bytes());
    header[4..8].copy_from_slice(&packet.microseconds.to_be_bytes());
    header[8..12].copy_from_slice(&length.to_be_bytes());
    header[12..16].copy_from_slice(&length.to_be_bytes());
    Ok(header)
}

async fn write_packet(file: &mut File, packet: &DevicePacket) -> CommandResult<u64> {
    file.write_all(&pcap_record_header(packet)?).await?;
    file.write_all(&packet.data).await?;
    Ok(16u64.saturating_add(u64::try_from(packet.data.len()).unwrap_or(u64::MAX)))
}

#[cfg(unix)]
fn available_space(path: &Path) -> io::Result<u64> {
    let path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains a null byte"))?;
    let mut stats = MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `path` is a live NUL-terminated string and `stats` points to
    // writable storage for the complete statvfs result.
    if unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: statvfs returned success and initialized the output structure.
    let stats = unsafe { stats.assume_init() };
    Ok(u64::from(stats.f_bavail).saturating_mul(stats.f_frsize))
}

#[cfg(not(unix))]
fn available_space(_path: &Path) -> io::Result<u64> {
    Ok(u64::MAX)
}

fn emit_status(
    app: &AppHandle,
    state: &str,
    message: Option<String>,
    destination: &Path,
    transport: Option<String>,
    filter: &NetworkCaptureFilter,
) {
    let _ = app.emit(
        "network-capture://status",
        NetworkCaptureStatus {
            state: state.into(),
            message,
            destination: destination.to_string_lossy().into_owned(),
            transport,
            filter: filter.clone(),
        },
    );
}

fn emit_progress(app: &AppHandle, progress: &NetworkCaptureProgress) {
    let _ = app.emit("network-capture://progress", progress.clone());
}

async fn context(state: &AppState, override_udid: Option<String>) -> CommandResult<CaptureContext> {
    let udid = state
        .selected(override_udid)
        .await
        .ok_or_else(|| CommandError::new("device", "No device selected", true))?;
    let catalog = state.discovery.read().await;
    Ok(CaptureContext {
        lockdown_target: catalog.lockdown_target(&udid),
        remote_target: catalog.remote_pairing_target(&udid),
        udid,
    })
}

async fn open_core_device_proxy(
    provider: &RoutedProvider,
) -> CommandResult<(AdapterHandle, RsdHandshake)> {
    let proxy = CoreDeviceProxy::connect(provider)
        .await
        .map_err(CommandError::from)?;
    let rsd_port = proxy.tunnel_info().server_rsd_port;
    let mut adapter = proxy
        .create_software_tunnel()
        .map_err(|error| CommandError::new("tunnel", error.to_string(), true))?
        .to_async_handle();
    let stream = adapter
        .connect(rsd_port)
        .await
        .map_err(|error| CommandError::new("tunnel", error.to_string(), true))?;
    let handshake = RsdHandshake::new(stream)
        .await
        .map_err(CommandError::from)?;
    Ok((adapter, handshake))
}

async fn connect_capture(
    app: &AppHandle,
    context: &CaptureContext,
) -> CommandResult<CaptureClient> {
    let provider = routed_provider_for(&context.udid, context.lockdown_target.as_ref()).await?;
    if !provider.is_bonjour() {
        return Ok(CaptureClient {
            client: PcapdClient::connect(&provider)
                .await
                .map_err(CommandError::from)?,
            transport: "Pcapd · USB".into(),
            _adapter: None,
        });
    }

    let generation = ios_version(&provider).await?.developer_generation();
    let (mut adapter, mut handshake, route) = match generation {
        DeveloperGeneration::Legacy => {
            return Err(CommandError::new(
                "network_capture",
                "Network capture on iOS 16 and earlier requires a USB connection",
                false,
            ));
        }
        DeveloperGeneration::CoreDeviceRemote => {
            let pairing_path = remote_pairing_path(app, &context.udid)?;
            let tunnel = open_remote_pairing_tunnel(
                &provider,
                &pairing_path,
                "idevice-desktop",
                context.remote_target.as_ref(),
            )
            .await?;
            (tunnel.adapter, tunnel.handshake, "RemotePairing/RSD")
        }
        DeveloperGeneration::CoreDeviceLockdown => {
            let (adapter, handshake) = open_core_device_proxy(&provider).await?;
            (adapter, handshake, "CoreDeviceProxy/RSD")
        }
    };
    let client = PcapdClient::connect_rsd(&mut adapter, &mut handshake)
        .await
        .map_err(|error| {
            CommandError::new(
                "network_capture",
                format!("The network pcapd service is unavailable: {error}"),
                true,
            )
        })?;
    Ok(CaptureClient {
        client,
        transport: format!("Pcapd · {route}"),
        _adapter: Some(adapter),
    })
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

async fn finish_file(
    mut file: File,
    temporary: &Path,
    destination: &Path,
    save: bool,
) -> CommandResult<()> {
    if save {
        file.flush().await?;
        file.sync_all().await?;
        drop(file);
        tokio::fs::rename(temporary, destination).await?;
    } else {
        drop(file);
        if let Err(error) = tokio::fs::remove_file(temporary).await
            && error.kind() != io::ErrorKind::NotFound
        {
            return Err(error.into());
        }
    }
    Ok(())
}

async fn remove_partial(path: &Path) {
    if let Err(error) = tokio::fs::remove_file(path).await
        && error.kind() != io::ErrorKind::NotFound
    {
        tracing::warn!(path = %path.display(), %error, "unable to remove partial PCAP");
    }
}

async fn run_capture(
    app: &AppHandle,
    context: CaptureContext,
    destination: &Path,
    temporary: &Path,
    filter: &NetworkCaptureFilter,
    token: &CancellationToken,
    action: &AtomicU8,
) -> CommandResult<CaptureOutcome> {
    let parent = destination.parent().ok_or_else(|| {
        CommandError::new(
            "network_capture",
            "Invalid PCAP destination directory",
            false,
        )
    })?;
    let metadata = tokio::fs::metadata(parent).await.map_err(|error| {
        CommandError::new(
            "network_capture",
            format!("The destination directory is unavailable: {error}"),
            false,
        )
    })?;
    if !metadata.is_dir() {
        return Err(CommandError::new(
            "network_capture",
            "The capture destination is not a directory",
            false,
        ));
    }
    let free = available_space(parent).map_err(|error| {
        CommandError::new(
            "network_capture",
            format!("Unable to check free disk space: {error}"),
            false,
        )
    })?;
    if free < MIN_FREE_SPACE_BYTES {
        return Err(CommandError::new(
            "network_capture",
            "At least 128 MB of free disk space is required to start a capture",
            false,
        ));
    }

    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(temporary)
        .await?;
    file.write_all(&PCAP_GLOBAL_HEADER).await?;
    let started = Instant::now();
    let mut progress = NetworkCaptureProgress {
        packets: 0,
        bytes: 0,
        output_bytes: u64::try_from(PCAP_GLOBAL_HEADER.len()).unwrap_or(u64::MAX),
        elapsed_ms: 0,
        last_process: None,
        last_interface: None,
    };

    let connection = tokio::select! {
        _ = token.cancelled() => None,
        result = tokio::time::timeout(CONNECT_TIMEOUT, connect_capture(app, &context)) => {
            Some(result.map_err(|_| CommandError::new(
                "network_capture",
                "Timed out opening the pcapd service. Keep the device unlocked and retry.",
                true,
            ))??)
        }
    };
    let Some(mut connection) = connection else {
        progress.elapsed_ms = elapsed_ms(started);
        let save = action.load(Ordering::Acquire) == ACTION_SAVE;
        finish_file(file, temporary, destination, save).await?;
        return Ok(CaptureOutcome {
            saved: save,
            message: None,
            transport: None,
            progress,
        });
    };

    emit_status(
        app,
        "running",
        None,
        destination,
        Some(connection.transport.clone()),
        filter,
    );
    let mut ticker = tokio::time::interval(PROGRESS_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    ticker.tick().await;
    let mut last_space_check = Instant::now();
    let (save, message) = loop {
        tokio::select! {
            biased;
            _ = token.cancelled() => {
                break (action.load(Ordering::Acquire) == ACTION_SAVE, None);
            }
            _ = ticker.tick() => {
                progress.elapsed_ms = elapsed_ms(started);
                emit_progress(app, &progress);
                if last_space_check.elapsed() >= SPACE_CHECK_INTERVAL {
                    last_space_check = Instant::now();
                    if available_space(parent).is_ok_and(|bytes| bytes < MIN_FREE_SPACE_BYTES) {
                        break (true, Some("Capture stopped before the destination ran out of space".into()));
                    }
                }
            }
            packet = connection.client.next_packet() => {
                let packet = packet.map_err(CommandError::from)?;
                if !packet_matches(filter, &packet) {
                    continue;
                }
                let record_bytes = 16u64.saturating_add(
                    u64::try_from(packet.data.len()).unwrap_or(u64::MAX),
                );
                if progress.output_bytes.saturating_add(record_bytes) > MAX_OUTPUT_BYTES {
                    break (true, Some("Capture reached the 2 GB safety limit and was saved automatically".into()));
                }
                let written = write_packet(&mut file, &packet).await?;
                progress.packets = progress.packets.saturating_add(1);
                progress.bytes = progress.bytes.saturating_add(
                    u64::try_from(packet.data.len()).unwrap_or(u64::MAX),
                );
                progress.output_bytes = progress.output_bytes.saturating_add(written);
                progress.last_process = packet_process(&packet);
                progress.last_interface = Some(packet.interface_name);
            }
        }
    };

    progress.elapsed_ms = elapsed_ms(started);
    emit_progress(app, &progress);
    finish_file(file, temporary, destination, save).await?;
    Ok(CaptureOutcome {
        saved: save,
        message,
        transport: Some(connection.transport),
        progress,
    })
}

#[tauri::command]
pub async fn network_capture_start(
    app: AppHandle,
    app_state: State<'_, AppState>,
    capture_state: State<'_, NetworkCaptureState>,
    local_path: String,
    udid: Option<String>,
    pid: Option<u32>,
    interface_name: Option<String>,
) -> CommandResult<()> {
    let destination = validate_destination(&local_path)?;
    let filter = capture_filter(pid, interface_name)?;
    let context = context(&app_state, udid).await?;
    let id = Uuid::new_v4();
    let temporary = temporary_path(&destination, id)?;
    let token = CancellationToken::new();
    let action = Arc::new(AtomicU8::new(ACTION_DELETE));
    let control = CaptureControl {
        id,
        token: token.clone(),
        action: action.clone(),
        destination: destination.to_string_lossy().into_owned(),
        filter: filter.clone(),
    };
    {
        let mut current = capture_state.current.lock().await;
        if current.is_some() {
            return Err(CommandError::new(
                "network_capture",
                "Another network capture is still running or finishing",
                false,
            ));
        }
        *current = Some(control);
    }
    app_state.replace_task(CAPTURE_TASK, token.clone()).await;
    emit_status(&app, "connecting", None, &destination, None, &filter);

    tauri::async_runtime::spawn(async move {
        let result = run_capture(
            &app,
            context,
            &destination,
            &temporary,
            &filter,
            &token,
            &action,
        )
        .await;
        match result {
            Ok(outcome) => {
                emit_progress(&app, &outcome.progress);
                emit_status(
                    &app,
                    if outcome.saved {
                        "completed"
                    } else {
                        "cancelled"
                    },
                    outcome.message,
                    &destination,
                    outcome.transport,
                    &filter,
                );
            }
            Err(error) => {
                remove_partial(&temporary).await;
                emit_status(
                    &app,
                    "error",
                    Some(error.message),
                    &destination,
                    None,
                    &filter,
                );
            }
        }

        let capture_state = app.state::<NetworkCaptureState>();
        let mut current = capture_state.current.lock().await;
        if current.as_ref().is_some_and(|control| control.id == id) {
            current.take();
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn network_capture_stop(
    app: AppHandle,
    capture_state: State<'_, NetworkCaptureState>,
) -> CommandResult<()> {
    let control = capture_state.current.lock().await.clone().ok_or_else(|| {
        CommandError::new("network_capture", "No network capture is running", false)
    })?;
    control.action.store(ACTION_SAVE, Ordering::Release);
    emit_status(
        &app,
        "stopping",
        None,
        Path::new(&control.destination),
        None,
        &control.filter,
    );
    control.token.cancel();
    Ok(())
}

#[tauri::command]
pub async fn network_capture_cancel(
    app: AppHandle,
    capture_state: State<'_, NetworkCaptureState>,
) -> CommandResult<()> {
    let Some(control) = capture_state.current.lock().await.clone() else {
        return Ok(());
    };
    control.action.store(ACTION_DELETE, Ordering::Release);
    emit_status(
        &app,
        "cancelling",
        None,
        Path::new(&control.destination),
        None,
        &control.filter,
    );
    control.token.cancel();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(pid: u32, epid: u32, interface: &str, data: Vec<u8>) -> DevicePacket {
        DevicePacket {
            header_length: 0,
            header_version: 2,
            packet_length: u32::try_from(data.len()).unwrap(),
            interface_type: 0,
            unit: 0,
            io: 0,
            protocol_family: 2,
            frame_pre_length: 0,
            frame_post_length: 0,
            interface_name: interface.into(),
            pid,
            comm: "Example".into(),
            svc: 0,
            epid,
            ecomm: String::new(),
            seconds: 42,
            microseconds: 250_000,
            data,
        }
    }

    #[test]
    fn validates_absolute_pcap_destinations() {
        assert!(validate_destination("/tmp/capture.pcap").is_ok());
        assert!(validate_destination("/tmp/CAPTURE.PCAP").is_ok());
        assert!(validate_destination("capture.pcap").is_err());
        assert!(validate_destination("/tmp/capture.txt").is_err());
    }

    #[test]
    fn normalizes_and_validates_optional_filters() {
        assert_eq!(
            capture_filter(Some(42), Some("  en0  ".into())).unwrap(),
            NetworkCaptureFilter {
                pid: Some(42),
                interface_name: Some("en0".into()),
            }
        );
        assert_eq!(
            capture_filter(None, Some("   ".into())).unwrap(),
            NetworkCaptureFilter {
                pid: None,
                interface_name: None,
            }
        );
        assert!(capture_filter(Some(0), None).is_err());
        assert!(capture_filter(None, Some("x".repeat(65))).is_err());
    }

    #[test]
    fn applies_pid_and_interface_filters_without_buffering_packets() {
        let captured = packet(42, 77, "en0", vec![1, 2, 3]);
        assert!(packet_matches(
            &NetworkCaptureFilter {
                pid: Some(42),
                interface_name: Some("en0".into()),
            },
            &captured,
        ));
        assert!(packet_matches(
            &NetworkCaptureFilter {
                pid: Some(77),
                interface_name: None,
            },
            &captured,
        ));
        assert!(!packet_matches(
            &NetworkCaptureFilter {
                pid: Some(99),
                interface_name: None,
            },
            &captured,
        ));
    }

    #[test]
    fn writes_standard_big_endian_pcap_headers() {
        assert_eq!(&PCAP_GLOBAL_HEADER[..4], &[0xa1, 0xb2, 0xc3, 0xd4]);
        assert_eq!(&PCAP_GLOBAL_HEADER[20..24], &[0, 0, 0, 1]);
        let header = pcap_record_header(&packet(42, 0, "en0", vec![1, 2, 3])).unwrap();
        assert_eq!(&header[0..4], &42u32.to_be_bytes());
        assert_eq!(&header[4..8], &250_000u32.to_be_bytes());
        assert_eq!(&header[8..12], &3u32.to_be_bytes());
        assert_eq!(&header[12..16], &3u32.to_be_bytes());
    }

    #[test]
    fn keeps_partial_files_adjacent_and_visibly_incomplete() {
        let id = Uuid::nil();
        let path = temporary_path(Path::new("/tmp/capture.pcap"), id).unwrap();
        assert_eq!(
            path,
            PathBuf::from("/tmp/capture.pcap.00000000-0000-0000-0000-000000000000.partial")
        );
    }

    #[tokio::test]
    async fn stop_flushes_and_promotes_the_partial_capture() {
        let id = Uuid::new_v4();
        let destination = std::env::temp_dir().join(format!("idevice-capture-{id}.pcap"));
        let partial = temporary_path(&destination, id).unwrap();
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&partial)
            .await
            .unwrap();
        file.write_all(&PCAP_GLOBAL_HEADER).await.unwrap();

        finish_file(file, &partial, &destination, true)
            .await
            .unwrap();

        assert_eq!(
            tokio::fs::read(&destination).await.unwrap(),
            PCAP_GLOBAL_HEADER
        );
        assert!(!partial.exists());
        tokio::fs::remove_file(destination).await.unwrap();
    }

    #[tokio::test]
    async fn cancel_deletes_the_partial_capture_without_publishing_it() {
        let id = Uuid::new_v4();
        let destination = std::env::temp_dir().join(format!("idevice-capture-{id}.pcap"));
        let partial = temporary_path(&destination, id).unwrap();
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&partial)
            .await
            .unwrap();
        file.write_all(&PCAP_GLOBAL_HEADER).await.unwrap();

        finish_file(file, &partial, &destination, false)
            .await
            .unwrap();

        assert!(!partial.exists());
        assert!(!destination.exists());
    }
}
