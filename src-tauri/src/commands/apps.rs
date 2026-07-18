use idevice::{IdeviceService, installation_proxy::InstallationProxyClient, utils::installation};
use tauri::{AppHandle, Emitter, State};

use crate::{
    error::{CommandError, CommandResult},
    provider::selected_provider,
    state::AppState,
    types::{InstalledApp, OperationProgress},
    utils::{dict_string, dict_u64, plist_to_json},
};

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
        .get_apps(Some("Any"), None)
        .await
        .map_err(CommandError::from)?;
    let mut result = apps
        .into_iter()
        .filter_map(|(bundle_id, value)| {
            let dict = value.as_dictionary()?;
            let name = dict_string(dict, &["CFBundleDisplayName", "CFBundleName"])
                .unwrap_or_else(|| bundle_id.clone());
            let version = dict_string(dict, &["CFBundleShortVersionString", "CFBundleVersion"])
                .unwrap_or_default();
            let size_bytes = dict_u64(dict, &["StaticDiskUsage"]).unwrap_or(0)
                + dict_u64(dict, &["DynamicDiskUsage"]).unwrap_or(0);
            let system = dict_string(dict, &["ApplicationType"])
                .is_some_and(|value| value.eq_ignore_ascii_case("system"));
            Some(InstalledApp {
                bundle_id,
                name,
                version,
                size_bytes,
                system,
                raw: plist_to_json(&value),
            })
        })
        .collect::<Vec<_>>();
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
