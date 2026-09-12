//! The Developer Disk Image used by iOS 16 and earlier.
//!
//! iOS 17 and later personalize their image on the device, so nothing has to be
//! supplied. Legacy devices mount a prebuilt `DeveloperDiskImage.dmg` that Apple
//! signs, and Xcode only ships images for the releases it supported. Rather than
//! depend on which Xcode a given Mac has, every Legacy device mounts one pinned
//! image, so behaviour does not change from machine to machine.
//!
//! The image is not redistributed with the application. It is downloaded once,
//! on request, into the user's own `~/Library/Developer/DeveloperDiskImages`,
//! which is also where Xcode-style images are read from. Both files are checked
//! against a pinned SHA-256 before they are installed.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{CommandError, CommandResult};

const IMAGE_NAME: &str = "DeveloperDiskImage.dmg";
const SIGNATURE_NAME: &str = "DeveloperDiskImage.dmg.signature";

/// The single image every Legacy device mounts.
pub const IMAGE_VERSION: &str = "14.2";

/// Where the image is installed and read from.
pub const INSTALL_DIRECTORY: &str = "~/Library/Developer/DeveloperDiskImages";

/// Source, pinned to a commit so the bytes cannot change under the digests
/// below. The digests are what actually protect the download; the commit only
/// keeps a moving branch from turning a working install into a failed one.
const SOURCE_COMMIT: &str = "5423e4e955fbb3a9eef3e1212acfbfc6e7a26236";
const SOURCE_REPOSITORY: &str = "doronz88/DeveloperDiskImage";

/// Measured on 2026-08-30 from the pinned commit.
const IMAGE_SHA256: &str = "8a507d70b0aee01667b0b3cb9663193acef776642b76ad25b21e633f75cbd4c0";
const SIGNATURE_SHA256: &str = "3b4ef128aa672cdd5e89d416b942ac26fada52d2f603be35c2dde043818a76a3";
pub const IMAGE_BYTES: u64 = 19_789_186;
const SIGNATURE_BYTES: u64 = 128;

/// A refusal ceiling, so a redirected or replaced URL cannot stream without
/// bound before the digest check would reject it.
const MAXIMUM_DOWNLOAD_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyDdi {
    pub image: PathBuf,
    pub signature: PathBuf,
}

fn expand_home(path: &str) -> Option<PathBuf> {
    let rest = path.strip_prefix("~/")?;
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(rest))
}

/// Where a downloaded image is written, and the first place one is looked for.
pub fn install_directory() -> Option<PathBuf> {
    expand_home(INSTALL_DIRECTORY).map(|root| root.join(IMAGE_VERSION))
}

/// Reads one Xcode-style directory: both files must be present to be usable.
fn read_pair(directory: &Path) -> Option<LegacyDdi> {
    let image = directory.join(IMAGE_NAME);
    let signature = directory.join(SIGNATURE_NAME);
    (image.is_file() && signature.is_file()).then_some(LegacyDdi { image, signature })
}

/// Every directory that may already hold the image, most specific first. A
/// matching Xcode installation is accepted so an existing Mac does not have to
/// download a file it already has.
pub fn search_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(installed) = install_directory() {
        directories.push(installed);
    }
    for developer in xcode_developer_directories() {
        directories.push(
            developer
                .join("Platforms")
                .join("iPhoneOS.platform")
                .join("DeviceSupport")
                .join(IMAGE_VERSION),
        );
    }
    directories
}

/// `xcode-select -p` is read from its link rather than by running the tool, so
/// selecting a device never blocks on spawning a process.
fn xcode_developer_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Ok(path) = std::env::var("DEVELOPER_DIR") {
        directories.push(PathBuf::from(path));
    }
    if let Ok(link) = std::fs::read_link("/var/db/xcode_select_link") {
        directories.push(link);
    }
    directories.push(PathBuf::from("/Applications/Xcode.app/Contents/Developer"));
    directories.sort();
    directories.dedup();
    directories
}

/// The installed image, if this Mac already has one.
pub fn installed() -> Option<LegacyDdi> {
    search_directories().iter().find_map(|path| read_pair(path))
}

/// Shown when a Legacy device connects and no image is installed. It is an
/// instruction rather than a failure, because the download is one action away.
pub fn missing_image_message() -> String {
    format!(
        "iOS 16 and earlier need a Developer Disk Image before developer services work. Open Developer to download it once ({} MB); it is stored in {INSTALL_DIRECTORY} and reused by every device afterwards.",
        IMAGE_BYTES / 1_000_000
    )
}

fn digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Rejects a payload that is not exactly the pinned file. Size is checked first
/// so a wrong answer is reported as a wrong answer rather than a hash mismatch.
fn verify(
    label: &str,
    bytes: &[u8],
    expected_len: u64,
    expected_digest: &str,
) -> CommandResult<()> {
    if bytes.len() as u64 != expected_len {
        return Err(CommandError::new(
            "ddi",
            format!(
                "The downloaded {label} is {} bytes, but {expected_len} were expected. The download was not installed.",
                bytes.len()
            ),
            true,
        ));
    }
    let found = digest(bytes);
    if found != expected_digest {
        return Err(CommandError::new(
            "ddi",
            format!(
                "The downloaded {label} does not match its pinned checksum ({found}). The download was not installed."
            ),
            false,
        ));
    }
    Ok(())
}

fn source_url(file: &str) -> String {
    format!(
        "https://raw.githubusercontent.com/{SOURCE_REPOSITORY}/{SOURCE_COMMIT}/DeveloperDiskImages/{IMAGE_VERSION}/{file}"
    )
}

/// A one-line description of where the download comes from, for the interface
/// and for the acceptance record.
pub fn source_description() -> String {
    format!("{SOURCE_REPOSITORY} @ {}", &SOURCE_COMMIT[..7])
}

async fn fetch(
    client: &reqwest::Client,
    file: &str,
    expected_len: u64,
    expected_digest: &str,
    mut progress: impl FnMut(u64, u64),
) -> CommandResult<Vec<u8>> {
    let response = client
        .get(source_url(file))
        .send()
        .await
        .map_err(|error| CommandError::new("ddi", format!("Download failed: {error}"), true))?;
    if !response.status().is_success() {
        return Err(CommandError::new(
            "ddi",
            format!("Download failed with HTTP {}", response.status()),
            true,
        ));
    }
    let total = response.content_length().unwrap_or(expected_len);
    let mut received = Vec::with_capacity(expected_len as usize);
    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk
            .map_err(|error| CommandError::new("ddi", format!("Download failed: {error}"), true))?;
        received.extend_from_slice(&chunk);
        if received.len() as u64 > MAXIMUM_DOWNLOAD_BYTES {
            return Err(CommandError::new(
                "ddi",
                "The download exceeded its expected size and was stopped".to_owned(),
                false,
            ));
        }
        progress(received.len() as u64, total);
    }
    verify(file, &received, expected_len, expected_digest)?;
    Ok(received)
}

/// Downloads and installs the image, reporting progress as a percentage.
///
/// Nothing is written until both files have passed their digest check, and each
/// file lands through a temporary name in the same directory, so an interrupted
/// download cannot leave a half-written image that later looks installed.
pub async fn download(mut progress: impl FnMut(u64)) -> CommandResult<LegacyDdi> {
    let directory = install_directory().ok_or_else(|| {
        CommandError::new("ddi", "Unable to locate the current home directory", false)
    })?;
    let client = reqwest::Client::builder()
        .user_agent("idevice_desktop")
        .build()
        .map_err(|error| CommandError::new("ddi", error.to_string(), false))?;

    // The signature is 128 bytes against the image's 19 MB, so the image alone
    // drives the reported percentage.
    let signature = fetch(
        &client,
        SIGNATURE_NAME,
        SIGNATURE_BYTES,
        SIGNATURE_SHA256,
        |_, _| {},
    )
    .await?;
    let image = fetch(
        &client,
        IMAGE_NAME,
        IMAGE_BYTES,
        IMAGE_SHA256,
        |current, total| {
            let percent = if total == 0 {
                0
            } else {
                ((current as f64 / total as f64) * 100.0).round() as u64
            };
            progress(percent.min(100));
        },
    )
    .await?;

    tokio::fs::create_dir_all(&directory).await?;
    write_atomically(&directory.join(IMAGE_NAME), &image).await?;
    write_atomically(&directory.join(SIGNATURE_NAME), &signature).await?;
    read_pair(&directory).ok_or_else(|| {
        CommandError::new(
            "ddi",
            format!(
                "The image was downloaded but is missing from {}",
                directory.display()
            ),
            false,
        )
    })
}

async fn write_atomically(destination: &Path, bytes: &[u8]) -> CommandResult<()> {
    let temporary = destination.with_extension(format!("partial-{}", std::process::id()));
    tokio::fs::write(&temporary, bytes).await?;
    match tokio::fs::rename(&temporary, destination).await {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = tokio::fs::remove_file(&temporary).await;
            Err(error.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        IMAGE_BYTES, IMAGE_SHA256, IMAGE_VERSION, digest, install_directory, missing_image_message,
        read_pair, search_directories, source_url, verify,
    };

    fn temporary_directory(label: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("ddi-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create");
        path
    }

    #[test]
    fn a_directory_counts_as_installed_only_with_both_files() {
        let directory = temporary_directory("pair");
        assert_eq!(read_pair(&directory), None);
        std::fs::write(directory.join("DeveloperDiskImage.dmg"), b"image").expect("write");
        // A signature is what the device verifies; an image alone is unusable
        // and must not be reported as installed.
        assert_eq!(read_pair(&directory), None);
        std::fs::write(directory.join("DeveloperDiskImage.dmg.signature"), b"sig").expect("write");
        let found = read_pair(&directory).expect("installed");
        assert!(found.image.ends_with("DeveloperDiskImage.dmg"));
        assert!(
            found
                .signature
                .ends_with("DeveloperDiskImage.dmg.signature")
        );
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_install_directory_is_searched_before_xcode() {
        let directories = search_directories();
        let installed = install_directory().expect("home");
        assert_eq!(directories.first(), Some(&installed));
        assert!(installed.ends_with(IMAGE_VERSION));
        assert!(
            directories
                .iter()
                .skip(1)
                .all(|path| path.ends_with(IMAGE_VERSION))
        );
    }

    #[test]
    fn a_payload_of_the_wrong_size_is_rejected_before_its_digest() {
        let error = verify("image", b"short", IMAGE_BYTES, IMAGE_SHA256).expect_err("rejected");
        assert!(error.message.contains("5 bytes"));
        assert!(error.message.contains("was not installed"));
        // A size mismatch is usually a truncated transfer, so it is retryable.
        assert!(error.retryable);
    }

    #[test]
    fn a_payload_with_the_wrong_contents_is_rejected() {
        let bytes = vec![0u8; 4];
        let error = verify("signature", &bytes, 4, IMAGE_SHA256).expect_err("rejected");
        assert!(error.message.contains("pinned checksum"));
        // Correct length with wrong contents is not fixed by retrying.
        assert!(!error.retryable);
        assert!(verify("signature", &bytes, 4, &digest(&bytes)).is_ok());
    }

    #[test]
    fn the_source_url_is_pinned_to_a_commit_and_the_single_version() {
        let url = source_url("DeveloperDiskImage.dmg");
        assert!(url.contains("/5423e4e955fbb3a9eef3e1212acfbfc6e7a26236/"));
        assert!(url.contains("/DeveloperDiskImages/14.2/"));
        assert!(!url.contains("/main/"));
    }

    #[test]
    fn the_missing_image_message_states_the_size_and_that_it_is_kept() {
        let message = missing_image_message();
        assert!(message.contains("19 MB"));
        assert!(message.contains("~/Library/Developer/DeveloperDiskImages"));
    }
}
