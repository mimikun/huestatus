use crate::config::Config;
use crate::error::{HueStatusError, Result};
use std::net::IpAddr;
use std::str::FromStr;

/// Validate complete configuration
pub fn validate_config(config: &Config) -> Result<()> {
    // Validate version compatibility
    if !config.version.is_compatible() {
        return Err(HueStatusError::ConfigVersionIncompatible);
    }

    // Validate bridge configuration
    validate_bridge_config(config)?;

    // Validate scenes configuration
    validate_scenes_config(config)?;

    // Validate settings
    validate_settings(config)?;

    // Validate advanced settings
    validate_advanced_settings(config)?;

    Ok(())
}

/// Validate bridge configuration
fn validate_bridge_config(config: &Config) -> Result<()> {
    // Validate IP address
    if config.bridge.ip.is_empty() {
        return Err(HueStatusError::InvalidConfig {
            reason: "Bridge IP address is empty".to_string(),
        });
    }

    // Try to parse as IP address
    if IpAddr::from_str(&config.bridge.ip).is_err() {
        return Err(HueStatusError::InvalidConfig {
            reason: format!("Invalid bridge IP address: {}", config.bridge.ip),
        });
    }

    // Validate application key
    if config.bridge.application_key.is_empty() {
        return Err(HueStatusError::InvalidConfig {
            reason: "Application key is empty".to_string(),
        });
    }

    // Application key should be reasonably long (Hue keys are typically 40 characters)
    if config.bridge.application_key.len() < 10 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Application key is too short".to_string(),
        });
    }

    // Validate capabilities cache if present
    if let Some(capabilities) = &config.bridge.capabilities_cache {
        if capabilities.max_scenes == 0 {
            return Err(HueStatusError::InvalidConfig {
                reason: "Capabilities cache has zero max_scenes".to_string(),
            });
        }
    }

    Ok(())
}

/// Validate scenes configuration
fn validate_scenes_config(config: &Config) -> Result<()> {
    // Validate success scene
    validate_scene_config(&config.scenes.success, "success")?;

    // Validate failure scene
    validate_scene_config(&config.scenes.failure, "failure")?;

    // Ensure scene IDs are different
    if config.scenes.success.id == config.scenes.failure.id {
        return Err(HueStatusError::InvalidConfig {
            reason: "Success and failure scenes have the same ID".to_string(),
        });
    }

    // Ensure scene names are different
    if config.scenes.success.name == config.scenes.failure.name {
        return Err(HueStatusError::InvalidConfig {
            reason: "Success and failure scenes have the same name".to_string(),
        });
    }

    Ok(())
}

/// Validate individual scene configuration
fn validate_scene_config(scene: &crate::config::SceneConfig, scene_type: &str) -> Result<()> {
    // Validate scene ID
    if scene.id.is_empty() {
        return Err(HueStatusError::InvalidConfig {
            reason: format!("{} scene ID is empty", scene_type),
        });
    }

    // Validate scene name
    if scene.name.is_empty() {
        return Err(HueStatusError::InvalidConfig {
            reason: format!("{} scene name is empty", scene_type),
        });
    }

    // Scene name should not contain invalid characters
    if scene.name.contains('\n') || scene.name.contains('\r') || scene.name.contains('\t') {
        return Err(HueStatusError::InvalidConfig {
            reason: format!("{} scene name contains invalid characters", scene_type),
        });
    }

    // Scene name should not be too long (Hue bridge limit is 32 characters)
    if scene.name.len() > 32 {
        return Err(HueStatusError::InvalidConfig {
            reason: format!("{} scene name is too long (max 32 characters)", scene_type),
        });
    }

    Ok(())
}

/// Validate settings configuration
fn validate_settings(config: &Config) -> Result<()> {
    // Validate timeout
    if config.settings.timeout_seconds == 0 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Timeout cannot be zero".to_string(),
        });
    }

    if config.settings.timeout_seconds > 300 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Timeout is too large (max 300 seconds)".to_string(),
        });
    }

    // Validate retry attempts
    if config.settings.retry_attempts == 0 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Retry attempts cannot be zero".to_string(),
        });
    }

    if config.settings.retry_attempts > 10 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Too many retry attempts (max 10)".to_string(),
        });
    }

    // Validate retry delay
    if config.settings.retry_delay_seconds == 0 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Retry delay cannot be zero".to_string(),
        });
    }

    if config.settings.retry_delay_seconds > 60 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Retry delay is too large (max 60 seconds)".to_string(),
        });
    }

    // Validate conflicting settings
    if config.settings.verbose_logging && config.settings.quiet_mode {
        return Err(HueStatusError::InvalidConfig {
            reason: "Cannot enable both verbose logging and quiet mode".to_string(),
        });
    }

    Ok(())
}

/// Validate advanced settings configuration
fn validate_advanced_settings(config: &Config) -> Result<()> {
    // Validate connection pool size
    if config.advanced.connection_pool_size == 0 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Connection pool size cannot be zero".to_string(),
        });
    }

    if config.advanced.connection_pool_size > 100 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Connection pool size is too large (max 100)".to_string(),
        });
    }

    // Validate cache duration
    if config.advanced.cache_duration_minutes == 0 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Cache duration cannot be zero".to_string(),
        });
    }

    if config.advanced.cache_duration_minutes > 1440 {
        // 24 hours
        return Err(HueStatusError::InvalidConfig {
            reason: "Cache duration is too large (max 24 hours)".to_string(),
        });
    }

    // Validate validation interval
    if config.advanced.scene_validation_interval_hours == 0 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Scene validation interval cannot be zero".to_string(),
        });
    }

    if config.advanced.scene_validation_interval_hours > 8760 {
        // 365 days
        return Err(HueStatusError::InvalidConfig {
            reason: "Scene validation interval is too large (max 365 days)".to_string(),
        });
    }

    Ok(())
}

/// Validate IP address format
pub fn validate_ip_address(ip: &str) -> Result<()> {
    IpAddr::from_str(ip).map_err(|_| HueStatusError::InvalidConfig {
        reason: format!("Invalid IP address: {}", ip),
    })?;

    Ok(())
}

/// Validate scene name format
pub fn validate_scene_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(HueStatusError::InvalidConfig {
            reason: "Scene name cannot be empty".to_string(),
        });
    }

    if name.len() > 32 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Scene name is too long (max 32 characters)".to_string(),
        });
    }

    if name.contains('\n') || name.contains('\r') || name.contains('\t') {
        return Err(HueStatusError::InvalidConfig {
            reason: "Scene name contains invalid characters".to_string(),
        });
    }

    // Check for reserved names
    if name.starts_with("hue_") || name.starts_with("Hue_") {
        return Err(HueStatusError::InvalidConfig {
            reason: "Scene name cannot start with 'hue_' or 'Hue_' (reserved)".to_string(),
        });
    }

    Ok(())
}

/// Validate application key format
pub fn validate_application_key(key: &str) -> Result<()> {
    if key.is_empty() {
        return Err(HueStatusError::InvalidConfig {
            reason: "Application key cannot be empty".to_string(),
        });
    }

    if key.len() < 10 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Application key is too short (min 10 characters)".to_string(),
        });
    }

    if key.len() > 100 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Application key is too long (max 100 characters)".to_string(),
        });
    }

    // Application key should only contain alphanumeric characters and hyphens
    if !key.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(HueStatusError::InvalidConfig {
            reason: "Application key contains invalid characters".to_string(),
        });
    }

    Ok(())
}

/// Validate timeout value
pub fn validate_timeout(timeout: u64) -> Result<()> {
    if timeout == 0 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Timeout cannot be zero".to_string(),
        });
    }

    if timeout > 300 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Timeout is too large (max 300 seconds)".to_string(),
        });
    }

    Ok(())
}

/// Validate retry attempts
pub fn validate_retry_attempts(attempts: usize) -> Result<()> {
    if attempts == 0 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Retry attempts cannot be zero".to_string(),
        });
    }

    if attempts > 10 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Too many retry attempts (max 10)".to_string(),
        });
    }

    Ok(())
}

/// Validate retry delay
pub fn validate_retry_delay(delay: u64) -> Result<()> {
    if delay == 0 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Retry delay cannot be zero".to_string(),
        });
    }

    if delay > 60 {
        return Err(HueStatusError::InvalidConfig {
            reason: "Retry delay is too large (max 60 seconds)".to_string(),
        });
    }

    Ok(())
}

/// Check if configuration has reasonable defaults
pub fn has_reasonable_defaults(config: &Config) -> bool {
    // Check if timeout is reasonable (5-30 seconds)
    if config.settings.timeout_seconds < 5 || config.settings.timeout_seconds > 30 {
        return false;
    }

    // Check if retry attempts are reasonable (1-5)
    if config.settings.retry_attempts < 1 || config.settings.retry_attempts > 5 {
        return false;
    }

    // Check if retry delay is reasonable (1-10 seconds)
    if config.settings.retry_delay_seconds < 1 || config.settings.retry_delay_seconds > 10 {
        return false;
    }

    // Check if advanced settings are reasonable
    if config.advanced.connection_pool_size < 1 || config.advanced.connection_pool_size > 20 {
        return false;
    }

    if config.advanced.cache_duration_minutes < 10 || config.advanced.cache_duration_minutes > 240 {
        return false;
    }

    if config.advanced.scene_validation_interval_hours < 1
        || config.advanced.scene_validation_interval_hours > 168
    {
        return false;
    }

    true
}

/// Get configuration health score (0-100)
pub fn get_config_health_score(config: &Config) -> u8 {
    let mut score = 100u8;

    // Deduct points for non-optimal settings
    if config.settings.timeout_seconds < 5 || config.settings.timeout_seconds > 30 {
        score = score.saturating_sub(10);
    }

    if config.settings.retry_attempts < 2 || config.settings.retry_attempts > 5 {
        score = score.saturating_sub(10);
    }

    if config.settings.retry_delay_seconds < 1 || config.settings.retry_delay_seconds > 5 {
        score = score.saturating_sub(10);
    }

    // Deduct points for missing capabilities cache
    if config.bridge.capabilities_cache.is_none() {
        score = score.saturating_sub(15);
    }

    // Deduct points for stale verification
    if config.is_bridge_verification_stale() {
        score = score.saturating_sub(20);
    }

    // Deduct points for stale capabilities cache
    if config.is_capabilities_cache_stale() {
        score = score.saturating_sub(10);
    }

    // Deduct points for conflicting settings
    if config.settings.verbose_logging && config.settings.quiet_mode {
        score = score.saturating_sub(25);
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_validate_ip_address() {
        assert!(validate_ip_address("192.168.1.1").is_ok());
        assert!(validate_ip_address("127.0.0.1").is_ok());
        assert!(validate_ip_address("::1").is_ok());
        assert!(validate_ip_address("invalid").is_err());
        assert!(validate_ip_address("").is_err());
    }

    #[test]
    fn test_validate_scene_name() {
        assert!(validate_scene_name("valid-scene").is_ok());
        assert!(validate_scene_name("Valid Scene").is_ok());
        assert!(validate_scene_name("").is_err());
        assert!(validate_scene_name("a".repeat(33).as_str()).is_err());
        assert!(validate_scene_name("scene\nwith\nnewlines").is_err());
        assert!(validate_scene_name("hue_reserved").is_err());
    }

    #[test]
    fn test_validate_application_key() {
        assert!(validate_application_key("valid-key-123").is_ok());
        assert!(validate_application_key("").is_err());
        assert!(validate_application_key("short").is_err());
        assert!(validate_application_key("a".repeat(101).as_str()).is_err());
        assert!(validate_application_key("invalid@key").is_err());
    }

    #[test]
    fn test_validate_timeout() {
        assert!(validate_timeout(10).is_ok());
        assert!(validate_timeout(0).is_err());
        assert!(validate_timeout(301).is_err());
    }

    #[test]
    fn test_validate_retry_attempts() {
        assert!(validate_retry_attempts(3).is_ok());
        assert!(validate_retry_attempts(0).is_err());
        assert!(validate_retry_attempts(11).is_err());
    }

    #[test]
    fn test_validate_retry_delay() {
        assert!(validate_retry_delay(1).is_ok());
        assert!(validate_retry_delay(0).is_err());
        assert!(validate_retry_delay(61).is_err());
    }

    #[test]
    fn test_validate_config() {
        let config = Config::new(
            "192.168.1.100".to_string(),
            "valid-application-key".to_string(),
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

        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_has_reasonable_defaults() {
        let config = Config::new(
            "192.168.1.100".to_string(),
            "valid-application-key".to_string(),
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

        assert!(has_reasonable_defaults(&config));
    }

    #[test]
    fn test_config_health_score() {
        let config = Config::new(
            "192.168.1.100".to_string(),
            "valid-application-key".to_string(),
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

        let score = get_config_health_score(&config);
        assert!(score >= 50); // Should have a reasonable score
    }
}
