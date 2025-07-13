use crate::bridge::{
    BridgeCapabilities, BridgeConfiguration, CreateSceneRequest, Group, Light, Scene,
    SceneActionRequest,
};
use crate::error::{HueStatusError, Result};
use reqwest::{Client, ClientBuilder};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::{sleep, timeout};

/// HTTP client for interacting with Hue Bridge API
#[derive(Debug, Clone)]
pub struct BridgeClient {
    client: Client,
    bridge_ip: String,
    username: Option<String>,
    timeout: Duration,
    retry_attempts: usize,
    retry_delay: Duration,
    verbose: bool,
}

impl BridgeClient {
    /// Create a new bridge client
    pub fn new(bridge_ip: String) -> Result<Self> {
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(10))
            .user_agent("huestatus/1.0")
            .build()
            .map_err(|e| HueStatusError::NetworkError { source: e })?;

        Ok(Self {
            client,
            bridge_ip,
            username: None,
            timeout: Duration::from_secs(10),
            retry_attempts: 3,
            retry_delay: Duration::from_secs(1),
            verbose: false,
        })
    }

    /// Create a new bridge client with custom configuration
    pub fn with_config(
        bridge_ip: String,
        timeout_seconds: u64,
        retry_attempts: usize,
        retry_delay_seconds: u64,
        verbose: bool,
    ) -> Result<Self> {
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(timeout_seconds))
            .user_agent("huestatus/1.0")
            .build()
            .map_err(|e| HueStatusError::NetworkError { source: e })?;

        Ok(Self {
            client,
            bridge_ip,
            username: None,
            timeout: Duration::from_secs(timeout_seconds),
            retry_attempts,
            retry_delay: Duration::from_secs(retry_delay_seconds),
            verbose,
        })
    }

    /// Set the username for authenticated requests
    pub fn with_username(mut self, username: String) -> Self {
        self.username = Some(username);
        self
    }

    /// Set verbose mode
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Get the base URL for API requests
    fn base_url(&self) -> String {
        format!("http://{}/api", self.bridge_ip)
    }

    /// Get the authenticated base URL
    fn authenticated_url(&self) -> Result<String> {
        let username = self
            .username
            .as_ref()
            .ok_or(HueStatusError::AuthenticationFailed)?;
        Ok(format!("http://{}/api/{}", self.bridge_ip, username))
    }

    /// Make a GET request with retry logic
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = if path.starts_with('/') {
            format!("{}{}", self.base_url(), path)
        } else {
            format!("{}/{}", self.authenticated_url()?, path)
        };

        self.request_with_retry(|| async {
            if self.verbose {
                eprintln!("🔍 GET {}", url);
            }

            let response = timeout(self.timeout, self.client.get(&url).send())
                .await
                .map_err(|_| HueStatusError::TimeoutError {
                    operation: format!("GET {}", url),
                })?
                .map_err(|e| HueStatusError::NetworkError { source: e })?;

            if self.verbose {
                eprintln!("📡 Response: {} {}", response.status(), response.url());
            }

            let json: serde_json::Value = response
                .json()
                .await
                .map_err(|e| HueStatusError::NetworkError { source: e })?;

            // Check if response is an error array
            if let Ok(errors) = serde_json::from_value::<Vec<crate::bridge::HueError>>(json.clone())
            {
                if !errors.is_empty() {
                    return Err(errors[0].clone().into());
                }
            }

            serde_json::from_value(json).map_err(|e| HueStatusError::JsonError { source: e })
        })
        .await
    }

    /// Make a POST request with retry logic
    async fn post<T: Serialize, R: DeserializeOwned>(&self, path: &str, body: &T) -> Result<R> {
        let url = if path.starts_with('/') {
            format!("{}{}", self.base_url(), path)
        } else {
            format!("{}/{}", self.authenticated_url()?, path)
        };

        self.request_with_retry(|| async {
            if self.verbose {
                eprintln!("🔍 POST {}", url);
                if let Ok(json) = serde_json::to_string_pretty(body) {
                    eprintln!("📤 Body: {}", json);
                }
            }

            let response = timeout(self.timeout, self.client.post(&url).json(body).send())
                .await
                .map_err(|_| HueStatusError::TimeoutError {
                    operation: format!("POST {}", url),
                })?
                .map_err(|e| HueStatusError::NetworkError { source: e })?;

            if self.verbose {
                eprintln!("📡 Response: {} {}", response.status(), response.url());
            }

            let json: serde_json::Value = response
                .json()
                .await
                .map_err(|e| HueStatusError::NetworkError { source: e })?;

            // Check if response is an error array
            if let Ok(errors) = serde_json::from_value::<Vec<crate::bridge::HueError>>(json.clone())
            {
                if !errors.is_empty() {
                    return Err(errors[0].clone().into());
                }
            }

            serde_json::from_value(json).map_err(|e| HueStatusError::JsonError { source: e })
        })
        .await
    }

    /// Make a PUT request with retry logic
    async fn put<T: Serialize, R: DeserializeOwned>(&self, path: &str, body: &T) -> Result<R> {
        let url = if path.starts_with('/') {
            format!("{}{}", self.base_url(), path)
        } else {
            format!("{}/{}", self.authenticated_url()?, path)
        };

        self.request_with_retry(|| async {
            if self.verbose {
                eprintln!("🔍 PUT {}", url);
                if let Ok(json) = serde_json::to_string_pretty(body) {
                    eprintln!("📤 Body: {}", json);
                }
            }

            let response = timeout(self.timeout, self.client.put(&url).json(body).send())
                .await
                .map_err(|_| HueStatusError::TimeoutError {
                    operation: format!("PUT {}", url),
                })?
                .map_err(|e| HueStatusError::NetworkError { source: e })?;

            if self.verbose {
                eprintln!("📡 Response: {} {}", response.status(), response.url());
            }

            let json: serde_json::Value = response
                .json()
                .await
                .map_err(|e| HueStatusError::NetworkError { source: e })?;

            // Check if response is an error array
            if let Ok(errors) = serde_json::from_value::<Vec<crate::bridge::HueError>>(json.clone())
            {
                if !errors.is_empty() {
                    return Err(errors[0].clone().into());
                }
            }

            serde_json::from_value(json).map_err(|e| HueStatusError::JsonError { source: e })
        })
        .await
    }

    /// Make a DELETE request with retry logic
    async fn delete<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        let url = if path.starts_with('/') {
            format!("{}{}", self.base_url(), path)
        } else {
            format!("{}/{}", self.authenticated_url()?, path)
        };

        self.request_with_retry(|| async {
            if self.verbose {
                eprintln!("🔍 DELETE {}", url);
            }

            let response = timeout(self.timeout, self.client.delete(&url).send())
                .await
                .map_err(|_| HueStatusError::TimeoutError {
                    operation: format!("DELETE {}", url),
                })?
                .map_err(|e| HueStatusError::NetworkError { source: e })?;

            if self.verbose {
                eprintln!("📡 Response: {} {}", response.status(), response.url());
            }

            let json: serde_json::Value = response
                .json()
                .await
                .map_err(|e| HueStatusError::NetworkError { source: e })?;

            // Check if response is an error array
            if let Ok(errors) = serde_json::from_value::<Vec<crate::bridge::HueError>>(json.clone())
            {
                if !errors.is_empty() {
                    return Err(errors[0].clone().into());
                }
            }

            serde_json::from_value(json).map_err(|e| HueStatusError::JsonError { source: e })
        })
        .await
    }

    /// Execute a request with retry logic
    async fn request_with_retry<F, Fut, T>(&self, request_fn: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;

        for attempt in 0..self.retry_attempts {
            match request_fn().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    last_error = Some(error);

                    if attempt < self.retry_attempts - 1 {
                        if self.verbose {
                            eprintln!(
                                "⏳ Retry attempt {} in {} seconds",
                                attempt + 1,
                                self.retry_delay.as_secs()
                            );
                        }
                        sleep(self.retry_delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| HueStatusError::ApiError {
            message: "Request failed after all retries".to_string(),
        }))
    }

    /// Test connection to bridge
    pub async fn test_connection(&self) -> Result<()> {
        let url = format!("http://{}/api/0/config", self.bridge_ip);

        if self.verbose {
            eprintln!("🔍 Testing connection to {}", self.bridge_ip);
        }

        timeout(self.timeout, self.client.get(&url).send())
            .await
            .map_err(|_| HueStatusError::TimeoutError {
                operation: "Connection test".to_string(),
            })?
            .map_err(|e| HueStatusError::BridgeConnectionFailed {
                reason: e.to_string(),
            })?;

        if self.verbose {
            eprintln!("✅ Connection test successful");
        }

        Ok(())
    }

    /// Get bridge configuration
    pub async fn get_config(&self) -> Result<BridgeConfiguration> {
        self.get("config").await
    }

    /// Get bridge capabilities
    pub async fn get_capabilities(&self) -> Result<BridgeCapabilities> {
        self.get("capabilities").await
    }

    /// Get all lights
    pub async fn get_lights(&self) -> Result<HashMap<String, Light>> {
        self.get("lights").await
    }

    /// Get specific light
    pub async fn get_light(&self, light_id: &str) -> Result<Light> {
        self.get(&format!("lights/{}", light_id)).await
    }

    /// Get all scenes
    pub async fn get_scenes(&self) -> Result<HashMap<String, Scene>> {
        self.get("scenes").await
    }

    /// Get specific scene
    pub async fn get_scene(&self, scene_id: &str) -> Result<Scene> {
        self.get(&format!("scenes/{}", scene_id)).await
    }

    /// Create a new scene
    pub async fn create_scene(
        &self,
        scene: &CreateSceneRequest,
    ) -> Result<Vec<CreateSceneResponse>> {
        scene.validate()?;
        self.post("scenes", scene).await
    }

    /// Delete a scene
    pub async fn delete_scene(&self, scene_id: &str) -> Result<Vec<DeleteResponse>> {
        self.delete(&format!("scenes/{}", scene_id)).await
    }

    /// Execute a scene on all lights (group 0)
    pub async fn execute_scene(&self, scene_id: &str) -> Result<Vec<ActionResponse>> {
        let action = SceneActionRequest::new(scene_id.to_string());
        self.put("groups/0/action", &action).await
    }

    /// Execute a scene on specific group
    pub async fn execute_scene_on_group(
        &self,
        group_id: &str,
        scene_id: &str,
    ) -> Result<Vec<ActionResponse>> {
        let action = SceneActionRequest::new(scene_id.to_string());
        self.put(&format!("groups/{}/action", group_id), &action)
            .await
    }

    /// Get all groups
    pub async fn get_groups(&self) -> Result<HashMap<String, Group>> {
        self.get("groups").await
    }

    /// Get specific group
    pub async fn get_group(&self, group_id: &str) -> Result<Group> {
        self.get(&format!("groups/{}", group_id)).await
    }

    /// Get reachable lights suitable for status indication
    pub async fn get_suitable_lights(&self) -> Result<Vec<(String, Light)>> {
        let lights = self.get_lights().await?;
        let mut suitable_lights = Vec::new();

        for (id, light) in lights {
            if light.is_suitable_for_status() {
                suitable_lights.push((id, light));
            }
        }

        if suitable_lights.is_empty() {
            return Err(HueStatusError::NoLightsFound);
        }

        Ok(suitable_lights)
    }

    /// Check if scene exists
    pub async fn scene_exists(&self, scene_id: &str) -> Result<bool> {
        match self.get_scene(scene_id).await {
            Ok(_) => Ok(true),
            Err(HueStatusError::ApiError { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Validate scene execution
    pub async fn validate_scene(&self, scene_id: &str) -> Result<()> {
        // Check if scene exists
        let scene = self.get_scene(scene_id).await?;

        // Check if scene is suitable for status indication
        if !scene.is_suitable_for_status() {
            return Err(HueStatusError::ValidationFailed {
                reason: format!(
                    "Scene '{}' is not suitable for status indication",
                    scene.name
                ),
            });
        }

        // Check if all lights in scene are reachable
        let lights = self.get_lights().await?;
        for light_id in &scene.lights {
            if let Some(light) = lights.get(light_id) {
                if !light.is_reachable() {
                    return Err(HueStatusError::ValidationFailed {
                        reason: format!(
                            "Light '{}' in scene '{}' is not reachable",
                            light.name, scene.name
                        ),
                    });
                }
            } else {
                return Err(HueStatusError::ValidationFailed {
                    reason: format!("Light '{}' in scene '{}' not found", light_id, scene.name),
                });
            }
        }

        Ok(())
    }

    /// Get bridge status summary
    pub async fn get_bridge_status(&self) -> Result<BridgeStatus> {
        let config = self.get_config().await?;
        let capabilities = self.get_capabilities().await?;
        let lights = self.get_lights().await?;
        let scenes = self.get_scenes().await?;

        let reachable_lights = lights.values().filter(|light| light.is_reachable()).count();

        let suitable_lights = lights
            .values()
            .filter(|light| light.is_suitable_for_status())
            .count();

        Ok(BridgeStatus {
            bridge_name: config.name,
            bridge_id: config.bridgeid,
            api_version: config.apiversion,
            sw_version: config.swversion,
            total_lights: lights.len(),
            reachable_lights,
            suitable_lights,
            total_scenes: scenes.len(),
            available_scenes: capabilities.scenes.available,
            max_scenes: capabilities.scenes.total,
        })
    }
}

/// Response for scene creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSceneResponse {
    pub success: CreateSceneSuccess,
}

/// Success response for scene creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSceneSuccess {
    pub id: String,
}

/// Response for delete operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResponse {
    pub success: String,
}

/// Response for action operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResponse {
    pub success: serde_json::Value,
}

/// Bridge status summary
#[derive(Debug, Clone)]
pub struct BridgeStatus {
    pub bridge_name: String,
    pub bridge_id: String,
    pub api_version: String,
    pub sw_version: String,
    pub total_lights: usize,
    pub reachable_lights: usize,
    pub suitable_lights: usize,
    pub total_scenes: usize,
    pub available_scenes: usize,
    pub max_scenes: usize,
}

impl BridgeStatus {
    /// Check if bridge is healthy
    pub fn is_healthy(&self) -> bool {
        self.reachable_lights > 0 && self.suitable_lights > 0
    }

    /// Get health score (0-100)
    pub fn health_score(&self) -> u8 {
        let mut score = 100u8;

        if self.reachable_lights == 0 {
            score = score.saturating_sub(50);
        } else if self.reachable_lights < self.total_lights / 2 {
            score = score.saturating_sub(20);
        }

        if self.suitable_lights == 0 {
            score = score.saturating_sub(30);
        } else if self.suitable_lights < self.total_lights / 2 {
            score = score.saturating_sub(10);
        }

        if self.available_scenes < 10 {
            score = score.saturating_sub(10);
        }

        score
    }

    /// Get status summary
    pub fn summary(&self) -> String {
        format!(
            "Bridge: {} ({}), API: {}, SW: {}, Lights: {}/{} reachable, Scenes: {}/{}",
            self.bridge_name,
            self.bridge_id,
            self.api_version,
            self.sw_version,
            self.reachable_lights,
            self.total_lights,
            self.total_scenes,
            self.max_scenes
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_client_creation() {
        let client = BridgeClient::new("192.168.1.100".to_string());
        assert!(client.is_ok());
    }

    #[test]
    fn test_bridge_client_with_config() {
        let client = BridgeClient::with_config("192.168.1.100".to_string(), 15, 5, 2, true);
        assert!(client.is_ok());
    }

    #[test]
    fn test_bridge_client_with_username() {
        let client = BridgeClient::new("192.168.1.100".to_string())
            .unwrap()
            .with_username("test-username".to_string());

        assert_eq!(client.username, Some("test-username".to_string()));
    }

    #[test]
    fn test_bridge_status_health() {
        let status = BridgeStatus {
            bridge_name: "Test Bridge".to_string(),
            bridge_id: "test-id".to_string(),
            api_version: "1.54.0".to_string(),
            sw_version: "1954.0".to_string(),
            total_lights: 5,
            reachable_lights: 5,
            suitable_lights: 5,
            total_scenes: 10,
            available_scenes: 190,
            max_scenes: 200,
        };

        assert!(status.is_healthy());
        assert_eq!(status.health_score(), 100);
    }

    #[test]
    fn test_bridge_status_unhealthy() {
        let status = BridgeStatus {
            bridge_name: "Test Bridge".to_string(),
            bridge_id: "test-id".to_string(),
            api_version: "1.54.0".to_string(),
            sw_version: "1954.0".to_string(),
            total_lights: 5,
            reachable_lights: 0,
            suitable_lights: 0,
            total_scenes: 10,
            available_scenes: 190,
            max_scenes: 200,
        };

        assert!(!status.is_healthy());
        assert_eq!(status.health_score(), 20); // 100 - 50 - 30
    }

    #[test]
    fn test_create_scene_response() {
        let response = CreateSceneResponse {
            success: CreateSceneSuccess {
                id: "test-scene-id".to_string(),
            },
        };

        assert_eq!(response.success.id, "test-scene-id");
    }

    #[test]
    fn test_bridge_status_summary() {
        let status = BridgeStatus {
            bridge_name: "Test Bridge".to_string(),
            bridge_id: "test-id".to_string(),
            api_version: "1.54.0".to_string(),
            sw_version: "1954.0".to_string(),
            total_lights: 5,
            reachable_lights: 4,
            suitable_lights: 3,
            total_scenes: 10,
            available_scenes: 190,
            max_scenes: 200,
        };

        let summary = status.summary();
        assert!(summary.contains("Test Bridge"));
        assert!(summary.contains("test-id"));
        assert!(summary.contains("4/5"));
        assert!(summary.contains("10/200"));
    }
}
