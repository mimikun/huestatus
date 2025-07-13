use crate::bridge::{BridgeClient, CreateSceneRequest, Light, LightState, Scene};
use crate::config::Config;
use crate::error::{HueStatusError, Result};

pub mod create;
pub mod execute;

pub use create::*;
pub use execute::*;

/// Scene manager for creating and executing status scenes
#[derive(Debug, Clone)]
pub struct SceneManager {
    client: BridgeClient,
    verbose: bool,
}

/// Scene creation result
#[derive(Debug, Clone)]
pub struct SceneCreationResult {
    pub success_scene_id: String,
    pub failure_scene_id: String,
    pub lights_used: Vec<String>,
    pub scenes_created: usize,
}

/// Scene execution result
#[derive(Debug, Clone)]
pub struct SceneExecutionResult {
    pub scene_id: String,
    pub scene_name: String,
    pub execution_time_ms: u64,
    pub success: bool,
}

/// Scene validation result
#[derive(Debug, Clone)]
pub struct SceneValidationResult {
    pub scene_id: String,
    pub scene_name: String,
    pub is_valid: bool,
    pub issues: Vec<String>,
    pub lights_status: Vec<LightStatus>,
}

/// Light status for validation
#[derive(Debug, Clone)]
pub struct LightStatus {
    pub light_id: String,
    pub light_name: String,
    pub is_reachable: bool,
    pub supports_color: bool,
    pub current_state: Option<LightState>,
}

/// Color definitions for status scenes
#[derive(Debug, Clone)]
pub struct StatusColors {
    pub success: ColorDefinition,
    pub failure: ColorDefinition,
}

/// Color definition with multiple formats
#[derive(Debug, Clone)]
pub struct ColorDefinition {
    pub hue: u16,
    pub saturation: u8,
    pub brightness: u8,
    pub xy: Option<[f64; 2]>,
    pub name: String,
}

impl SceneManager {
    /// Create a new scene manager
    pub fn new(client: BridgeClient) -> Self {
        Self {
            client,
            verbose: false,
        }
    }

    /// Enable verbose output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Create status scenes (success and failure)
    pub async fn create_status_scenes(&self, config: &mut Config) -> Result<SceneCreationResult> {
        if self.verbose {
            eprintln!("🎨 Creating status scenes...");
        }

        // Get suitable lights for status indication
        let suitable_lights = self.client.get_suitable_lights().await?;

        if suitable_lights.is_empty() {
            return Err(HueStatusError::NoLightsFound);
        }

        let light_ids: Vec<String> = suitable_lights.iter().map(|(id, _)| id.clone()).collect();

        if self.verbose {
            eprintln!("💡 Using {} lights for status scenes:", light_ids.len());
            for (id, light) in &suitable_lights {
                eprintln!("  - {} ({})", light.name, id);
            }
        }

        // Create success scene (green)
        let success_scene_name = "huestatus-success".to_string();
        let success_scene_request =
            CreateSceneRequest::new_success_scene(success_scene_name.clone(), light_ids.clone());

        let success_response = self.client.create_scene(&success_scene_request).await?;
        let success_scene_id = success_response
            .first()
            .ok_or_else(|| HueStatusError::SceneExecutionFailed {
                reason: "No response from scene creation".to_string(),
            })?
            .success
            .id
            .clone();

        if self.verbose {
            eprintln!(
                "✅ Created success scene: {} ({})",
                success_scene_name, success_scene_id
            );
        }

        // Create failure scene (red)
        let failure_scene_name = "huestatus-failure".to_string();
        let failure_scene_request =
            CreateSceneRequest::new_failure_scene(failure_scene_name.clone(), light_ids.clone());

        let failure_response = self.client.create_scene(&failure_scene_request).await?;
        let failure_scene_id = failure_response
            .first()
            .ok_or_else(|| HueStatusError::SceneExecutionFailed {
                reason: "No response from scene creation".to_string(),
            })?
            .success
            .id
            .clone();

        if self.verbose {
            eprintln!(
                "✅ Created failure scene: {} ({})",
                failure_scene_name, failure_scene_id
            );
        }

        // Update configuration with new scene IDs
        config.scenes.success.id = success_scene_id.clone();
        config.scenes.success.name = success_scene_name;
        config.scenes.success.auto_created = true;
        config.scenes.failure.id = failure_scene_id.clone();
        config.scenes.failure.name = failure_scene_name;
        config.scenes.failure.auto_created = true;

        let result = SceneCreationResult {
            success_scene_id,
            failure_scene_id,
            lights_used: light_ids,
            scenes_created: 2,
        };

        if self.verbose {
            eprintln!("🎉 Scene creation completed successfully!");
        }

        Ok(result)
    }

    /// Execute a status scene
    pub async fn execute_status_scene(
        &self,
        scene_type: &str,
        config: &Config,
    ) -> Result<SceneExecutionResult> {
        let scene_config =
            config
                .get_scene(scene_type)
                .ok_or_else(|| HueStatusError::SceneNotFound {
                    scene_name: scene_type.to_string(),
                })?;

        if self.verbose {
            eprintln!(
                "🎬 Executing {} scene: {} ({})",
                scene_type, scene_config.name, scene_config.id
            );
        }

        let start_time = std::time::Instant::now();

        // Execute the scene
        let response = self.client.execute_scene(&scene_config.id).await?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Check if execution was successful
        let success = !response.is_empty();

        if self.verbose {
            if success {
                eprintln!("✅ Scene executed successfully ({}ms)", execution_time);
            } else {
                eprintln!("❌ Scene execution failed");
            }
        }

        Ok(SceneExecutionResult {
            scene_id: scene_config.id.clone(),
            scene_name: scene_config.name.clone(),
            execution_time_ms: execution_time,
            success,
        })
    }

    /// Validate status scenes
    pub async fn validate_status_scenes(
        &self,
        config: &Config,
    ) -> Result<Vec<SceneValidationResult>> {
        let mut results = Vec::new();

        // Validate success scene
        if let Some(success_scene) = config.get_scene("success") {
            let result = self
                .validate_scene(&success_scene.id, &success_scene.name)
                .await?;
            results.push(result);
        }

        // Validate failure scene
        if let Some(failure_scene) = config.get_scene("failure") {
            let result = self
                .validate_scene(&failure_scene.id, &failure_scene.name)
                .await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Validate a specific scene
    async fn validate_scene(
        &self,
        scene_id: &str,
        scene_name: &str,
    ) -> Result<SceneValidationResult> {
        if self.verbose {
            eprintln!("🔍 Validating scene: {} ({})", scene_name, scene_id);
        }

        let mut issues = Vec::new();
        let mut lights_status = Vec::new();
        let mut is_valid = true;

        // Check if scene exists
        let scene = match self.client.get_scene(scene_id).await {
            Ok(scene) => scene,
            Err(_) => {
                issues.push(format!("Scene '{}' not found", scene_name));
                is_valid = false;
                return Ok(SceneValidationResult {
                    scene_id: scene_id.to_string(),
                    scene_name: scene_name.to_string(),
                    is_valid,
                    issues,
                    lights_status,
                });
            }
        };

        // Check if scene is suitable for status indication
        if !scene.is_suitable_for_status() {
            issues.push("Scene is not suitable for status indication".to_string());
            is_valid = false;
        }

        // Validate lights in scene
        let all_lights = self.client.get_lights().await?;

        for light_id in &scene.lights {
            if let Some(light) = all_lights.get(light_id) {
                let light_status = LightStatus {
                    light_id: light_id.clone(),
                    light_name: light.name.clone(),
                    is_reachable: light.is_reachable(),
                    supports_color: light.supports_color(),
                    current_state: Some(light.state.clone()),
                };

                if !light.is_reachable() {
                    issues.push(format!("Light '{}' is not reachable", light.name));
                    is_valid = false;
                }

                if !light.supports_color() {
                    issues.push(format!("Light '{}' does not support color", light.name));
                }

                lights_status.push(light_status);
            } else {
                issues.push(format!("Light '{}' not found", light_id));
                is_valid = false;
            }
        }

        if self.verbose {
            if is_valid {
                eprintln!("✅ Scene validation passed");
            } else {
                eprintln!("❌ Scene validation failed: {} issues", issues.len());
                for issue in &issues {
                    eprintln!("  - {}", issue);
                }
            }
        }

        Ok(SceneValidationResult {
            scene_id: scene_id.to_string(),
            scene_name: scene_name.to_string(),
            is_valid,
            issues,
            lights_status,
        })
    }

    /// Test scene execution without changing light states
    pub async fn test_scene_execution(&self, scene_id: &str) -> Result<bool> {
        if self.verbose {
            eprintln!("🧪 Testing scene execution: {}", scene_id);
        }

        // Just check if scene exists and can be accessed
        match self.client.get_scene(scene_id).await {
            Ok(_) => {
                if self.verbose {
                    eprintln!("✅ Scene test passed");
                }
                Ok(true)
            }
            Err(e) => {
                if self.verbose {
                    eprintln!("❌ Scene test failed: {}", e);
                }
                Err(e)
            }
        }
    }

    /// Get scene information
    pub async fn get_scene_info(&self, scene_id: &str) -> Result<Scene> {
        self.client.get_scene(scene_id).await
    }

    /// Delete status scenes
    pub async fn delete_status_scenes(&self, config: &Config) -> Result<()> {
        if self.verbose {
            eprintln!("🗑️ Deleting status scenes...");
        }

        // Delete success scene
        if let Some(success_scene) = config.get_scene("success") {
            if success_scene.auto_created {
                match self.client.delete_scene(&success_scene.id).await {
                    Ok(_) => {
                        if self.verbose {
                            eprintln!("✅ Deleted success scene: {}", success_scene.name);
                        }
                    }
                    Err(e) => {
                        if self.verbose {
                            eprintln!("⚠️ Failed to delete success scene: {}", e);
                        }
                    }
                }
            }
        }

        // Delete failure scene
        if let Some(failure_scene) = config.get_scene("failure") {
            if failure_scene.auto_created {
                match self.client.delete_scene(&failure_scene.id).await {
                    Ok(_) => {
                        if self.verbose {
                            eprintln!("✅ Deleted failure scene: {}", failure_scene.name);
                        }
                    }
                    Err(e) => {
                        if self.verbose {
                            eprintln!("⚠️ Failed to delete failure scene: {}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Refresh status scenes (delete and recreate)
    pub async fn refresh_status_scenes(&self, config: &mut Config) -> Result<SceneCreationResult> {
        if self.verbose {
            eprintln!("🔄 Refreshing status scenes...");
        }

        // Delete existing scenes
        self.delete_status_scenes(config).await?;

        // Create new scenes
        self.create_status_scenes(config).await
    }

    /// Get status colors definition
    pub fn get_status_colors() -> StatusColors {
        StatusColors {
            success: ColorDefinition {
                hue: 21845, // Green: 120° × 65536/360°
                saturation: 254,
                brightness: 254,
                xy: Some([0.409, 0.518]), // Green in CIE 1931 color space
                name: "Green".to_string(),
            },
            failure: ColorDefinition {
                hue: 0, // Red: 0°
                saturation: 254,
                brightness: 254,
                xy: Some([0.675, 0.322]), // Red in CIE 1931 color space
                name: "Red".to_string(),
            },
        }
    }

    /// Create custom color scene
    pub async fn create_custom_scene(
        &self,
        name: String,
        lights: Vec<String>,
        color: &ColorDefinition,
    ) -> Result<String> {
        if self.verbose {
            eprintln!("🎨 Creating custom scene: {} ({})", name, color.name);
        }

        let scene_request = CreateSceneRequest::new_custom_scene(
            name,
            lights,
            color.hue,
            color.saturation,
            color.brightness,
        );

        let response = self.client.create_scene(&scene_request).await?;
        let scene_id = response
            .first()
            .ok_or_else(|| HueStatusError::SceneExecutionFailed {
                reason: "No response from scene creation".to_string(),
            })?
            .success
            .id
            .clone();

        if self.verbose {
            eprintln!("✅ Created custom scene: {}", scene_id);
        }

        Ok(scene_id)
    }

    /// Get all available lights suitable for status scenes
    pub async fn get_available_lights(&self) -> Result<Vec<(String, Light)>> {
        self.client.get_suitable_lights().await
    }

    /// Get scene execution history (mock implementation for future extension)
    pub fn get_execution_history(&self) -> Vec<SceneExecutionResult> {
        // This would be implemented with persistent storage in a real application
        Vec::new()
    }
}

impl SceneCreationResult {
    /// Get summary of creation result
    pub fn summary(&self) -> String {
        format!(
            "Created {} scenes using {} lights (Success: {}, Failure: {})",
            self.scenes_created,
            self.lights_used.len(),
            self.success_scene_id,
            self.failure_scene_id
        )
    }

    /// Check if creation was successful
    pub fn is_successful(&self) -> bool {
        self.scenes_created == 2
            && !self.success_scene_id.is_empty()
            && !self.failure_scene_id.is_empty()
    }
}

impl SceneExecutionResult {
    /// Check if execution was fast (under 500ms)
    pub fn is_fast(&self) -> bool {
        self.execution_time_ms < 500
    }

    /// Check if execution was slow (over 2 seconds)
    pub fn is_slow(&self) -> bool {
        self.execution_time_ms > 2000
    }

    /// Get performance rating
    pub fn performance_rating(&self) -> &'static str {
        if !self.success {
            "Failed"
        } else if self.is_fast() {
            "Excellent"
        } else if self.execution_time_ms < 1000 {
            "Good"
        } else if self.execution_time_ms < 2000 {
            "Fair"
        } else {
            "Poor"
        }
    }

    /// Get summary of execution result
    pub fn summary(&self) -> String {
        format!(
            "Scene '{}' executed in {}ms ({})",
            self.scene_name,
            self.execution_time_ms,
            self.performance_rating()
        )
    }
}

impl SceneValidationResult {
    /// Get validation summary
    pub fn summary(&self) -> String {
        if self.is_valid {
            format!("Scene '{}' is valid", self.scene_name)
        } else {
            format!(
                "Scene '{}' has {} issue(s): {}",
                self.scene_name,
                self.issues.len(),
                self.issues.join(", ")
            )
        }
    }

    /// Get reachable lights count
    pub fn reachable_lights_count(&self) -> usize {
        self.lights_status.iter().filter(|l| l.is_reachable).count()
    }

    /// Get color-capable lights count
    pub fn color_capable_lights_count(&self) -> usize {
        self.lights_status
            .iter()
            .filter(|l| l.supports_color)
            .count()
    }
}

impl LightStatus {
    /// Check if light is suitable for status indication
    pub fn is_suitable(&self) -> bool {
        self.is_reachable && self.supports_color
    }

    /// Get status summary
    pub fn summary(&self) -> String {
        let status = if self.is_suitable() {
            "Suitable"
        } else if !self.is_reachable {
            "Unreachable"
        } else if !self.supports_color {
            "No color support"
        } else {
            "Not suitable"
        };

        format!("{} ({}): {}", self.light_name, self.light_id, status)
    }
}

impl ColorDefinition {
    /// Create a new color definition
    pub fn new(name: String, hue: u16, saturation: u8, brightness: u8) -> Self {
        Self {
            hue,
            saturation,
            brightness,
            xy: None,
            name,
        }
    }

    /// Create with XY coordinates
    pub fn with_xy(mut self, xy: [f64; 2]) -> Self {
        self.xy = Some(xy);
        self
    }

    /// Convert to light state
    pub fn to_light_state(&self) -> LightState {
        LightState {
            on: true,
            bri: Some(self.brightness),
            hue: Some(self.hue),
            sat: Some(self.saturation),
            xy: self.xy,
            effect: None,
            ct: None,
            alert: None,
            colormode: Some("hs".to_string()),
            mode: None,
            reachable: None,
        }
    }

    /// Get color summary
    pub fn summary(&self) -> String {
        format!(
            "{}: H:{} S:{} B:{}",
            self.name, self.hue, self.saturation, self.brightness
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would require mocking in a real implementation
    #[test]
    fn test_status_colors() {
        let colors = SceneManager::get_status_colors();

        assert_eq!(colors.success.hue, 21845); // Green
        assert_eq!(colors.failure.hue, 0); // Red
        assert_eq!(colors.success.name, "Green");
        assert_eq!(colors.failure.name, "Red");
    }

    #[test]
    fn test_color_definition() {
        let color = ColorDefinition::new("Blue".to_string(), 43690, 254, 200);

        assert_eq!(color.name, "Blue");
        assert_eq!(color.hue, 43690);
        assert_eq!(color.saturation, 254);
        assert_eq!(color.brightness, 200);

        let light_state = color.to_light_state();
        assert_eq!(light_state.hue, Some(43690));
        assert!(light_state.on);
    }

    #[test]
    fn test_scene_execution_result_performance() {
        let fast_result = SceneExecutionResult {
            scene_id: "test".to_string(),
            scene_name: "Test Scene".to_string(),
            execution_time_ms: 200,
            success: true,
        };

        assert!(fast_result.is_fast());
        assert!(!fast_result.is_slow());
        assert_eq!(fast_result.performance_rating(), "Excellent");

        let slow_result = SceneExecutionResult {
            scene_id: "test".to_string(),
            scene_name: "Test Scene".to_string(),
            execution_time_ms: 3000,
            success: true,
        };

        assert!(!slow_result.is_fast());
        assert!(slow_result.is_slow());
        assert_eq!(slow_result.performance_rating(), "Poor");
    }

    #[test]
    fn test_light_status() {
        let suitable_light = LightStatus {
            light_id: "1".to_string(),
            light_name: "Living Room Light".to_string(),
            is_reachable: true,
            supports_color: true,
            current_state: None,
        };

        assert!(suitable_light.is_suitable());
        assert!(suitable_light.summary().contains("Suitable"));

        let unsuitable_light = LightStatus {
            light_id: "2".to_string(),
            light_name: "Bedroom Light".to_string(),
            is_reachable: false,
            supports_color: true,
            current_state: None,
        };

        assert!(!unsuitable_light.is_suitable());
        assert!(unsuitable_light.summary().contains("Unreachable"));
    }

    #[test]
    fn test_scene_creation_result() {
        let result = SceneCreationResult {
            success_scene_id: "success-123".to_string(),
            failure_scene_id: "failure-456".to_string(),
            lights_used: vec!["1".to_string(), "2".to_string()],
            scenes_created: 2,
        };

        assert!(result.is_successful());
        assert!(result.summary().contains("2 scenes"));
        assert!(result.summary().contains("2 lights"));
    }
}
