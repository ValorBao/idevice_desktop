use idevice::{IdeviceService, diagnostics_relay::DiagnosticsRelayClient};
use tauri::State;

use crate::{
    error::{CommandError, CommandResult},
    provider::selected_provider,
    state::AppState,
    utils::plist_to_json,
};

async fn client(state: &AppState, udid: Option<String>) -> CommandResult<DiagnosticsRelayClient> {
    let (_, provider) = selected_provider(state, udid).await?;
    DiagnosticsRelayClient::connect(&provider)
        .await
        .map_err(CommandError::from)
}

fn optional_value(value: Option<plist::Dictionary>) -> serde_json::Value {
    value
        .as_ref()
        .map(|value| plist_to_json(&plist::Value::Dictionary(value.clone())))
        .unwrap_or(serde_json::Value::Null)
}

#[tauri::command]
pub async fn diagnostics_battery(
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<serde_json::Value> {
    let mut client = client(&state, udid).await?;
    let value = client.gasguage().await.map_err(CommandError::from)?;
    Ok(optional_value(value))
}

#[tauri::command]
pub async fn diagnostics_gestalt(
    state: State<'_, AppState>,
    udid: Option<String>,
    keys: Option<Vec<String>>,
) -> CommandResult<serde_json::Value> {
    let mut client = client(&state, udid).await?;
    let value = client
        .mobilegestalt(keys)
        .await
        .map_err(CommandError::from)?;
    Ok(optional_value(value))
}

#[tauri::command]
pub async fn diagnostics_ioregistry(
    state: State<'_, AppState>,
    udid: Option<String>,
    plane: Option<String>,
    name: Option<String>,
    class: Option<String>,
) -> CommandResult<serde_json::Value> {
    let mut client = client(&state, udid).await?;
    let value = client
        .ioregistry(plane.as_deref(), name.as_deref(), class.as_deref())
        .await
        .map_err(CommandError::from)?;
    Ok(optional_value(value))
}

#[tauri::command]
pub async fn diagnostics_nand(
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<serde_json::Value> {
    let mut client = client(&state, udid).await?;
    let value = client.nand().await.map_err(CommandError::from)?;
    Ok(optional_value(value))
}

#[tauri::command]
pub async fn diagnostics_wifi(
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<serde_json::Value> {
    let mut client = client(&state, udid).await?;
    let value = client.wifi().await.map_err(CommandError::from)?;
    Ok(optional_value(value))
}
