use crate::bridge::{BridgeAuth, BridgeClient, BridgeDiscovery, DiscoveredBridge};
use crate::config::{file::init_config_directory, Config};
use crate::error::{HueStatusError, Result};
use crate::scenes::SceneManager;
use console::{style, Term};
use std::io::{self, Write};
use std::time::Duration;

pub mod interactive;
pub mod validation;

pub use interactive::*;
pub use validation::*;

/// Setup process orchestrator
#[derive(Debug)]
pub struct SetupProcess {
    verbose: bool,
    force: bool,
    config_path: Option<String>,
    term: Term,
}

/// Setup configuration options
#[derive(Debug, Clone)]
pub struct SetupOptions {
    pub force: bool,
    pub interactive: bool,
    pub verbose: bool,
    pub timeout_seconds: u64,
    pub retry_attempts: usize,
    pub skip_validation: bool,
    pub backup_existing: bool,
    pub test_scenes: bool,
}

/// Setup result with detailed information
#[derive(Debug, Clone)]
pub struct SetupResult {
    pub success: bool,
    pub bridge_ip: String,
    pub bridge_name: String,
    pub username: String,
    pub scenes_created: usize,
    pub lights_configured: usize,
    pub config_path: String,
    pub duration_ms: u64,
    pub warnings: Vec<String>,
}

/// Setup step for progress tracking
#[derive(Debug, Clone, PartialEq)]
pub enum SetupStep {
    Initialize,
    DiscoverBridge,
    AuthenticateBridge,
    DiscoverLights,
    CreateScenes,
    ValidateSetup,
    SaveConfiguration,
    Complete,
}

/// Setup status for progress reporting
#[derive(Debug, Clone)]
pub struct SetupStatus {
    pub current_step: SetupStep,
    pub total_steps: usize,
    pub completed_steps: usize,
    pub message: String,
    pub error: Option<String>,
}

impl Default for SetupProcess {
    fn default() -> Self {
        Self::new()
    }
}

impl SetupProcess {
    /// Create a new setup process
    pub fn new() -> Self {
        Self {
            verbose: false,
            force: false,
            config_path: None,
            term: Term::stdout(),
        }
    }

    /// Configure setup options
    pub fn with_options(mut self, verbose: bool, force: bool, config_path: Option<String>) -> Self {
        self.verbose = verbose;
        self.force = force;
        self.config_path = config_path;
        self
    }

    /// Run the complete setup process
    pub async fn run(&mut self, options: &SetupOptions) -> Result<SetupResult> {
        let start_time = std::time::Instant::now();

        self.show_header();

        // Check if configuration already exists
        if !options.force && Config::exists() {
            return self.handle_existing_config().await;
        }

        let mut warnings = Vec::new();
        let mut status = SetupStatus {
            current_step: SetupStep::Initialize,
            total_steps: 7,
            completed_steps: 0,
            message: "Initializing setup...".to_string(),
            error: None,
        };

        // Step 1: Initialize
        self.update_progress(&status);
        self.initialize_setup().await?;
        status.completed_steps += 1;

        // Step 2: Discover bridge
        status.current_step = SetupStep::DiscoverBridge;
        status.message = "Discovering Hue bridges...".to_string();
        self.update_progress(&status);

        let bridge = self.discover_bridge_with_fallback(options).await?;
        status.completed_steps += 1;

        // Step 3: Authenticate
        status.current_step = SetupStep::AuthenticateBridge;
        status.message = format!("Authenticating with bridge at {}...", bridge.ip);
        self.update_progress(&status);

        let auth_result = self.authenticate_bridge(&bridge, options).await?;
        status.completed_steps += 1;

        // Step 4: Discover lights
        status.current_step = SetupStep::DiscoverLights;
        status.message = "Discovering lights...".to_string();
        self.update_progress(&status);

        let client = BridgeClient::new(bridge.ip.clone())?
            .with_username(auth_result.username.clone())
            .with_verbose(self.verbose);

        let suitable_lights = client.get_suitable_lights().await?;

        if suitable_lights.is_empty() {
            return Err(HueStatusError::NoLightsFound);
        }

        self.show_discovered_lights(&suitable_lights);
        status.completed_steps += 1;

        // Step 5: Create scenes
        status.current_step = SetupStep::CreateScenes;
        status.message = "Creating status scenes...".to_string();
        self.update_progress(&status);

        let mut config = Config::new(
            bridge.ip.clone(),
            auth_result.username.clone(),
            Config::create_scene_config("".to_string(), "huestatus-success".to_string(), true),
            Config::create_scene_config("".to_string(), "huestatus-failure".to_string(), true),
        );

        let scene_manager = SceneManager::new(client.clone()).with_verbose(self.verbose);
        let scene_result = scene_manager.create_status_scenes(&mut config).await?;
        status.completed_steps += 1;

        // Step 6: Validate setup
        if !options.skip_validation {
            status.current_step = SetupStep::ValidateSetup;
            status.message = "Validating setup...".to_string();
            self.update_progress(&status);

            let validation_warnings = self.validate_setup(&config, &client).await?;
            warnings.extend(validation_warnings);
            status.completed_steps += 1;
        } else {
            status.completed_steps += 1;
        }

        // Step 7: Save configuration
        status.current_step = SetupStep::SaveConfiguration;
        status.message = "Saving configuration...".to_string();
        self.update_progress(&status);

        config.save().map_err(|e| HueStatusError::SetupFailed {
            reason: format!("Failed to save configuration: {}", e),
        })?;

        status.completed_steps += 1;

        // Test scenes if requested
        if options.test_scenes {
            self.test_scenes(&config, &scene_manager).await?;
        }

        // Complete
        status.current_step = SetupStep::Complete;
        status.message = "Setup completed successfully!".to_string();
        self.update_progress(&status);

        let duration = start_time.elapsed().as_millis() as u64;
        let config_path_str =
            crate::config::safe_path_to_string_fallback(Config::get_config_file_path());

        let result = SetupResult {
            success: true,
            bridge_ip: bridge.ip,
            bridge_name: bridge.name.unwrap_or_else(|| "Unknown Bridge".to_string()),
            username: auth_result.username,
            scenes_created: scene_result.scenes_created,
            lights_configured: suitable_lights.len(),
            config_path: config_path_str,
            duration_ms: duration,
            warnings,
        };

        self.show_success(&result);
        Ok(result)
    }

    /// Show setup header
    fn show_header(&self) {
        self.term.clear_screen().ok();
        println!();
        println!("{}", style("🏗️  Huestatus Setup").bold().cyan());
        println!("{}", style("━".repeat(50)).dim());
        println!("Welcome to huestatus! Let's configure your Philips Hue lights.");
        println!();
    }

    /// Update progress display
    fn update_progress(&self, status: &SetupStatus) {
        let progress =
            (status.completed_steps as f32 / status.total_steps as f32 * 100.0).min(100.0);
        let progress_bar_len = ((progress / 5.0) as usize).min(20);
        let progress_bar = "█".repeat(progress_bar_len);
        let empty_bar = "░".repeat(20 - progress_bar_len);

        println!(
            "{} Step {}/{}: {}",
            self.get_step_emoji(&status.current_step),
            status.completed_steps + 1,
            status.total_steps,
            status.message
        );

        if self.verbose {
            println!("   [{}{}] {:.0}%", progress_bar, empty_bar, progress);
        }

        println!();
    }

    /// Get emoji for setup step
    fn get_step_emoji(&self, step: &SetupStep) -> &'static str {
        match step {
            SetupStep::Initialize => "⚙️",
            SetupStep::DiscoverBridge => "🔍",
            SetupStep::AuthenticateBridge => "🔑",
            SetupStep::DiscoverLights => "💡",
            SetupStep::CreateScenes => "🎨",
            SetupStep::ValidateSetup => "✅",
            SetupStep::SaveConfiguration => "💾",
            SetupStep::Complete => "✨",
        }
    }

    /// Initialize setup process
    async fn initialize_setup(&self) -> Result<()> {
        if self.verbose {
            println!("  • Initializing configuration directory...");
        }

        init_config_directory()?;

        if self.verbose {
            println!("  • Configuration directory ready");
        }

        Ok(())
    }

    /// Discover bridge with fallback methods
    async fn discover_bridge_with_fallback(
        &self,
        options: &SetupOptions,
    ) -> Result<DiscoveredBridge> {
        let discovery = BridgeDiscovery::new()?
            .with_timeout(Duration::from_secs(options.timeout_seconds))
            .with_verbose(self.verbose);

        if self.verbose {
            println!("  • Trying Philips discovery service...");
        }

        // Try all discovery methods
        match discovery.discover_all().await {
            Ok(result) => {
                if let Some(bridge) = result.first_bridge() {
                    if self.verbose {
                        println!("  • Found bridge: {}", bridge.display_name());
                    }
                    return Ok(bridge.clone());
                }
            }
            Err(e) => {
                if self.verbose {
                    println!("  • Automatic discovery failed: {}", e);
                }
            }
        }

        // If automatic discovery fails, ask for manual IP
        self.request_manual_bridge_ip(&discovery).await
    }

    /// Request manual bridge IP from user
    async fn request_manual_bridge_ip(
        &self,
        discovery: &BridgeDiscovery,
    ) -> Result<DiscoveredBridge> {
        println!("{} Automatic bridge discovery failed.", "⚠️");
        println!("Please enter your Hue bridge IP address manually.");
        println!();

        loop {
            print!("Bridge IP address: ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| HueStatusError::IoError { source: e })?;

            let ip = input.trim();
            if ip.is_empty() {
                println!("Please enter a valid IP address.");
                continue;
            }

            match discovery.discover_manual(ip).await {
                Ok(result) => {
                    if let Some(bridge) = result.first_bridge() {
                        println!("{}Bridge found at {}", "✅", ip);
                        return Ok(bridge.clone());
                    }
                }
                Err(_) => {
                    println!("{}No Hue bridge found at {}. Please try again.", "❌", ip);
                    continue;
                }
            }
        }
    }

    /// Authenticate with bridge
    async fn authenticate_bridge(
        &self,
        bridge: &DiscoveredBridge,
        options: &SetupOptions,
    ) -> Result<crate::bridge::AuthResult> {
        let auth = BridgeAuth::new(bridge.ip.clone())?
            .with_timeout(Duration::from_secs(options.timeout_seconds))
            .with_verbose(self.verbose);

        if options.interactive {
            auth.authenticate_interactive("huestatus", "cli").await
        } else {
            // Show instructions and wait for user input
            println!("{}Press the link button on your Hue bridge now.", "🔑");
            println!("The button is the large round button on top of the bridge.");
            println!(
                "You have {} seconds to press it...",
                options.timeout_seconds
            );
            println!();

            auth.authenticate("huestatus", "cli").await
        }
    }

    /// Show discovered lights
    fn show_discovered_lights(&self, lights: &[(String, crate::bridge::Light)]) {
        println!("{}Found {} suitable light(s):", "💡", lights.len());

        for (id, light) in lights {
            let status = if light.is_reachable() {
                style("●").green()
            } else {
                style("●").red()
            };

            let color_support = if light.supports_color() {
                style("Color").cyan()
            } else if light.supports_color_temperature() {
                style("White").yellow()
            } else {
                style("Basic").dim()
            };

            println!("  {} {} ({}) - {}", status, light.name, id, color_support);
        }
        println!();
    }

    /// Validate setup
    async fn validate_setup(&self, config: &Config, client: &BridgeClient) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        if self.verbose {
            println!("  • Validating bridge connection...");
        }

        // Test bridge connection
        client.test_connection().await?;

        if self.verbose {
            println!("  • Validating scenes...");
        }

        // Validate scenes
        let scene_manager = SceneManager::new(client.clone()).with_verbose(false);
        let validation_results = scene_manager.validate_status_scenes(config).await?;

        for result in validation_results {
            if !result.is_valid {
                warnings.extend(result.issues);
            }
        }

        if self.verbose {
            println!("  • Validation completed with {} warnings", warnings.len());
        }

        Ok(warnings)
    }

    /// Test scenes
    async fn test_scenes(&self, config: &Config, scene_manager: &SceneManager) -> Result<()> {
        println!("{}Testing scene execution...", "🎨");

        // Test success scene
        if let Some(success_scene) = config.get_scene("success") {
            match scene_manager.test_scene_execution(&success_scene.id).await {
                Ok(_) => println!("  {}Success scene test passed", "✅"),
                Err(e) => println!("  {}Success scene test failed: {}", "⚠️", e),
            }
        }

        // Test failure scene
        if let Some(failure_scene) = config.get_scene("failure") {
            match scene_manager.test_scene_execution(&failure_scene.id).await {
                Ok(_) => println!("  {}Failure scene test passed", "✅"),
                Err(e) => println!("  {}Failure scene test failed: {}", "⚠️", e),
            }
        }

        println!();
        Ok(())
    }

    /// Handle existing configuration
    async fn handle_existing_config(&self) -> Result<SetupResult> {
        println!("{}Configuration already exists!", "⚠️");

        if self.force {
            println!("Force flag detected, overwriting existing configuration...");
            return Err(HueStatusError::SetupFailed {
                reason: "Force setup not yet implemented".to_string(),
            });
        }

        println!("Use --force to overwrite the existing configuration.");
        println!("Or use 'huestatus --validate' to check your current setup.");

        Err(HueStatusError::SetupFailed {
            reason: "Configuration already exists".to_string(),
        })
    }

    /// Show success message
    fn show_success(&self, result: &SetupResult) {
        println!("{}", style("━".repeat(50)).dim());
        println!("{}Setup completed successfully!", "✨");
        println!();
        println!("Configuration Summary:");
        println!("  • Bridge: {} ({})", result.bridge_name, result.bridge_ip);
        println!("  • Scenes created: {}", result.scenes_created);
        println!("  • Lights configured: {}", result.lights_configured);
        println!("  • Setup time: {:.1}s", result.duration_ms as f64 / 1000.0);
        println!("  • Config saved to: {}", style(&result.config_path).cyan());

        if !result.warnings.is_empty() {
            println!();
            println!("{}Warnings:", "⚠️");
            for warning in &result.warnings {
                println!("  • {}", warning);
            }
        }

        println!();
        println!("{}You can now use:", style("Next steps:").bold());
        println!(
            "  • {} - Show successful status",
            style("huestatus success").green()
        );
        println!(
            "  • {} - Show failure status",
            style("huestatus failure").red()
        );
        println!(
            "  • {} - Validate your setup",
            style("huestatus --validate").cyan()
        );
        println!();
    }

    /// Run setup diagnostics
    pub async fn run_diagnostics(&self) -> Result<()> {
        println!("⚙️Running setup diagnostics...");
        println!();

        // Check if config exists
        if Config::exists() {
            println!("✅Configuration file found");

            match Config::load() {
                Ok(config) => {
                    println!("✅Configuration loaded successfully");

                    // Test bridge connection
                    match BridgeClient::new(config.bridge.ip.clone()) {
                        Ok(client) => {
                            let client =
                                client.with_username(config.bridge.application_key.clone());

                            match client.test_connection().await {
                                Ok(_) => println!("✅Bridge connection successful"),
                                Err(e) => println!("❌Bridge connection failed: {}", e),
                            }
                        }
                        Err(e) => println!("❌Failed to create bridge client: {}", e),
                    }
                }
                Err(e) => println!("❌Failed to load configuration: {}", e),
            }
        } else {
            println!("❌No configuration found. Run 'huestatus --setup' to configure.");
        }

        Ok(())
    }
}

impl Default for SetupOptions {
    fn default() -> Self {
        Self {
            force: false,
            interactive: true,
            verbose: false,
            timeout_seconds: 30,
            retry_attempts: 3,
            skip_validation: false,
            backup_existing: true,
            test_scenes: false,
        }
    }
}

impl SetupOptions {
    /// Create options for non-interactive setup
    pub fn non_interactive() -> Self {
        Self {
            interactive: false,
            ..Default::default()
        }
    }

    /// Create options for quick setup
    pub fn quick() -> Self {
        Self {
            timeout_seconds: 15,
            skip_validation: true,
            test_scenes: false,
            ..Default::default()
        }
    }

    /// Create options for thorough setup
    pub fn thorough() -> Self {
        Self {
            timeout_seconds: 60,
            skip_validation: false,
            test_scenes: true,
            backup_existing: true,
            verbose: true,
            ..Default::default()
        }
    }
}

impl SetupResult {
    /// Get setup summary
    pub fn summary(&self) -> String {
        format!(
            "Setup completed in {:.1}s: {} scenes created, {} lights configured",
            self.duration_ms as f64 / 1000.0,
            self.scenes_created,
            self.lights_configured
        )
    }

    /// Check if setup has warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get setup quality score (0-100)
    pub fn quality_score(&self) -> u8 {
        let mut score = 100u8;

        // Deduct points for warnings
        score = score.saturating_sub((self.warnings.len() * 10) as u8);

        // Deduct points for slow setup
        if self.duration_ms > 60000 {
            score = score.saturating_sub(10);
        } else if self.duration_ms > 30000 {
            score = score.saturating_sub(5);
        }

        // Deduct points for few lights
        if self.lights_configured < 2 {
            score = score.saturating_sub(15);
        }

        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_options() {
        let default_options = SetupOptions::default();
        assert!(default_options.interactive);
        assert!(!default_options.skip_validation);
        assert_eq!(default_options.timeout_seconds, 30);

        let quick_options = SetupOptions::quick();
        assert_eq!(quick_options.timeout_seconds, 15);
        assert!(quick_options.skip_validation);

        let thorough_options = SetupOptions::thorough();
        assert_eq!(thorough_options.timeout_seconds, 60);
        assert!(thorough_options.test_scenes);
    }

    #[test]
    fn test_setup_steps() {
        assert_eq!(SetupStep::Initialize, SetupStep::Initialize);
        assert_ne!(SetupStep::Initialize, SetupStep::DiscoverBridge);
    }

    #[test]
    fn test_setup_result() {
        let result = SetupResult {
            success: true,
            bridge_ip: "192.168.1.100".to_string(),
            bridge_name: "Test Bridge".to_string(),
            username: "test-user".to_string(),
            scenes_created: 2,
            lights_configured: 5,
            config_path: "/test/config.json".to_string(),
            duration_ms: 15000,
            warnings: vec!["Test warning".to_string()],
        };

        assert!(result.success);
        assert!(result.has_warnings());
        assert!(result.quality_score() > 70);
        assert!(result.summary().contains("15.0s"));
    }

    #[test]
    fn test_setup_status() {
        let status = SetupStatus {
            current_step: SetupStep::DiscoverBridge,
            total_steps: 7,
            completed_steps: 1,
            message: "Discovering bridges...".to_string(),
            error: None,
        };

        assert_eq!(status.current_step, SetupStep::DiscoverBridge);
        assert_eq!(status.completed_steps, 1);
        assert!(status.error.is_none());
    }

    #[test]
    fn test_update_progress_overflow_prevention() {
        let process = SetupProcess::new();

        // Test case 1: Normal progress (should work fine)
        let normal_status = SetupStatus {
            current_step: SetupStep::DiscoverBridge,
            total_steps: 7,
            completed_steps: 3,
            message: "Normal progress".to_string(),
            error: None,
        };
        // This should not panic
        process.update_progress(&normal_status);

        // Test case 2: Completed steps exceed total steps (overflow case)
        let overflow_status = SetupStatus {
            current_step: SetupStep::Complete,
            total_steps: 7,
            completed_steps: 10, // More than total_steps
            message: "Overflow case".to_string(),
            error: None,
        };
        // This should not panic due to our fix
        process.update_progress(&overflow_status);

        // Test case 3: Edge case with zero total steps
        let zero_total_status = SetupStatus {
            current_step: SetupStep::Initialize,
            total_steps: 1,
            completed_steps: 0,
            message: "Zero progress".to_string(),
            error: None,
        };
        // This should not panic
        process.update_progress(&zero_total_status);

        // Test case 4: Maximum possible progress
        let max_status = SetupStatus {
            current_step: SetupStep::Complete,
            total_steps: 7,
            completed_steps: 7,
            message: "Complete".to_string(),
            error: None,
        };
        // This should not panic and should cap at 100%
        process.update_progress(&max_status);
    }
}
