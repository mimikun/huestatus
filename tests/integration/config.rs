use huestatus::config::Config;
use tempfile::NamedTempFile;

/// Test configuration serialization and deserialization
#[test]
fn test_config_roundtrip() {
    let temp_file = NamedTempFile::new().unwrap();
    
    let original_config = Config::new(
        "192.168.1.100".to_string(),
        "test-application-key".to_string(),
        Config::create_scene_config(
            "success-scene-id".to_string(),
            "huestatus-success".to_string(),
            true,
        ),
        Config::create_scene_config(
            "failure-scene-id".to_string(),
            "huestatus-failure".to_string(),
            true,
        ),
    );

    // Save configuration
    huestatus::config::file::save_config(&original_config, temp_file.path()).unwrap();

    // Load configuration
    let loaded_config = huestatus::config::file::load_config(temp_file.path()).unwrap();

    // Verify configuration matches
    assert_eq!(original_config.bridge.ip, loaded_config.bridge.ip);
    assert_eq!(original_config.bridge.application_key, loaded_config.bridge.application_key);
    assert_eq!(original_config.scenes.success.id, loaded_config.scenes.success.id);
    assert_eq!(original_config.scenes.failure.id, loaded_config.scenes.failure.id);
}

/// Test configuration validation
#[test]
fn test_config_validation() {
    let valid_config = Config::new(
        "192.168.1.100".to_string(),
        "test-application-key".to_string(),
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

    assert!(valid_config.validate().is_ok());
}

/// Test environment variable overrides
#[test]
fn test_env_overrides() {
    let mut config = Config::new(
        "192.168.1.100".to_string(),
        "test-key".to_string(),
        Config::create_scene_config("s1".to_string(), "success".to_string(), true),
        Config::create_scene_config("f1".to_string(), "failure".to_string(), true),
    );

    // Set environment variables
    std::env::set_var("HUESTATUS_BRIDGE_IP", "192.168.1.200");
    std::env::set_var("HUESTATUS_TIMEOUT", "15");
    std::env::set_var("HUESTATUS_VERBOSE", "true");

    config.apply_env_overrides().unwrap();

    assert_eq!(config.bridge.ip, "192.168.1.200");
    assert_eq!(config.settings.timeout_seconds, 15);
    assert!(config.settings.verbose_logging);

    // Clean up environment variables
    std::env::remove_var("HUESTATUS_BRIDGE_IP");
    std::env::remove_var("HUESTATUS_TIMEOUT");
    std::env::remove_var("HUESTATUS_VERBOSE");
}