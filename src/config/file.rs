use crate::config::Config;
use crate::error::{HueStatusError, Result};
use std::fs;
use std::path::Path;

/// Load configuration from file
pub fn load_config(path: &Path) -> Result<Config> {
    // Check if file exists
    if !path.exists() {
        return Err(HueStatusError::ConfigNotFound);
    }

    // Read file content
    let content = fs::read_to_string(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => HueStatusError::ConfigNotFound,
        std::io::ErrorKind::PermissionDenied => HueStatusError::PermissionDenied {
            reason: format!("Cannot read config file: {}", path.display()),
        },
        _ => HueStatusError::IoError { source: e },
    })?;

    // Parse JSON
    let mut config: Config = serde_json::from_str(&content).map_err(|e| {
        if e.is_syntax() {
            HueStatusError::ConfigCorrupted
        } else {
            HueStatusError::InvalidConfig {
                reason: format!("JSON parsing error: {e}"),
            }
        }
    })?;

    // Check version compatibility
    if !config.version.is_compatible() {
        return Err(HueStatusError::ConfigVersionIncompatible);
    }

    // Migrate if needed
    if config.version.needs_migration() {
        config.migrate()?;
        // Save migrated configuration
        save_config(&config, path)?;
    }

    // Validate configuration
    config.validate()?;

    // Apply environment variable overrides
    config.apply_env_overrides()?;

    Ok(config)
}

/// Save configuration to file
pub fn save_config(config: &Config, path: &Path) -> Result<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_e| HueStatusError::ConfigDirectoryCreationFailed {
            path: parent.display().to_string(),
        })?;
    }

    // Serialize configuration to JSON
    let json = serde_json::to_string_pretty(config).map_err(|e| HueStatusError::InvalidConfig {
        reason: format!("JSON serialization error: {e}"),
    })?;

    // Write to file
    fs::write(path, json).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => HueStatusError::PermissionDenied {
            reason: format!("Cannot write config file: {}", path.display()),
        },
        _ => HueStatusError::IoError { source: e },
    })?;

    // Set secure permissions on Unix systems
    set_secure_permissions(path)?;

    Ok(())
}

/// Set secure file permissions (Unix only)
#[cfg(unix)]
fn set_secure_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(path, permissions).map_err(|e| {
        // Don't fail if we can't set permissions, just warn
        eprintln!(
            "Warning: Could not set secure permissions on config file: {e}"
        );
        HueStatusError::PermissionDenied {
            reason: format!("Cannot set file permissions: {e}"),
        }
    })?;

    Ok(())
}

/// Set secure file permissions (non-Unix systems)
#[cfg(not(unix))]
fn set_secure_permissions(_path: &Path) -> Result<()> {
    // Windows and other systems don't use Unix-style permissions
    Ok(())
}

/// Create a backup of the configuration file
pub fn backup_config(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let backup_path = path.with_extension("json.backup");
    fs::copy(path, &backup_path).map_err(|e| HueStatusError::IoError { source: e })?;

    Ok(())
}

/// Load configuration from custom path or default location
pub fn load_config_from_path_or_default(custom_path: Option<&Path>) -> Result<Config> {
    let path = if let Some(custom) = custom_path {
        custom.to_path_buf()
    } else {
        Config::get_config_file_path()?
    };

    load_config(&path)
}

/// Initialize configuration directory with proper permissions
pub fn init_config_directory() -> Result<()> {
    let config_dir = Config::get_config_dir()?;

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).map_err(|_e| {
            HueStatusError::ConfigDirectoryCreationFailed {
                path: config_dir.display().to_string(),
            }
        })?;
    }

    // Set directory permissions on Unix systems
    set_directory_permissions(&config_dir)?;

    Ok(())
}

/// Set secure directory permissions (Unix only)
#[cfg(unix)]
fn set_directory_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = std::fs::Permissions::from_mode(0o755);
    std::fs::set_permissions(path, permissions).map_err(|e| HueStatusError::PermissionDenied {
        reason: format!("Cannot set directory permissions: {e}"),
    })?;

    Ok(())
}

/// Set secure directory permissions (non-Unix systems)
#[cfg(not(unix))]
fn set_directory_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

/// Check if configuration file has secure permissions
#[cfg(unix)]
pub fn check_config_permissions(path: &Path) -> Result<bool> {
    use std::os::unix::fs::PermissionsExt;

    if !path.exists() {
        return Ok(true); // File doesn't exist, so permissions are irrelevant
    }

    let metadata = fs::metadata(path).map_err(|e| HueStatusError::IoError { source: e })?;
    let permissions = metadata.permissions();
    let mode = permissions.mode();

    // Check if file is readable/writable by owner only (0o600)
    Ok(mode & 0o077 == 0)
}

/// Check if configuration file has secure permissions (non-Unix systems)
#[cfg(not(unix))]
pub fn check_config_permissions(_path: &Path) -> Result<bool> {
    Ok(true) // Always return true for non-Unix systems
}

/// Get configuration file size
pub fn get_config_file_size(path: &Path) -> Result<u64> {
    let metadata = fs::metadata(path).map_err(|e| HueStatusError::IoError { source: e })?;
    Ok(metadata.len())
}

/// Check if configuration file is valid JSON
pub fn validate_config_json(path: &Path) -> Result<()> {
    let content = fs::read_to_string(path).map_err(|e| HueStatusError::IoError { source: e })?;

    // Try to parse as JSON
    serde_json::from_str::<serde_json::Value>(&content).map_err(|e| {
        if e.is_syntax() {
            HueStatusError::ConfigCorrupted
        } else {
            HueStatusError::InvalidConfig {
                reason: format!("JSON validation error: {e}"),
            }
        }
    })?;

    Ok(())
}

/// Remove configuration file
pub fn remove_config(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path).map_err(|e| HueStatusError::IoError { source: e })?;
    }
    Ok(())
}

/// Get configuration file last modified time
pub fn get_config_last_modified(path: &Path) -> Result<std::time::SystemTime> {
    let metadata = fs::metadata(path).map_err(|e| HueStatusError::IoError { source: e })?;
    Ok(metadata.modified().unwrap_or(std::time::UNIX_EPOCH))
}

/// Check if configuration file is writable
pub fn is_config_writable(path: &Path) -> bool {
    if !path.exists() {
        // Check if parent directory is writable
        if let Some(parent) = path.parent() {
            parent.exists()
                && fs::metadata(parent)
                    .map(|m| !m.permissions().readonly())
                    .unwrap_or(false)
        } else {
            false
        }
    } else {
        // Check if file is writable
        fs::metadata(path)
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use tempfile::NamedTempFile;

    #[test]
    fn test_save_and_load_config() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(
            "192.168.1.100".to_string(),
            "test-application-key-with-proper-length".to_string(),
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

        // Save configuration
        save_config(&config, temp_file.path()).unwrap();

        // Load configuration
        let loaded = load_config(temp_file.path()).unwrap();

        assert_eq!(config.bridge.ip, loaded.bridge.ip);
        assert_eq!(config.bridge.application_key, loaded.bridge.application_key);
        assert_eq!(config.scenes.success.id, loaded.scenes.success.id);
        assert_eq!(config.scenes.failure.id, loaded.scenes.failure.id);
    }

    #[test]
    fn test_config_not_found() {
        let result = load_config(Path::new("/nonexistent/path/config.json"));
        assert!(matches!(result, Err(HueStatusError::ConfigNotFound)));
    }

    #[test]
    fn test_config_file_size() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(
            "192.168.1.100".to_string(),
            "test-application-key-with-proper-length".to_string(),
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

        save_config(&config, temp_file.path()).unwrap();
        let size = get_config_file_size(temp_file.path()).unwrap();
        assert!(size > 0);
    }

    #[test]
    fn test_validate_config_json() {
        let temp_file = NamedTempFile::new().unwrap();

        // Write invalid JSON
        fs::write(temp_file.path(), "invalid json").unwrap();
        let result = validate_config_json(temp_file.path());
        assert!(matches!(result, Err(HueStatusError::ConfigCorrupted)));

        // Write valid JSON
        fs::write(temp_file.path(), r#"{"valid": "json"}"#).unwrap();
        let result = validate_config_json(temp_file.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_writable() {
        let temp_file = NamedTempFile::new().unwrap();
        assert!(is_config_writable(temp_file.path()));
    }

    #[test]
    fn test_backup_config() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(
            "192.168.1.100".to_string(),
            "test-application-key-with-proper-length".to_string(),
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

        save_config(&config, temp_file.path()).unwrap();
        backup_config(temp_file.path()).unwrap();

        let backup_path = temp_file.path().with_extension("json.backup");
        assert!(backup_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_secure_permissions() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::new(
            "192.168.1.100".to_string(),
            "test-application-key-with-proper-length".to_string(),
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

        save_config(&config, temp_file.path()).unwrap();
        assert!(check_config_permissions(temp_file.path()).unwrap());
    }
}
