use idevice::{IdeviceService, lockdown::LockdownClient, provider::IdeviceProvider};

use crate::error::{CommandError, CommandResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IosVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeveloperGeneration {
    /// iOS 16 and earlier: DeveloperDiskImage + lockdownd developer services.
    Legacy,
    /// iOS 17.0 through 17.3.x: personalized DDI + remoted/RemotePairing tunnel.
    CoreDeviceRemote,
    /// iOS 17.4 and later: personalized DDI + lockdownd CoreDeviceProxy tunnel.
    CoreDeviceLockdown,
}

impl IosVersion {
    pub fn parse(value: &str) -> Option<Self> {
        let mut components = value.split('.');
        let major = components.next()?.parse().ok()?;
        let minor = components
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let patch = components
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        Some(Self {
            major,
            minor,
            patch,
        })
    }

    pub fn developer_generation(self) -> DeveloperGeneration {
        if self.major < 17 {
            DeveloperGeneration::Legacy
        } else if self.major == 17 && self.minor < 4 {
            DeveloperGeneration::CoreDeviceRemote
        } else {
            DeveloperGeneration::CoreDeviceLockdown
        }
    }
}

pub async fn ios_version(provider: &impl IdeviceProvider) -> CommandResult<IosVersion> {
    let mut lockdown = LockdownClient::connect(provider)
        .await
        .map_err(CommandError::from)?;
    let pairing = provider
        .get_pairing_file()
        .await
        .map_err(CommandError::from)?;
    lockdown
        .start_session(&pairing)
        .await
        .map_err(CommandError::from)?;
    let value = lockdown
        .get_value(Some("ProductVersion"), None)
        .await
        .map_err(CommandError::from)?
        .into_string()
        .unwrap_or_default();
    IosVersion::parse(&value).ok_or_else(|| {
        CommandError::new(
            "device",
            format!("Device returned an invalid iOS version: {value}"),
            false,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{DeveloperGeneration, IosVersion};

    fn generation(value: &str) -> DeveloperGeneration {
        IosVersion::parse(value).unwrap().developer_generation()
    }

    #[test]
    fn developer_generation_boundaries() {
        assert_eq!(generation("16.7.10"), DeveloperGeneration::Legacy);
        assert_eq!(generation("17.0"), DeveloperGeneration::CoreDeviceRemote);
        assert_eq!(generation("17.3.1"), DeveloperGeneration::CoreDeviceRemote);
        assert_eq!(generation("17.4"), DeveloperGeneration::CoreDeviceLockdown);
        assert_eq!(generation("18.0"), DeveloperGeneration::CoreDeviceLockdown);
    }
}
