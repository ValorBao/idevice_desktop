use std::{
    fs::File,
    io::{Read, Seek},
    path::Path,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use idevice::{
    IdeviceService, installation_proxy::InstallationProxyClient,
    springboardservices::SpringBoardServicesClient, utils::installation,
};
use tauri::{AppHandle, Emitter, State};
use zip::ZipArchive;

use crate::{
    error::{CommandError, CommandResult},
    provider::selected_provider,
    state::AppState,
    types::{InstalledApp, OperationProgress},
    utils::{dict_string, dict_u64, plist_to_json},
};

const MAX_INFO_PLIST_SIZE: u64 = 4 * 1024 * 1024;
const MAX_EXECUTABLE_SIZE: u64 = 512 * 1024 * 1024;
const LC_CODE_SIGNATURE: u32 = 0x1d;
const EMBEDDED_SIGNATURE_MAGIC: u32 = 0xfade0cc0;

#[derive(Clone, Copy)]
enum ByteOrder {
    Big,
    Little,
}

fn ipa_error(message: impl Into<String>) -> CommandError {
    CommandError::new("ipa_signature", message, false)
}

fn zip_error(context: &str, error: impl std::fmt::Display) -> CommandError {
    ipa_error(format!("{context}: {error}"))
}

fn read_zip_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
    maximum_size: u64,
) -> CommandResult<Vec<u8>> {
    let mut entry = archive
        .by_name(name)
        .map_err(|error| zip_error("Invalid IPA archive", error))?;
    if entry.size() > maximum_size {
        return Err(ipa_error(format!("IPA entry is too large: {name}")));
    }
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry
        .read_to_end(&mut bytes)
        .map_err(|error| zip_error("Unable to read IPA archive", error))?;
    Ok(bytes)
}

fn read_u32(bytes: &[u8], offset: usize, order: ByteOrder) -> Option<u32> {
    let value: [u8; 4] = bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
    Some(match order {
        ByteOrder::Big => u32::from_be_bytes(value),
        ByteOrder::Little => u32::from_le_bytes(value),
    })
}

fn read_u64(bytes: &[u8], offset: usize, order: ByteOrder) -> Option<u64> {
    let value: [u8; 8] = bytes.get(offset..offset.checked_add(8)?)?.try_into().ok()?;
    Some(match order {
        ByteOrder::Big => u64::from_be_bytes(value),
        ByteOrder::Little => u64::from_le_bytes(value),
    })
}

fn thin_macho_has_signature(bytes: &[u8]) -> bool {
    let (order, header_size) = match bytes.get(..4) {
        Some([0xce, 0xfa, 0xed, 0xfe]) => (ByteOrder::Little, 28usize),
        Some([0xcf, 0xfa, 0xed, 0xfe]) => (ByteOrder::Little, 32usize),
        Some([0xfe, 0xed, 0xfa, 0xce]) => (ByteOrder::Big, 28usize),
        Some([0xfe, 0xed, 0xfa, 0xcf]) => (ByteOrder::Big, 32usize),
        _ => return false,
    };
    let Some(command_count) = read_u32(bytes, 16, order).map(|value| value as usize) else {
        return false;
    };
    let Some(commands_size) = read_u32(bytes, 20, order).map(|value| value as usize) else {
        return false;
    };
    let Some(commands_end) = header_size.checked_add(commands_size) else {
        return false;
    };
    if commands_end > bytes.len() {
        return false;
    }

    let mut cursor = header_size;
    for _ in 0..command_count {
        let Some(command) = read_u32(bytes, cursor, order) else {
            return false;
        };
        let Some(command_size) = read_u32(bytes, cursor + 4, order).map(|value| value as usize)
        else {
            return false;
        };
        let Some(command_end) = cursor.checked_add(command_size) else {
            return false;
        };
        if command_size < 8 || command_end > commands_end {
            return false;
        }
        if command == LC_CODE_SIGNATURE {
            if command_size < 16 {
                return false;
            }
            let Some(signature_offset) =
                read_u32(bytes, cursor + 8, order).map(|value| value as usize)
            else {
                return false;
            };
            let Some(signature_size) =
                read_u32(bytes, cursor + 12, order).map(|value| value as usize)
            else {
                return false;
            };
            let Some(signature_end) = signature_offset.checked_add(signature_size) else {
                return false;
            };
            return signature_size >= 12
                && signature_end <= bytes.len()
                && read_u32(bytes, signature_offset, ByteOrder::Big)
                    == Some(EMBEDDED_SIGNATURE_MAGIC);
        }
        cursor = command_end;
    }
    false
}

fn macho_has_signature(bytes: &[u8]) -> bool {
    let (order, is_64_bit_fat) = match bytes.get(..4) {
        Some([0xca, 0xfe, 0xba, 0xbe]) => (ByteOrder::Big, false),
        Some([0xbe, 0xba, 0xfe, 0xca]) => (ByteOrder::Little, false),
        Some([0xca, 0xfe, 0xba, 0xbf]) => (ByteOrder::Big, true),
        Some([0xbf, 0xba, 0xfe, 0xca]) => (ByteOrder::Little, true),
        _ => return thin_macho_has_signature(bytes),
    };
    let Some(architecture_count) = read_u32(bytes, 4, order).map(|value| value as usize) else {
        return false;
    };
    if architecture_count == 0 || architecture_count > 64 {
        return false;
    }
    let record_size = if is_64_bit_fat { 32usize } else { 20usize };
    (0..architecture_count).all(|index| {
        let Some(record_offset) = index
            .checked_mul(record_size)
            .and_then(|value| 8usize.checked_add(value))
        else {
            return false;
        };
        let (slice_offset, slice_size) = if is_64_bit_fat {
            (
                read_u64(bytes, record_offset + 8, order),
                read_u64(bytes, record_offset + 16, order),
            )
        } else {
            (
                read_u32(bytes, record_offset + 8, order).map(u64::from),
                read_u32(bytes, record_offset + 12, order).map(u64::from),
            )
        };
        let (Some(slice_offset), Some(slice_size)) = (slice_offset, slice_size) else {
            return false;
        };
        let Ok(slice_offset) = usize::try_from(slice_offset) else {
            return false;
        };
        let Ok(slice_size) = usize::try_from(slice_size) else {
            return false;
        };
        let Some(slice_end) = slice_offset.checked_add(slice_size) else {
            return false;
        };
        bytes
            .get(slice_offset..slice_end)
            .is_some_and(thin_macho_has_signature)
    })
}

fn validate_signed_ipa(path: &Path) -> CommandResult<()> {
    if !path.is_file()
        || !path
            .extension()
            .is_some_and(|value| value.eq_ignore_ascii_case("ipa"))
    {
        return Err(ipa_error("Please choose a valid .ipa file"));
    }
    let file = File::open(path)?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| zip_error("Invalid IPA archive", error))?;
    let info_path = (0..archive.len())
        .find_map(|index| {
            let entry = archive.by_index(index).ok()?;
            let components = entry.name().split('/').collect::<Vec<_>>();
            (components.len() == 3
                && components[0] == "Payload"
                && components[1].ends_with(".app")
                && components[2] == "Info.plist")
                .then(|| entry.name().to_string())
        })
        .ok_or_else(|| ipa_error("Invalid IPA: Payload/*.app/Info.plist was not found"))?;
    let app_root = info_path
        .strip_suffix("/Info.plist")
        .ok_or_else(|| ipa_error("Invalid IPA application structure"))?;

    let code_resources_path = format!("{app_root}/_CodeSignature/CodeResources");
    let code_resources = archive.by_name(&code_resources_path).map_err(|_| {
        ipa_error("This IPA is not signed: _CodeSignature/CodeResources is missing")
    })?;
    if code_resources.size() == 0 {
        return Err(ipa_error("This IPA is not signed: CodeResources is empty"));
    }
    drop(code_resources);

    let info_bytes = read_zip_entry(&mut archive, &info_path, MAX_INFO_PLIST_SIZE)?;
    let info = plist::Value::from_reader(std::io::Cursor::new(info_bytes))
        .map_err(|error| ipa_error(format!("Invalid IPA Info.plist: {error}")))?;
    let executable_name = info
        .as_dictionary()
        .and_then(|dict| dict.get("CFBundleExecutable"))
        .and_then(plist::Value::as_string)
        .filter(|value| !value.is_empty() && !value.contains('/') && !value.contains('\\'))
        .ok_or_else(|| ipa_error("Invalid IPA: CFBundleExecutable is missing"))?;
    let executable_path = format!("{app_root}/{executable_name}");
    let executable = read_zip_entry(&mut archive, &executable_path, MAX_EXECUTABLE_SIZE)?;
    if !macho_has_signature(&executable) {
        return Err(ipa_error(
            "This IPA is not signed: the app executable has no valid embedded code signature",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod signature_tests {
    use super::*;

    fn signed_thin_macho() -> Vec<u8> {
        let mut bytes = vec![0; 128];
        bytes[0..4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
        bytes[16..20].copy_from_slice(&1u32.to_le_bytes());
        bytes[20..24].copy_from_slice(&16u32.to_le_bytes());
        bytes[32..36].copy_from_slice(&LC_CODE_SIGNATURE.to_le_bytes());
        bytes[36..40].copy_from_slice(&16u32.to_le_bytes());
        bytes[40..44].copy_from_slice(&64u32.to_le_bytes());
        bytes[44..48].copy_from_slice(&12u32.to_le_bytes());
        bytes[64..68].copy_from_slice(&EMBEDDED_SIGNATURE_MAGIC.to_be_bytes());
        bytes
    }

    #[test]
    fn accepts_embedded_signature() {
        assert!(macho_has_signature(&signed_thin_macho()));
    }

    #[test]
    fn rejects_missing_embedded_signature() {
        let mut bytes = signed_thin_macho();
        bytes[64..68].fill(0);
        assert!(!macho_has_signature(&bytes));
    }

    #[test]
    fn requires_every_fat_architecture_to_be_signed() {
        let signed = signed_thin_macho();
        let mut fat = vec![0; 320];
        fat[0..4].copy_from_slice(&[0xca, 0xfe, 0xba, 0xbe]);
        fat[4..8].copy_from_slice(&2u32.to_be_bytes());
        fat[16..20].copy_from_slice(&64u32.to_be_bytes());
        fat[20..24].copy_from_slice(&(signed.len() as u32).to_be_bytes());
        fat[36..40].copy_from_slice(&192u32.to_be_bytes());
        fat[40..44].copy_from_slice(&(signed.len() as u32).to_be_bytes());
        fat[64..192].copy_from_slice(&signed);
        fat[192..320].copy_from_slice(&signed);
        assert!(macho_has_signature(&fat));

        fat[256..260].fill(0);
        assert!(!macho_has_signature(&fat));
    }
}

#[tauri::command]
pub async fn apps_list(
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<Vec<InstalledApp>> {
    let (_, provider) = selected_provider(&state, udid).await?;
    let mut client = InstallationProxyClient::connect(&provider)
        .await
        .map_err(CommandError::from)?;
    let apps = client
        .get_apps(Some("User"), None)
        .await
        .map_err(CommandError::from)?;
    let mut result = apps
        .into_iter()
        .filter_map(|(bundle_id, value)| {
            let dict = value.as_dictionary()?;
            let system = dict_string(dict, &["ApplicationType"])
                .is_some_and(|value| value.eq_ignore_ascii_case("system"));
            if system {
                return None;
            }
            let name = dict_string(dict, &["CFBundleDisplayName", "CFBundleName"])
                .unwrap_or_else(|| bundle_id.clone());
            let version = dict_string(dict, &["CFBundleShortVersionString", "CFBundleVersion"])
                .unwrap_or_default();
            let size_bytes = dict_u64(dict, &["StaticDiskUsage"]).unwrap_or(0)
                + dict_u64(dict, &["DynamicDiskUsage"]).unwrap_or(0);
            Some(InstalledApp {
                bundle_id,
                name,
                version,
                size_bytes,
                system: false,
                icon_data_url: None,
                raw: plist_to_json(&value),
            })
        })
        .collect::<Vec<_>>();

    if let Ok(mut springboard) = SpringBoardServicesClient::connect(&provider).await {
        for installed_app in &mut result {
            if let Ok(icon) = springboard
                .get_icon_pngdata(installed_app.bundle_id.clone())
                .await
            {
                installed_app.icon_data_url =
                    Some(format!("data:image/png;base64,{}", STANDARD.encode(icon)));
            }
        }
    }
    result.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(result)
}

#[tauri::command]
pub async fn app_install(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
    local_path: String,
) -> CommandResult<()> {
    let validation_path = local_path.clone();
    tokio::task::spawn_blocking(move || validate_signed_ipa(Path::new(&validation_path)))
        .await
        .map_err(|error| ipa_error(format!("IPA signature validation failed: {error}")))??;
    let (_, provider) = selected_provider(&state, udid).await?;
    let display_name = std::path::Path::new(&local_path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Application.ipa")
        .to_string();
    let event_app = app.clone();
    let event_name = display_name.clone();
    installation::install_package_with_callback(
        &provider,
        local_path,
        None,
        move |(percent, ())| {
            let app = event_app.clone();
            let item = event_name.clone();
            async move {
                let _ = app.emit(
                    "apps://install-progress",
                    OperationProgress {
                        operation: "install".into(),
                        item,
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

#[tauri::command]
pub async fn app_uninstall(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
    bundle_id: String,
) -> CommandResult<()> {
    let (_, provider) = selected_provider(&state, udid).await?;
    let mut client = InstallationProxyClient::connect(&provider)
        .await
        .map_err(CommandError::from)?;
    let event_app = app.clone();
    let event_bundle = bundle_id.clone();
    client
        .uninstall_with_callback(
            bundle_id,
            None,
            move |(percent, ())| {
                let app = event_app.clone();
                let item = event_bundle.clone();
                async move {
                    let _ = app.emit(
                        "apps://install-progress",
                        OperationProgress {
                            operation: "uninstall".into(),
                            item,
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
