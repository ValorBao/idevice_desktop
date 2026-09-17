//! Shared transport resolution for developer services.
//!
//! Every developer-service command used to carry its own copy of the same
//! three steps: resolve the selected device into a context, route a provider
//! to it, and open the RSD tunnel that matches the device's generation. This
//! module owns those steps so a command module only decides what to do with
//! the tunnel once it has one.

use std::{path::PathBuf, time::Duration};

use idevice::{
    IdeviceService, core_device_proxy::CoreDeviceProxy, provider::IdeviceProvider,
    rsd::RsdHandshake,
};
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use crate::{
    device_version::DeveloperGeneration,
    discovery::{LockdownTarget, RemotePairingTarget},
    error::{CommandError, CommandResult},
    provider::{RoutedProvider, routed_provider_for},
    state::AppState,
    task::stage,
    tunnel::{RsdTunnel, open_remote_pairing_tunnel, remote_pairing_path},
};

/// The host identifier written into RemotePairing records.
pub const HOST_IDENTIFIER: &str = "idevice-desktop";
/// Route label for iOS 17.0–17.3 tunnels.
pub const ROUTE_REMOTE_PAIRING: &str = "RemotePairing/RSD";
/// Route label for iOS 17.4+ tunnels.
pub const ROUTE_CORE_DEVICE_PROXY: &str = "CoreDeviceProxy/RSD";

/// Everything a command needs to reach the selected device after the Tauri
/// state is no longer available, such as on a dedicated runtime thread.
#[derive(Debug, Clone)]
pub struct DeviceContext {
    pub udid: String,
    pub pairing_path: PathBuf,
    pub lockdown_target: Option<LockdownTarget>,
    pub remote_target: Option<RemotePairingTarget>,
}

impl DeviceContext {
    /// Resolves the selected device, or the override, into a context.
    pub async fn resolve(
        app: &AppHandle,
        state: &AppState,
        override_udid: Option<String>,
    ) -> CommandResult<Self> {
        let udid = state
            .selected(override_udid)
            .await
            .ok_or_else(|| CommandError::new("device", "No device selected", true))?;
        let catalog = state.discovery.read().await;
        Ok(Self {
            pairing_path: remote_pairing_path(app, &udid)?,
            lockdown_target: catalog.lockdown_target(&udid),
            remote_target: catalog.remote_pairing_target(&udid),
            udid,
        })
    }

    /// Builds a context for a device outside the desktop session, such as a
    /// hardware verification harness.
    pub fn standalone(udid: String, pairing_path: PathBuf) -> Self {
        Self {
            udid,
            pairing_path,
            lockdown_target: None,
            remote_target: None,
        }
    }

    /// Routes a provider to the device, preferring usbmuxd.
    pub async fn provider(&self) -> CommandResult<RoutedProvider> {
        routed_provider_for(&self.udid, self.lockdown_target.as_ref()).await
    }

    /// Opens the RSD tunnel for the device's generation.
    ///
    /// Legacy devices have no RSD, so they return `None`; the caller decides
    /// whether a Lockdown fallback exists. `remote_pairing_attempts` bounds the
    /// retries for transient RemotePairing failures on iOS 17.0–17.3; pass 1 to
    /// try once.
    pub async fn open_rsd_tunnel(
        &self,
        provider: &RoutedProvider,
        generation: DeveloperGeneration,
        remote_pairing_attempts: usize,
    ) -> CommandResult<Option<(&'static str, RsdTunnel)>> {
        match generation {
            DeveloperGeneration::Legacy => Ok(None),
            DeveloperGeneration::CoreDeviceRemote => {
                let tunnel = self
                    .open_remote_pairing_tunnel(provider, remote_pairing_attempts)
                    .await?;
                Ok(Some((ROUTE_REMOTE_PAIRING, tunnel)))
            }
            DeveloperGeneration::CoreDeviceLockdown => Ok(Some((
                ROUTE_CORE_DEVICE_PROXY,
                open_core_device_proxy(provider).await?,
            ))),
        }
    }

    /// Opens the iOS 17.0–17.3 RemotePairing tunnel, retrying transient
    /// failures with a short backoff.
    pub async fn open_remote_pairing_tunnel(
        &self,
        provider: &impl IdeviceProvider,
        attempts: usize,
    ) -> CommandResult<RsdTunnel> {
        let mut attempt = 1;
        loop {
            match open_remote_pairing_tunnel(
                provider,
                &self.pairing_path,
                HOST_IDENTIFIER,
                self.remote_target.as_ref(),
            )
            .await
            {
                Ok(tunnel) => return Ok(tunnel),
                Err(error) if error.retryable && attempt < attempts => {
                    tracing::warn!(
                        attempt,
                        error = %error.message,
                        "retrying the RemotePairing tunnel"
                    );
                    tokio::time::sleep(Duration::from_millis(250 * attempt as u64)).await;
                    attempt += 1;
                }
                Err(error) => return Err(error),
            }
        }
    }
}

/// Opens the RSD tunnel through Lockdown CoreDeviceProxy, the iOS 17.4+ route.
pub async fn open_core_device_proxy(provider: &impl IdeviceProvider) -> CommandResult<RsdTunnel> {
    let proxy = CoreDeviceProxy::connect(provider)
        .await
        .map_err(CommandError::from)?;
    let rsd_port = proxy.tunnel_info().server_rsd_port;
    let mut adapter = proxy
        .create_software_tunnel()
        .map_err(tunnel_error)?
        .to_async_handle();
    let stream = adapter.connect(rsd_port).await.map_err(tunnel_error)?;
    let handshake = RsdHandshake::new(stream)
        .await
        .map_err(CommandError::from)?;
    Ok(RsdTunnel { adapter, handshake })
}

/// The same as [`open_core_device_proxy`], with each stage bounded by
/// `stage_timeout` and cancellable through `token`.
pub async fn open_core_device_proxy_bounded(
    provider: &impl IdeviceProvider,
    token: &CancellationToken,
    stage_timeout: Duration,
) -> CommandResult<RsdTunnel> {
    let proxy = stage(token, "Opening CoreDeviceProxy", stage_timeout, async {
        CoreDeviceProxy::connect(provider)
            .await
            .map_err(CommandError::from)
    })
    .await?;
    let rsd_port = proxy.tunnel_info().server_rsd_port;
    let mut adapter = proxy
        .create_software_tunnel()
        .map_err(tunnel_error)?
        .to_async_handle();
    let stream = stage(token, "Connecting to RSD", stage_timeout, async {
        adapter.connect(rsd_port).await.map_err(tunnel_error)
    })
    .await?;
    let handshake = stage(token, "The RSD handshake", stage_timeout, async {
        RsdHandshake::new(stream).await.map_err(CommandError::from)
    })
    .await?;
    Ok(RsdTunnel { adapter, handshake })
}

fn tunnel_error(error: impl std::fmt::Display) -> CommandError {
    CommandError::new("tunnel", error.to_string(), true)
}
