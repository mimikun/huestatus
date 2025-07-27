use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod file;
pub mod validation;

pub use file::*;
pub use validation::*;

/// Maximum allowed path length to prevent capacity overflow
const MAX_PATH_LENGTH: usize = 4096;

/// Fallback configuration directory name
const FALLBACK_CONFIG_NAME: &str = "huestatus-config";

/// Configuration file version for future compatibility
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum ConfigVersion {
    #[serde(rename = "1.0")]
    V1_0,
    #[serde(rename = "1.1")]
    V1_1,
    #[serde(rename = "1.2")]
    #[default]
    V1_2,
}

impl ConfigVersion {
    /// Check if version is compatible with current version
    pub fn is_compatible(&self) -> bool {
        matches!(
            self,
            ConfigVersion::V1_0 | ConfigVersion::V1_1 | ConfigVersion::V1_2
        )
    }

    /// Check if version needs migration
    pub fn needs_migration(&self) -> bool {
        !matches!(self, ConfigVersion::V1_2)
    }
}

/// Bridge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    /// Bridge IP address
    pub ip: String,
    /// Application key for authentication
    pub application_key: String,
    /// Last time bridge connection was verified
    pub last_verified: DateTime<Utc>,
    /// Cached bridge capabilities
    #[serde(default)]
    pub capabilities_cache: Option<CapabilitiesCache>,
}

/// Cached bridge capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesCache {
    /// Maximum number of scenes bridge can store
    pub max_scenes: usize,
    /// Number of lights connected to bridge
    pub lights_count: usize,
    /// When capabilities were last cached
    pub cached_at: DateTime<Utc>,
}

/// Scene configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneConfig {
    /// Scene ID assigned by bridge
    pub id: String,
    /// Scene name
    pub name: String,
    /// Whether scene was auto-created by huestatus
    pub auto_created: bool,
    /// Last time scene was validated
    #[serde(default)]
    pub last_validated: Option<DateTime<Utc>>,
}

/// All configured scenes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenesConfig {
    /// Success scene configuration
    pub success: SceneConfig,
    /// Failure scene configuration
    pub failure: SceneConfig,
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// API timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Number of retry attempts
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: usize,
    /// Retry delay in seconds
    #[serde(default = "default_retry_delay")]
    pub retry_delay_seconds: u64,
    /// Enable verbose logging
    #[serde(default)]
    pub verbose_logging: bool,
    /// Enable quiet mode
    #[serde(default)]
    pub quiet_mode: bool,
    /// Automatically refresh scenes if they don't exist
    #[serde(default = "default_auto_refresh")]
    pub auto_refresh_scenes: bool,
    /// Validate scenes on startup
    #[serde(default)]
    pub validate_scenes_on_startup: bool,
}

/// Advanced settings for performance optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSettings {
    /// HTTP connection pool size
    #[serde(default = "default_pool_size")]
    pub connection_pool_size: usize,
    /// Cache duration in minutes
    #[serde(default = "default_cache_duration")]
    pub cache_duration_minutes: u64,
    /// Scene validation interval in hours
    #[serde(default = "default_validation_interval")]
    pub scene_validation_interval_hours: u64,
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Configuration version
    #[serde(default)]
    pub version: ConfigVersion,
    /// Bridge configuration
    pub bridge: BridgeConfig,
    /// Scene configurations
    pub scenes: ScenesConfig,
    /// Application settings
    #[serde(default)]
    pub settings: Settings,
    /// Advanced settings
    #[serde(default)]
    pub advanced: AdvancedSettings,
}

// Default value functions
fn default_timeout() -> u64 {
    10
}

fn default_retry_attempts() -> usize {
    3
}

fn default_retry_delay() -> u64 {
    1
}

fn default_auto_refresh() -> bool {
    true
}

fn default_pool_size() -> usize {
    5
}

fn default_cache_duration() -> u64 {
    30
}

fn default_validation_interval() -> u64 {
    24
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            timeout_seconds: default_timeout(),
            retry_attempts: default_retry_attempts(),
            retry_delay_seconds: default_retry_delay(),
            verbose_logging: false,
            quiet_mode: false,
            auto_refresh_scenes: default_auto_refresh(),
            validate_scenes_on_startup: false,
        }
    }
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        AdvancedSettings {
            connection_pool_size: default_pool_size(),
            cache_duration_minutes: default_cache_duration(),
            scene_validation_interval_hours: default_validation_interval(),
        }
    }
}

impl Config {
    /// Create a new configuration
    pub fn new(
        bridge_ip: String,
        application_key: String,
        success_scene: SceneConfig,
        failure_scene: SceneConfig,
    ) -> Self {
        Config {
            version: ConfigVersion::default(),
            bridge: BridgeConfig {
                ip: bridge_ip,
                application_key,
                last_verified: Utc::now(),
                capabilities_cache: None,
            },
            scenes: ScenesConfig {
                success: success_scene,
                failure: failure_scene,
            },
            settings: Settings::default(),
            advanced: AdvancedSettings::default(),
        }
    }

    /// Get configuration directory path
    pub fn get_config_dir() -> crate::error::Result<PathBuf> {
        dirs::config_dir()
            .ok_or_else(|| crate::error::HueStatusError::ConfigNotFound)?
            .join("huestatus")
            .pipe(Ok)
    }

    /// Get configuration file path
    pub fn get_config_file_path() -> crate::error::Result<PathBuf> {
        Self::get_config_dir().map(|dir| dir.join("config.json"))
    }

    /// Check if configuration file exists
    pub fn exists() -> bool {
        Self::get_config_file_path()
            .map(|path| path.exists())
            .unwrap_or(false)
    }

    /// Load configuration from file
    pub fn load() -> crate::error::Result<Self> {
        let config_path = Self::get_config_file_path()?;
        file::load_config(&config_path)
    }

    /// Save configuration to file
    pub fn save(&self) -> crate::error::Result<()> {
        let config_path = Self::get_config_file_path()?;
        file::save_config(self, &config_path)
    }

    /// Update bridge verification timestamp
    pub fn update_last_verified(&mut self) {
        self.bridge.last_verified = Utc::now();
    }

    /// Update capabilities cache
    pub fn update_capabilities_cache(&mut self, max_scenes: usize, lights_count: usize) {
        self.bridge.capabilities_cache = Some(CapabilitiesCache {
            max_scenes,
            lights_count,
            cached_at: Utc::now(),
        });
    }

    /// Update scene validation timestamp
    pub fn update_scene_validation(&mut self, scene_type: &str) {
        let now = Some(Utc::now());
        match scene_type {
            "success" => self.scenes.success.last_validated = now,
            "failure" => self.scenes.failure.last_validated = now,
            _ => {}
        }
    }

    /// Check if bridge verification is stale
    pub fn is_bridge_verification_stale(&self) -> bool {
        let now = Utc::now();
        let stale_duration =
            chrono::Duration::hours(self.advanced.scene_validation_interval_hours as i64);
        now.signed_duration_since(self.bridge.last_verified) > stale_duration
    }

    /// Check if capabilities cache is stale
    pub fn is_capabilities_cache_stale(&self) -> bool {
        if let Some(cache) = &self.bridge.capabilities_cache {
            let now = Utc::now();
            let stale_duration =
                chrono::Duration::minutes(self.advanced.cache_duration_minutes as i64);
            now.signed_duration_since(cache.cached_at) > stale_duration
        } else {
            true
        }
    }

    /// Check if scene validation is stale
    pub fn is_scene_validation_stale(&self, scene_type: &str) -> bool {
        let last_validated = match scene_type {
            "success" => self.scenes.success.last_validated,
            "failure" => self.scenes.failure.last_validated,
            _ => return true,
        };

        if let Some(validated) = last_validated {
            let now = Utc::now();
            let stale_duration =
                chrono::Duration::hours(self.advanced.scene_validation_interval_hours as i64);
            now.signed_duration_since(validated) > stale_duration
        } else {
            true
        }
    }

    /// Migrate configuration to latest version
    pub fn migrate(&mut self) -> crate::error::Result<()> {
        if !self.version.needs_migration() {
            return Ok(());
        }

        match self.version {
            ConfigVersion::V1_0 => {
                // Migrate from v1.0 to v1.1
                // Add default retry settings if missing
                if self.settings.retry_attempts == 0 {
                    self.settings.retry_attempts = default_retry_attempts();
                }
                if self.settings.retry_delay_seconds == 0 {
                    self.settings.retry_delay_seconds = default_retry_delay();
                }
                self.version = ConfigVersion::V1_1;
            }
            ConfigVersion::V1_1 => {
                // Migrate from v1.1 to v1.2
                // Add advanced settings
                self.advanced = AdvancedSettings::default();
                self.version = ConfigVersion::V1_2;
            }
            ConfigVersion::V1_2 => {
                // Current version, no migration needed
            }
        }

        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> crate::error::Result<()> {
        validation::validate_config(self)
    }

    /// Get scene configuration by type
    pub fn get_scene(&self, scene_type: &str) -> Option<&SceneConfig> {
        match scene_type {
            "success" => Some(&self.scenes.success),
            "failure" => Some(&self.scenes.failure),
            _ => None,
        }
    }

    /// Get mutable scene configuration by type
    pub fn get_scene_mut(&mut self, scene_type: &str) -> Option<&mut SceneConfig> {
        match scene_type {
            "success" => Some(&mut self.scenes.success),
            "failure" => Some(&mut self.scenes.failure),
            _ => None,
        }
    }

    /// Create scene configuration
    pub fn create_scene_config(id: String, name: String, auto_created: bool) -> SceneConfig {
        SceneConfig {
            id,
            name,
            auto_created,
            last_validated: None,
        }
    }

    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) -> crate::error::Result<()> {
        use std::env;

        // Override bridge IP
        if let Ok(bridge_ip) = env::var("HUESTATUS_BRIDGE_IP") {
            self.bridge.ip = bridge_ip;
        }

        // Override timeout
        if let Ok(timeout) = env::var("HUESTATUS_TIMEOUT") {
            self.settings.timeout_seconds = timeout.parse().map_err(|_| {
                crate::error::HueStatusError::EnvironmentVariableError {
                    var_name: "HUESTATUS_TIMEOUT".to_string(),
                }
            })?;
        }

        // Override verbose mode
        if let Ok(verbose) = env::var("HUESTATUS_VERBOSE") {
            self.settings.verbose_logging = verbose.parse().unwrap_or(false);
        }

        // Override quiet mode
        if let Ok(quiet) = env::var("HUESTATUS_QUIET") {
            self.settings.quiet_mode = quiet.parse().unwrap_or(false);
        }

        Ok(())
    }

    /// Get effective timeout considering environment variables
    pub fn effective_timeout(&self) -> u64 {
        std::env::var("HUESTATUS_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(self.settings.timeout_seconds)
    }

    /// Get effective verbose mode considering environment variables
    pub fn effective_verbose(&self) -> bool {
        std::env::var("HUESTATUS_VERBOSE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(self.settings.verbose_logging)
    }

    /// Get effective quiet mode considering environment variables
    pub fn effective_quiet(&self) -> bool {
        std::env::var("HUESTATUS_QUIET")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(self.settings.quiet_mode)
    }
}

/// Validate path length to prevent capacity overflow
pub fn validate_path_length(path: &Path) -> crate::error::Result<()> {
    let path_str = path.to_string_lossy();
    if path_str.len() > MAX_PATH_LENGTH {
        let truncated_path = if path_str.len() > 100 {
            format!("{}...{}", &path_str[..50], &path_str[path_str.len() - 47..])
        } else {
            path_str.to_string()
        };

        Err(crate::error::HueStatusError::PathTooLong {
            path: truncated_path,
        })
    } else {
        Ok(())
    }
}

/// Safely convert path to string with validation
pub fn safe_path_to_string(
    path_result: crate::error::Result<PathBuf>,
) -> crate::error::Result<String> {
    match path_result {
        Ok(path) => {
            validate_path_length(&path)?;
            let path_str = path.to_string_lossy();
            Ok(path_str.into_owned())
        }
        Err(err) => Err(err),
    }
}

/// Safe fallback for path to string conversion that never panics
pub fn safe_path_to_string_fallback(path_result: crate::error::Result<PathBuf>) -> String {
    match safe_path_to_string(path_result) {
        Ok(path_str) => path_str,
        Err(crate::error::HueStatusError::PathTooLong { .. }) => {
            format!("{FALLBACK_CONFIG_NAME}-fallback")
        }
        Err(_) => "unknown".to_string(),
    }
}

// Helper trait for ergonomic method chaining
trait Pipe<T> {
    fn pipe<U, F: FnOnce(T) -> U>(self, f: F) -> U;
}

impl<T> Pipe<T> for T {
    fn pipe<U, F: FnOnce(T) -> U>(self, f: F) -> U {
        f(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_version_compatibility() {
        assert!(ConfigVersion::V1_0.is_compatible());
        assert!(ConfigVersion::V1_1.is_compatible());
        assert!(ConfigVersion::V1_2.is_compatible());
    }

    #[test]
    fn test_config_version_migration() {
        assert!(ConfigVersion::V1_0.needs_migration());
        assert!(ConfigVersion::V1_1.needs_migration());
        assert!(!ConfigVersion::V1_2.needs_migration());
    }

    #[test]
    fn test_config_creation() {
        let config = Config::new(
            "192.168.1.100".to_string(),
            "test-key".to_string(),
            Config::create_scene_config(
                "success-id".to_string(),
                "success-scene".to_string(),
                true,
            ),
            Config::create_scene_config(
                "failure-id".to_string(),
                "failure-scene".to_string(),
                true,
            ),
        );

        assert_eq!(config.bridge.ip, "192.168.1.100");
        assert_eq!(config.bridge.application_key, "test-key");
        assert_eq!(config.scenes.success.id, "success-id");
        assert_eq!(config.scenes.failure.id, "failure-id");
    }

    #[test]
    fn test_scene_getters() {
        let config = Config::new(
            "192.168.1.100".to_string(),
            "test-key".to_string(),
            Config::create_scene_config(
                "success-id".to_string(),
                "success-scene".to_string(),
                true,
            ),
            Config::create_scene_config(
                "failure-id".to_string(),
                "failure-scene".to_string(),
                true,
            ),
        );

        assert!(config.get_scene("success").is_some());
        assert!(config.get_scene("failure").is_some());
        assert!(config.get_scene("invalid").is_none());
    }

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.timeout_seconds, 10);
        assert_eq!(settings.retry_attempts, 3);
        assert_eq!(settings.retry_delay_seconds, 1);
        assert!(!settings.verbose_logging);
        assert!(!settings.quiet_mode);
        assert!(settings.auto_refresh_scenes);
        assert!(!settings.validate_scenes_on_startup);
    }

    #[test]
    fn test_advanced_settings() {
        let advanced = AdvancedSettings::default();
        assert_eq!(advanced.connection_pool_size, 5);
        assert_eq!(advanced.cache_duration_minutes, 30);
        assert_eq!(advanced.scene_validation_interval_hours, 24);
    }

    #[test]
    fn test_path_length_validation() {
        // Test normal length path
        let normal_path = PathBuf::from("normal/path/config.json");
        assert!(validate_path_length(&normal_path).is_ok());

        // Test extremely long path (exceed MAX_PATH_LENGTH of 4096)
        let long_dir = "a".repeat(4200); // Longer than MAX_PATH_LENGTH
        let long_path = PathBuf::from(format!("{}/config.json", long_dir));
        assert!(matches!(
            validate_path_length(&long_path),
            Err(crate::error::HueStatusError::PathTooLong { .. })
        ));
    }

    #[test]
    fn test_safe_path_to_string() {
        // Test successful conversion
        let normal_path = PathBuf::from("normal/path/config.json");
        let result = safe_path_to_string(Ok(normal_path));
        assert!(result.is_ok());

        // Test error propagation
        let error_result = safe_path_to_string(Err(crate::error::HueStatusError::ConfigNotFound));
        assert!(error_result.is_err());
    }

    #[test]
    fn test_safe_path_to_string_fallback() {
        // Test normal path
        let normal_path = PathBuf::from("normal/path/config.json");
        let result = safe_path_to_string_fallback(Ok(normal_path));
        assert!(result.contains("normal"));

        // Test fallback for error
        let result =
            safe_path_to_string_fallback(Err(crate::error::HueStatusError::ConfigNotFound));
        assert_eq!(result, "unknown");

        // Test fallback for path too long
        let long_dir = "a".repeat(4200); // Longer than MAX_PATH_LENGTH
        let long_path = PathBuf::from(format!("{}/config.json", long_dir));
        let result = safe_path_to_string_fallback(Ok(long_path));
        assert_eq!(result, format!("{}-fallback", FALLBACK_CONFIG_NAME));
    }
}
