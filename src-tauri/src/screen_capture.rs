//! Native iPhone screen recording through AVFoundation.
//!
//! Once CoreMediaIO's screen-capture opt-in is set, macOS exposes a USB-connected
//! iOS device as a muxed capture device — the path QuickTime's movie recording
//! uses. It needs no developer disk image and is independent of the device's
//! developer-service generation, and it records the native resolution at up to
//! 60 FPS with the device's audio. It does not work over Wi-Fi.
//!
//! Every AVFoundation object is created, used, and dropped on the thread that
//! calls into this module, so nothing here crosses threads except the event
//! channel the recording delegate writes to.

use std::{
    ffi::c_void,
    path::{Path, PathBuf},
    sync::{Mutex, mpsc},
    time::{Duration, Instant},
};

use block2::RcBlock;
use objc2::{
    AllocAnyThread, DefinedClass, define_class, msg_send,
    rc::{Retained, autoreleasepool},
    runtime::{Bool, ProtocolObject},
};
use objc2_av_foundation::{
    AVAssetExportPresetHighestQuality, AVAssetExportSession, AVAssetExportSessionStatus,
    AVAuthorizationStatus, AVCaptureConnection, AVCaptureDevice, AVCaptureDeviceDiscoverySession,
    AVCaptureDeviceInput, AVCaptureDevicePosition, AVCaptureDeviceTypeExternal,
    AVCaptureFileOutput, AVCaptureFileOutputRecordingDelegate, AVCaptureMovieFileOutput,
    AVCaptureSession, AVError, AVErrorRecordingSuccessfullyFinishedKey, AVFileTypeMPEG4,
    AVMediaTypeMuxed, AVMediaTypeVideo, AVMutableComposition, AVURLAsset,
};
use objc2_core_media::{CMTime, CMTimeRange};
use objc2_foundation::{NSArray, NSError, NSNumber, NSObject, NSObjectProtocol, NSString, NSURL};

/// Muxed capture devices also include tape decks and other DV hardware; iOS
/// devices identify themselves with this model ID.
const IOS_MODEL_ID: &str = "iOS Device";
/// CoreMediaIO publishes the device shortly after the opt-in; 0.6 s was
/// measured with one iPhone attached.
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(10);
/// The session can take about two seconds to deliver its first sample.
const FIRST_SAMPLE_TIMEOUT: Duration = Duration::from_secs(15);
const PERMISSION_TIMEOUT: Duration = Duration::from_secs(300);
const POLL: Duration = Duration::from_millis(100);
const TIMESCALE: i32 = 600;
/// Shorter kept segments are rounding noise from the editor, not real content.
pub const MIN_SEGMENT_MS: u64 = 100;

const CMIO_SYSTEM_OBJECT: u32 = 1;
const CMIO_ALLOW_SCREEN_CAPTURE_DEVICES: u32 = u32::from_be_bytes(*b"yes ");
const CMIO_SCOPE_GLOBAL: u32 = u32::from_be_bytes(*b"glob");
const CMIO_ELEMENT_MAIN: u32 = 0;

#[repr(C)]
struct CmioPropertyAddress {
    selector: u32,
    scope: u32,
    element: u32,
}

#[link(name = "CoreMediaIO", kind = "framework")]
unsafe extern "C" {
    fn CMIOObjectSetPropertyData(
        object: u32,
        address: *const CmioPropertyAddress,
        qualifier_size: u32,
        qualifier: *const c_void,
        data_size: u32,
        data: *const c_void,
    ) -> i32;
}

#[derive(Debug, PartialEq, Eq)]
pub enum CaptureError {
    Cancelled,
    Failed(String),
}

impl CaptureError {
    fn failed(message: impl Into<String>) -> Self {
        Self::Failed(message.into())
    }
}

pub type CaptureResult<T> = Result<T, CaptureError>;

#[derive(Debug, Clone, PartialEq)]
pub struct RecordedMovie {
    pub device_name: String,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
}

pub enum RecorderMessage {
    Started,
    Finished(Option<String>),
    Stop,
}

/// Lets the process see iOS devices as capture devices. QuickTime sets the
/// same system property; it applies to this process only.
fn allow_screen_capture_devices() -> CaptureResult<()> {
    let address = CmioPropertyAddress {
        selector: CMIO_ALLOW_SCREEN_CAPTURE_DEVICES,
        scope: CMIO_SCOPE_GLOBAL,
        element: CMIO_ELEMENT_MAIN,
    };
    let allow: u32 = 1;
    // SAFETY: the address and data point at live values of the declared sizes.
    let status = unsafe {
        CMIOObjectSetPropertyData(
            CMIO_SYSTEM_OBJECT,
            &address,
            0,
            std::ptr::null(),
            std::mem::size_of::<u32>() as u32,
            (&allow as *const u32).cast(),
        )
    };
    if status == 0 {
        Ok(())
    } else {
        Err(CaptureError::failed(format!(
            "macOS refused iPhone screen capture (CoreMediaIO status {status})"
        )))
    }
}

const CAMERA_DENIED: &str = "Camera access is off for idevice_desktop. macOS files iPhone screen \
     capture under the camera permission: allow it in System Settings → Privacy & Security → \
     Camera, then retry.";

fn ensure_capture_permission(cancelled: &dyn Fn() -> bool) -> CaptureResult<()> {
    // SAFETY: AVMediaTypeVideo is a framework constant that is always present.
    let video = unsafe { AVMediaTypeVideo }
        .ok_or_else(|| CaptureError::failed("AVFoundation is unavailable"))?;
    // SAFETY: video is a valid media type for this query.
    match unsafe { AVCaptureDevice::authorizationStatusForMediaType(video) } {
        AVAuthorizationStatus::Authorized => Ok(()),
        AVAuthorizationStatus::NotDetermined => {
            let (sender, receiver) = mpsc::channel();
            let sender = Mutex::new(sender);
            let handler = RcBlock::new(move |granted: Bool| {
                if let Ok(sender) = sender.lock() {
                    let _ = sender.send(granted.as_bool());
                }
            });
            // SAFETY: the block lives until AVFoundation releases it.
            unsafe {
                AVCaptureDevice::requestAccessForMediaType_completionHandler(video, &handler)
            };
            let started = Instant::now();
            loop {
                match receiver.recv_timeout(POLL) {
                    Ok(true) => return Ok(()),
                    Ok(false) => return Err(CaptureError::failed(CAMERA_DENIED)),
                    Err(_) if cancelled() => return Err(CaptureError::Cancelled),
                    Err(_) if started.elapsed() > PERMISSION_TIMEOUT => {
                        return Err(CaptureError::failed(CAMERA_DENIED));
                    }
                    Err(_) => {}
                }
            }
        }
        _ => Err(CaptureError::failed(CAMERA_DENIED)),
    }
}

fn screen_devices() -> Vec<Retained<AVCaptureDevice>> {
    // SAFETY: framework constants; the discovery session only reads them.
    let devices = unsafe {
        let types = NSArray::from_slice(&[AVCaptureDeviceTypeExternal]);
        AVCaptureDeviceDiscoverySession::discoverySessionWithDeviceTypes_mediaType_position(
            &types,
            AVMediaTypeMuxed,
            AVCaptureDevicePosition::Unspecified,
        )
        .devices()
    };
    devices
        .to_vec()
        .into_iter()
        // SAFETY: plain property reads.
        .filter(|device| unsafe { device.modelID() }.to_string() == IOS_MODEL_ID)
        .collect()
}

fn normalized_name(name: &str) -> String {
    name.trim()
        .replace(['\u{2018}', '\u{2019}'], "'")
        .to_lowercase()
}

/// Picks the capture device for the selected iPhone.
///
/// A capture device's unique ID is not the device UDID, so the match is by the
/// name the user gave the phone, which Lockdown also reports. With no usable
/// name, a single attached iPhone is unambiguous.
pub fn choose_device(names: &[String], wanted: Option<&str>) -> Result<usize, String> {
    if names.is_empty() {
        return Err("No iPhone screen is available. Connect the iPhone with a USB cable, unlock it, and trust this Mac.".into());
    }
    if let Some(wanted) = wanted.map(normalized_name).filter(|name| !name.is_empty())
        && let Some(index) = names
            .iter()
            .position(|name| normalized_name(name) == wanted)
    {
        return Ok(index);
    }
    if names.len() == 1 {
        return Ok(0);
    }
    Err(format!(
        "Several iPhones are connected over USB ({}) and none matches the selected device. Disconnect the others and retry.",
        names.join(", ")
    ))
}

fn find_device(
    wanted: Option<&str>,
    cancelled: &dyn Fn() -> bool,
) -> CaptureResult<Retained<AVCaptureDevice>> {
    let started = Instant::now();
    loop {
        let devices = screen_devices();
        // SAFETY: plain property reads.
        let names: Vec<String> = devices
            .iter()
            .map(|device| unsafe { device.localizedName() }.to_string())
            .collect();
        match choose_device(&names, wanted) {
            Ok(index) => return Ok(devices[index].clone()),
            Err(message) if !devices.is_empty() || started.elapsed() > DISCOVERY_TIMEOUT => {
                return Err(CaptureError::Failed(message));
            }
            Err(_) => {}
        }
        if cancelled() {
            return Err(CaptureError::Cancelled);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "IdeviceDesktopRecordingDelegate"]
    #[ivars = Mutex<mpsc::Sender<RecorderMessage>>]
    struct RecordingDelegate;

    unsafe impl NSObjectProtocol for RecordingDelegate {}

    unsafe impl AVCaptureFileOutputRecordingDelegate for RecordingDelegate {
        #[unsafe(method(captureOutput:didStartRecordingToOutputFileAtURL:fromConnections:))]
        fn did_start(
            &self,
            _output: &AVCaptureFileOutput,
            _url: &NSURL,
            _connections: &NSArray<AVCaptureConnection>,
        ) {
            self.send(RecorderMessage::Started);
        }

        #[unsafe(method(captureOutput:didFinishRecordingToOutputFileAtURL:fromConnections:error:))]
        fn did_finish(
            &self,
            _output: &AVCaptureFileOutput,
            _url: &NSURL,
            _connections: &NSArray<AVCaptureConnection>,
            error: Option<&NSError>,
        ) {
            self.send(RecorderMessage::Finished(error.and_then(fatal_recording_error)));
        }
    }
);

impl RecordingDelegate {
    fn new(sender: mpsc::Sender<RecorderMessage>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(Mutex::new(sender));
        // SAFETY: NSObject's designated initializer.
        unsafe { msg_send![super(this), init] }
    }

    fn send(&self, message: RecorderMessage) {
        if let Ok(sender) = self.ivars().lock() {
            let _ = sender.send(message);
        }
    }
}

/// AVFoundation reports an error for some recordings that still finished with
/// a playable file — an unplugged cable, for one — and flags those in userInfo.
fn fatal_recording_error(error: &NSError) -> Option<String> {
    // SAFETY: framework constant.
    let finished = unsafe { AVErrorRecordingSuccessfullyFinishedKey }
        .and_then(|key| error.userInfo().objectForKey(key))
        .and_then(|value| value.downcast::<NSNumber>().ok())
        .is_some_and(|value| value.boolValue());
    if finished {
        None
    } else if error.code() == AVError::NoDataCaptured.0 {
        // A locked or sleeping iPhone sends no picture.
        Some("the iPhone sent no picture. Unlock it, keep the screen on, and retry".into())
    } else {
        Some(error.localizedDescription().to_string())
    }
}

fn file_url(path: &Path) -> Retained<NSURL> {
    NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()))
}

/// Records the selected iPhone's screen to `path` until a [`RecorderMessage::Stop`]
/// arrives on `receiver`, the device stops delivering, or `cancelled` turns true.
///
/// `sender` must feed `receiver`; the recording delegate reports through it.
/// `on_started` runs once the first sample is written. A cancelled recording
/// is deleted.
pub fn record(
    wanted_name: Option<&str>,
    path: &Path,
    sender: mpsc::Sender<RecorderMessage>,
    receiver: &mpsc::Receiver<RecorderMessage>,
    on_started: &dyn Fn(&str),
    cancelled: &dyn Fn() -> bool,
) -> CaptureResult<RecordedMovie> {
    let result = autoreleasepool(|_| {
        record_inner(wanted_name, path, sender, receiver, on_started, cancelled)
    });
    if result.is_err() {
        let _ = std::fs::remove_file(path);
    }
    result
}

fn record_inner(
    wanted_name: Option<&str>,
    path: &Path,
    sender: mpsc::Sender<RecorderMessage>,
    receiver: &mpsc::Receiver<RecorderMessage>,
    on_started: &dyn Fn(&str),
    cancelled: &dyn Fn() -> bool,
) -> CaptureResult<RecordedMovie> {
    allow_screen_capture_devices()?;
    ensure_capture_permission(cancelled)?;
    let device = find_device(wanted_name, cancelled)?;
    // SAFETY: plain property read.
    let device_name = unsafe { device.localizedName() }.to_string();

    // SAFETY: the device, session, and output are used on this thread only and
    // outlive the recording, which finishes before they are dropped.
    unsafe {
        let input =
            AVCaptureDeviceInput::deviceInputWithDevice_error(&device).map_err(|error| {
                CaptureError::Failed(format!(
                    "The iPhone screen could not be opened: {}",
                    error.localizedDescription()
                ))
            })?;
        let session = AVCaptureSession::new();
        if !session.canAddInput(&input) {
            return Err(CaptureError::failed(
                "The iPhone screen is in use by another app, such as QuickTime. Close it and retry.",
            ));
        }
        session.addInput(&input);
        let output = AVCaptureMovieFileOutput::new();
        if !session.canAddOutput(&output) {
            return Err(CaptureError::failed(
                "macOS cannot record this iPhone screen",
            ));
        }
        session.addOutput(&output);
        session.startRunning();
        if !session.isRunning() {
            return Err(CaptureError::failed(
                "The iPhone screen did not start. Unlock the iPhone and retry.",
            ));
        }

        let delegate = RecordingDelegate::new(sender);
        output.startRecordingToOutputFileURL_recordingDelegate(
            &file_url(path),
            ProtocolObject::from_ref(&*delegate),
        );

        let started_at = Instant::now();
        let mut started = false;
        let mut stopping = false;
        let mut discard = false;
        let mut stalled = false;
        let outcome = loop {
            match receiver.recv_timeout(POLL) {
                Ok(RecorderMessage::Started) if !started => {
                    started = true;
                    on_started(&device_name);
                }
                Ok(RecorderMessage::Started) => {}
                Ok(RecorderMessage::Stop) if !stopping => {
                    stopping = true;
                    output.stopRecording();
                }
                Ok(RecorderMessage::Stop) => {}
                Ok(RecorderMessage::Finished(error)) => break error,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break Some("The recording stopped unexpectedly".into());
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if stopping {
                        continue;
                    }
                    if cancelled() {
                        stopping = true;
                        discard = true;
                        output.stopRecording();
                    } else if !started && started_at.elapsed() > FIRST_SAMPLE_TIMEOUT {
                        stopping = true;
                        stalled = true;
                        output.stopRecording();
                    }
                }
            }
        };
        session.stopRunning();

        if discard {
            return Err(CaptureError::Cancelled);
        }
        if stalled {
            return Err(CaptureError::failed(
                "The iPhone sent no picture. Unlock it, keep the screen on, and retry.",
            ));
        }
        if !started {
            return Err(CaptureError::failed(
                "The recording stopped before the iPhone sent its first picture.",
            ));
        }
        if let Some(error) = outcome {
            return Err(CaptureError::Failed(format!(
                "The recording failed: {error}"
            )));
        }
    }

    let mut movie = inspect(path)?;
    movie.device_name = device_name;
    Ok(movie)
}

/// Reads the duration, picture size, and file size of a finished movie.
#[allow(deprecated)]
pub fn inspect(path: &Path) -> CaptureResult<RecordedMovie> {
    autoreleasepool(|_| {
        let bytes = std::fs::metadata(path)
            .map_err(|error| CaptureError::Failed(format!("The recording is missing: {error}")))?
            .len();
        // SAFETY: synchronous reads on a local file asset; the deprecated
        // synchronous accessors block until the values load.
        let (duration_ms, width, height) = unsafe {
            let asset = AVURLAsset::URLAssetWithURL_options(&file_url(path), None);
            let seconds = asset.duration().seconds();
            let video = AVMediaTypeVideo
                .ok_or_else(|| CaptureError::failed("AVFoundation is unavailable"))?;
            let size = asset
                .tracksWithMediaType(video)
                .firstObject()
                .map(|track| track.naturalSize())
                .ok_or_else(|| CaptureError::failed("The recording contains no video"))?;
            (
                if seconds.is_finite() && seconds > 0.0 {
                    (seconds * 1000.0).round() as u64
                } else {
                    0
                },
                size.width.round().max(0.0) as u32,
                size.height.round().max(0.0) as u32,
            )
        };
        if duration_ms == 0 {
            return Err(CaptureError::failed("The recording contains no video"));
        }
        Ok(RecordedMovie {
            device_name: String::new(),
            duration_ms,
            width,
            height,
            bytes,
        })
    })
}

/// Checks the segments an export keeps against the recording.
///
/// They must be ordered, non-overlapping, inside the recording, and at least
/// [`MIN_SEGMENT_MS`] long, and at least one must remain.
pub fn validate_segments(duration_ms: u64, segments: &[(u64, u64)]) -> Result<(), String> {
    if segments.is_empty() {
        return Err("Keep at least part of the recording before exporting".into());
    }
    let mut previous_end = 0;
    for &(start, end) in segments {
        if end > duration_ms {
            return Err("A kept segment runs past the end of the recording".into());
        }
        if end < start.saturating_add(MIN_SEGMENT_MS) {
            return Err("A kept segment is too short to export".into());
        }
        if start < previous_end {
            return Err("Kept segments must be in order and must not overlap".into());
        }
        previous_end = end;
    }
    Ok(())
}

fn cm_time(milliseconds: u64) -> CMTime {
    // SAFETY: pure value construction.
    unsafe { CMTime::with_seconds(milliseconds as f64 / 1000.0, TIMESCALE) }
}

/// The partial file an export writes before it is moved over `destination`.
pub fn partial_export_path(destination: &Path) -> PathBuf {
    let name = destination
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "recording".into());
    destination.with_file_name(format!(".{name}.exporting.mp4"))
}

/// Joins the kept segments of `source` into an H.264 MP4 at `destination`.
///
/// The movie is written beside the destination and moved into place only once
/// it is complete, so a failed or cancelled export never leaves a truncated file.
#[allow(deprecated)]
pub fn export(
    source: &Path,
    destination: &Path,
    segments: &[(u64, u64)],
    progress: &dyn Fn(f64),
    cancelled: &dyn Fn() -> bool,
) -> CaptureResult<()> {
    let partial = partial_export_path(destination);
    let _ = std::fs::remove_file(&partial);
    let result = autoreleasepool(|_| export_inner(source, &partial, segments, progress, cancelled));
    match result {
        Ok(()) => std::fs::rename(&partial, destination).map_err(|error| {
            let _ = std::fs::remove_file(&partial);
            CaptureError::Failed(format!("The exported video could not be saved: {error}"))
        }),
        Err(error) => {
            let _ = std::fs::remove_file(&partial);
            Err(error)
        }
    }
}

#[allow(deprecated)]
fn export_inner(
    source: &Path,
    partial: &Path,
    segments: &[(u64, u64)],
    progress: &dyn Fn(f64),
    cancelled: &dyn Fn() -> bool,
) -> CaptureResult<()> {
    // SAFETY: every object is local to this call, and the export finishes —
    // completion handler included — before they are dropped.
    unsafe {
        let asset = AVURLAsset::URLAssetWithURL_options(&file_url(source), None);
        let composition = AVMutableComposition::composition();
        let mut cursor = 0;
        for &(start, end) in segments {
            let range = CMTimeRange::new(cm_time(start), cm_time(end - start));
            composition
                .insertTimeRange_ofAsset_atTime_error(range, &asset, cm_time(cursor))
                .map_err(|error| {
                    CaptureError::Failed(format!(
                        "A segment could not be added: {}",
                        error.localizedDescription()
                    ))
                })?;
            cursor += end - start;
        }

        let session = AVAssetExportSession::exportSessionWithAsset_presetName(
            &composition,
            AVAssetExportPresetHighestQuality,
        )
        .ok_or_else(|| CaptureError::failed("macOS cannot export this recording as MP4"))?;
        session.setOutputURL(Some(&file_url(partial)));
        session.setOutputFileType(AVFileTypeMPEG4);
        session.setShouldOptimizeForNetworkUse(true);

        let (sender, receiver) = mpsc::channel();
        let sender = Mutex::new(sender);
        let handler = RcBlock::new(move || {
            if let Ok(sender) = sender.lock() {
                let _ = sender.send(());
            }
        });
        session.exportAsynchronouslyWithCompletionHandler(&handler);
        let mut cancel_sent = false;
        while receiver.recv_timeout(Duration::from_millis(200)).is_err() {
            progress(f64::from(session.progress()));
            if !cancel_sent && cancelled() {
                cancel_sent = true;
                session.cancelExport();
            }
        }

        match session.status() {
            AVAssetExportSessionStatus::Completed => {
                progress(1.0);
                Ok(())
            }
            AVAssetExportSessionStatus::Cancelled => Err(CaptureError::Cancelled),
            _ => Err(CaptureError::Failed(format!(
                "The export failed: {}",
                session
                    .error()
                    .map(|error| error.localizedDescription().to_string())
                    .unwrap_or_else(|| "unknown error".into())
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn picks_the_device_whose_name_matches_ignoring_apostrophe_style() {
        let devices = names(&["Work iPhone", "baotrouble\u{2019}s iPhone"]);
        assert_eq!(choose_device(&devices, Some("baotrouble's iPhone")), Ok(1));
        assert_eq!(choose_device(&devices, Some(" WORK IPHONE ")), Ok(0));
    }

    #[test]
    fn a_single_device_is_used_even_when_the_name_differs() {
        assert_eq!(choose_device(&names(&["iPhone"]), Some("Renamed")), Ok(0));
        assert_eq!(choose_device(&names(&["iPhone"]), None), Ok(0));
    }

    #[test]
    fn refuses_to_guess_between_several_devices_or_none() {
        assert!(choose_device(&[], Some("iPhone")).is_err());
        let error = choose_device(&names(&["A", "B"]), Some("C")).unwrap_err();
        assert!(error.contains("A, B"), "{error}");
        assert!(choose_device(&names(&["A", "B"]), None).is_err());
    }

    #[test]
    fn accepts_ordered_segments_inside_the_recording() {
        assert!(validate_segments(10_000, &[(0, 2_000), (2_000, 5_000), (8_000, 10_000)]).is_ok());
    }

    #[test]
    fn rejects_empty_overlapping_short_or_overrunning_segments() {
        assert!(validate_segments(10_000, &[]).is_err());
        assert!(validate_segments(10_000, &[(0, 5_000), (4_000, 6_000)]).is_err());
        assert!(validate_segments(10_000, &[(3_000, 1_000)]).is_err());
        assert!(validate_segments(10_000, &[(1_000, 1_050)]).is_err());
        assert!(validate_segments(10_000, &[(9_000, 10_001)]).is_err());
    }

    #[test]
    fn the_partial_export_sits_hidden_beside_its_destination() {
        assert_eq!(
            partial_export_path(Path::new("/tmp/clips/demo.mp4")),
            PathBuf::from("/tmp/clips/.demo.exporting.mp4")
        );
    }
}
