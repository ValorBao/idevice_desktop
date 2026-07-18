use std::path::Path;

use idevice::{
    IdeviceService,
    afc::{AfcClient, opcode::AfcFopenMode},
};
use tauri::State;

use crate::{
    error::{CommandError, CommandResult},
    provider::selected_provider,
    state::AppState,
    types::RemoteFileEntry,
};

fn remote_join(parent: &str, name: &str) -> String {
    let parent = if parent.is_empty() { "/" } else { parent };
    if parent == "/" {
        format!("/{name}")
    } else {
        format!("{}/{name}", parent.trim_end_matches('/'))
    }
}

async fn afc_client(state: &AppState, udid: Option<String>) -> CommandResult<AfcClient> {
    let (_, provider) = selected_provider(state, udid).await?;
    AfcClient::connect(&provider)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn afc_list(
    state: State<'_, AppState>,
    udid: Option<String>,
    path: String,
) -> CommandResult<Vec<RemoteFileEntry>> {
    let mut afc = afc_client(&state, udid).await?;
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
) -> CommandResult<()> {
    let mut afc = afc_client(&state, udid).await?;
    afc.mk_dir(path).await.map_err(CommandError::from)
}

#[tauri::command]
pub async fn afc_remove(
    state: State<'_, AppState>,
    udid: Option<String>,
    path: String,
    recursive: bool,
) -> CommandResult<()> {
    let mut afc = afc_client(&state, udid).await?;
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
) -> CommandResult<()> {
    let bytes = tokio::fs::read(local_path).await?;
    let mut afc = afc_client(&state, udid).await?;
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
) -> CommandResult<()> {
    let mut afc = afc_client(&state, udid).await?;
    let mut file = afc
        .open(remote_path, AfcFopenMode::RdOnly)
        .await
        .map_err(CommandError::from)?;
    let bytes = file.read_entire().await.map_err(CommandError::from)?;
    file.close().await.map_err(CommandError::from)?;
    tokio::fs::write(local_path, bytes).await?;
    Ok(())
}
