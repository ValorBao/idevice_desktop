//! Shared transport resolution for developer services.
//!
//! The one way a command opens a device service is [`DeviceContext::open_service`].
//! It owns generation selection (Legacy / CoreDeviceRemote / CoreDeviceLockdown)
//! and per-stage bounded cancel. Callers match [`DeviceService`] — they do not
//! rematch generation or pick an opener.
//!
//! USB-versus-Bonjour policy stays at the caller: a USB Lockdown service is
//! still allowed to skip the tunnel. New commands that need a developer
//! service should copy this module, not a neighbour's `match generation`.

use std::{path::PathBuf, sync::OnceLock, time::Duration};

use idevice::{
    IdeviceService, core_device_proxy::CoreDeviceProxy, provider::IdeviceProvider,
    rsd::RsdHandshake,
};
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use crate::{
    device_version::{DeveloperGeneration, IosVersion, ios_version},
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

const DEFAULT_STAGE_TIMEOUT: Duration = Duration::from_secs(45);

/// Options for [`DeviceContext::open_service`].
///
/// Defaults: one RemotePairing attempt, 45s per stage, a token that never
/// cancels. Live Screen and other cancellable commands pass their task token
/// and keep the same per-stage bound they had when they opened the tunnel
/// themselves.
#[derive(Debug, Clone)]
pub struct OpenOptions {
    token: CancellationToken,
    remote_pairing_attempts: usize,
    stage_timeout: Duration,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenOptions {
    pub fn new() -> Self {
        Self {
            token: never_cancelled(),
            remote_pairing_attempts: 1,
            stage_timeout: DEFAULT_STAGE_TIMEOUT,
        }
    }

    pub fn with_cancel(mut self, token: &CancellationToken) -> Self {
        self.token = token.clone();
        self
    }

    pub fn remote_pairing_attempts(mut self, attempts: usize) -> Self {
        self.remote_pairing_attempts = attempts.max(1);
        self
    }

    pub fn stage_timeout(mut self, timeout: Duration) -> Self {
        self.stage_timeout = timeout;
        self
    }
}

fn never_cancelled() -> CancellationToken {
    static TOKEN: OnceLock<CancellationToken> = OnceLock::new();
    TOKEN.get_or_init(CancellationToken::new).clone()
}

/// The opened developer-service transport. Generation has already been applied.
///
/// The caller keeps the [`RoutedProvider`] it passed in — USB-versus-Bonjour
/// policy and Lockdown clients still need it, and owning it here would force
/// every command to unpack it again.
pub enum DeviceService {
    Legacy {
        version: IosVersion,
    },
    Core {
        version: IosVersion,
        generation: DeveloperGeneration,
        route: &'static str,
        tunnel: RsdTunnel,
    },
}

impl DeviceService {
    pub fn version(&self) -> IosVersion {
        match self {
            Self::Legacy { version } | Self::Core { version, .. } => *version,
        }
    }

    pub fn generation(&self) -> DeveloperGeneration {
        match self {
            Self::Legacy { .. } => DeveloperGeneration::Legacy,
            Self::Core { generation, .. } => *generation,
        }
    }

    pub fn route(&self) -> Option<&'static str> {
        match self {
            Self::Legacy { .. } => None,
            Self::Core { route, .. } => Some(*route),
        }
    }
}

/// Maps a generation onto the tunnel route it uses. `None` means Lockdown only.
/// This is the decision [`DeviceContext::open_service`] applies; it is not a
/// second opener.
pub fn tunnel_plan(generation: DeveloperGeneration) -> Option<&'static str> {
    match generation {
        DeveloperGeneration::Legacy => None,
        DeveloperGeneration::CoreDeviceRemote => Some(ROUTE_REMOTE_PAIRING),
        DeveloperGeneration::CoreDeviceLockdown => Some(ROUTE_CORE_DEVICE_PROXY),
    }
}

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

    /// Builds a context for a device outside the desktop session.
    ///
    /// Production commands must pass the catalog's RemotePairing target when
    /// the device is iOS 17.0–17.3. Omitting it no longer falls back to
    /// browsing the first `_remotepairing._tcp` service on the LAN — that
    /// silently attaches to the wrong phone. Harnesses that still want "pick
    /// the first advertisement" call
    /// [`crate::tunnel::discover_first_remote_pairing`] by that name.
    pub fn standalone(
        udid: String,
        pairing_path: PathBuf,
        lockdown_target: Option<LockdownTarget>,
        remote_target: Option<RemotePairingTarget>,
    ) -> Self {
        Self {
            udid,
            pairing_path,
            lockdown_target,
            remote_target,
        }
    }

    /// Routes a provider to the device, preferring usbmuxd.
    pub async fn provider(&self) -> CommandResult<RoutedProvider> {
        routed_provider_for(&self.udid, self.lockdown_target.as_ref()).await
    }

    /// Opens the developer-service transport for this device.
    ///
    /// Generation and cancellation are applied here. Callers match
    /// [`DeviceService`]; they do not rematch generation or pick an opener.
    pub async fn open_service(
        &self,
        provider: &RoutedProvider,
        options: &OpenOptions,
    ) -> CommandResult<DeviceService> {
        let version = stage(
            &options.token,
            "Reading the iOS version",
            options.stage_timeout,
            ios_version(provider),
        )
        .await?;
        let generation = version.developer_generation();
        match tunnel_plan(generation) {
            None => Ok(DeviceService::Legacy { version }),
            Some(ROUTE_REMOTE_PAIRING) => {
                let tunnel = stage(
                    &options.token,
                    "Opening the RemotePairing tunnel",
                    options.stage_timeout,
                    self.open_remote_pairing_tunnel(provider, options.remote_pairing_attempts),
                )
                .await?;
                Ok(DeviceService::Core {
                    version,
                    generation,
                    route: ROUTE_REMOTE_PAIRING,
                    tunnel,
                })
            }
            Some(ROUTE_CORE_DEVICE_PROXY) => {
                let tunnel =
                    open_core_device_proxy_bounded(provider, &options.token, options.stage_timeout)
                        .await?;
                Ok(DeviceService::Core {
                    version,
                    generation,
                    route: ROUTE_CORE_DEVICE_PROXY,
                    tunnel,
                })
            }
            Some(route) => Err(CommandError::new(
                "tunnel",
                format!("Unknown developer-service route {route}"),
                false,
            )),
        }
    }

    /// Opens the iOS 17.0–17.3 RemotePairing tunnel, retrying transient
    /// failures with a short backoff.
    pub(crate) async fn open_remote_pairing_tunnel(
        &self,
        provider: &impl IdeviceProvider,
        attempts: usize,
    ) -> CommandResult<RsdTunnel> {
        let target = require_remote_pairing_target(self.remote_target.as_ref())?;
        let mut attempt = 1;
        loop {
            match open_remote_pairing_tunnel(
                provider,
                &self.pairing_path,
                HOST_IDENTIFIER,
                Some(target),
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

/// Refuses a missing RemotePairing target instead of browsing the LAN.
pub fn require_remote_pairing_target(
    target: Option<&RemotePairingTarget>,
) -> CommandResult<&RemotePairingTarget> {
    target.ok_or_else(|| {
        CommandError::new(
            "pairing",
            "RemotePairing requires a discovered service for this device; standalone callers must pass one",
            false,
        )
    })
}

/// Opens the RSD tunnel through Lockdown CoreDeviceProxy, the iOS 17.4+ route.
async fn open_core_device_proxy_bounded(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::RemotePairingTarget;
    use std::path::PathBuf;

    #[test]
    fn tunnel_plan_follows_generation_boundaries() {
        assert_eq!(tunnel_plan(DeveloperGeneration::Legacy), None);
        assert_eq!(
            tunnel_plan(DeveloperGeneration::CoreDeviceRemote),
            Some(ROUTE_REMOTE_PAIRING)
        );
        assert_eq!(
            tunnel_plan(DeveloperGeneration::CoreDeviceLockdown),
            Some(ROUTE_CORE_DEVICE_PROXY)
        );
    }

    #[test]
    fn missing_remote_pairing_target_is_a_pairing_error() {
        let error = require_remote_pairing_target(None).unwrap_err();
        assert_eq!(error.kind, "pairing");
        assert!(error.message.contains("standalone"));
        assert!(!error.retryable);
    }

    #[test]
    fn present_remote_pairing_target_is_returned() {
        let target = RemotePairingTarget {
            hostname: "phone.local".into(),
            port: 62078,
        };
        let returned = require_remote_pairing_target(Some(&target)).unwrap();
        assert_eq!(returned.hostname, "phone.local");
        assert_eq!(returned.port, 62078);
    }

    #[test]
    fn standalone_does_not_invent_a_remote_pairing_target() {
        let context = DeviceContext::standalone(
            "udid".into(),
            PathBuf::from("/tmp/pairing.plist"),
            None,
            None,
        );
        assert!(context.remote_target.is_none());
        assert!(require_remote_pairing_target(context.remote_target.as_ref()).is_err());
    }

    #[test]
    fn open_options_default_to_one_attempt_and_45s() {
        let options = OpenOptions::new();
        assert_eq!(options.remote_pairing_attempts, 1);
        assert_eq!(options.stage_timeout, Duration::from_secs(45));
        assert!(!options.token.is_cancelled());
    }

    #[test]
    fn open_options_zero_attempts_becomes_one() {
        let options = OpenOptions::new().remote_pairing_attempts(0);
        assert_eq!(options.remote_pairing_attempts, 1);
    }
}
