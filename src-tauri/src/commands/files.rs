use std::path::Path;

use idevice::{
    IdeviceService,
    afc::{AfcClient, opcode::AfcFopenMode},
    installation_proxy::InstallationProxyClient,
    services::house_arrest::HouseArrestClient,
};
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

use crate::{
    error::{CommandError, CommandResult},
    provider::selected_provider,
    state::AppState,
    types::{FileSharingApp, OperationProgress, RemoteFileEntry},
    utils::dict_string,
};

/// Matches the AFC wire limit, so a chunk here is one protocol transfer rather
/// than something the library has to split again.
const TRANSFER_CHUNK: usize = 1024 * 1024;

/// Key under which a transfer registers its cancellation token. Transfers are
/// mutually exclusive by design: starting a second one cancels the first, and
/// switching or disconnecting a device cancels whichever is running.
const TRANSFER_TASK: &str = "file-transfer";

/// Reports transfer progress, but only when the whole-percent figure actually
/// changes. A gigabyte at 1 MB per chunk is a thousand iterations; emitting on
/// each one would flood the event channel to say the same thing.
struct TransferProgress {
    app: AppHandle,
    operation: &'static str,
    item: String,
    total: u64,
    transferred: u64,
    last_percent: u64,
}

impl TransferProgress {
    fn new(app: AppHandle, operation: &'static str, item: String, total: u64) -> Self {
        let progress = Self {
            app,
            operation,
            item,
            total,
            transferred: 0,
            last_percent: 0,
        };
        progress.emit(0);
        progress
    }

    fn advance(&mut self, bytes: usize) {
        self.transferred += bytes as u64;
        let percent = transfer_percent(self.transferred, self.total);
        if percent != self.last_percent {
            self.last_percent = percent;
            self.emit(percent);
        }
    }

    fn finish(&self) {
        if self.last_percent != 100 {
            self.emit(100);
        }
    }

    fn emit(&self, percent: u64) {
        let _ = self.app.emit(
            "files://transfer-progress",
            OperationProgress {
                operation: self.operation.into(),
                item: self.item.clone(),
                percent,
            },
        );
    }
}

/// Whole-percent progress. An empty file has no meaningful ratio, so it reports
/// complete rather than dividing by zero, and the result is clamped because a
/// file can grow between the size reading and the transfer.
fn transfer_percent(transferred: u64, total: u64) -> u64 {
    if total == 0 {
        return 100;
    }
    (transferred.saturating_mul(100) / total).min(100)
}

fn cancelled() -> CommandError {
    CommandError::new("cancelled", "Transfer cancelled", false)
}

fn transfer_name(path: &str) -> String {
    path.rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or(path)
        .to_string()
}

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
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
    local_path: String,
    remote_path: String,
    bundle_id: Option<String>,
) -> CommandResult<()> {
    ensure_mutation_allowed(&remote_path, bundle_id.as_deref())?;
    let total = tokio::fs::metadata(&local_path).await?.len();
    let mut source = tokio::fs::File::open(&local_path).await?;

    let token = CancellationToken::new();
    state.replace_task(TRANSFER_TASK, token.clone()).await;

    let mut afc = afc_client(&state, udid, bundle_id.as_deref()).await?;
    let mut file = afc
        .open(remote_path.clone(), AfcFopenMode::WrOnly)
        .await
        .map_err(CommandError::from)?;

    let mut progress = TransferProgress::new(app, "upload", transfer_name(&remote_path), total);
    let mut buffer = vec![0u8; TRANSFER_CHUNK];
    let result = loop {
        if token.is_cancelled() {
            break Err(cancelled());
        }
        let read = match source.read(&mut buffer).await {
            Ok(0) => break Ok(()),
            Ok(read) => read,
            Err(error) => break Err(CommandError::from(error)),
        };
        // Each write lands at the descriptor's current position, so successive
        // chunks append rather than rewriting from the start.
        if let Err(error) = file.write_entire(&buffer[..read]).await {
            break Err(CommandError::from(error));
        }
        progress.advance(read);
    };

    file.close().await.map_err(CommandError::from)?;
    if result.is_err() {
        // A partial file on the device is worse than none: it looks complete in
        // the listing and would be read as valid.
        let _ = afc.remove(remote_path).await;
    }
    result?;
    progress.finish();
    Ok(())
}

#[tauri::command]
pub async fn afc_download(
    app: AppHandle,
    state: State<'_, AppState>,
    udid: Option<String>,
    remote_path: String,
    local_path: String,
    bundle_id: Option<String>,
) -> CommandResult<()> {
    validate_remote_path(&remote_path)?;

    let token = CancellationToken::new();
    state.replace_task(TRANSFER_TASK, token.clone()).await;

    let mut afc = afc_client(&state, udid, bundle_id.as_deref()).await?;
    let total = afc
        .get_file_info(&remote_path)
        .await
        .map(|info| info.size as u64)
        .unwrap_or(0);
    let mut file = afc
        .open(remote_path.clone(), AfcFopenMode::RdOnly)
        .await
        .map_err(CommandError::from)?;
    let mut target = tokio::fs::File::create(&local_path).await?;

    let mut progress = TransferProgress::new(app, "download", transfer_name(&remote_path), total);
    let result = loop {
        if token.is_cancelled() {
            break Err(cancelled());
        }
        let chunk = match file.read_n(TRANSFER_CHUNK).await {
            Ok(chunk) if chunk.is_empty() => break Ok(()),
            Ok(chunk) => chunk,
            Err(error) => break Err(CommandError::from(error)),
        };
        if let Err(error) = target.write_all(&chunk).await {
            break Err(CommandError::from(error));
        }
        progress.advance(chunk.len());
    };

    file.close().await.map_err(CommandError::from)?;
    if let Err(error) = result {
        // Close the handle before unlinking so the partial file does not linger
        // as something the user could mistake for a finished download.
        drop(target);
        let _ = tokio::fs::remove_file(&local_path).await;
        return Err(error);
    }
    target.flush().await?;
    progress.finish();
    Ok(())
}

#[tauri::command]
pub async fn afc_transfer_cancel(state: State<'_, AppState>) -> CommandResult<()> {
    state.cancel_task(TRANSFER_TASK).await;
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

    #[test]
    fn reports_transfer_percent() {
        assert_eq!(transfer_percent(0, 400), 0);
        assert_eq!(transfer_percent(100, 400), 25);
        assert_eq!(transfer_percent(400, 400), 100);
    }

    /// An empty file is a legitimate transfer, and dividing by its size is not.
    #[test]
    fn treats_an_empty_file_as_complete() {
        assert_eq!(transfer_percent(0, 0), 100);
    }

    /// The size is read before the transfer starts, so a file that grows in
    /// between would otherwise report more than 100.
    #[test]
    fn clamps_when_more_arrives_than_expected() {
        assert_eq!(transfer_percent(900, 400), 100);
    }

    #[test]
    fn names_a_transfer_by_its_last_path_component() {
        assert_eq!(transfer_name("/Downloads/report.pdf"), "report.pdf");
        assert_eq!(transfer_name("/Exports/"), "Exports");
        assert_eq!(transfer_name("plain.txt"), "plain.txt");
    }
}
