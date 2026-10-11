//! Version, path, and backup gates for a TrollStore helper install.
//!
//! The allowlist matches the check in TrollRestore's `trollstore.py`: iOS 15.0
//! through 16.6.x, iOS 16.7 only when the build is 20H18, and iOS 17.0.
//! The backup writer only builds that tool's helper restore. It is not a
//! general way to place a file on the device.

use std::{
    fs,
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use sha1::{Digest, Sha1};
use sha2::Sha256;

use crate::device_version::IosVersion;

/// `PersistenceHelper_Embedded` from TrollStore 2.1.1. The URL is the release
/// asset, not `latest`, and the digest is what the download is checked against.
pub const HELPER_URL: &str =
    "https://github.com/opa334/TrollStore/releases/download/2.1.1/PersistenceHelper_Embedded";
pub const HELPER_SHA256: &str = "f0915cb7a608522ef84a43c6617433037e030ff68f5db89a6bb982f8430a0cb9";
pub const HELPER_BYTES: u64 = 213_635;

pub fn helper_bytes_match(bytes: &[u8]) -> bool {
    if bytes.len() as u64 != HELPER_BYTES {
        return false;
    }
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    digest == HELPER_SHA256
}

pub const TROLLSTORE_BUNDLE_ID: &str = "com.opa334.TrollStore";

const SUPPORTED_WINDOW: &str =
    "Helper install supports iOS 15.0 through 16.7 RC (build 20H18) and iOS 17.0.";

/// Why a helper install must be refused, or `None` when the build is in range.
pub fn helper_install_refusal(product_version: &str, build: &str) -> Option<String> {
    let Some(version) = IosVersion::parse(product_version.trim()) else {
        return Some(format!(
            "iOS version \"{product_version}\" could not be read. {SUPPORTED_WINDOW}"
        ));
    };
    let build = build.trim();
    if version < version_of(15, 0, 0) {
        return Some(format!(
            "iOS {product_version} is below 15.0. {SUPPORTED_WINDOW}"
        ));
    }
    if version > version_of(17, 0, 0) {
        return Some(format!(
            "iOS {product_version} is newer than 17.0. TrollStore does not support it."
        ));
    }
    if version > version_of(16, 7, 0) && version < version_of(17, 0, 0) {
        let build_label = if build.is_empty() {
            "no build number".to_owned()
        } else {
            format!("build {build}")
        };
        return Some(format!(
            "iOS {product_version} ({build_label}) is after 16.7 and before 17.0. Those builds are not supported."
        ));
    }
    if version == version_of(16, 7, 0) && build != "20H18" {
        let shown = if build.is_empty() {
            "no build number".to_owned()
        } else {
            format!("build {build}")
        };
        return Some(format!(
            "iOS 16.7 is supported only as build 20H18, the 16.7 release candidate. This device reports {shown}."
        ));
    }
    None
}

/// TrollRestore allows 15.0 through 15.1.1 and also reported restore failures
/// there. Callers that display eligibility should show this alongside a pass.
pub fn helper_install_caution(product_version: &str) -> Option<&'static str> {
    let version = IosVersion::parse(product_version.trim())?;
    if version >= version_of(15, 0, 0) && version < version_of(15, 2, 0) {
        Some(
            "This version is inside the published window, and TrollRestore also reported restore failures on iOS 15.0 through 15.1.1.",
        )
    } else {
        None
    }
}

/// Bundle file name when `path` is a removable system app container.
///
/// Removable system apps live at
/// `/private/var/containers/Bundle/Application/<uuid>/<Name>.app`. A path on
/// the system partition, a traversal segment, or a file inside the bundle is
/// not a candidate.
pub fn removable_bundle_name(path: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty()
        || path
            .split('/')
            .any(|part| part == ".." || part.contains('\0'))
    {
        return None;
    }
    let rest = path
        .strip_prefix("/private/var/containers/Bundle/Application/")
        .or_else(|| path.strip_prefix("/var/containers/Bundle/Application/"))?;
    let mut parts = rest.split('/');
    let directory = parts.next().filter(|part| !part.is_empty())?;
    let bundle = parts.next().filter(|part| !part.is_empty())?;
    if parts.next().is_some() || directory.contains('.') {
        return None;
    }
    if !bundle.to_ascii_lowercase().ends_with(".app") {
        return None;
    }
    Some(bundle.to_owned())
}

/// Only Apple's own removable apps can be replaced by the helper.
///
/// Apps installed by TrollStore or a jailbreak register as `System` and live in
/// the same container directory, so the path alone would offer them too.
pub fn is_replaceable_apple_app(bundle_id: &str, application_type: Option<&str>) -> bool {
    bundle_id.starts_with("com.apple.")
        && application_type.is_some_and(|kind| kind.eq_ignore_ascii_case("system"))
}

/// Container id and bundle file name taken from a removable system-app path.
pub struct RemovableAppLocation {
    pub container_id: String,
    pub bundle_name: String,
}

pub fn removable_app_location(path: &str) -> Option<RemovableAppLocation> {
    let bundle_name = removable_bundle_name(path)?;
    let rest = path
        .trim()
        .strip_prefix("/private/var/containers/Bundle/Application/")
        .or_else(|| {
            path.trim()
                .strip_prefix("/var/containers/Bundle/Application/")
        })?;
    let container_id = rest.split('/').next()?.to_owned();
    accepted_target(&container_id, &bundle_name)?;
    Some(RemovableAppLocation {
        container_id,
        bundle_name,
    })
}

/// Names that are safe to interpolate into the helper backup.
///
/// The backup domains are fixed to TrollRestore's helper layout. A slash or a
/// traversal segment in either name would aim that layout somewhere else, so
/// both are refused before a directory is written.
pub fn accepted_target(container_id: &str, bundle_name: &str) -> Option<String> {
    if container_id.len() < 8
        || container_id.len() > 64
        || !container_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() || byte == b'-')
    {
        return None;
    }
    if !bundle_name.to_ascii_lowercase().ends_with(".app")
        || bundle_name.len() < 5
        || bundle_name.len() > 128
        || !bundle_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        return None;
    }
    let executable = bundle_name.split('.').next()?;
    if executable.is_empty() {
        return None;
    }
    Some(executable.to_owned())
}

/// Writes the helper backup into `directory`, which becomes the source folder
/// named by the caller (the device UDID). Returns nothing the device can be
/// pointed at other than this one restore.
pub fn write_helper_backup(
    directory: &Path,
    helper: &[u8],
    container_id: &str,
    bundle_name: &str,
) -> Result<(), String> {
    let executable = accepted_target(container_id, bundle_name)
        .ok_or_else(|| "The selected app is not a removable system app.".to_owned())?;
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;

    let container = format!(
        "SysContainerDomain-../../../../../../../../var/backup/var/containers/Bundle/Application/{container_id}/{bundle_name}"
    );
    let executable_domain = format!("{container}/{executable}");
    let files = [
        BackupEntry::dir("", "RootDomain"),
        BackupEntry::dir("Library", "RootDomain"),
        BackupEntry::dir("Library/Preferences", "RootDomain"),
        BackupEntry::file("Library/Preferences/temp", "RootDomain", helper, 33, 33, 0),
        BackupEntry::dir("", &container),
        BackupEntry::file("", &executable_domain, b"", 33, 33, 0),
        BackupEntry::file(
            "",
            "SysContainerDomain-../../../../../../../../var/.backup.i/var/root/Library/Preferences/temp",
            b"",
            501,
            501,
            random_inode(),
        ),
        BackupEntry::file(
            "",
            "SysContainerDomain-../../../../../../../../crash_on_purpose",
            b"",
            0,
            0,
            random_inode(),
        ),
    ];

    let mut manifest = Vec::new();
    manifest.extend_from_slice(b"mbdb\x05\x00");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as u32)
        .unwrap_or(0);
    for entry in &files {
        if let Some(contents) = &entry.contents {
            let name = file_id(&entry.domain, &entry.path);
            let mut blob =
                fs::File::create(directory.join(name)).map_err(|error| error.to_string())?;
            blob.write_all(contents)
                .map_err(|error| error.to_string())?;
        }
        manifest.extend_from_slice(&entry.record(now));
    }
    fs::write(directory.join("Manifest.mbdb"), manifest).map_err(|error| error.to_string())?;
    fs::write(
        directory.join("Info.plist"),
        plist_xml(&plist::Dictionary::new()).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    fs::write(
        directory.join("Status.plist"),
        status_plist().map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    fs::write(
        directory.join("Manifest.plist"),
        manifest_plist().map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

struct BackupEntry {
    path: String,
    domain: String,
    contents: Option<Vec<u8>>,
    owner: u32,
    group: u32,
    inode: u64,
    directory: bool,
}

impl BackupEntry {
    fn dir(path: &str, domain: &str) -> Self {
        Self {
            path: path.to_owned(),
            domain: domain.to_owned(),
            contents: None,
            owner: 0,
            group: 0,
            inode: 0,
            directory: true,
        }
    }

    fn file(path: &str, domain: &str, contents: &[u8], owner: u32, group: u32, inode: u64) -> Self {
        Self {
            path: path.to_owned(),
            domain: domain.to_owned(),
            contents: Some(contents.to_vec()),
            owner,
            group,
            inode,
            directory: false,
        }
    }

    fn record(&self, now: u32) -> Vec<u8> {
        let mode: u16 = if self.directory { 0o040755 } else { 0o100755 };
        let hash = self
            .contents
            .as_deref()
            .map(|contents| Sha1::digest(contents).to_vec())
            .unwrap_or_default();
        let size = self
            .contents
            .as_ref()
            .map(|contents| contents.len())
            .unwrap_or(0) as u64;
        let mut record = Vec::new();
        push_str(&mut record, &self.domain);
        push_str(&mut record, &self.path);
        push_str(&mut record, "");
        push_bytes(&mut record, &hash);
        push_bytes(&mut record, b"");
        record.extend_from_slice(&mode.to_be_bytes());
        record.extend_from_slice(&self.inode.to_be_bytes());
        record.extend_from_slice(&self.owner.to_be_bytes());
        record.extend_from_slice(&self.group.to_be_bytes());
        record.extend_from_slice(&now.to_be_bytes());
        record.extend_from_slice(&now.to_be_bytes());
        record.extend_from_slice(&now.to_be_bytes());
        record.extend_from_slice(&size.to_be_bytes());
        record.push(4);
        record.push(0);
        record
    }
}

fn push_str(out: &mut Vec<u8>, value: &str) {
    push_bytes(out, value.as_bytes());
}

fn push_bytes(out: &mut Vec<u8>, value: &[u8]) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value);
}

pub fn file_id(domain: &str, path: &str) -> String {
    Sha1::digest(format!("{domain}-{path}").as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn random_inode() -> u64 {
    u64::from_be_bytes(
        uuid::Uuid::new_v4().as_bytes()[..8]
            .try_into()
            .unwrap_or([1; 8]),
    )
}

fn plist_xml(value: &plist::Dictionary) -> Result<Vec<u8>, plist::Error> {
    let mut bytes = Vec::new();
    plist::Value::Dictionary(value.clone()).to_writer_xml(&mut bytes)?;
    Ok(bytes)
}

fn status_plist() -> Result<Vec<u8>, plist::Error> {
    let mut dict = plist::Dictionary::new();
    dict.insert("BackupState".into(), "new".into());
    dict.insert(
        "Date".into(),
        plist::Value::Date(
            plist::Date::from_xml_format("1970-01-01T00:00:00Z").expect("the epoch date is valid"),
        ),
    );
    dict.insert("IsFullBackup".into(), false.into());
    dict.insert("SnapshotState".into(), "finished".into());
    dict.insert("UUID".into(), "00000000-0000-0000-0000-000000000000".into());
    dict.insert("Version".into(), "2.4".into());
    plist_xml(&dict)
}

fn manifest_plist() -> Result<Vec<u8>, plist::Error> {
    let mut dict = plist::Dictionary::new();
    dict.insert("BackupKeyBag".into(), plist::Value::Data(backup_keybag()));
    dict.insert(
        "Lockdown".into(),
        plist::Value::Dictionary(plist::Dictionary::new()),
    );
    dict.insert("SystemDomainsVersion".into(), "20.0".into());
    dict.insert("Version".into(), "9.1".into());
    plist_xml(&dict)
}

fn backup_keybag() -> Vec<u8> {
    const KEYBAG: &str = "\
VkVSUwAAAAQAAAAFVFlQRQAAAAQAAAABVVVJRAAAABDud41d1b9NBICR1BH9JfVtSE1D\
SwAAACgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAV1JBUAAA\
AAQAAAAAU0FMVAAAABRY5Ne2bthGQ5rf4O3gikep1e6tZUlURVIAAAAEAAAnEFVVSUQA\
AAAQB7R8awiGR9aba1UuVahGPENMQVMAAAAEAAAAAVdSQVAAAAAEAAAAAktUWVAAAAAE\
AAAAAFdQS1kAAAAoN3kQAJloFg+ukEUY+v5P+dhc/Welw/oucsyS40UBh67ZHef5ZMk9\
UVVVSUQAAAAQgd0cg0hSTgaxR3PVUbcEkUNMQVMAAAAEAAAAAldSQVAAAAAEAAAAAktU\
WVAAAAAEAAAAAFdQS1kAAAAoMiQTXx0SJlyrGJzdKZQ+SfL124w+2Tf/3d1R2i9yNj9z\
ZCHNJhnorVVVSUQAAAAQf7JFQiBOS12JDD7qwKNTSkNMQVMAAAAEAAAAA1dSQVAAAAAE\
AAAAAktUWVAAAAAEAAAAAFdQS1kAAAAoSEelorROJA46ZUdwDHhMKiRguQyqHukotrxh\
jIfqiZ5ESBXX9txi51VVSUQAAAAQfF0G/837QLq01xH9+66vx0NMQVMAAAAEAAAABFdS\
QVAAAAAEAAAAAktUWVAAAAAEAAAAAFdQS1kAAAAol0BvFhd5bu4Hr75XqzNf4g0fMqZA\
ie6OxI+x/pgm6Y95XW17N+ZIDVVVSUQAAAAQimkT2dp1QeadMu1KhJKNTUNMQVMAAAAE\
AAAABVdSQVAAAAAEAAAAA0tUWVAAAAAEAAAAAFdQS1kAAAAo2N2DZarQ6GPoWRgTiy/t\
djKArOqTaH0tPSG9KLbIjGTOcLodhx23xFVVSUQAAAAQQV37JVZHQFiKpoNiGmT6+ENM\
QVMAAAAEAAAABldSQVAAAAAEAAAAA0tUWVAAAAAEAAAAAFdQS1kAAAAofe2QSvDC2cV7\
Etk4fSBbgqDx5ne/z1VHwmJ6NdVrTyWi80Sy869DM1VVSUQAAAAQFzkdH+VgSOmTj3yE\
cfWmMUNMQVMAAAAEAAAAB1dSQVAAAAAEAAAAA0tUWVAAAAAEAAAAAFdQS1kAAAAo7kLY\
PQ/DnHBERGpaz37eyntIX/XzovsS0mpHW3SoHvrb9RBgOB+WblVVSUQAAAAQEBpgKOz9\
Tni8F9kmSXd0sENMQVMAAAAEAAAACFdSQVAAAAAEAAAAA0tUWVAAAAAEAAAAAFdQS1kA\
AAAo5mxVoyNFgPMzphYhm1VG8Fhsin/xX+r6mCd9gByF5SxeolAIT/ICF1VVSUQAAAAQ\
rfKB2uPSQtWh82yx6w4BoUNMQVMAAAAEAAAACVdSQVAAAAAEAAAAA0tUWVAAAAAEAAAA\
AFdQS1kAAAAo5iayZBwcRa1c1MMx7vh6lOYux3oDI/bdxFCW1WHCQR/Ub1MOv+QaYFVV\
SUQAAAAQiLXvK3qvQza/mea5inss/0NMQVMAAAAEAAAACldSQVAAAAAEAAAAA0tUWVAA\
AAAEAAAAAFdQS1kAAAAoD2wHX7KriEe1E31z7SQ7/+AVymcpARMYnQgegtZD0Mq2U55u\
xwNr2FVVSUQAAAAQ/Q9feZxLS++qSe/a4emRRENMQVMAAAAEAAAAC1dSQVAAAAAEAAAA\
A0tUWVAAAAAEAAAAAFdQS1kAAAAocYda2jyYzzSKggRPw/qgh6QPESlkZedgDUKpTr4Z\
Z8FDgd7YoALY1g==";
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, KEYBAG)
        .expect("the TrollRestore keybag is valid base64")
}

fn version_of(major: u32, minor: u32, patch: u32) -> IosVersion {
    IosVersion {
        major,
        minor,
        patch,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        file_id, helper_install_caution, helper_install_refusal, is_replaceable_apple_app,
        removable_bundle_name, write_helper_backup,
    };

    #[test]
    fn offers_only_apple_system_apps() {
        assert!(is_replaceable_apple_app("com.apple.tips", Some("System")));
        assert!(!is_replaceable_apple_app(
            "com.opa334.TrollStore",
            Some("System")
        ));
        assert!(!is_replaceable_apple_app(
            "com.opa334.Dopamine",
            Some("System")
        ));
        assert!(!is_replaceable_apple_app(
            "com.tigisoftware.Filza",
            Some("System")
        ));
        assert!(!is_replaceable_apple_app("com.apple.tips", Some("User")));
        assert!(!is_replaceable_apple_app("com.apple.tips", None));
    }

    #[test]
    fn allows_the_published_window() {
        assert!(helper_install_refusal("15.0", "19A346").is_none());
        assert!(helper_install_refusal("15.1.1", "19B81").is_none());
        assert!(helper_install_refusal("16.6.1", "20G81").is_none());
        assert!(helper_install_refusal("16.7", "20H18").is_none());
        assert!(helper_install_refusal("17.0", "21A329").is_none());
        assert!(helper_install_refusal("17.0.0", "21A331").is_none());
    }

    #[test]
    fn refuses_builds_outside_the_window() {
        assert!(helper_install_refusal("14.2", "18B92").is_some());
        assert!(helper_install_refusal("16.7", "20H19").is_some());
        assert!(helper_install_refusal("16.7", "").is_some());
        assert!(helper_install_refusal("16.7.1", "20H30").is_some());
        assert!(helper_install_refusal("16.7.10", "20H360").is_some());
        assert!(helper_install_refusal("17.0.1", "21A340").is_some());
        assert!(helper_install_refusal("26.5", "23F77").is_some());
        assert!(helper_install_refusal("not-a-version", "21A329").is_some());
    }

    #[test]
    fn warns_on_the_flaky_early_15_builds() {
        assert!(helper_install_caution("15.0").is_some());
        assert!(helper_install_caution("15.1.1").is_some());
        assert!(helper_install_caution("15.2").is_none());
        assert!(helper_install_caution("17.0").is_none());
        assert!(helper_install_caution("14.2").is_none());
    }

    #[test]
    fn accepts_only_a_removable_app_bundle_path() {
        assert_eq!(
            removable_bundle_name(
                "/private/var/containers/Bundle/Application/8E1C0A0A-0000-4000-8000-000000000001/Tips.app"
            )
            .as_deref(),
            Some("Tips.app")
        );
        assert_eq!(
            removable_bundle_name(
                "/var/containers/Bundle/Application/8E1C0A0A-0000-4000-8000-000000000001/Tips.app"
            )
            .as_deref(),
            Some("Tips.app")
        );
        assert!(removable_bundle_name("/Applications/Tips.app").is_none());
        assert!(
            removable_bundle_name("/private/var/containers/Bundle/Application/uuid/Tips.app/Tips")
                .is_none()
        );
        assert!(
            removable_bundle_name(
                "/private/var/containers/Bundle/Application/../Bundle/Application/Tips.app"
            )
            .is_none()
        );
        assert!(removable_bundle_name("").is_none());
    }

    #[test]
    fn writes_only_the_helper_backup_for_a_removable_app() {
        let directory =
            std::env::temp_dir().join(format!("trollstore-backup-{}", uuid::Uuid::new_v4()));
        write_helper_backup(
            &directory,
            b"helper-bytes",
            "8E1C0A0A-0000-4000-8000-000000000001",
            "Tips.app",
        )
        .unwrap();
        let manifest = std::fs::read(directory.join("Manifest.mbdb")).unwrap();
        assert!(manifest.starts_with(b"mbdb\x05\x00"));
        let text = String::from_utf8_lossy(&manifest);
        assert!(text.contains("SysContainerDomain-../../../../../../../../var/backup/var/containers/Bundle/Application/8E1C0A0A-0000-4000-8000-000000000001/Tips.app/Tips"));
        assert!(text.contains("crash_on_purpose"));
        let helper_id = file_id("RootDomain", "Library/Preferences/temp");
        assert_eq!(
            std::fs::read(directory.join(helper_id)).unwrap(),
            b"helper-bytes"
        );
        assert!(directory.join("Manifest.plist").is_file());
        assert!(directory.join("Status.plist").is_file());
        assert!(directory.join("Info.plist").is_file());
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn refuses_a_target_that_would_leave_the_helper_layout() {
        let directory =
            std::env::temp_dir().join(format!("trollstore-reject-{}", uuid::Uuid::new_v4()));
        assert!(write_helper_backup(&directory, b"helper", "../escaped", "Tips.app").is_err());
        assert!(
            write_helper_backup(
                &directory,
                b"helper",
                "8E1C0A0A-0000-4000-8000-000000000001",
                "../Tips.app"
            )
            .is_err()
        );
        assert!(!directory.exists());
    }
}
