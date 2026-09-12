use std::{
    collections::HashSet,
    ffi::OsString,
    fs::{self, File},
    io::{Read, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::SystemTime,
};

use plist::{Dictionary, Value};
use sha1::{Digest, Sha1};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;
use zip::ZipArchive;

use crate::{
    error::{CommandError, CommandResult},
    types::{
        OperationProgress, PersonalSigningIdentity, PersonalSigningPreflight,
        PersonalSigningRequest, PersonalSigningResult,
    },
};

const MAX_PROFILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_INFO_PLIST_BYTES: u64 = 4 * 1024 * 1024;
const MAX_IPA_ENTRIES: usize = 100_000;
const MAX_UNPACKED_BYTES: u64 = 4 * 1024 * 1024 * 1024;

fn signing_error(message: impl Into<String>) -> CommandError {
    CommandError::new("personal_signing", message, false)
}

fn find_bytes(haystack: &[u8], needle: &[u8], offset: usize) -> Option<usize> {
    haystack
        .get(offset..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|index| offset + index)
}

fn embedded_profile_plist(profile: &[u8]) -> CommandResult<&[u8]> {
    if u64::try_from(profile.len()).unwrap_or(u64::MAX) > MAX_PROFILE_BYTES {
        return Err(signing_error("The provisioning profile exceeds 8 MB"));
    }
    let xml = find_bytes(profile, b"<?xml", 0);
    let plist = find_bytes(profile, b"<plist", 0);
    let start = match (xml, plist) {
        (Some(xml), Some(plist)) => xml.min(plist),
        (Some(xml), None) => xml,
        (None, Some(plist)) => plist,
        (None, None) => {
            return Err(signing_error(
                "The provisioning profile contains no readable XML plist",
            ));
        }
    };
    let close = b"</plist>";
    let end = find_bytes(profile, close, start)
        .map(|index| index + close.len())
        .ok_or_else(|| signing_error("The provisioning profile plist is truncated"))?;
    Ok(&profile[start..end])
}

fn dictionary_string(dictionary: &Dictionary, key: &str) -> Option<String> {
    dictionary
        .get(key)
        .and_then(Value::as_string)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[derive(Clone)]
struct SigningProfile {
    bytes: Vec<u8>,
    name: String,
    uuid: Option<String>,
    team_identifier: Option<String>,
    application_identifier: Option<String>,
    expires_at: Option<String>,
    expired: bool,
    devices: Vec<String>,
    provisions_all_devices: bool,
    get_task_allow: Option<bool>,
    entitlements: Dictionary,
    certificate_hashes: HashSet<String>,
}

fn read_profile(path: &Path) -> CommandResult<SigningProfile> {
    if !path.is_file()
        || !path.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("mobileprovision")
                || extension.eq_ignore_ascii_case("provisionprofile")
        })
    {
        return Err(signing_error(
            "Choose a valid .mobileprovision or .provisionprofile file",
        ));
    }
    let size = fs::metadata(path)?.len();
    if size > MAX_PROFILE_BYTES {
        return Err(signing_error("The provisioning profile exceeds 8 MB"));
    }
    let bytes = fs::read(path)?;
    let value = plist::from_bytes::<Value>(embedded_profile_plist(&bytes)?).map_err(|error| {
        signing_error(format!("Unable to parse the provisioning profile: {error}"))
    })?;
    let dictionary = value
        .into_dictionary()
        .ok_or_else(|| signing_error("The provisioning profile plist is not a dictionary"))?;
    let entitlements = dictionary
        .get("Entitlements")
        .and_then(Value::as_dictionary)
        .cloned()
        .ok_or_else(|| signing_error("The provisioning profile has no Entitlements dictionary"))?;
    let team_identifier = dictionary
        .get("TeamIdentifier")
        .and_then(Value::as_array)
        .and_then(|values| values.first())
        .and_then(Value::as_string)
        .map(str::to_owned)
        .or_else(|| {
            dictionary
                .get("ApplicationIdentifierPrefix")
                .and_then(Value::as_array)
                .and_then(|values| values.first())
                .and_then(Value::as_string)
                .map(str::to_owned)
        });
    let application_identifier = dictionary_string(&entitlements, "application-identifier")
        .or_else(|| dictionary_string(&entitlements, "com.apple.application-identifier"));
    let expiration = dictionary.get("ExpirationDate").and_then(Value::as_date);
    let expires_at = expiration.map(|date| date.to_xml_format());
    let expired = expiration
        .map(SystemTime::from)
        .is_some_and(|date| date <= SystemTime::now());
    let devices = dictionary
        .get("ProvisionedDevices")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_string)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let certificate_hashes = dictionary
        .get("DeveloperCertificates")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_data)
                .map(|certificate| format!("{:X}", Sha1::digest(certificate)))
                .collect()
        })
        .unwrap_or_default();

    Ok(SigningProfile {
        bytes,
        name: dictionary_string(&dictionary, "Name")
            .unwrap_or_else(|| "Unnamed provisioning profile".into()),
        uuid: dictionary_string(&dictionary, "UUID"),
        team_identifier,
        application_identifier,
        expires_at,
        expired,
        devices,
        provisions_all_devices: dictionary
            .get("ProvisionsAllDevices")
            .and_then(Value::as_boolean)
            .unwrap_or(false),
        get_task_allow: entitlements
            .get("get-task-allow")
            .and_then(Value::as_boolean),
        entitlements,
        certificate_hashes,
    })
}

#[derive(Clone)]
struct IpaSummary {
    ipa_name: String,
    app_root: String,
    app_name: String,
    bundle_id: String,
    version: String,
    unsupported_content: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct AccountSigningIpaSummary {
    pub app_name: String,
    pub version: String,
}

fn zip_error(context: &str, error: impl std::fmt::Display) -> CommandError {
    signing_error(format!("{context}: {error}"))
}

fn read_zip_entry(
    archive: &mut ZipArchive<File>,
    name: &str,
    maximum_size: u64,
) -> CommandResult<Vec<u8>> {
    let mut entry = archive
        .by_name(name)
        .map_err(|error| zip_error("Invalid IPA archive", error))?;
    if entry.size() > maximum_size {
        return Err(signing_error(format!("IPA entry is too large: {name}")));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(entry.size()).unwrap_or(0));
    entry
        .read_to_end(&mut bytes)
        .map_err(|error| zip_error("Unable to read IPA archive", error))?;
    Ok(bytes)
}

fn is_symlink(mode: Option<u32>) -> bool {
    mode.is_some_and(|mode| mode & 0o170_000 == 0o120_000)
}

fn inspect_ipa(path: &Path) -> CommandResult<IpaSummary> {
    if !path.is_file()
        || !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("ipa"))
    {
        return Err(signing_error("Choose a valid .ipa file"));
    }
    let file = File::open(path)?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| zip_error("Invalid IPA archive", error))?;
    if archive.len() > MAX_IPA_ENTRIES {
        return Err(signing_error("The IPA contains too many archive entries"));
    }

    let mut total_size = 0u64;
    let mut info_paths = Vec::new();
    let mut unsupported = HashSet::new();
    let mut top_level_apps = HashSet::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| zip_error("Invalid IPA entry", error))?;
        if entry.enclosed_name().is_none() {
            return Err(signing_error("The IPA contains an unsafe archive path"));
        }
        if is_symlink(entry.unix_mode()) {
            return Err(signing_error(
                "The first signing release does not accept IPA archives containing symbolic links",
            ));
        }
        total_size = total_size
            .checked_add(entry.size())
            .ok_or_else(|| signing_error("The IPA expanded size is invalid"))?;
        if total_size > MAX_UNPACKED_BYTES {
            return Err(signing_error(
                "The IPA expands beyond the 4 GB safety limit",
            ));
        }
        let name = entry.name();
        let components = name
            .split('/')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if components.len() >= 2 && components[0] == "Payload" && components[1].ends_with(".app") {
            top_level_apps.insert(components[1].to_owned());
        }
        if components.len() == 3
            && components[0] == "Payload"
            && components[1].ends_with(".app")
            && components[2] == "Info.plist"
        {
            info_paths.push(name.to_owned());
        }
        if components.iter().any(|part| part.ends_with(".appex")) {
            unsupported.insert("app extensions".to_string());
        }
        let app_bundle_count = components
            .iter()
            .filter(|part| part.ends_with(".app"))
            .count();
        if app_bundle_count > 1 || components.contains(&"Watch") {
            unsupported.insert("nested or Watch applications".to_string());
        }
    }
    if info_paths.len() != 1 || top_level_apps.len() != 1 {
        return Err(signing_error(
            "The IPA must contain exactly one top-level Payload/*.app bundle and Info.plist",
        ));
    }
    let info_path = info_paths.pop().expect("one Info.plist path");
    let app_root = info_path
        .strip_suffix("/Info.plist")
        .ok_or_else(|| signing_error("Invalid IPA application structure"))?
        .to_owned();
    let info = plist::from_bytes::<Value>(&read_zip_entry(
        &mut archive,
        &info_path,
        MAX_INFO_PLIST_BYTES,
    )?)
    .map_err(|error| signing_error(format!("Unable to parse IPA Info.plist: {error}")))?;
    let info = info
        .as_dictionary()
        .ok_or_else(|| signing_error("IPA Info.plist is not a dictionary"))?;
    let bundle_id = dictionary_string(info, "CFBundleIdentifier")
        .ok_or_else(|| signing_error("IPA Info.plist has no CFBundleIdentifier"))?;
    let executable = dictionary_string(info, "CFBundleExecutable")
        .filter(|value| !value.contains('/') && !value.contains('\\'))
        .ok_or_else(|| signing_error("IPA Info.plist has no safe CFBundleExecutable"))?;
    let executable_path = format!("{app_root}/{executable}");
    archive
        .by_name(&executable_path)
        .map_err(|_| signing_error("The IPA main executable is missing"))?;

    Ok(IpaSummary {
        ipa_name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Application.ipa")
            .to_owned(),
        app_root,
        app_name: dictionary_string(info, "CFBundleDisplayName")
            .or_else(|| dictionary_string(info, "CFBundleName"))
            .unwrap_or_else(|| bundle_id.clone()),
        bundle_id,
        version: dictionary_string(info, "CFBundleShortVersionString")
            .or_else(|| dictionary_string(info, "CFBundleVersion"))
            .unwrap_or_else(|| "—".into()),
        unsupported_content: unsupported.into_iter().collect(),
    })
}

pub(crate) fn inspect_account_signing_ipa(path: &Path) -> CommandResult<AccountSigningIpaSummary> {
    let summary = inspect_ipa(path)?;
    Ok(AccountSigningIpaSummary {
        app_name: summary.app_name,
        version: summary.version,
    })
}

fn identity_team(name: &str) -> Option<String> {
    let (_, suffix) = name.rsplit_once('(')?;
    let team = suffix.strip_suffix(')')?.trim();
    (team.len() >= 8
        && team.len() <= 16
        && team
            .chars()
            .all(|character| character.is_ascii_alphanumeric()))
    .then(|| team.to_owned())
}

fn parse_security_identities(output: &str) -> Vec<(String, String)> {
    let mut seen = HashSet::new();
    output
        .lines()
        .filter_map(|line| {
            let hash = line
                .split_whitespace()
                .find(|part| {
                    part.len() == 40 && part.chars().all(|value| value.is_ascii_hexdigit())
                })?
                .to_ascii_uppercase();
            let first_quote = line.find('"')?;
            let last_quote = line.rfind('"')?;
            if last_quote <= first_quote {
                return None;
            }
            let name = line[first_quote + 1..last_quote].trim().to_owned();
            seen.insert(hash.clone()).then_some((hash, name))
        })
        .collect()
}

fn command_message(output: &Output) -> String {
    let text = if output.stderr.is_empty() {
        String::from_utf8_lossy(&output.stdout)
    } else {
        String::from_utf8_lossy(&output.stderr)
    };
    let text = text.trim();
    let start = text.len().saturating_sub(4_000);
    text.get(start..).unwrap_or(text).to_owned()
}

fn installed_identities(profile: &SigningProfile) -> CommandResult<Vec<PersonalSigningIdentity>> {
    let output = Command::new("/usr/bin/security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output()
        .map_err(|error| signing_error(format!("Unable to read signing identities: {error}")))?;
    if !output.status.success() {
        return Err(signing_error(format!(
            "Unable to read signing identities: {}",
            command_message(&output)
        )));
    }
    let identities = parse_security_identities(&String::from_utf8_lossy(&output.stdout))
        .into_iter()
        .map(|(hash, name)| {
            let matches_profile = profile.certificate_hashes.contains(&hash);
            PersonalSigningIdentity {
                team_identifier: identity_team(&name),
                hash,
                name,
                matches_profile,
            }
        })
        .collect::<Vec<_>>();
    Ok(identities)
}

fn profile_allows_bundle(application_identifier: &str, bundle_id: &str) -> bool {
    let Some((_, pattern)) = application_identifier.split_once('.') else {
        return false;
    };
    pattern == bundle_id
        || pattern == "*"
        || pattern
            .strip_suffix(".*")
            .is_some_and(|prefix| bundle_id.starts_with(&format!("{prefix}.")))
}

struct PreparedSigning {
    preflight: PersonalSigningPreflight,
    ipa: IpaSummary,
    profile: SigningProfile,
}

fn prepare_signing(
    ipa_path: &Path,
    profile_path: &Path,
    udid: &str,
) -> CommandResult<PreparedSigning> {
    let ipa = inspect_ipa(ipa_path)?;
    let profile = read_profile(profile_path)?;
    let identities = installed_identities(&profile)?;
    let matching_identity = identities.iter().find(|identity| identity.matches_profile);
    let device_included =
        profile.provisions_all_devices || profile.devices.iter().any(|device| device == udid);
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if profile.expired {
        blockers.push("The provisioning profile has expired.".into());
    }
    if !device_included {
        blockers.push("The connected device is not included in this provisioning profile.".into());
    }
    match profile.application_identifier.as_deref() {
        Some(identifier) if profile_allows_bundle(identifier, &ipa.bundle_id) => {}
        Some(identifier) => blockers.push(format!(
            "Profile application identifier {identifier} does not allow {}.",
            ipa.bundle_id
        )),
        None => blockers.push("The profile has no application-identifier entitlement.".into()),
    }
    if profile.certificate_hashes.is_empty() {
        blockers.push("The profile contains no developer certificates.".into());
    } else if matching_identity.is_none() {
        blockers.push(
            "No macOS Keychain signing identity matches a certificate embedded in this profile."
                .into(),
        );
    }
    if !ipa.unsupported_content.is_empty() {
        blockers.push(format!(
            "This first release does not sign {}.",
            ipa.unsupported_content.join(" or ")
        ));
    }
    if profile.get_task_allow != Some(true) {
        warnings
            .push("This profile does not allow debugging, so JIT will remain unavailable.".into());
    }
    warnings.push(
        "The selected profile's entitlement set replaces the source app entitlements; capabilities absent from the profile may not work."
            .into(),
    );
    if profile
        .application_identifier
        .as_deref()
        .is_some_and(|identifier| identifier.contains('*'))
    {
        warnings.push(
            "Wildcard entitlements will be specialized to this app bundle identifier.".into(),
        );
    }

    let selected_identity_hash = matching_identity.map(|identity| identity.hash.clone());
    let ready = blockers.is_empty();
    let preflight = PersonalSigningPreflight {
        ipa_name: ipa.ipa_name.clone(),
        app_name: ipa.app_name.clone(),
        bundle_id: ipa.bundle_id.clone(),
        version: ipa.version.clone(),
        profile_name: profile.name.clone(),
        profile_uuid: profile.uuid.clone(),
        team_identifier: profile.team_identifier.clone(),
        application_identifier: profile.application_identifier.clone(),
        expires_at: profile.expires_at.clone(),
        device_count: u64::try_from(profile.devices.len()).unwrap_or(u64::MAX),
        device_included,
        identities,
        selected_identity_hash,
        ready,
        blockers,
        warnings,
    };
    Ok(PreparedSigning {
        preflight,
        ipa,
        profile,
    })
}

#[tauri::command]
pub async fn personal_signing_preflight(
    ipa_path: String,
    profile_path: String,
    udid: String,
) -> CommandResult<PersonalSigningPreflight> {
    tokio::task::spawn_blocking(move || {
        prepare_signing(Path::new(&ipa_path), Path::new(&profile_path), &udid)
            .map(|prepared| prepared.preflight)
    })
    .await
    .map_err(|error| signing_error(format!("Signing preflight task failed: {error}")))?
}

struct SigningWorkspace {
    path: PathBuf,
}

impl SigningWorkspace {
    fn create() -> CommandResult<Self> {
        let path = std::env::temp_dir().join(format!(
            "idevice_desktop-personal-signing-{}",
            Uuid::new_v4()
        ));
        fs::create_dir(&path)?;
        Ok(Self { path })
    }
}

impl Drop for SigningWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct StagedOutput {
    path: PathBuf,
    committed: bool,
}

impl Drop for StagedOutput {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn validate_output_path(input: &Path, output: &Path) -> CommandResult<PathBuf> {
    if output
        .extension()
        .is_none_or(|extension| !extension.eq_ignore_ascii_case("ipa"))
    {
        return Err(signing_error("The export destination must end in .ipa"));
    }
    if output.file_name().is_none() {
        return Err(signing_error("Choose a complete IPA export path"));
    }
    if output
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(signing_error(
            "The export destination cannot be a symbolic link",
        ));
    }
    let parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| signing_error("The export destination has no parent folder"))?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|error| signing_error(format!("The export folder is unavailable: {error}")))?;
    if !canonical_parent.is_dir() {
        return Err(signing_error("The export parent is not a folder"));
    }
    let canonical_output = canonical_parent.join(output.file_name().expect("checked file name"));
    if input.canonicalize()? == canonical_output {
        return Err(signing_error(
            "Export to a new file; the source IPA is never overwritten",
        ));
    }
    Ok(canonical_output)
}

pub(crate) fn validate_account_signing_output_path(
    input: &Path,
    output: &Path,
) -> CommandResult<PathBuf> {
    validate_output_path(input, output)
}

fn extract_ipa(input: &Path, destination: &Path) -> CommandResult<()> {
    let file = File::open(input)?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| zip_error("Invalid IPA archive", error))?;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| zip_error("Invalid IPA entry", error))?;
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| signing_error("The IPA contains an unsafe archive path"))?
            .to_owned();
        if is_symlink(entry.unix_mode()) {
            return Err(signing_error(
                "The IPA contains an unsupported symbolic link",
            ));
        }
        let output = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output)?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&output)?;
        std::io::copy(&mut entry, &mut file)?;
        file.flush()?;
        if let Some(mode) = entry.unix_mode() {
            let permissions = mode & 0o777;
            if permissions != 0 {
                fs::set_permissions(&output, fs::Permissions::from_mode(permissions))?;
            }
        }
    }
    Ok(())
}

fn remove_old_signatures(path: &Path) -> CommandResult<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if child.is_dir() {
            if entry.file_name() == "_CodeSignature" {
                fs::remove_dir_all(child)?;
            } else {
                remove_old_signatures(&child)?;
            }
        }
    }
    Ok(())
}

fn materialize_entitlements(
    source: &Dictionary,
    team_identifier: &str,
    bundle_id: &str,
) -> Dictionary {
    let mut entitlements = source.clone();
    let application_identifier = format!("{team_identifier}.{bundle_id}");
    for key in ["application-identifier", "com.apple.application-identifier"] {
        if entitlements.contains_key(key) {
            entitlements.insert(key.into(), application_identifier.clone().into());
        }
    }
    if let Some(groups) = entitlements
        .get_mut("keychain-access-groups")
        .and_then(Value::as_array_mut)
    {
        for group in groups {
            if group.as_string().is_some_and(|value| value.ends_with(".*")) {
                *group = application_identifier.clone().into();
            }
        }
    }
    entitlements
}

fn collect_sign_targets(
    path: &Path,
    main_app: &Path,
    targets: &mut Vec<PathBuf>,
) -> CommandResult<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if child.is_dir() {
            collect_sign_targets(&child, main_app, targets)?;
            if child != main_app
                && child.extension().is_some_and(|extension| {
                    extension.eq_ignore_ascii_case("framework")
                        || extension.eq_ignore_ascii_case("xctest")
                        || extension.eq_ignore_ascii_case("app")
                })
            {
                targets.push(child);
            }
        } else if child
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("dylib"))
        {
            targets.push(child);
        }
    }
    Ok(())
}

fn run_codesign(arguments: &[OsString], context: &str) -> CommandResult<()> {
    let output = Command::new("/usr/bin/codesign")
        .args(arguments)
        .output()
        .map_err(|error| signing_error(format!("{context}: {error}")))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(signing_error(format!(
            "{context}: {}",
            command_message(&output)
        )))
    }
}

fn sign_path(path: &Path, identity_hash: &str, entitlements: Option<&Path>) -> CommandResult<()> {
    let mut arguments = vec![
        "--force".into(),
        "--sign".into(),
        identity_hash.into(),
        "--timestamp=none".into(),
        "--generate-entitlement-der".into(),
    ];
    if let Some(entitlements) = entitlements {
        arguments.push("--entitlements".into());
        arguments.push(entitlements.as_os_str().to_owned());
    }
    arguments.push(path.as_os_str().to_owned());
    run_codesign(&arguments, &format!("Unable to sign {}", path.display()))
}

fn emit_progress(app: &AppHandle, item: &str, percent: u64) {
    let _ = app.emit(
        "personal-signing://progress",
        OperationProgress {
            operation: "personal-signing".into(),
            item: item.into(),
            percent,
        },
    );
}

fn perform_signing(
    app: &AppHandle,
    request: &PersonalSigningRequest,
    udid: &str,
) -> CommandResult<PersonalSigningResult> {
    let input = Path::new(&request.ipa_path);
    let profile_path = Path::new(&request.profile_path);
    let output = validate_output_path(input, Path::new(&request.output_path))?;
    let identity_hash = request.identity_hash.trim().to_ascii_uppercase();
    if identity_hash.len() != 40 || !identity_hash.chars().all(|value| value.is_ascii_hexdigit()) {
        return Err(signing_error("Choose a valid Keychain signing identity"));
    }

    emit_progress(app, "Checking IPA, profile, device, and identity", 5);
    let prepared = prepare_signing(input, profile_path, udid)?;
    if !prepared.preflight.ready {
        return Err(signing_error(prepared.preflight.blockers.join(" ")));
    }
    let identity = prepared
        .preflight
        .identities
        .iter()
        .find(|identity| identity.hash == identity_hash && identity.matches_profile)
        .ok_or_else(|| {
            signing_error("The selected Keychain identity does not match this profile")
        })?;
    let team_identifier = prepared
        .profile
        .team_identifier
        .as_deref()
        .ok_or_else(|| signing_error("The profile has no team identifier"))?;

    emit_progress(app, "Opening a protected temporary workspace", 18);
    let workspace = SigningWorkspace::create()?;
    extract_ipa(input, &workspace.path)?;
    let app_path = workspace.path.join(&prepared.ipa.app_root);
    if !app_path.is_dir() {
        return Err(signing_error("The extracted application bundle is missing"));
    }
    remove_old_signatures(&app_path)?;
    fs::write(
        app_path.join("embedded.mobileprovision"),
        &prepared.profile.bytes,
    )?;

    let entitlements = materialize_entitlements(
        &prepared.profile.entitlements,
        team_identifier,
        &prepared.ipa.bundle_id,
    );
    let entitlements_path = workspace.path.join("idevice_desktop-entitlements.plist");
    let mut entitlements_file = File::create(&entitlements_path)?;
    Value::Dictionary(entitlements)
        .to_writer_xml(&mut entitlements_file)
        .map_err(|error| signing_error(format!("Unable to write signing entitlements: {error}")))?;
    entitlements_file.flush()?;

    emit_progress(app, "Signing embedded frameworks", 42);
    let mut targets = Vec::new();
    collect_sign_targets(&app_path, &app_path, &mut targets)?;
    for target in targets {
        sign_path(&target, &identity_hash, None)?;
    }
    emit_progress(app, "Signing the application", 68);
    sign_path(&app_path, &identity_hash, Some(&entitlements_path))?;
    run_codesign(
        &[
            "--verify".into(),
            "--deep".into(),
            "--strict".into(),
            "--verbose=2".into(),
            app_path.as_os_str().to_owned(),
        ],
        "The exported application failed code-signature verification",
    )?;
    fs::remove_file(entitlements_path)?;

    emit_progress(app, "Packaging the signed IPA", 84);
    let staging_path = output.with_file_name(format!(".idevice_desktop-{}.ipa", Uuid::new_v4()));
    let mut staged = StagedOutput {
        path: staging_path.clone(),
        committed: false,
    };
    let zip_output = Command::new("/usr/bin/zip")
        .args(["-qry"])
        .arg(&staging_path)
        .arg(".")
        .current_dir(&workspace.path)
        .output()
        .map_err(|error| signing_error(format!("Unable to package the signed IPA: {error}")))?;
    if !zip_output.status.success() {
        return Err(signing_error(format!(
            "Unable to package the signed IPA: {}",
            command_message(&zip_output)
        )));
    }
    if output.exists() {
        if !output.is_file() {
            return Err(signing_error("The export destination is not a file"));
        }
        fs::remove_file(&output)?;
    }
    fs::rename(&staging_path, &output)?;
    staged.committed = true;
    let size_bytes = fs::metadata(&output)?.len();
    emit_progress(app, "Signed IPA ready", 100);

    Ok(PersonalSigningResult {
        output_path: output.to_string_lossy().into_owned(),
        app_name: prepared.ipa.app_name,
        bundle_id: prepared.ipa.bundle_id,
        profile_name: prepared.profile.name,
        identity_name: identity.name.clone(),
        size_bytes,
    })
}

#[tauri::command]
pub async fn personal_signing_export(
    app: AppHandle,
    request: PersonalSigningRequest,
    udid: String,
) -> CommandResult<PersonalSigningResult> {
    tokio::task::spawn_blocking(move || perform_signing(&app, &request, &udid))
        .await
        .map_err(|error| signing_error(format!("Signing task failed: {error}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_security_identities_and_ignores_the_summary() {
        let output = r#"
  1) 0123456789abcdef0123456789ABCDEF01234567 "Apple Development: Example (TEAM123456)"
  2) ABCDEF0123456789ABCDEF0123456789ABCDEF01 "Apple Distribution: Example (TEAM123456)"
     2 valid identities found
"#;
        let identities = parse_security_identities(output);
        assert_eq!(identities.len(), 2);
        assert_eq!(identities[0].0, "0123456789ABCDEF0123456789ABCDEF01234567");
        assert_eq!(
            identity_team(&identities[0].1).as_deref(),
            Some("TEAM123456")
        );
    }

    #[test]
    fn matches_exact_and_wildcard_application_identifiers() {
        assert!(profile_allows_bundle(
            "TEAM123456.com.example.demo",
            "com.example.demo"
        ));
        assert!(profile_allows_bundle("TEAM123456.*", "com.example.demo"));
        assert!(profile_allows_bundle(
            "TEAM123456.com.example.*",
            "com.example.demo"
        ));
        assert!(!profile_allows_bundle(
            "TEAM123456.com.other.*",
            "com.example.demo"
        ));
    }

    #[test]
    fn specializes_wildcard_entitlements_to_the_application() {
        let mut source = Dictionary::new();
        source.insert("application-identifier".into(), "TEAM123456.*".into());
        source.insert(
            "keychain-access-groups".into(),
            Value::Array(vec!["TEAM123456.*".into(), "shared.fixed.group".into()]),
        );
        let result = materialize_entitlements(&source, "TEAM123456", "com.example.demo");
        assert_eq!(
            result
                .get("application-identifier")
                .and_then(Value::as_string),
            Some("TEAM123456.com.example.demo")
        );
        let groups = result
            .get("keychain-access-groups")
            .and_then(Value::as_array)
            .expect("groups");
        assert_eq!(groups[0].as_string(), Some("TEAM123456.com.example.demo"));
        assert_eq!(groups[1].as_string(), Some("shared.fixed.group"));
    }
}
