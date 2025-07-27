use thiserror::Error;

/// Custom error types for huestatus application
#[derive(Error, Debug)]
pub enum HueStatusError {
    #[error("Configuration file not found. Run 'huestatus --setup' to configure.")]
    ConfigNotFound,

    #[error("Invalid configuration: {reason}")]
    InvalidConfig { reason: String },

    #[error("Configuration file corrupted. Run 'huestatus --setup' to reconfigure.")]
    ConfigCorrupted,

    #[error("Configuration version incompatible. Run 'huestatus --setup' to update.")]
    ConfigVersionIncompatible,

    #[error("Bridge not found. Check network connection and try again.")]
    BridgeNotFound,

    #[error("Bridge connection failed: {reason}")]
    BridgeConnectionFailed { reason: String },

    #[error("Authentication failed. Run 'huestatus --setup' to re-authenticate.")]
    AuthenticationFailed,

    #[error("Link button not pressed. Press the link button on your Hue bridge and try again.")]
    LinkButtonNotPressed,

    #[error("Scene '{scene_name}' not found. Run 'huestatus --setup' to recreate scenes.")]
    SceneNotFound { scene_name: String },

    #[error("Scene execution failed: {reason}")]
    SceneExecutionFailed { reason: String },

    #[error("Network error: {source}")]
    NetworkError {
        #[from]
        source: reqwest::Error,
    },

    #[error("API error: {message}")]
    ApiError { message: String },

    #[error("Timeout error: {operation}")]
    TimeoutError { operation: String },

    #[error("IO error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    #[error("JSON parsing error: {source}")]
    JsonError {
        #[from]
        source: serde_json::Error,
    },

    #[error("No lights found. Ensure your Hue bridge has lights connected.")]
    NoLightsFound,

    #[error("Bridge capability check failed: {reason}")]
    CapabilityCheckFailed { reason: String },

    #[error("Setup process failed: {reason}")]
    SetupFailed { reason: String },

    #[error("Validation failed: {reason}")]
    ValidationFailed { reason: String },

    #[error("Permission denied: {reason}")]
    PermissionDenied { reason: String },

    #[error("Discovery service unreachable: {reason}")]
    DiscoveryServiceUnreachable { reason: String },

    #[error("mDNS discovery failed: {reason}")]
    MdnsDiscoveryFailed { reason: String },

    #[error("Scene storage limit exceeded. Bridge can store maximum {max_scenes} scenes.")]
    SceneStorageLimitExceeded { max_scenes: usize },

    #[error("Invalid scene data: {reason}")]
    InvalidSceneData { reason: String },

    #[error("Color conversion error: {reason}")]
    ColorConversionError { reason: String },

    #[error("Configuration directory creation failed: {path}")]
    ConfigDirectoryCreationFailed { path: String },

    #[error("Unsupported platform: {platform}")]
    UnsupportedPlatform { platform: String },

    #[error("Environment variable error: {var_name}")]
    EnvironmentVariableError { var_name: String },

    #[error("Path too long: {path}")]
    PathTooLong { path: String },

    #[error("Capacity overflow during {operation}")]
    CapacityOverflow { operation: String },
}

impl HueStatusError {
    /// Convert error to appropriate exit code
    pub fn exit_code(&self) -> i32 {
        match self {
            HueStatusError::ConfigNotFound
            | HueStatusError::InvalidConfig { .. }
            | HueStatusError::ConfigCorrupted
            | HueStatusError::ConfigVersionIncompatible => 1,

            HueStatusError::BridgeNotFound
            | HueStatusError::BridgeConnectionFailed { .. }
            | HueStatusError::NetworkError { .. }
            | HueStatusError::TimeoutError { .. }
            | HueStatusError::ApiError { .. }
            | HueStatusError::DiscoveryServiceUnreachable { .. }
            | HueStatusError::MdnsDiscoveryFailed { .. } => 2,

            HueStatusError::AuthenticationFailed | HueStatusError::LinkButtonNotPressed => 3,

            HueStatusError::SceneNotFound { .. }
            | HueStatusError::SceneExecutionFailed { .. }
            | HueStatusError::SceneStorageLimitExceeded { .. }
            | HueStatusError::InvalidSceneData { .. } => 4,

            HueStatusError::IoError { .. }
            | HueStatusError::JsonError { .. }
            | HueStatusError::PermissionDenied { .. }
            | HueStatusError::ConfigDirectoryCreationFailed { .. } => 5,

            HueStatusError::NoLightsFound
            | HueStatusError::CapabilityCheckFailed { .. }
            | HueStatusError::SetupFailed { .. }
            | HueStatusError::ValidationFailed { .. }
            | HueStatusError::ColorConversionError { .. }
            | HueStatusError::UnsupportedPlatform { .. }
            | HueStatusError::EnvironmentVariableError { .. }
            | HueStatusError::PathTooLong { .. }
            | HueStatusError::CapacityOverflow { .. } => 6,
        }
    }

    /// Get user-friendly error message with suggested actions
    pub fn user_message(&self) -> String {
        match self {
            HueStatusError::ConfigNotFound => {
                "No configuration found. Run 'huestatus --setup' to get started.".to_string()
            }
            HueStatusError::InvalidConfig { reason } => {
                format!("Configuration invalid: {}. Run 'huestatus --setup' to fix.", reason)
            }
            HueStatusError::ConfigCorrupted => {
                "Configuration file is corrupted. Run 'huestatus --setup' to recreate.".to_string()
            }
            HueStatusError::ConfigVersionIncompatible => {
                "Configuration version is incompatible. Run 'huestatus --setup' to update.".to_string()
            }
            HueStatusError::BridgeNotFound => {
                "Hue bridge not found. Check that your bridge is connected and on the same network.".to_string()
            }
            HueStatusError::BridgeConnectionFailed { reason } => {
                format!("Cannot connect to bridge: {}. Check network connection.", reason)
            }
            HueStatusError::AuthenticationFailed => {
                "Authentication failed. Run 'huestatus --setup' to re-authenticate.".to_string()
            }
            HueStatusError::LinkButtonNotPressed => {
                "Link button not pressed. Press the button on your Hue bridge and try again.".to_string()
            }
            HueStatusError::SceneNotFound { scene_name } => {
                format!("Scene '{}' not found. Run 'huestatus --setup' to recreate scenes.", scene_name)
            }
            HueStatusError::SceneExecutionFailed { reason } => {
                format!("Scene execution failed: {}. Check bridge connection.", reason)
            }
            HueStatusError::NoLightsFound => {
                "No lights found. Ensure your Hue bridge has lights connected and they are turned on.".to_string()
            }
            HueStatusError::SceneStorageLimitExceeded { max_scenes } => {
                format!("Bridge scene storage full (max: {}). Delete some scenes and try again.", max_scenes)
            }
            HueStatusError::TimeoutError { operation } => {
                format!("Operation timed out: {}. Check network connection and try again.", operation)
            }
            HueStatusError::PermissionDenied { reason } => {
                format!("Permission denied: {}. Check file permissions.", reason)
            }
            HueStatusError::DiscoveryServiceUnreachable { reason } => {
                format!("Cannot reach Philips discovery service: {}. Check internet connection.", reason)
            }
            HueStatusError::MdnsDiscoveryFailed { reason } => {
                format!("mDNS discovery failed: {}. Try manual bridge IP entry.", reason)
            }
            HueStatusError::SetupFailed { reason } => {
                format!("Setup failed: {}. Please try again.", reason)
            }
            HueStatusError::ValidationFailed { reason } => {
                format!("Validation failed: {}. Run 'huestatus --setup' to fix.", reason)
            }
            HueStatusError::UnsupportedPlatform { platform } => {
                format!("Platform '{}' is not supported.", platform)
            }
            HueStatusError::EnvironmentVariableError { var_name } => {
                format!("Environment variable '{}' is invalid or missing.", var_name)
            }
            HueStatusError::PathTooLong { path } => {
                format!("Configuration path is too long: {}. Try using a shorter path or set HUESTATUS_CONFIG_DIR environment variable.", path)
            }
            HueStatusError::CapacityOverflow { operation } => {
                format!("Memory capacity overflow during {}. This may be caused by extremely long file paths in WSL environment.", operation)
            }
            _ => self.to_string(),
        }
    }

    /// Check if error is recoverable with setup
    pub fn is_recoverable_with_setup(&self) -> bool {
        matches!(
            self,
            HueStatusError::ConfigNotFound
                | HueStatusError::InvalidConfig { .. }
                | HueStatusError::ConfigCorrupted
                | HueStatusError::ConfigVersionIncompatible
                | HueStatusError::AuthenticationFailed
                | HueStatusError::SceneNotFound { .. }
                | HueStatusError::NoLightsFound
                | HueStatusError::ValidationFailed { .. }
                | HueStatusError::PathTooLong { .. }
        )
    }

    /// Check if error requires network connectivity
    pub fn requires_network(&self) -> bool {
        matches!(
            self,
            HueStatusError::BridgeNotFound
                | HueStatusError::BridgeConnectionFailed { .. }
                | HueStatusError::NetworkError { .. }
                | HueStatusError::TimeoutError { .. }
                | HueStatusError::ApiError { .. }
                | HueStatusError::DiscoveryServiceUnreachable { .. }
                | HueStatusError::MdnsDiscoveryFailed { .. }
        )
    }

    /// Check if error is temporary and might be resolved with retry
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            HueStatusError::NetworkError { .. }
                | HueStatusError::TimeoutError { .. }
                | HueStatusError::BridgeConnectionFailed { .. }
                | HueStatusError::SceneExecutionFailed { .. }
                | HueStatusError::DiscoveryServiceUnreachable { .. }
        )
    }
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, HueStatusError>;

/// Convert reqwest::Error to HueStatusError with context
pub fn network_error(err: reqwest::Error) -> HueStatusError {
    if err.is_timeout() {
        HueStatusError::TimeoutError {
            operation: "HTTP request".to_string(),
        }
    } else if err.is_connect() {
        HueStatusError::BridgeConnectionFailed {
            reason: "Connection refused".to_string(),
        }
    } else {
        HueStatusError::NetworkError { source: err }
    }
}

/// Convert std::io::Error to HueStatusError with context
pub fn io_error(err: std::io::Error) -> HueStatusError {
    match err.kind() {
        std::io::ErrorKind::NotFound => HueStatusError::ConfigNotFound,
        std::io::ErrorKind::PermissionDenied => HueStatusError::PermissionDenied {
            reason: "File access denied".to_string(),
        },
        _ => HueStatusError::IoError { source: err },
    }
}

/// Convert serde_json::Error to HueStatusError with context
pub fn json_error(err: serde_json::Error) -> HueStatusError {
    if err.is_syntax() {
        HueStatusError::ConfigCorrupted
    } else {
        HueStatusError::JsonError { source: err }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_codes() {
        assert_eq!(HueStatusError::ConfigNotFound.exit_code(), 1);
        assert_eq!(HueStatusError::BridgeNotFound.exit_code(), 2);
        assert_eq!(HueStatusError::AuthenticationFailed.exit_code(), 3);
        assert_eq!(
            HueStatusError::SceneNotFound {
                scene_name: "test".to_string()
            }
            .exit_code(),
            4
        );
    }

    #[test]
    fn test_user_messages() {
        let error = HueStatusError::ConfigNotFound;
        assert!(error.user_message().contains("huestatus --setup"));

        let error = HueStatusError::SceneNotFound {
            scene_name: "test-scene".to_string(),
        };
        assert!(error.user_message().contains("test-scene"));
    }

    #[test]
    fn test_error_properties() {
        assert!(HueStatusError::ConfigNotFound.is_recoverable_with_setup());
        assert!(HueStatusError::BridgeNotFound.requires_network());
        assert!(HueStatusError::TimeoutError {
            operation: "test".to_string(),
        }
        .is_retryable());
    }
}
