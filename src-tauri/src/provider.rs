use idevice::{
    provider::UsbmuxdProvider,
    usbmuxd::{UsbmuxdAddr, UsbmuxdConnection},
};

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

pub async fn provider_for(udid: &str) -> CommandResult<UsbmuxdProvider> {
    let mut mux = UsbmuxdConnection::default()
        .await
        .map_err(CommandError::from)?;
    let device = mux.get_device(udid).await.map_err(CommandError::from)?;
    let addr = UsbmuxdAddr::from_env_var().map_err(CommandError::from)?;
    Ok(device.to_provider(addr, "idevice-desktop"))
}

pub async fn selected_provider(
    state: &AppState,
    override_udid: Option<String>,
) -> CommandResult<(String, UsbmuxdProvider)> {
    let udid = state
        .selected(override_udid)
        .await
        .ok_or_else(|| CommandError::new("device", "No device selected", true))?;
    let provider = provider_for(&udid).await?;
    Ok((udid, provider))
}
