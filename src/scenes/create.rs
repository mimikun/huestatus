use crate::bridge::{BridgeClient, CreateSceneRequest, Light, LightState};
use crate::error::{HueStatusError, Result};
use crate::scenes::ColorDefinition;
use std::collections::HashMap;

/// Scene creation builder for customizing scene creation
#[derive(Debug, Clone)]
pub struct SceneBuilder {
    name: String,
    lights: Vec<String>,
    color: Option<ColorDefinition>,
    brightness: Option<u8>,
    recyclable: bool,
    custom_states: HashMap<String, LightState>,
}

/// Scene creation options
#[derive(Debug, Clone)]
pub struct SceneCreationOptions {
    pub brightness: u8,
    pub use_xy_color: bool,
    pub validate_lights: bool,
    pub test_execution: bool,
    pub backup_existing: bool,
}

/// Light selection criteria
#[derive(Debug, Clone)]
pub struct LightSelectionCriteria {
    pub require_color_support: bool,
    pub require_reachable: bool,
    pub exclude_light_types: Vec<String>,
    pub include_only_light_types: Vec<String>,
    pub min_brightness_support: Option<u8>,
    pub room_filter: Option<Vec<String>>,
}

impl SceneBuilder {
    /// Create a new scene builder
    pub fn new(name: String) -> Self {
        Self {
            name,
            lights: Vec::new(),
            color: None,
            brightness: None,
            recyclable: true,
            custom_states: HashMap::new(),
        }
    }

    /// Add lights to the scene
    pub fn with_lights(mut self, lights: Vec<String>) -> Self {
        self.lights = lights;
        self
    }

    /// Set the color for all lights
    pub fn with_color(mut self, color: ColorDefinition) -> Self {
        self.color = Some(color);
        self
    }

    /// Set brightness for all lights
    pub fn with_brightness(mut self, brightness: u8) -> Self {
        self.brightness = Some(brightness);
        self
    }

    /// Set whether scene should be recyclable
    pub fn recyclable(mut self, recyclable: bool) -> Self {
        self.recyclable = recyclable;
        self
    }

    /// Add custom state for specific light
    pub fn with_light_state(mut self, light_id: String, state: LightState) -> Self {
        self.custom_states.insert(light_id, state);
        self
    }

    /// Build the scene request
    pub fn build(self) -> Result<CreateSceneRequest> {
        if self.name.is_empty() {
            return Err(HueStatusError::InvalidSceneData {
                reason: "Scene name cannot be empty".to_string(),
            });
        }

        if self.lights.is_empty() {
            return Err(HueStatusError::InvalidSceneData {
                reason: "Scene must have at least one light".to_string(),
            });
        }

        let mut lightstates = HashMap::new();

        // Create light states
        for light_id in &self.lights {
            let light_state = if let Some(custom_state) = self.custom_states.get(light_id) {
                // Use custom state
                custom_state.clone()
            } else if let Some(color) = &self.color {
                // Use specified color
                let mut state = color.to_light_state();
                if let Some(brightness) = self.brightness {
                    state.bri = Some(brightness);
                }
                state
            } else {
                // Default to white light
                LightState {
                    on: true,
                    bri: self.brightness.or(Some(254)),
                    hue: None,
                    sat: None,
                    xy: None,
                    ct: Some(366), // Default color temperature
                    effect: None,
                    alert: None,
                    colormode: Some("ct".to_string()),
                    mode: None,
                    reachable: None,
                }
            };

            lightstates.insert(light_id.clone(), light_state);
        }

        Ok(CreateSceneRequest {
            name: self.name,
            lights: self.lights,
            recycle: self.recyclable,
            lightstates,
        })
    }

    /// Build with validation
    pub fn build_validated(self) -> Result<CreateSceneRequest> {
        let request = self.build()?;
        request.validate()?;
        Ok(request)
    }
}

impl Default for SceneCreationOptions {
    fn default() -> Self {
        Self {
            brightness: 254,
            use_xy_color: false,
            validate_lights: true,
            test_execution: false,
            backup_existing: false,
        }
    }
}

impl Default for LightSelectionCriteria {
    fn default() -> Self {
        Self {
            require_color_support: true,
            require_reachable: true,
            exclude_light_types: Vec::new(),
            include_only_light_types: Vec::new(),
            min_brightness_support: None,
            room_filter: None,
        }
    }
}

impl LightSelectionCriteria {
    /// Create criteria for status scenes
    pub fn for_status_scenes() -> Self {
        Self {
            require_color_support: true,
            require_reachable: true,
            exclude_light_types: vec!["Motion sensor".to_string(), "Daylight".to_string()],
            include_only_light_types: Vec::new(),
            min_brightness_support: Some(1),
            room_filter: None,
        }
    }

    /// Create permissive criteria
    pub fn permissive() -> Self {
        Self {
            require_color_support: false,
            require_reachable: false,
            exclude_light_types: Vec::new(),
            include_only_light_types: Vec::new(),
            min_brightness_support: None,
            room_filter: None,
        }
    }

    /// Filter lights based on criteria
    pub fn filter_lights(&self, lights: &[(String, Light)]) -> Vec<(String, Light)> {
        lights
            .iter()
            .filter(|(_, light)| self.matches_criteria(light))
            .cloned()
            .collect()
    }

    /// Check if a light matches the criteria
    pub fn matches_criteria(&self, light: &Light) -> bool {
        // Check reachability
        if self.require_reachable && !light.is_reachable() {
            return false;
        }

        // Check color support
        if self.require_color_support && !light.supports_color() {
            return false;
        }

        // Check excluded light types
        if self.exclude_light_types.contains(&light.light_type) {
            return false;
        }

        // Check included light types (if specified)
        if !self.include_only_light_types.is_empty()
            && !self.include_only_light_types.contains(&light.light_type)
        {
            return false;
        }

        // Check minimum brightness support
        if let Some(min_brightness) = self.min_brightness_support {
            if light.brightness() < min_brightness {
                return false;
            }
        }

        true
    }

    /// Get criteria summary
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        if self.require_reachable {
            parts.push("reachable".to_string());
        }

        if self.require_color_support {
            parts.push("color-capable".to_string());
        }

        if !self.exclude_light_types.is_empty() {
            parts.push(format!(
                "excluding: {}",
                self.exclude_light_types.join(", ")
            ));
        }

        if !self.include_only_light_types.is_empty() {
            parts.push(format!(
                "only: {}",
                self.include_only_light_types.join(", ")
            ));
        }

        if let Some(min_brightness) = self.min_brightness_support {
            parts.push(format!("min brightness: {}", min_brightness));
        }

        if parts.is_empty() {
            "any lights".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// Advanced scene creation functions
pub struct SceneCreator {
    client: BridgeClient,
    verbose: bool,
}

impl SceneCreator {
    /// Create a new scene creator
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

    /// Create scene with automatic light selection
    pub async fn create_with_auto_selection(
        &self,
        name: String,
        color: ColorDefinition,
        criteria: &LightSelectionCriteria,
        options: &SceneCreationOptions,
    ) -> Result<String> {
        if self.verbose {
            eprintln!("🔍 Selecting lights with criteria: {}", criteria.summary());
        }

        // Get all lights
        let all_lights = self.client.get_lights().await?;
        let lights_vec: Vec<(String, Light)> = all_lights.into_iter().collect();

        // Filter lights based on criteria
        let suitable_lights = criteria.filter_lights(&lights_vec);

        if suitable_lights.is_empty() {
            return Err(HueStatusError::NoLightsFound);
        }

        if self.verbose {
            eprintln!("💡 Selected {} lights:", suitable_lights.len());
            for (id, light) in &suitable_lights {
                eprintln!("  - {} ({})", light.name, id);
            }
        }

        // Validate lights if requested
        if options.validate_lights {
            self.validate_lights(&suitable_lights).await?;
        }

        // Create scene
        let light_ids: Vec<String> = suitable_lights.iter().map(|(id, _)| id.clone()).collect();

        let scene_request = SceneBuilder::new(name)
            .with_lights(light_ids)
            .with_color(color)
            .with_brightness(options.brightness)
            .build_validated()?;

        let response = self.client.create_scene(&scene_request).await?;
        let scene_id = response
            .first()
            .ok_or_else(|| HueStatusError::SceneExecutionFailed {
                reason: "No response from scene creation".to_string(),
            })?
            .success
            .id
            .clone();

        // Test execution if requested
        if options.test_execution {
            self.test_scene_execution(&scene_id).await?;
        }

        if self.verbose {
            eprintln!("✅ Scene created successfully: {}", scene_id);
        }

        Ok(scene_id)
    }

    /// Create gradient scene (multiple colors across lights)
    pub async fn create_gradient_scene(
        &self,
        name: String,
        lights: Vec<String>,
        colors: Vec<ColorDefinition>,
        options: &SceneCreationOptions,
    ) -> Result<String> {
        if lights.is_empty() {
            return Err(HueStatusError::InvalidSceneData {
                reason: "No lights specified for gradient scene".to_string(),
            });
        }

        if colors.is_empty() {
            return Err(HueStatusError::InvalidSceneData {
                reason: "No colors specified for gradient scene".to_string(),
            });
        }

        if self.verbose {
            eprintln!(
                "🌈 Creating gradient scene with {} colors across {} lights",
                colors.len(),
                lights.len()
            );
        }

        let mut scene_builder = SceneBuilder::new(name).with_lights(lights.clone());

        // Distribute colors across lights
        for (i, light_id) in lights.iter().enumerate() {
            let color_index = i % colors.len();
            let color = &colors[color_index];

            let mut state = color.to_light_state();
            state.bri = Some(options.brightness);

            scene_builder = scene_builder.with_light_state(light_id.clone(), state);
        }

        let scene_request = scene_builder.build_validated()?;
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
            eprintln!("✅ Gradient scene created: {}", scene_id);
        }

        Ok(scene_id)
    }

    /// Create breathing effect scene
    pub async fn create_breathing_scene(
        &self,
        name: String,
        lights: Vec<String>,
        base_color: ColorDefinition,
        min_brightness: u8,
        max_brightness: u8,
    ) -> Result<String> {
        if min_brightness >= max_brightness {
            return Err(HueStatusError::InvalidSceneData {
                reason: "Minimum brightness must be less than maximum brightness".to_string(),
            });
        }

        if self.verbose {
            eprintln!(
                "💨 Creating breathing scene with brightness range {}-{}",
                min_brightness, max_brightness
            );
        }

        let mut scene_builder = SceneBuilder::new(name).with_lights(lights.clone());

        // Create alternating brightness levels
        for (i, light_id) in lights.iter().enumerate() {
            let brightness = if i % 2 == 0 {
                max_brightness
            } else {
                min_brightness
            };

            let mut state = base_color.to_light_state();
            state.bri = Some(brightness);
            state.effect = None; // Remove unsupported effect to prevent API errors

            scene_builder = scene_builder.with_light_state(light_id.clone(), state);
        }

        let scene_request = scene_builder.build_validated()?;
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
            eprintln!("✅ Breathing scene created: {}", scene_id);
        }

        Ok(scene_id)
    }

    /// Validate lights before scene creation
    async fn validate_lights(&self, lights: &[(String, Light)]) -> Result<()> {
        if self.verbose {
            eprintln!("🔍 Validating {} lights...", lights.len());
        }

        let mut issues = Vec::new();

        for (id, light) in lights {
            if !light.is_reachable() {
                issues.push(format!("Light '{}' ({}) is not reachable", light.name, id));
            }

            if !light.supports_color() && !light.supports_color_temperature() {
                issues.push(format!(
                    "Light '{}' ({}) does not support color or color temperature",
                    light.name, id
                ));
            }
        }

        if !issues.is_empty() {
            if self.verbose {
                eprintln!("❌ Light validation failed:");
                for issue in &issues {
                    eprintln!("  - {}", issue);
                }
            }

            return Err(HueStatusError::ValidationFailed {
                reason: format!("Light validation failed: {}", issues.join(", ")),
            });
        }

        if self.verbose {
            eprintln!("✅ All lights validated successfully");
        }

        Ok(())
    }

    /// Test scene execution
    async fn test_scene_execution(&self, scene_id: &str) -> Result<()> {
        if self.verbose {
            eprintln!("🧪 Testing scene execution: {}", scene_id);
        }

        // Get scene info to verify it exists
        let scene = self.client.get_scene(scene_id).await?;

        if !scene.is_suitable_for_status() {
            return Err(HueStatusError::ValidationFailed {
                reason: "Scene is not suitable for execution".to_string(),
            });
        }

        if self.verbose {
            eprintln!("✅ Scene execution test passed");
        }

        Ok(())
    }

    /// Clone an existing scene with modifications
    pub async fn clone_scene(
        &self,
        source_scene_id: &str,
        new_name: String,
        modifications: Option<HashMap<String, LightState>>,
    ) -> Result<String> {
        if self.verbose {
            eprintln!("📋 Cloning scene: {} -> {}", source_scene_id, new_name);
        }

        // Get source scene
        let source_scene = self.client.get_scene(source_scene_id).await?;

        let mut scene_builder =
            SceneBuilder::new(new_name).with_lights(source_scene.lights.clone());

        // Copy light states with optional modifications
        if let Some(source_states) = source_scene.lightstates {
            for (light_id, mut state) in source_states {
                // Apply modifications if provided
                if let Some(modifications) = &modifications {
                    if let Some(modified_state) = modifications.get(&light_id) {
                        state = modified_state.clone();
                    }
                }
                scene_builder = scene_builder.with_light_state(light_id, state);
            }
        }

        let scene_request = scene_builder.build_validated()?;
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
            eprintln!("✅ Scene cloned successfully: {}", scene_id);
        }

        Ok(scene_id)
    }
}

/// Common color presets
pub struct ColorPresets;

impl ColorPresets {
    /// Get warm white color
    pub fn warm_white() -> ColorDefinition {
        ColorDefinition {
            hue: 0,
            saturation: 0,
            brightness: 254,
            xy: Some([0.448, 0.407]), // Warm white in CIE 1931
            name: "Warm White".to_string(),
        }
    }

    /// Get cool white color
    pub fn cool_white() -> ColorDefinition {
        ColorDefinition {
            hue: 0,
            saturation: 0,
            brightness: 254,
            xy: Some([0.313, 0.329]), // Cool white in CIE 1931
            name: "Cool White".to_string(),
        }
    }

    /// Get blue color
    pub fn blue() -> ColorDefinition {
        ColorDefinition {
            hue: 43690, // Blue: 240° × 65536/360°
            saturation: 254,
            brightness: 254,
            xy: Some([0.167, 0.040]), // Blue in CIE 1931
            name: "Blue".to_string(),
        }
    }

    /// Get orange color
    pub fn orange() -> ColorDefinition {
        ColorDefinition {
            hue: 5461, // Orange: 30° × 65536/360°
            saturation: 254,
            brightness: 254,
            xy: Some([0.592, 0.382]), // Orange in CIE 1931
            name: "Orange".to_string(),
        }
    }

    /// Get purple color
    pub fn purple() -> ColorDefinition {
        ColorDefinition {
            hue: 49151, // Purple: 270° × 65536/360°
            saturation: 254,
            brightness: 254,
            xy: Some([0.245, 0.098]), // Purple in CIE 1931
            name: "Purple".to_string(),
        }
    }

    /// Get all preset colors
    pub fn all_presets() -> Vec<ColorDefinition> {
        vec![
            Self::warm_white(),
            Self::cool_white(),
            Self::blue(),
            Self::orange(),
            Self::purple(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_builder() {
        let builder = SceneBuilder::new("Test Scene".to_string())
            .with_lights(vec!["1".to_string(), "2".to_string()])
            .with_color(ColorPresets::blue())
            .with_brightness(200)
            .recyclable(true);

        let request = builder.build().unwrap();

        assert_eq!(request.name, "Test Scene");
        assert_eq!(request.lights.len(), 2);
        assert!(request.recycle);
        assert_eq!(request.lightstates.len(), 2);
    }

    #[test]
    fn test_light_selection_criteria() {
        let criteria = LightSelectionCriteria::for_status_scenes();

        assert!(criteria.require_color_support);
        assert!(criteria.require_reachable);
        assert!(criteria
            .exclude_light_types
            .contains(&"Motion sensor".to_string()));
    }

    #[test]
    fn test_scene_creation_options() {
        let options = SceneCreationOptions::default();

        assert_eq!(options.brightness, 254);
        assert!(!options.use_xy_color);
        assert!(options.validate_lights);
    }

    #[test]
    fn test_color_presets() {
        let blue = ColorPresets::blue();
        assert_eq!(blue.name, "Blue");
        assert_eq!(blue.hue, 43690);

        let warm_white = ColorPresets::warm_white();
        assert_eq!(warm_white.name, "Warm White");
        assert_eq!(warm_white.saturation, 0);

        let all_presets = ColorPresets::all_presets();
        assert_eq!(all_presets.len(), 5);
    }

    #[test]
    fn test_scene_builder_validation() {
        // Empty name should fail
        let builder = SceneBuilder::new("".to_string());
        assert!(builder.build().is_err());

        // No lights should fail
        let builder = SceneBuilder::new("Test".to_string());
        assert!(builder.build().is_err());

        // Valid builder should succeed
        let builder = SceneBuilder::new("Test".to_string()).with_lights(vec!["1".to_string()]);
        assert!(builder.build().is_ok());
    }

    #[test]
    fn test_criteria_summary() {
        let criteria = LightSelectionCriteria::for_status_scenes();
        let summary = criteria.summary();

        assert!(summary.contains("reachable"));
        assert!(summary.contains("color-capable"));
    }
}
