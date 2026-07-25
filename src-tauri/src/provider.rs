use std::{future::Future, net::IpAddr, pin::Pin, str::FromStr};

use idevice::{
    Idevice, IdeviceError,
    pairing_file::PairingFile,
    provider::{IdeviceProvider, TcpProvider, UsbmuxdProvider},
    usbmuxd::{UsbmuxdAddr, UsbmuxdConnection},
};

use crate::discovery::LockdownTarget;
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

#[derive(Debug)]
pub enum RoutedProvider {
    Usbmuxd(UsbmuxdProvider),
    Bonjour(TcpProvider),
}

impl IdeviceProvider for RoutedProvider {
    fn connect(
        &self,
        port: u16,
    ) -> Pin<Box<dyn Future<Output = Result<Idevice, IdeviceError>> + Send>> {
        match self {
            Self::Usbmuxd(provider) => provider.connect(port),
            Self::Bonjour(provider) => provider.connect(port),
        }
    }

    fn label(&self) -> &str {
        match self {
            Self::Usbmuxd(provider) => provider.label(),
            Self::Bonjour(provider) => provider.label(),
        }
    }

    fn get_pairing_file(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<PairingFile, IdeviceError>> + Send>> {
        match self {
            Self::Usbmuxd(provider) => provider.get_pairing_file(),
            Self::Bonjour(provider) => provider.get_pairing_file(),
        }
    }
}

pub async fn routed_provider_for(
    udid: &str,
    target: Option<&LockdownTarget>,
) -> CommandResult<RoutedProvider> {
    if let Ok(provider) = provider_for(udid).await {
        return Ok(RoutedProvider::Usbmuxd(provider));
    }

    let target = target.ok_or_else(|| {
        CommandError::new(
            "device",
            "The device has no active usbmuxd or Bonjour Lockdown route",
            true,
        )
    })?;
    let address = target
        .addresses
        .iter()
        .filter_map(|address| IpAddr::from_str(address).ok())
        .min_by_key(|address| match address {
            IpAddr::V4(_) => 0,
            IpAddr::V6(address) if !address.is_unicast_link_local() => 1,
            IpAddr::V6(_) => 2,
        })
        .ok_or_else(|| {
            CommandError::new(
                "bonjour",
                format!("Bonjour returned no usable address for {}", target.hostname),
                true,
            )
        })?;
    if matches!(address, IpAddr::V6(address) if address.is_unicast_link_local()) {
        return Err(CommandError::new(
            "bonjour",
            "The device only advertised a link-local IPv6 address; reconnect it to the same IPv4 network",
            true,
        ));
    }

    let mut mux = UsbmuxdConnection::default()
        .await
        .map_err(CommandError::from)?;
    let pairing_file = mux.get_pair_record(udid).await.map_err(|error| {
        CommandError::new(
            "pairing",
            format!("No saved Wi-Fi pairing record is available for {udid}: {error}"),
            true,
        )
    })?;
    Ok(RoutedProvider::Bonjour(TcpProvider {
        addr: address,
        scope_id: None,
        pairing_file,
        label: "idevice-desktop-bonjour".into(),
    }))
}

pub async fn selected_provider(
    state: &AppState,
    override_udid: Option<String>,
) -> CommandResult<(String, RoutedProvider)> {
    let udid = state
        .selected(override_udid)
        .await
        .ok_or_else(|| CommandError::new("device", "No device selected", true))?;
    let target = state.discovery.read().await.lockdown_target(&udid);
    let provider = routed_provider_for(&udid, target.as_ref()).await?;
    Ok((udid, provider))
}
