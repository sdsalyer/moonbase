use std::fmt;

/// Custom configuration errors
#[derive(Debug)]
pub enum ConfigError {
    InvalidValue(String, String),
    UnknownKey(String),
    UnknownSection(String),
    // IoError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidValue(key, value) => {
                write!(f, "Invalid value '{}' for key '{}'", value, key)
            }
            ConfigError::UnknownKey(key) => write!(f, "Unknown configuration key: '{}'", key),
            ConfigError::UnknownSection(section) => write!(f, "Unknown section: '{}'", section),
            // ConfigError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Custom BBS errors
#[derive(Debug)]
pub enum BbsError {
    /// I/O related errors (network, file operations, etc.)
    Io(std::io::Error),

    /// Invalid user input
    InvalidInput(String),

    /// Authentication failed (too many attempts, invalid credentials, etc.)
    AuthenticationFailed(String),

    /// Feature is disabled by configuration
    // FeatureDisabled(String),

    /// Client disconnected unexpectedly
    ClientDisconnected,

    /// Configuration error
    Configuration(String),
}

impl fmt::Display for BbsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BbsError::Io(err) => write!(f, "I/O error: {}", err),
            BbsError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            BbsError::AuthenticationFailed(msg) => write!(f, "Authentication failed: {}", msg),
            // BbsError::FeatureDisabled(feature) => write!(f, "Feature '{}' is disabled", feature),
            BbsError::ClientDisconnected => write!(f, "Client disconnected"),
            BbsError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for BbsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BbsError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for BbsError {
    fn from(err: std::io::Error) -> Self {
        use std::io::ErrorKind;

        match err.kind() {
            ErrorKind::UnexpectedEof
            | ErrorKind::ConnectionReset
            | ErrorKind::ConnectionAborted => BbsError::ClientDisconnected,
            _ => BbsError::Io(err),
        }
    }
}

impl From<ConfigError> for BbsError {
    fn from(err: ConfigError) -> Self {
        BbsError::Configuration(err.to_string())
    }
}

/// Result type alias for BBS operations
pub type BbsResult<T> = Result<T, BbsError>;
