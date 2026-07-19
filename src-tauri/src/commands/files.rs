use std::path::Path;

use idevice::{
    IdeviceService,
    afc::{AfcClient, opcode::AfcFopenMode},
    installation_proxy::InstallationProxyClient,
    services::house_arrest::HouseArrestClient,
};
use tauri::State;

use crate::{
    error::{CommandError, CommandResult},
    provider::selected_provider,
    state::AppState,
    types::{FileSharingApp, RemoteFileEntry},
    utils::dict_string,
};

const PROTECTED_MEDIA_ROOTS: &[&str] =
    &["Books", "DCIM", "PhotoData", "Purchases", "iTunes_Control"];

fn validate_remote_path(path: &str) -> CommandResult<()> {
    if !path.starts_with('/') || path.contains('\0') {
        return Err(CommandError::new(
            "files",
            "Invalid device file path",
            false,
        ));
    }
    if path
        .split('/')
        .any(|component| component == "." || component == "..")
    {
        return Err(CommandError::new(
            "files",
            "Relative path components are not allowed",
            false,
        ));
    }
    Ok(())
}

fn protected_media_path(path: &str) -> bool {
    path.trim_start_matches('/')
        .split('/')
        .next()
        .is_some_and(|root| PROTECTED_MEDIA_ROOTS.contains(&root))
}

fn ensure_mutation_allowed(path: &str, bundle_id: Option<&str>) -> CommandResult<()> {
    validate_remote_path(path)?;
    if path == "/" {
        return Err(CommandError::new(
            "files",
            "The file root cannot be modified",
            false,
        ));
    }
    if bundle_id.is_none() && protected_media_path(path) {
        return Err(CommandError::new(
            "files_read_only",
            "This iOS-managed media folder is read-only to prevent library corruption",
            false,
        ));
    }
    Ok(())
}

fn remote_join(parent: &str, name: &str) -> String {
    let parent = if parent.is_empty() { "/" } else { parent };
    if parent == "/" {
        format!("/{name}")
    } else {
        format!("{}/{name}", parent.trim_end_matches('/'))
    }
}

async fn afc_client(
    state: &AppState,
    udid: Option<String>,
    bundle_id: Option<&str>,
) -> CommandResult<AfcClient> {
    let (_, provider) = selected_provider(state, udid).await?;
    if let Some(bundle_id) = bundle_id {
        if bundle_id.is_empty() {
            return Err(CommandError::new(
                "files",
                "No file-sharing app selected",
                false,
            ));
        }
        let client = HouseArrestClient::connect(&provider)
            .await
            .map_err(CommandError::from)?;
        client
            .vend_documents(bundle_id)
            .await
            .map_err(CommandError::from)
    } else {
        AfcClient::connect(&provider)
            .await
            .map_err(CommandError::from)
    }
}

fn plist_flag(dict: &plist::Dictionary, key: &str) -> bool {
    dict.get(key).is_some_and(|value| {
        value.as_boolean().unwrap_or(false)
            || value.as_unsigned_integer() == Some(1)
            || value
                .as_string()
                .is_some_and(|value| value.eq_ignore_ascii_case("yes") || value == "1")
    })
}

#[tauri::command]
pub async fn file_sharing_apps(
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<Vec<FileSharingApp>> {
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
            plist_flag(dict, "UIFileSharingEnabled").then(|| FileSharingApp {
                name: dict_string(dict, &["CFBundleDisplayName", "CFBundleName"])
                    .unwrap_or_else(|| bundle_id.clone()),
                bundle_id,
            })
        })
        .collect::<Vec<_>>();
    result.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(result)
}

#[tauri::command]
pub async fn afc_list(
    state: State<'_, AppState>,
    udid: Option<String>,
    path: String,
    bundle_id: Option<String>,
) -> CommandResult<Vec<RemoteFileEntry>> {
    validate_remote_path(&path)?;
    let mut afc = afc_client(&state, udid, bundle_id.as_deref()).await?;
    let names = afc.list_dir(&path).await.map_err(CommandError::from)?;
    let mut entries = Vec::new();
    for name in names.into_iter().filter(|name| name != "." && name != "..") {
        let remote_path = remote_join(&path, &name);
        let info = match afc.get_file_info(&remote_path).await {
            Ok(info) => info,
            Err(error) => {
                tracing::warn!(%remote_path, ?error, "unable to read AFC file info");
                continue;
            }
        };
        let is_directory = info.st_ifmt == "S_IFDIR";
        let kind = if is_directory {
            "Folder".to_string()
        } else if let Some(extension) = Path::new(&name)
            .extension()
            .and_then(|value| value.to_str())
        {
            extension.to_uppercase()
        } else {
            "Document".to_string()
        };
        entries.push(RemoteFileEntry {
            name,
            path: remote_path,
            kind,
            is_directory,
            size: info.size as u64,
            modified: info.modified.format("%Y-%m-%d %H:%M:%S").to_string(),
        });
    }
    entries.sort_by(|left, right| {
        right
            .is_directory
            .cmp(&left.is_directory)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(entries)
}

#[tauri::command]
pub async fn afc_mkdir(
    state: State<'_, AppState>,
    udid: Option<String>,
    path: String,
    bundle_id: Option<String>,
) -> CommandResult<()> {
    ensure_mutation_allowed(&path, bundle_id.as_deref())?;
    let mut afc = afc_client(&state, udid, bundle_id.as_deref()).await?;
    afc.mk_dir(path).await.map_err(CommandError::from)
}

#[tauri::command]
pub async fn afc_remove(
    state: State<'_, AppState>,
    udid: Option<String>,
    path: String,
    recursive: bool,
    bundle_id: Option<String>,
) -> CommandResult<()> {
    ensure_mutation_allowed(&path, bundle_id.as_deref())?;
    let mut afc = afc_client(&state, udid, bundle_id.as_deref()).await?;
    if recursive {
        afc.remove_all(path).await.map_err(CommandError::from)
    } else {
        afc.remove(path).await.map_err(CommandError::from)
    }
}

#[tauri::command]
pub async fn afc_upload(
    state: State<'_, AppState>,
    udid: Option<String>,
    local_path: String,
    remote_path: String,
    bundle_id: Option<String>,
) -> CommandResult<()> {
    ensure_mutation_allowed(&remote_path, bundle_id.as_deref())?;
    let bytes = tokio::fs::read(local_path).await?;
    let mut afc = afc_client(&state, udid, bundle_id.as_deref()).await?;
    let mut file = afc
        .open(remote_path, AfcFopenMode::WrOnly)
        .await
        .map_err(CommandError::from)?;
    file.write_entire(&bytes)
        .await
        .map_err(CommandError::from)?;
    file.close().await.map_err(CommandError::from)
}

#[tauri::command]
pub async fn afc_download(
    state: State<'_, AppState>,
    udid: Option<String>,
    remote_path: String,
    local_path: String,
    bundle_id: Option<String>,
) -> CommandResult<()> {
    validate_remote_path(&remote_path)?;
    let mut afc = afc_client(&state, udid, bundle_id.as_deref()).await?;
    let mut file = afc
        .open(remote_path, AfcFopenMode::RdOnly)
        .await
        .map_err(CommandError::from)?;
    let bytes = file.read_entire().await.map_err(CommandError::from)?;
    file.close().await.map_err(CommandError::from)?;
    tokio::fs::write(local_path, bytes).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protects_ios_managed_media_roots() {
        assert!(protected_media_path("/DCIM/100APPLE/IMG_0001.HEIC"));
        assert!(protected_media_path("/PhotoData/Photos.sqlite"));
        assert!(!protected_media_path("/Downloads/example.pdf"));
    }

    #[test]
    fn app_documents_are_not_subject_to_media_protection() {
        assert!(ensure_mutation_allowed("/DCIM/file.txt", None).is_err());
        assert!(ensure_mutation_allowed("/DCIM/file.txt", Some("com.example.app")).is_ok());
    }

    #[test]
    fn rejects_relative_paths_and_root_mutation() {
        assert!(validate_remote_path("/Documents/../Library").is_err());
        assert!(ensure_mutation_allowed("/", Some("com.example.app")).is_err());
    }
}
