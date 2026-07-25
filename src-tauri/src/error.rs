use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub kind: String,
    pub message: String,
    pub retryable: bool,
}

impl CommandError {
    pub fn new(kind: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
            retryable,
        }
    }
}

impl From<idevice::IdeviceError> for CommandError {
    fn from(value: idevice::IdeviceError) -> Self {
        let retryable = matches!(
            value,
            idevice::IdeviceError::DeviceNotFound
                | idevice::IdeviceError::NoEstablishedConnection
                | idevice::IdeviceError::DeviceLocked
        );
        Self::new("idevice", value.to_string(), retryable)
    }
}

impl From<std::net::AddrParseError> for CommandError {
    fn from(value: std::net::AddrParseError) -> Self {
        Self::new("configuration", value.to_string(), false)
    }
}

impl From<std::io::Error> for CommandError {
    fn from(value: std::io::Error) -> Self {
        Self::new("io", value.to_string(), false)
    }
}

pub type CommandResult<T> = Result<T, CommandError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// `retryable` drives whether the interface invites the user to try again.
    /// Marking a permanent failure retryable sends them in circles; marking a
    /// transient one permanent makes them reconnect the device for nothing.
    #[test]
    fn treats_transient_device_conditions_as_retryable() {
        for error in [
            idevice::IdeviceError::DeviceNotFound,
            idevice::IdeviceError::NoEstablishedConnection,
            idevice::IdeviceError::DeviceLocked,
        ] {
            let mapped = CommandError::from(error);
            assert!(mapped.retryable, "{} should be retryable", mapped.message);
            assert_eq!(mapped.kind, "idevice");
        }
    }

    #[test]
    fn treats_other_device_errors_as_permanent() {
        // A missing pairing record or an unsupported service will not fix
        // itself by repeating the same call.
        let mapped = CommandError::from(idevice::IdeviceError::InvalidHostID);
        assert!(!mapped.retryable);
        assert_eq!(mapped.kind, "idevice");
    }

    #[test]
    fn labels_non_device_failures_by_their_source() {
        let io = CommandError::from(std::io::Error::other("disk full"));
        assert_eq!(io.kind, "io");
        assert!(!io.retryable);

        let parse = CommandError::from("not-an-address".parse::<std::net::IpAddr>().unwrap_err());
        assert_eq!(parse.kind, "configuration");
        assert!(!parse.retryable);
    }
}
