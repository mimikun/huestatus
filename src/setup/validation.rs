use crate::bridge::BridgeClient;
use crate::config::Config;
use crate::error::Result;

/// Validation checks for setup
pub struct SetupValidator {
    #[allow(dead_code)]
    verbose: bool,
}

impl SetupValidator {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    /// Validate complete setup
    pub async fn validate_setup(&self, config: &Config) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Validate configuration
        config.validate()?;

        // Test bridge connection
        let client = BridgeClient::new(config.bridge.ip.clone())?
            .with_username(config.bridge.application_key.clone());

        if let Err(e) = client.test_connection().await {
            warnings.push(format!("Bridge connection issue: {}", e));
        }

        // Check lights
        match client.get_suitable_lights().await {
            Ok(lights) => {
                if lights.is_empty() {
                    warnings.push("No suitable lights found for status indication".to_string());
                } else if lights.len() < 2 {
                    warnings.push("Only one suitable light found - consider adding more for better visibility".to_string());
                }
            }
            Err(e) => warnings.push(format!("Failed to get lights: {}", e)),
        }

        // Validate scenes
        for scene_type in &["success", "failure"] {
            if let Some(scene_config) = config.get_scene(scene_type) {
                if let Err(e) = client.get_scene(&scene_config.id).await {
                    warnings.push(format!("Scene '{}' not found: {}", scene_config.name, e));
                }
            }
        }

        Ok(warnings)
    }

    /// Check bridge capabilities
    pub async fn check_bridge_capabilities(&self, client: &BridgeClient) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        match client.get_capabilities().await {
            Ok(capabilities) => {
                if capabilities.scenes.available < 10 {
                    warnings.push("Bridge is running low on scene storage".to_string());
                }

                if capabilities.lights.total < 2 {
                    warnings.push("Bridge has very few lights connected".to_string());
                }
            }
            Err(e) => warnings.push(format!("Could not check bridge capabilities: {}", e)),
        }

        Ok(warnings)
    }
}
