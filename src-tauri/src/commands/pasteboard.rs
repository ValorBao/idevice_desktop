use std::{
    future::Future,
    path::{Path, PathBuf},
    time::Duration,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use idevice::{
    IdeviceService, ReadWrite, RsdService,
    core_device::{
        DataInclusionPolicy, GENERAL_PASTEBOARD, PasteboardPayload, PasteboardServiceClient,
        PasteboardSnapshot,
    },
    core_device_proxy::CoreDeviceProxy,
    rsd::RsdHandshake,
    tcp::handle::AdapterHandle,
};
use tauri::{AppHandle, State};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    device_version::{DeveloperGeneration, ios_version},
    discovery::{LockdownTarget, RemotePairingTarget},
    error::{CommandError, CommandResult},
    provider::{RoutedProvider, routed_provider_for},
    state::AppState,
    tunnel::{open_remote_pairing_tunnel, remote_pairing_path},
    types::{
        PasteboardImagePreparation, PasteboardImageSnapshot, PasteboardImageWriteResult,
        PasteboardTextSnapshot, PasteboardWriteResult,
    },
};

const OPERATION_TIMEOUT: Duration = Duration::from_secs(45);
const MAX_TEXT_BYTES: usize = 1024 * 1024;
const MAX_IMAGE_BYTES: usize = 12 * 1024 * 1024;
const MAX_IMAGE_DIMENSION: u32 = 8192;
const MAX_IMAGE_PIXELS: u64 = 32 * 1024 * 1024;
const TEXT_UTIS: [&str; 3] = ["public.utf8-plain-text", "public.plain-text", "public.text"];
const IMAGE_UTIS: [(&str, &str); 2] = [("public.png", "image/png"), ("public.jpeg", "image/jpeg")];

#[derive(Clone)]
struct PreparedPasteboardImage {
    preparation_id: String,
    udid: String,
    bytes: Vec<u8>,
    uti: String,
    mime_type: String,
    width: u32,
    height: u32,
    file_name: String,
}

#[derive(Default)]
pub struct PasteboardState {
    prepared_image: Mutex<Option<PreparedPasteboardImage>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PasteboardTransport {
    Unsupported,
    RemoteRsd,
    CoreDeviceRsd,
}

#[derive(Clone)]
struct PasteboardContext {
    udid: String,
    pairing_path: PathBuf,
    lockdown_target: Option<LockdownTarget>,
    remote_target: Option<RemotePairingTarget>,
}

struct PasteboardConnection {
    client: PasteboardServiceClient<Box<dyn ReadWrite>>,
    transport: String,
    _adapter: AdapterHandle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TextCandidate {
    Inline(Vec<u8>),
    Promised {
        item_index: i64,
        uti: String,
        size: Option<i64>,
    },
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ImageCandidate {
    Inline {
        bytes: Vec<u8>,
        uti: String,
        mime_type: String,
    },
    Promised {
        item_index: i64,
        uti: String,
        mime_type: String,
        size: Option<i64>,
    },
    Error,
}

struct NormalizedText {
    text: Option<String>,
    byte_length: u64,
    character_count: u64,
    state: String,
    message: Option<String>,
}

struct NormalizedImage {
    data_url: Option<String>,
    mime_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    byte_length: u64,
    state: String,
    message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ImageMetadata {
    uti: &'static str,
    mime_type: &'static str,
    width: u32,
    height: u32,
}

enum ImagePromiseDecision {
    Resolve { advertised_size: u64 },
    Reject(NormalizedImage),
}

fn pasteboard_transport(generation: DeveloperGeneration) -> PasteboardTransport {
    match generation {
        DeveloperGeneration::Legacy => PasteboardTransport::Unsupported,
        DeveloperGeneration::CoreDeviceRemote => PasteboardTransport::RemoteRsd,
        DeveloperGeneration::CoreDeviceLockdown => PasteboardTransport::CoreDeviceRsd,
    }
}

fn validate_write_text(text: &str) -> CommandResult<(u64, u64)> {
    if text.is_empty() {
        return Err(CommandError::new(
            "pasteboard",
            "Enter text before replacing the device pasteboard",
            false,
        ));
    }
    if text.len() > MAX_TEXT_BYTES {
        return Err(CommandError::new(
            "pasteboard",
            "Pasteboard text must be 1 MB or smaller",
            false,
        ));
    }
    Ok((
        u64::try_from(text.len()).unwrap_or(u64::MAX),
        u64::try_from(text.chars().count()).unwrap_or(u64::MAX),
    ))
}

fn text_candidate(snapshot: &PasteboardSnapshot) -> Option<TextCandidate> {
    for item in &snapshot.items {
        for uti in TEXT_UTIS {
            if let Some(entry) = item.data.iter().find(|entry| entry.uti == uti) {
                return Some(match &entry.payload {
                    PasteboardPayload::Inline(bytes) => TextCandidate::Inline(bytes.clone()),
                    PasteboardPayload::Promised { size } => TextCandidate::Promised {
                        item_index: i64::try_from(item.index).unwrap_or(i64::MAX),
                        uti: entry.uti.clone(),
                        size: *size,
                    },
                    PasteboardPayload::Error(_) => TextCandidate::Error,
                });
            }
        }
    }
    None
}

fn normalized_bytes(bytes: Vec<u8>) -> NormalizedText {
    let byte_length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if bytes.len() > MAX_TEXT_BYTES {
        return NormalizedText {
            text: None,
            byte_length,
            character_count: 0,
            state: "oversized".into(),
            message: Some("The device text exceeds the 1 MB reading limit".into()),
        };
    }
    match String::from_utf8(bytes) {
        Ok(text) => NormalizedText {
            character_count: u64::try_from(text.chars().count()).unwrap_or(u64::MAX),
            text: Some(text),
            byte_length,
            state: "text".into(),
            message: None,
        },
        Err(_) => NormalizedText {
            text: None,
            byte_length,
            character_count: 0,
            state: "invalid-text".into(),
            message: Some("The device plain-text item is not valid UTF-8".into()),
        },
    }
}

fn unavailable_text(state: &str, message: &str, byte_length: u64) -> NormalizedText {
    NormalizedText {
        text: None,
        byte_length,
        character_count: 0,
        state: state.into(),
        message: Some(message.into()),
    }
}

fn image_candidate(snapshot: &PasteboardSnapshot) -> Option<ImageCandidate> {
    for item in &snapshot.items {
        for (uti, mime_type) in IMAGE_UTIS {
            if let Some(entry) = item.data.iter().find(|entry| entry.uti == uti) {
                return Some(match &entry.payload {
                    PasteboardPayload::Inline(bytes) => ImageCandidate::Inline {
                        bytes: bytes.clone(),
                        uti: uti.into(),
                        mime_type: mime_type.into(),
                    },
                    PasteboardPayload::Promised { size } => ImageCandidate::Promised {
                        item_index: i64::try_from(item.index).unwrap_or(i64::MAX),
                        uti: uti.into(),
                        mime_type: mime_type.into(),
                        size: *size,
                    },
                    PasteboardPayload::Error(_) => ImageCandidate::Error,
                });
            }
        }
    }
    None
}

fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < 24
        || &bytes[..8] != SIGNATURE
        || u32::from_be_bytes(bytes[8..12].try_into().ok()?) != 13
        || &bytes[12..16] != b"IHDR"
    {
        return None;
    }
    Some((
        u32::from_be_bytes(bytes[16..20].try_into().ok()?),
        u32::from_be_bytes(bytes[20..24].try_into().ok()?),
    ))
}

fn is_jpeg_sof(marker: u8) -> bool {
    matches!(
        marker,
        0xc0 | 0xc1 | 0xc2 | 0xc3 | 0xc5 | 0xc6 | 0xc7 | 0xc9 | 0xca | 0xcb | 0xcd | 0xce | 0xcf
    )
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return None;
    }
    let mut cursor = 2;
    while cursor < bytes.len() {
        if bytes[cursor] != 0xff {
            cursor += 1;
            continue;
        }
        while cursor < bytes.len() && bytes[cursor] == 0xff {
            cursor += 1;
        }
        let marker = *bytes.get(cursor)?;
        cursor += 1;
        if marker == 0x00 || marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        if marker == 0xd9 || marker == 0xda {
            return None;
        }
        let segment_length = usize::from(u16::from_be_bytes([
            *bytes.get(cursor)?,
            *bytes.get(cursor + 1)?,
        ]));
        if segment_length < 2 || cursor.checked_add(segment_length)? > bytes.len() {
            return None;
        }
        if is_jpeg_sof(marker) {
            if segment_length < 7 {
                return None;
            }
            let height = u32::from(u16::from_be_bytes([bytes[cursor + 3], bytes[cursor + 4]]));
            let width = u32::from(u16::from_be_bytes([bytes[cursor + 5], bytes[cursor + 6]]));
            return Some((width, height));
        }
        cursor += segment_length;
    }
    None
}

fn validated_image_metadata(bytes: &[u8]) -> CommandResult<ImageMetadata> {
    let (uti, mime_type, dimensions) = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        ("public.png", "image/png", png_dimensions(bytes))
    } else if bytes.starts_with(&[0xff, 0xd8]) {
        ("public.jpeg", "image/jpeg", jpeg_dimensions(bytes))
    } else {
        return Err(CommandError::new(
            "pasteboard",
            "The selected file is not a valid PNG or JPEG image",
            false,
        ));
    };
    let (width, height) = dimensions.ok_or_else(|| {
        CommandError::new(
            "pasteboard",
            "The image header is incomplete or invalid",
            false,
        )
    })?;
    let pixel_count = u64::from(width) * u64::from(height);
    if width == 0
        || height == 0
        || width > MAX_IMAGE_DIMENSION
        || height > MAX_IMAGE_DIMENSION
        || pixel_count > MAX_IMAGE_PIXELS
    {
        return Err(CommandError::new(
            "pasteboard",
            "The image dimensions exceed the safe preview limit",
            false,
        ));
    }
    Ok(ImageMetadata {
        uti,
        mime_type,
        width,
        height,
    })
}

fn expected_image_mime(path: &Path) -> CommandResult<&'static str> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => Ok("image/png"),
        Some("jpg" | "jpeg") => Ok("image/jpeg"),
        _ => Err(CommandError::new(
            "pasteboard",
            "Choose a PNG, JPG, or JPEG image",
            false,
        )),
    }
}

fn normalized_image(bytes: Vec<u8>, expected_uti: &str, expected_mime: &str) -> NormalizedImage {
    let byte_length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if bytes.len() > MAX_IMAGE_BYTES {
        return unavailable_image(
            "oversized",
            "The device image exceeds the 12 MB reading limit",
            byte_length,
        );
    }
    match validated_image_metadata(&bytes) {
        Ok(metadata) if metadata.uti == expected_uti && metadata.mime_type == expected_mime => {
            NormalizedImage {
                data_url: Some(format!(
                    "data:{};base64,{}",
                    metadata.mime_type,
                    STANDARD.encode(&bytes)
                )),
                mime_type: Some(metadata.mime_type.into()),
                width: Some(metadata.width),
                height: Some(metadata.height),
                byte_length,
                state: "image".into(),
                message: None,
            }
        }
        Ok(_) => unavailable_image(
            "invalid-image",
            "The device image bytes do not match their declared format",
            byte_length,
        ),
        Err(error) => unavailable_image("invalid-image", &error.message, byte_length),
    }
}

fn unavailable_image(state: &str, message: &str, byte_length: u64) -> NormalizedImage {
    NormalizedImage {
        data_url: None,
        mime_type: None,
        width: None,
        height: None,
        byte_length,
        state: state.into(),
        message: Some(message.into()),
    }
}

fn bounded_image_promise(size: Option<i64>) -> ImagePromiseDecision {
    match size {
        Some(size) if size < 0 => ImagePromiseDecision::Reject(unavailable_image(
            "unavailable",
            "The device reported an invalid image size",
            0,
        )),
        Some(size) if usize::try_from(size).is_ok_and(|size| size <= MAX_IMAGE_BYTES) => {
            ImagePromiseDecision::Resolve {
                advertised_size: u64::try_from(size).unwrap_or_default(),
            }
        }
        Some(size) => ImagePromiseDecision::Reject(unavailable_image(
            "oversized",
            "The device image exceeds the 12 MB reading limit",
            u64::try_from(size).unwrap_or(u64::MAX),
        )),
        None => ImagePromiseDecision::Reject(unavailable_image(
            "unbounded",
            "The device did not report the image size, so it was not downloaded",
            0,
        )),
    }
}

fn image_preparation(image: &PreparedPasteboardImage) -> PasteboardImagePreparation {
    PasteboardImagePreparation {
        preparation_id: image.preparation_id.clone(),
        data_url: format!(
            "data:{};base64,{}",
            image.mime_type,
            STANDARD.encode(&image.bytes)
        ),
        mime_type: image.mime_type.clone(),
        width: image.width,
        height: image.height,
        byte_length: u64::try_from(image.bytes.len()).unwrap_or(u64::MAX),
        file_name: image.file_name.clone(),
    }
}

async fn context(
    app: &AppHandle,
    state: &AppState,
    override_udid: Option<String>,
) -> CommandResult<PasteboardContext> {
    let udid = state
        .selected(override_udid)
        .await
        .ok_or_else(|| CommandError::new("device", "No device selected", true))?;
    let catalog = state.discovery.read().await;
    Ok(PasteboardContext {
        pairing_path: remote_pairing_path(app, &udid)?,
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
        .map_err(|error| CommandError::new("pasteboard", error.to_string(), true))?
        .to_async_handle();
    let stream = adapter
        .connect(rsd_port)
        .await
        .map_err(|error| CommandError::new("pasteboard", error.to_string(), true))?;
    let handshake = RsdHandshake::new(stream)
        .await
        .map_err(CommandError::from)?;
    Ok((adapter, handshake))
}

async fn connect(context: &PasteboardContext) -> CommandResult<PasteboardConnection> {
    let provider = routed_provider_for(&context.udid, context.lockdown_target.as_ref()).await?;
    let generation = ios_version(&provider).await?.developer_generation();
    let (route, mut adapter, mut handshake) = match pasteboard_transport(generation) {
        PasteboardTransport::Unsupported => {
            return Err(CommandError::new(
                "pasteboard",
                "Pasteboard transfer requires iOS 17 or later",
                false,
            ));
        }
        PasteboardTransport::RemoteRsd => {
            let tunnel = open_remote_pairing_tunnel(
                &provider,
                &context.pairing_path,
                "idevice-desktop",
                context.remote_target.as_ref(),
            )
            .await?;
            ("RemotePairing/RSD", tunnel.adapter, tunnel.handshake)
        }
        PasteboardTransport::CoreDeviceRsd => {
            let (adapter, handshake) = open_core_device_proxy(&provider).await?;
            ("CoreDeviceProxy/RSD", adapter, handshake)
        }
    };
    let client = PasteboardServiceClient::connect_rsd(&mut adapter, &mut handshake)
        .await
        .map_err(|error| {
            CommandError::new(
                "pasteboard",
                format!("The CoreDevice pasteboard service is unavailable: {error}"),
                true,
            )
        })?;
    Ok(PasteboardConnection {
        client,
        transport: format!("CoreDevice Pasteboard · {route}"),
        _adapter: adapter,
    })
}

async fn bounded_operation<T, F>(
    token: &CancellationToken,
    label: &str,
    future: F,
) -> CommandResult<T>
where
    F: Future<Output = CommandResult<T>>,
{
    tokio::select! {
        biased;
        _ = token.cancelled() => Err(CommandError::new(
            "cancelled",
            "Pasteboard operation stopped because the device session changed",
            false,
        )),
        result = tokio::time::timeout(OPERATION_TIMEOUT, future) => result
            .map_err(|_| CommandError::new(
                "pasteboard",
                format!("Timed out while {label}. Keep the device unlocked and retry."),
                true,
            ))?,
    }
}

async fn read_text(context: PasteboardContext) -> CommandResult<PasteboardTextSnapshot> {
    let mut connection = connect(&context).await?;
    // Promise all item bytes first. Only a bounded plain-text item is resolved,
    // so reading text never pulls image or arbitrary pasteboard payloads.
    let snapshot = connection
        .client
        .get_with_policy(GENERAL_PASTEBOARD, DataInclusionPolicy::AllPromised)
        .await
        .map_err(CommandError::from)?;
    let item_count = u64::try_from(snapshot.items.len()).unwrap_or(u64::MAX);
    let normalized = match text_candidate(&snapshot) {
        Some(TextCandidate::Inline(bytes)) => normalized_bytes(bytes),
        Some(TextCandidate::Promised {
            item_index,
            uti,
            size,
        }) => match size {
            Some(size) if size < 0 => {
                unavailable_text("unavailable", "The device reported an invalid text size", 0)
            }
            Some(size) if usize::try_from(size).is_ok_and(|size| size <= MAX_TEXT_BYTES) => {
                match connection
                    .client
                    .resolve(GENERAL_PASTEBOARD, item_index, &uti)
                    .await
                    .map_err(CommandError::from)?
                {
                    Some(bytes) => normalized_bytes(bytes),
                    None => unavailable_text(
                        "unavailable",
                        "The device could not resolve the current text item",
                        u64::try_from(size).unwrap_or_default(),
                    ),
                }
            }
            Some(size) => unavailable_text(
                "oversized",
                "The device text exceeds the 1 MB reading limit",
                u64::try_from(size).unwrap_or(u64::MAX),
            ),
            None => unavailable_text(
                "unbounded",
                "The device did not report the text size, so it was not downloaded",
                0,
            ),
        },
        Some(TextCandidate::Error) => unavailable_text(
            "unavailable",
            "The device could not provide the current text item",
            0,
        ),
        None if snapshot.items.is_empty() => {
            unavailable_text("empty", "The device pasteboard is empty", 0)
        }
        None => unavailable_text(
            "non-text",
            "The device pasteboard contains no supported plain-text item",
            0,
        ),
    };
    Ok(PasteboardTextSnapshot {
        text: normalized.text,
        byte_length: normalized.byte_length,
        character_count: normalized.character_count,
        change_count: snapshot.change_count,
        item_count,
        state: normalized.state,
        message: normalized.message,
        transport: connection.transport,
    })
}

async fn read_image(context: PasteboardContext) -> CommandResult<PasteboardImageSnapshot> {
    let mut connection = connect(&context).await?;
    // Promise all item bytes first, then resolve only a PNG/JPEG whose declared
    // size fits the cap. Arbitrary or unbounded pasteboard content stays remote.
    let snapshot = connection
        .client
        .get_with_policy(GENERAL_PASTEBOARD, DataInclusionPolicy::AllPromised)
        .await
        .map_err(CommandError::from)?;
    let item_count = u64::try_from(snapshot.items.len()).unwrap_or(u64::MAX);
    let normalized = match image_candidate(&snapshot) {
        Some(ImageCandidate::Inline {
            bytes,
            uti,
            mime_type,
        }) => normalized_image(bytes, &uti, &mime_type),
        Some(ImageCandidate::Promised {
            item_index,
            uti,
            mime_type,
            size,
        }) => match bounded_image_promise(size) {
            ImagePromiseDecision::Resolve { advertised_size } => {
                match connection
                    .client
                    .resolve(GENERAL_PASTEBOARD, item_index, &uti)
                    .await
                    .map_err(CommandError::from)?
                {
                    Some(bytes) => normalized_image(bytes, &uti, &mime_type),
                    None => unavailable_image(
                        "unavailable",
                        "The device could not resolve the current image item",
                        advertised_size,
                    ),
                }
            }
            ImagePromiseDecision::Reject(normalized) => normalized,
        },
        Some(ImageCandidate::Error) => unavailable_image(
            "unavailable",
            "The device could not provide the current image item",
            0,
        ),
        None if snapshot.items.is_empty() => {
            unavailable_image("empty", "The device pasteboard is empty", 0)
        }
        None => unavailable_image(
            "non-image",
            "The device pasteboard contains no supported PNG or JPEG image",
            0,
        ),
    };
    Ok(PasteboardImageSnapshot {
        data_url: normalized.data_url,
        mime_type: normalized.mime_type,
        width: normalized.width,
        height: normalized.height,
        byte_length: normalized.byte_length,
        change_count: snapshot.change_count,
        item_count,
        state: normalized.state,
        message: normalized.message,
        transport: connection.transport,
    })
}

async fn write_text(
    context: PasteboardContext,
    text: String,
    byte_length: u64,
    character_count: u64,
) -> CommandResult<PasteboardWriteResult> {
    let mut connection = connect(&context).await?;
    connection
        .client
        .set_text(&text, GENERAL_PASTEBOARD)
        .await
        .map_err(CommandError::from)?;
    Ok(PasteboardWriteResult {
        byte_length,
        character_count,
        transport: connection.transport,
    })
}

async fn write_image(
    context: PasteboardContext,
    image: PreparedPasteboardImage,
) -> CommandResult<PasteboardImageWriteResult> {
    let mut connection = connect(&context).await?;
    connection
        .client
        .set_image(&image.bytes, &image.uti, GENERAL_PASTEBOARD)
        .await
        .map_err(CommandError::from)?;
    Ok(PasteboardImageWriteResult {
        mime_type: image.mime_type,
        width: image.width,
        height: image.height,
        byte_length: u64::try_from(image.bytes.len()).unwrap_or(u64::MAX),
        transport: connection.transport,
    })
}

async fn finish_operation<T>(
    state: &AppState,
    task_key: String,
    result: CommandResult<T>,
) -> CommandResult<T> {
    state.cancel_task(&task_key).await;
    result
}

#[tauri::command]
pub async fn pasteboard_text_read(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<PasteboardTextSnapshot> {
    let context = context(&app, &state, udid).await?;
    let token = CancellationToken::new();
    let task_key = format!("pasteboard-read-{}", Uuid::new_v4());
    state.replace_task(task_key.clone(), token.clone()).await;
    let worker_token = token.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| CommandError::new("runtime", error.to_string(), true))?;
        runtime.block_on(bounded_operation(
            &worker_token,
            "reading the device pasteboard",
            read_text(context),
        ))
    })
    .await
    .unwrap_or_else(|error| Err(CommandError::new("runtime", error.to_string(), true)));
    finish_operation(&state, task_key, result).await
}

#[tauri::command]
pub async fn pasteboard_text_write(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
    text: String,
) -> CommandResult<PasteboardWriteResult> {
    let (byte_length, character_count) = validate_write_text(&text)?;
    let context = context(&app, &state, udid).await?;
    let token = CancellationToken::new();
    let task_key = format!("pasteboard-write-{}", Uuid::new_v4());
    state.replace_task(task_key.clone(), token.clone()).await;
    let worker_token = token.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| CommandError::new("runtime", error.to_string(), true))?;
        runtime.block_on(bounded_operation(
            &worker_token,
            "replacing the device pasteboard",
            write_text(context, text, byte_length, character_count),
        ))
    })
    .await
    .unwrap_or_else(|error| Err(CommandError::new("runtime", error.to_string(), true)));
    finish_operation(&state, task_key, result).await
}

#[tauri::command]
pub async fn pasteboard_image_read(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<PasteboardImageSnapshot> {
    let context = context(&app, &state, udid).await?;
    let token = CancellationToken::new();
    let task_key = format!("pasteboard-image-read-{}", Uuid::new_v4());
    state.replace_task(task_key.clone(), token.clone()).await;
    let worker_token = token.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| CommandError::new("runtime", error.to_string(), true))?;
        runtime.block_on(bounded_operation(
            &worker_token,
            "reading the device pasteboard image",
            read_image(context),
        ))
    })
    .await
    .unwrap_or_else(|error| Err(CommandError::new("runtime", error.to_string(), true)));
    finish_operation(&state, task_key, result).await
}

#[tauri::command]
pub async fn pasteboard_image_prepare(
    app_state: State<'_, AppState>,
    pasteboard_state: State<'_, PasteboardState>,
    udid: Option<String>,
    local_path: String,
) -> CommandResult<PasteboardImagePreparation> {
    let selected_udid = app_state
        .selected(udid)
        .await
        .ok_or_else(|| CommandError::new("device", "No device selected", true))?;
    if local_path.trim().is_empty() {
        return Err(CommandError::new(
            "pasteboard",
            "Choose a PNG or JPEG image",
            false,
        ));
    }
    let path = PathBuf::from(local_path);
    if !path.is_absolute() {
        return Err(CommandError::new(
            "pasteboard",
            "Choose an image using an absolute local path",
            false,
        ));
    }
    let expected_mime = expected_image_mime(&path)?;
    let metadata = tokio::fs::metadata(&path).await?;
    if !metadata.is_file() {
        return Err(CommandError::new(
            "pasteboard",
            "The selected path is not a regular file",
            false,
        ));
    }
    if metadata.len() == 0 || metadata.len() > MAX_IMAGE_BYTES as u64 {
        return Err(CommandError::new(
            "pasteboard",
            "Pasteboard images must be non-empty and 12 MB or smaller",
            false,
        ));
    }
    let bytes = tokio::fs::read(&path).await?;
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err(CommandError::new(
            "pasteboard",
            "The image changed while it was being prepared or exceeds 12 MB",
            false,
        ));
    }
    let image_metadata = validated_image_metadata(&bytes)?;
    if image_metadata.mime_type != expected_mime {
        return Err(CommandError::new(
            "pasteboard",
            "The image contents do not match the file extension",
            false,
        ));
    }
    let image = PreparedPasteboardImage {
        preparation_id: Uuid::new_v4().to_string(),
        udid: selected_udid,
        bytes,
        uti: image_metadata.uti.into(),
        mime_type: image_metadata.mime_type.into(),
        width: image_metadata.width,
        height: image_metadata.height,
        file_name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "image".into()),
    };
    let response = image_preparation(&image);
    *pasteboard_state.prepared_image.lock().await = Some(image);
    Ok(response)
}

#[tauri::command]
pub async fn pasteboard_image_write(
    app: AppHandle,
    app_state: State<'_, AppState>,
    pasteboard_state: State<'_, PasteboardState>,
    udid: Option<String>,
    preparation_id: String,
) -> CommandResult<PasteboardImageWriteResult> {
    let context = context(&app, &app_state, udid).await?;
    let image = pasteboard_state
        .prepared_image
        .lock()
        .await
        .as_ref()
        .filter(|image| image.preparation_id == preparation_id)
        .cloned()
        .ok_or_else(|| {
            CommandError::new(
                "pasteboard",
                "The prepared image is no longer available; choose it again",
                false,
            )
        })?;
    if image.udid != context.udid {
        return Err(CommandError::new(
            "pasteboard",
            "The prepared image belongs to a different device; choose it again",
            false,
        ));
    }
    let token = CancellationToken::new();
    let task_key = format!("pasteboard-image-write-{}", Uuid::new_v4());
    app_state
        .replace_task(task_key.clone(), token.clone())
        .await;
    let worker_token = token.clone();
    let completed_id = image.preparation_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| CommandError::new("runtime", error.to_string(), true))?;
        runtime.block_on(bounded_operation(
            &worker_token,
            "replacing the device pasteboard image",
            write_image(context, image),
        ))
    })
    .await
    .unwrap_or_else(|error| Err(CommandError::new("runtime", error.to_string(), true)));
    let result = finish_operation(&app_state, task_key, result).await;
    if result.is_ok() {
        let mut prepared = pasteboard_state.prepared_image.lock().await;
        if prepared
            .as_ref()
            .is_some_and(|image| image.preparation_id == completed_id)
        {
            *prepared = None;
        }
    }
    result
}

#[tauri::command]
pub async fn pasteboard_image_discard(
    pasteboard_state: State<'_, PasteboardState>,
    preparation_id: String,
) -> CommandResult<bool> {
    let mut prepared = pasteboard_state.prepared_image.lock().await;
    let matches = prepared
        .as_ref()
        .is_some_and(|image| image.preparation_id == preparation_id);
    if matches {
        *prepared = None;
    }
    Ok(matches)
}

#[cfg(test)]
mod tests {
    use idevice::core_device::{PasteboardEntry, PasteboardItem};

    use super::*;

    fn snapshot(payload: PasteboardPayload) -> PasteboardSnapshot {
        PasteboardSnapshot {
            pasteboard_name: Some(GENERAL_PASTEBOARD.into()),
            change_count: Some(42),
            items: vec![PasteboardItem {
                index: 3,
                types: vec!["public.utf8-plain-text".into()],
                data: vec![PasteboardEntry {
                    uti: "public.utf8-plain-text".into(),
                    payload,
                }],
            }],
        }
    }

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR".to_vec();
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes
    }

    fn jpeg(width: u16, height: u16) -> Vec<u8> {
        vec![
            0xff,
            0xd8,
            0xff,
            0xe0,
            0x00,
            0x04,
            0x00,
            0x00,
            0xff,
            0xc0,
            0x00,
            0x07,
            0x08,
            (height >> 8) as u8,
            height as u8,
            (width >> 8) as u8,
            width as u8,
        ]
    }

    #[test]
    fn accepts_bounded_unicode_text_without_changing_it() {
        let text = "hello 世界\n";
        let (bytes, characters) = validate_write_text(text).unwrap();
        assert_eq!(bytes, 13);
        assert_eq!(characters, 9);
    }

    #[test]
    fn rejects_empty_and_oversized_writes() {
        assert!(validate_write_text("").is_err());
        assert!(validate_write_text(&"x".repeat(MAX_TEXT_BYTES + 1)).is_err());
    }

    #[test]
    fn finds_inline_and_promised_text_candidates() {
        assert_eq!(
            text_candidate(&snapshot(PasteboardPayload::Inline(b"hello".to_vec()))),
            Some(TextCandidate::Inline(b"hello".to_vec())),
        );
        assert_eq!(
            text_candidate(&snapshot(PasteboardPayload::Promised { size: Some(5) })),
            Some(TextCandidate::Promised {
                item_index: 3,
                uti: "public.utf8-plain-text".into(),
                size: Some(5),
            }),
        );
    }

    #[test]
    fn ignores_non_text_items() {
        let mut snapshot = snapshot(PasteboardPayload::Inline(vec![1, 2, 3]));
        snapshot.items[0].data[0].uti = "public.png".into();
        snapshot.items[0].types[0] = "public.png".into();
        assert_eq!(text_candidate(&snapshot), None);
    }

    #[test]
    fn finds_only_supported_image_candidates() {
        let mut promised = snapshot(PasteboardPayload::Promised { size: Some(24) });
        promised.items[0].data[0].uti = "public.png".into();
        promised.items[0].types[0] = "public.png".into();
        assert_eq!(
            image_candidate(&promised),
            Some(ImageCandidate::Promised {
                item_index: 3,
                uti: "public.png".into(),
                mime_type: "image/png".into(),
                size: Some(24),
            }),
        );

        promised.items[0].data[0].uti = "public.tiff".into();
        promised.items[0].types[0] = "public.tiff".into();
        assert_eq!(image_candidate(&promised), None);

        assert!(matches!(
            bounded_image_promise(Some(MAX_IMAGE_BYTES as i64)),
            ImagePromiseDecision::Resolve { .. }
        ));
        let ImagePromiseDecision::Reject(unbounded) = bounded_image_promise(None) else {
            panic!("unknown-size images must stay remote")
        };
        assert_eq!(unbounded.state, "unbounded");
        let ImagePromiseDecision::Reject(oversized) =
            bounded_image_promise(Some(MAX_IMAGE_BYTES as i64 + 1))
        else {
            panic!("oversized images must stay remote")
        };
        assert_eq!(oversized.state, "oversized");
    }

    #[test]
    fn reads_png_and_jpeg_dimensions_from_bounded_headers() {
        assert_eq!(png_dimensions(&png(1179, 2556)), Some((1179, 2556)));
        assert_eq!(jpeg_dimensions(&jpeg(4032, 3024)), Some((4032, 3024)));
        assert_eq!(png_dimensions(b"not png"), None);
        assert_eq!(jpeg_dimensions(&[0xff, 0xd8, 0xff, 0xc0, 0, 20]), None);
    }

    #[test]
    fn validates_image_format_dimensions_and_pixel_budget() {
        assert_eq!(
            validated_image_metadata(&png(1179, 2556)).unwrap(),
            ImageMetadata {
                uti: "public.png",
                mime_type: "image/png",
                width: 1179,
                height: 2556,
            },
        );
        assert!(validated_image_metadata(&png(0, 1)).is_err());
        assert!(validated_image_metadata(&png(MAX_IMAGE_DIMENSION + 1, 1)).is_err());
        assert!(validated_image_metadata(&png(8000, 8000)).is_err());
        assert!(validated_image_metadata(b"not an image").is_err());
    }

    #[test]
    fn requires_supported_extensions_and_matching_device_formats() {
        assert_eq!(
            expected_image_mime(Path::new("/tmp/photo.PNG")).unwrap(),
            "image/png",
        );
        assert_eq!(
            expected_image_mime(Path::new("/tmp/photo.jpeg")).unwrap(),
            "image/jpeg",
        );
        assert!(expected_image_mime(Path::new("/tmp/photo.gif")).is_err());

        let valid = normalized_image(png(10, 20), "public.png", "image/png");
        assert_eq!(valid.state, "image");
        assert_eq!(valid.width, Some(10));
        assert!(
            valid
                .data_url
                .unwrap()
                .starts_with("data:image/png;base64,")
        );
        assert_eq!(
            normalized_image(png(10, 20), "public.jpeg", "image/jpeg").state,
            "invalid-image",
        );
    }

    #[test]
    fn rejects_oversized_inline_images_before_previewing() {
        let normalized = normalized_image(vec![0; MAX_IMAGE_BYTES + 1], "public.png", "image/png");
        assert_eq!(normalized.state, "oversized");
        assert!(normalized.data_url.is_none());
        assert_eq!(normalized.byte_length, (MAX_IMAGE_BYTES + 1) as u64);
    }

    #[test]
    fn normalizes_utf8_and_rejects_invalid_or_oversized_bytes() {
        let normalized = normalized_bytes("hello 世界".as_bytes().to_vec());
        assert_eq!(normalized.text.as_deref(), Some("hello 世界"));
        assert_eq!(normalized.byte_length, 12);
        assert_eq!(normalized.character_count, 8);
        assert_eq!(normalized.state, "text");

        assert_eq!(normalized_bytes(vec![0xff]).state, "invalid-text");
        assert_eq!(
            normalized_bytes(vec![b'x'; MAX_TEXT_BYTES + 1]).state,
            "oversized",
        );
    }

    #[test]
    fn pasteboard_is_coredevice_only() {
        assert_eq!(
            pasteboard_transport(DeveloperGeneration::Legacy),
            PasteboardTransport::Unsupported,
        );
        assert_eq!(
            pasteboard_transport(DeveloperGeneration::CoreDeviceRemote),
            PasteboardTransport::RemoteRsd,
        );
        assert_eq!(
            pasteboard_transport(DeveloperGeneration::CoreDeviceLockdown),
            PasteboardTransport::CoreDeviceRsd,
        );
    }
}
