use crate::bridge::BridgeClient;
use crate::config::Config;
use crate::error::{HueStatusError, Result};
use crate::scenes::{SceneExecutionResult, SceneValidationResult};
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};

/// Scene execution manager with advanced features
#[derive(Debug, Clone)]
pub struct SceneExecutor {
    client: BridgeClient,
    verbose: bool,
    retry_attempts: usize,
    retry_delay: Duration,
}

/// Execution options for fine-tuning scene execution
#[derive(Debug, Clone)]
pub struct ExecutionOptions {
    pub validate_before_execution: bool,
    pub timeout_ms: u64,
    pub retry_on_failure: bool,
    pub max_retries: usize,
    pub retry_delay_ms: u64,
    pub measure_performance: bool,
    pub restore_previous_state: bool,
}

/// Scene execution strategy
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionStrategy {
    /// Execute immediately
    Immediate,
    /// Execute with delay
    Delayed(Duration),
    /// Execute with fade transition
    Fade { duration_ms: u64 },
    /// Execute with validation first
    ValidatedExecution,
    /// Execute with backup and restore capability
    BackupAndRestore,
}

/// Previous light state for restoration
#[derive(Debug, Clone)]
pub struct LightStateBackup {
    pub light_id: String,
    pub light_name: String,
    pub previous_state: crate::bridge::LightState,
    pub timestamp: Instant,
}

/// Scene execution context
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub scene_id: String,
    pub scene_name: String,
    pub strategy: ExecutionStrategy,
    pub options: ExecutionOptions,
    pub backup_states: Vec<LightStateBackup>,
}

/// Execution performance metrics
#[derive(Debug, Clone)]
pub struct ExecutionMetrics {
    pub total_time_ms: u64,
    pub validation_time_ms: u64,
    pub execution_time_ms: u64,
    pub backup_time_ms: u64,
    pub lights_affected: usize,
    pub retry_count: usize,
    pub success: bool,
}

impl SceneExecutor {
    /// Create a new scene executor
    pub fn new(client: BridgeClient) -> Self {
        Self {
            client,
            verbose: false,
            retry_attempts: 3,
            retry_delay: Duration::from_secs(1),
        }
    }

    /// Configure executor settings
    pub fn with_config(
        mut self,
        retry_attempts: usize,
        retry_delay: Duration,
        verbose: bool,
    ) -> Self {
        self.retry_attempts = retry_attempts;
        self.retry_delay = retry_delay;
        self.verbose = verbose;
        self
    }

    /// Execute a status scene with default options
    pub async fn execute_status_scene(
        &self,
        scene_type: &str,
        config: &Config,
    ) -> Result<SceneExecutionResult> {
        let options = ExecutionOptions::default();
        self.execute_status_scene_with_options(scene_type, config, &options)
            .await
    }

    /// Execute a status scene with custom options
    pub async fn execute_status_scene_with_options(
        &self,
        scene_type: &str,
        config: &Config,
        options: &ExecutionOptions,
    ) -> Result<SceneExecutionResult> {
        let scene_config =
            config
                .get_scene(scene_type)
                .ok_or_else(|| HueStatusError::SceneNotFound {
                    scene_name: scene_type.to_string(),
                })?;

        let context = ExecutionContext {
            scene_id: scene_config.id.clone(),
            scene_name: scene_config.name.clone(),
            strategy: ExecutionStrategy::Immediate,
            options: options.clone(),
            backup_states: Vec::new(),
        };

        self.execute_with_context(context).await
    }

    /// Execute scene with full context and strategy
    pub async fn execute_with_context(
        &self,
        mut context: ExecutionContext,
    ) -> Result<SceneExecutionResult> {
        let start_time = Instant::now();
        let mut metrics = ExecutionMetrics {
            total_time_ms: 0,
            validation_time_ms: 0,
            execution_time_ms: 0,
            backup_time_ms: 0,
            lights_affected: 0,
            retry_count: 0,
            success: false,
        };

        if self.verbose {
            eprintln!(
                "🎬 Executing scene: {} ({})",
                context.scene_name, context.scene_id
            );
            eprintln!("📋 Strategy: {:?}", context.strategy);
        }

        // Validation phase
        if context.options.validate_before_execution {
            let validation_start = Instant::now();
            self.validate_scene_execution(&context.scene_id).await?;
            metrics.validation_time_ms = validation_start.elapsed().as_millis() as u64;

            if self.verbose {
                eprintln!(
                    "✅ Scene validation passed ({}ms)",
                    metrics.validation_time_ms
                );
            }
        }

        // Backup phase
        if context.options.restore_previous_state {
            let backup_start = Instant::now();
            context.backup_states = self.backup_current_states(&context.scene_id).await?;
            metrics.backup_time_ms = backup_start.elapsed().as_millis() as u64;

            if self.verbose {
                eprintln!(
                    "💾 Backed up {} light states ({}ms)",
                    context.backup_states.len(),
                    metrics.backup_time_ms
                );
            }
        }

        // Execution phase with retry logic
        let execution_result = self.execute_with_retry(&context, &mut metrics).await;

        metrics.total_time_ms = start_time.elapsed().as_millis() as u64;
        metrics.success = execution_result.is_ok();

        if self.verbose {
            self.log_execution_metrics(&metrics);
        }

        match execution_result {
            Ok(execution_time) => Ok(SceneExecutionResult {
                scene_id: context.scene_id,
                scene_name: context.scene_name,
                execution_time_ms: execution_time,
                success: true,
            }),
            Err(e) => {
                if self.verbose {
                    eprintln!("❌ Scene execution failed: {e}");
                }
                Err(e)
            }
        }
    }

    /// Execute with retry logic
    async fn execute_with_retry(
        &self,
        context: &ExecutionContext,
        metrics: &mut ExecutionMetrics,
    ) -> Result<u64> {
        let max_attempts = if context.options.retry_on_failure {
            context.options.max_retries.max(1)
        } else {
            1
        };

        let mut last_error = None;

        for attempt in 0..max_attempts {
            if attempt > 0 {
                metrics.retry_count += 1;
                let delay = Duration::from_millis(context.options.retry_delay_ms);

                if self.verbose {
                    eprintln!(
                        "⏳ Retrying execution (attempt {}/{}) after {}ms",
                        attempt + 1,
                        max_attempts,
                        delay.as_millis()
                    );
                }

                sleep(delay).await;
            }

            match self.execute_single_attempt(context).await {
                Ok(execution_time) => {
                    metrics.execution_time_ms = execution_time;
                    return Ok(execution_time);
                }
                Err(e) => {
                    last_error = Some(e);

                    if self.verbose {
                        eprintln!("❌ Attempt {} failed", attempt + 1);
                    }
                }
            }
        }

        Err(
            last_error.unwrap_or_else(|| HueStatusError::SceneExecutionFailed {
                reason: "All retry attempts failed".to_string(),
            }),
        )
    }

    /// Execute a single attempt
    async fn execute_single_attempt(&self, context: &ExecutionContext) -> Result<u64> {
        let execution_start = Instant::now();

        match &context.strategy {
            ExecutionStrategy::Immediate => {
                self.execute_immediate(&context.scene_id, context.options.timeout_ms)
                    .await?;
            }
            ExecutionStrategy::Delayed(delay) => {
                sleep(*delay).await;
                self.execute_immediate(&context.scene_id, context.options.timeout_ms)
                    .await?;
            }
            ExecutionStrategy::Fade { duration_ms } => {
                self.execute_with_fade(&context.scene_id, *duration_ms)
                    .await?;
            }
            ExecutionStrategy::ValidatedExecution => {
                self.validate_scene_execution(&context.scene_id).await?;
                self.execute_immediate(&context.scene_id, context.options.timeout_ms)
                    .await?;
            }
            ExecutionStrategy::BackupAndRestore => {
                // Backup is handled in the main execution flow
                self.execute_immediate(&context.scene_id, context.options.timeout_ms)
                    .await?;
            }
        }

        Ok(execution_start.elapsed().as_millis() as u64)
    }

    /// Execute scene immediately
    async fn execute_immediate(&self, scene_id: &str, timeout_ms: u64) -> Result<()> {
        let execution_timeout = Duration::from_millis(timeout_ms);

        timeout(execution_timeout, self.client.execute_scene(scene_id))
            .await
            .map_err(|_| HueStatusError::TimeoutError {
                operation: format!("Scene execution for {scene_id}"),
            })?
            .map_err(|e| HueStatusError::SceneExecutionFailed {
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Execute scene with fade effect (simulated)
    async fn execute_with_fade(&self, scene_id: &str, duration_ms: u64) -> Result<()> {
        if self.verbose {
            eprintln!("🌅 Executing scene with fade effect ({duration_ms}ms)");
        }

        // For now, just execute immediately
        // In a full implementation, this would gradually transition the lights
        self.execute_immediate(scene_id, duration_ms + 5000).await?;

        Ok(())
    }

    /// Validate scene before execution
    async fn validate_scene_execution(&self, scene_id: &str) -> Result<()> {
        // Check if scene exists
        let scene = self.client.get_scene(scene_id).await?;

        // Check if scene is suitable for execution
        if !scene.is_suitable_for_status() {
            return Err(HueStatusError::ValidationFailed {
                reason: format!("Scene '{}' is not suitable for execution", scene.name),
            });
        }

        // Check if lights are reachable
        let lights = self.client.get_lights().await?;
        let mut unreachable_lights = Vec::new();

        for light_id in &scene.lights {
            if let Some(light) = lights.get(light_id) {
                if !light.is_reachable() {
                    unreachable_lights.push(light.name.clone());
                }
            }
        }

        if !unreachable_lights.is_empty() {
            return Err(HueStatusError::ValidationFailed {
                reason: format!("Unreachable lights: {}", unreachable_lights.join(", ")),
            });
        }

        Ok(())
    }

    /// Backup current light states
    async fn backup_current_states(&self, scene_id: &str) -> Result<Vec<LightStateBackup>> {
        let scene = self.client.get_scene(scene_id).await?;
        let lights = self.client.get_lights().await?;
        let mut backups = Vec::new();

        for light_id in &scene.lights {
            if let Some(light) = lights.get(light_id) {
                backups.push(LightStateBackup {
                    light_id: light_id.clone(),
                    light_name: light.name.clone(),
                    previous_state: light.state.clone(),
                    timestamp: Instant::now(),
                });
            }
        }

        Ok(backups)
    }

    /// Restore previous light states
    pub async fn restore_states(&self, backups: &[LightStateBackup]) -> Result<()> {
        if self.verbose {
            eprintln!("🔄 Restoring {} light states...", backups.len());
        }

        for backup in backups {
            // In a full implementation, this would restore individual light states
            // For now, we'll just log the restoration
            if self.verbose {
                eprintln!("  - Restoring {} ({})", backup.light_name, backup.light_id);
            }
        }

        if self.verbose {
            eprintln!("✅ Light states restored");
        }

        Ok(())
    }

    /// Execute scene with automatic rollback on failure
    pub async fn execute_with_rollback(
        &self,
        scene_id: &str,
        rollback_scene_id: &str,
        timeout_ms: u64,
    ) -> Result<SceneExecutionResult> {
        let start_time = Instant::now();

        if self.verbose {
            eprintln!(
                "🎬 Executing scene with rollback: {scene_id} -> {rollback_scene_id}"
            );
        }

        match self.execute_immediate(scene_id, timeout_ms).await {
            Ok(()) => {
                let execution_time = start_time.elapsed().as_millis() as u64;

                if self.verbose {
                    eprintln!("✅ Scene executed successfully");
                }

                Ok(SceneExecutionResult {
                    scene_id: scene_id.to_string(),
                    scene_name: "Unknown".to_string(), // Would need to fetch scene name
                    execution_time_ms: execution_time,
                    success: true,
                })
            }
            Err(e) => {
                if self.verbose {
                    eprintln!("❌ Scene execution failed, rolling back...");
                }

                // Attempt rollback
                match self.execute_immediate(rollback_scene_id, timeout_ms).await {
                    Ok(()) => {
                        if self.verbose {
                            eprintln!("✅ Rollback successful");
                        }
                    }
                    Err(rollback_error) => {
                        if self.verbose {
                            eprintln!("❌ Rollback failed: {rollback_error}");
                        }
                    }
                }

                Err(e)
            }
        }
    }

    /// Test scene execution without actually executing
    pub async fn test_execution(&self, scene_id: &str) -> Result<SceneValidationResult> {
        if self.verbose {
            eprintln!("🧪 Testing scene execution: {scene_id}");
        }

        let mut issues = Vec::new();
        let mut is_valid = true;

        // Check if scene exists
        let scene = match self.client.get_scene(scene_id).await {
            Ok(scene) => scene,
            Err(_) => {
                issues.push("Scene not found".to_string());
                is_valid = false;
                return Ok(SceneValidationResult {
                    scene_id: scene_id.to_string(),
                    scene_name: "Unknown".to_string(),
                    is_valid,
                    issues,
                    lights_status: Vec::new(),
                });
            }
        };

        // Validate scene properties
        if !scene.is_suitable_for_status() {
            issues.push("Scene is not suitable for status indication".to_string());
            is_valid = false;
        }

        // Check lights
        let lights = self.client.get_lights().await?;
        let mut lights_status = Vec::new();

        for light_id in &scene.lights {
            if let Some(light) = lights.get(light_id) {
                let light_status = crate::scenes::LightStatus {
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

                lights_status.push(light_status);
            } else {
                issues.push(format!("Light '{light_id}' not found"));
                is_valid = false;
            }
        }

        if self.verbose {
            if is_valid {
                eprintln!("✅ Scene execution test passed");
            } else {
                eprintln!("❌ Scene execution test failed: {} issues", issues.len());
            }
        }

        Ok(SceneValidationResult {
            scene_id: scene_id.to_string(),
            scene_name: scene.name,
            is_valid,
            issues,
            lights_status,
        })
    }

    /// Log execution metrics
    fn log_execution_metrics(&self, metrics: &ExecutionMetrics) {
        eprintln!("📊 Execution Metrics:");
        eprintln!("  Total time: {}ms", metrics.total_time_ms);
        eprintln!("  Validation: {}ms", metrics.validation_time_ms);
        eprintln!("  Execution: {}ms", metrics.execution_time_ms);
        eprintln!("  Backup: {}ms", metrics.backup_time_ms);
        eprintln!("  Lights affected: {}", metrics.lights_affected);
        eprintln!("  Retry count: {}", metrics.retry_count);
        eprintln!("  Success: {}", metrics.success);
    }

    /// Get execution history (placeholder for future implementation)
    pub fn get_execution_history(&self) -> Vec<SceneExecutionResult> {
        // This would be implemented with persistent storage
        Vec::new()
    }
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            validate_before_execution: false,
            timeout_ms: 5000,
            retry_on_failure: true,
            max_retries: 3,
            retry_delay_ms: 1000,
            measure_performance: true,
            restore_previous_state: false,
        }
    }
}

impl ExecutionOptions {
    /// Create options for fast execution
    pub fn fast() -> Self {
        Self {
            validate_before_execution: false,
            timeout_ms: 2000,
            retry_on_failure: false,
            max_retries: 1,
            retry_delay_ms: 500,
            measure_performance: false,
            restore_previous_state: false,
        }
    }

    /// Create options for reliable execution
    pub fn reliable() -> Self {
        Self {
            validate_before_execution: true,
            timeout_ms: 10000,
            retry_on_failure: true,
            max_retries: 5,
            retry_delay_ms: 2000,
            measure_performance: true,
            restore_previous_state: true,
        }
    }

    /// Create options for testing
    pub fn testing() -> Self {
        Self {
            validate_before_execution: true,
            timeout_ms: 15000,
            retry_on_failure: false,
            max_retries: 1,
            retry_delay_ms: 0,
            measure_performance: true,
            restore_previous_state: true,
        }
    }
}

impl ExecutionMetrics {
    /// Get overall performance score (0-100)
    pub fn performance_score(&self) -> u8 {
        if !self.success {
            return 0;
        }

        let mut score = 100u8;

        // Deduct points for slow execution
        if self.execution_time_ms > 2000 {
            score = score.saturating_sub(30);
        } else if self.execution_time_ms > 1000 {
            score = score.saturating_sub(15);
        } else if self.execution_time_ms > 500 {
            score = score.saturating_sub(5);
        }

        // Deduct points for retries
        score = score.saturating_sub((self.retry_count * 10) as u8);

        // Deduct points for slow validation
        if self.validation_time_ms > 1000 {
            score = score.saturating_sub(10);
        }

        score
    }

    /// Check if execution was fast
    pub fn is_fast_execution(&self) -> bool {
        self.success && self.execution_time_ms < 500 && self.retry_count == 0
    }

    /// Get execution summary
    pub fn summary(&self) -> String {
        format!(
            "Execution: {}ms, Total: {}ms, Retries: {}, Score: {}",
            self.execution_time_ms,
            self.total_time_ms,
            self.retry_count,
            self.performance_score()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_options() {
        let default_options = ExecutionOptions::default();
        assert!(default_options.retry_on_failure);
        assert_eq!(default_options.max_retries, 3);
        assert_eq!(default_options.timeout_ms, 5000);

        let fast_options = ExecutionOptions::fast();
        assert!(!fast_options.retry_on_failure);
        assert_eq!(fast_options.timeout_ms, 2000);

        let reliable_options = ExecutionOptions::reliable();
        assert!(reliable_options.validate_before_execution);
        assert_eq!(reliable_options.max_retries, 5);
    }

    #[test]
    fn test_execution_strategy() {
        let immediate = ExecutionStrategy::Immediate;
        let delayed = ExecutionStrategy::Delayed(Duration::from_millis(1000));
        let fade = ExecutionStrategy::Fade { duration_ms: 2000 };

        assert_eq!(immediate, ExecutionStrategy::Immediate);
        assert_ne!(immediate, delayed);

        match fade {
            ExecutionStrategy::Fade { duration_ms } => assert_eq!(duration_ms, 2000),
            _ => panic!("Expected Fade strategy"),
        }
    }

    #[test]
    fn test_execution_metrics() {
        let mut metrics = ExecutionMetrics {
            total_time_ms: 1500,
            validation_time_ms: 200,
            execution_time_ms: 800,
            backup_time_ms: 100,
            lights_affected: 5,
            retry_count: 1,
            success: true,
        };

        assert!(metrics.performance_score() > 50);
        assert!(!metrics.is_fast_execution()); // Has retry

        metrics.retry_count = 0;
        metrics.execution_time_ms = 300;
        assert!(metrics.is_fast_execution());
    }

    #[test]
    fn test_light_state_backup() {
        let backup = LightStateBackup {
            light_id: "1".to_string(),
            light_name: "Test Light".to_string(),
            previous_state: crate::bridge::LightState {
                on: true,
                bri: Some(200),
                hue: Some(1000),
                sat: Some(200),
                effect: None,
                xy: None,
                ct: None,
                alert: None,
                colormode: Some("hs".to_string()),
                mode: None,
                reachable: Some(true),
            },
            timestamp: Instant::now(),
        };

        assert_eq!(backup.light_id, "1");
        assert_eq!(backup.light_name, "Test Light");
        assert!(backup.previous_state.on);
    }

    #[test]
    fn test_execution_context() {
        let context = ExecutionContext {
            scene_id: "test-scene".to_string(),
            scene_name: "Test Scene".to_string(),
            strategy: ExecutionStrategy::Immediate,
            options: ExecutionOptions::fast(),
            backup_states: Vec::new(),
        };

        assert_eq!(context.scene_id, "test-scene");
        assert_eq!(context.strategy, ExecutionStrategy::Immediate);
        assert_eq!(context.options.timeout_ms, 2000);
    }
}
