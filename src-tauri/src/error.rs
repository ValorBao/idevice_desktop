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
