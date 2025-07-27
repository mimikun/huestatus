use crate::bridge::{BridgeClient, HueError};
use crate::error::{HueStatusError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::{interval, sleep, timeout, Instant};

/// Authentication manager for Hue Bridge
#[derive(Debug, Clone)]
pub struct BridgeAuth {
    client: Client,
    bridge_ip: String,
    timeout: Duration,
    verbose: bool,
}

/// Authentication request payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub devicetype: String,
}

/// Authentication success response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSuccess {
    pub username: String,
}

/// Authentication response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub success: AuthSuccess,
}

/// Authentication result
#[derive(Debug, Clone)]
pub struct AuthResult {
    pub username: String,
    pub device_type: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Authentication status during the process
#[derive(Debug, Clone, PartialEq)]
pub enum AuthStatus {
    WaitingForButton,
    ButtonPressed,
    Success(String),
    Timeout,
    Error(String),
}

impl BridgeAuth {
    /// Create a new authentication manager
    pub fn new(bridge_ip: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("huestatus/1.0")
            .build()
            .map_err(|e| HueStatusError::NetworkError { source: e })?;

        Ok(Self {
            client,
            bridge_ip,
            timeout: Duration::from_secs(30),
            verbose: false,
        })
    }

    /// Set authentication timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable verbose output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Authenticate with the bridge using link button
    pub async fn authenticate(&self, app_name: &str, instance_name: &str) -> Result<AuthResult> {
        let device_type = format!("{app_name}#{instance_name}");

        if self.verbose {
            eprintln!(
                "🔑 Starting authentication with device type: {device_type}"
            );
            eprintln!("👆 Press the link button on your Hue bridge now!");
        }

        let start_time = Instant::now();
        let mut poll_interval = interval(Duration::from_secs(1));

        loop {
            // Check if we've exceeded the timeout
            if start_time.elapsed() > self.timeout {
                if self.verbose {
                    eprintln!(
                        "⏰ Authentication timed out after {} seconds",
                        self.timeout.as_secs()
                    );
                }
                return Err(HueStatusError::TimeoutError {
                    operation: "Authentication".to_string(),
                });
            }

            // Wait for next poll interval
            poll_interval.tick().await;

            // Try to authenticate
            match self.try_authenticate(&device_type).await {
                Ok(username) => {
                    if self.verbose {
                        eprintln!("✅ Authentication successful! Username: {username}");
                    }

                    return Ok(AuthResult {
                        username,
                        device_type,
                        created_at: chrono::Utc::now(),
                    });
                }
                Err(HueStatusError::LinkButtonNotPressed) => {
                    // Continue polling
                    if self.verbose {
                        let remaining = self
                            .timeout
                            .as_secs()
                            .saturating_sub(start_time.elapsed().as_secs());
                        eprintln!(
                            "⏳ Waiting for button press... ({remaining} seconds remaining)"
                        );
                    }
                    continue;
                }
                Err(e) => {
                    if self.verbose {
                        eprintln!("❌ Authentication error: {e}");
                    }
                    return Err(e);
                }
            }
        }
    }

    /// Try to authenticate once
    async fn try_authenticate(&self, device_type: &str) -> Result<String> {
        let url = format!("http://{}/api", self.bridge_ip);
        let request = AuthRequest {
            devicetype: device_type.to_string(),
        };

        if self.verbose {
            eprintln!("📡 POST {url} with devicetype: {device_type}");
        }

        let response = timeout(
            Duration::from_secs(10),
            self.client.post(&url).json(&request).send(),
        )
        .await
        .map_err(|_| HueStatusError::TimeoutError {
            operation: "Authentication request".to_string(),
        })?
        .map_err(|e| HueStatusError::NetworkError { source: e })?;

        let response_text = response
            .text()
            .await
            .map_err(|e| HueStatusError::NetworkError { source: e })?;

        if self.verbose {
            eprintln!("📥 Response: {response_text}");
        }

        // Parse response as array
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&response_text)
            .map_err(|e| HueStatusError::JsonError { source: e })?;

        if parsed.is_empty() {
            return Err(HueStatusError::ApiError {
                message: "Empty response from bridge".to_string(),
            });
        }

        let first_item = &parsed[0];

        // Check for success
        if let Some(success) = first_item.get("success") {
            if let Some(username) = success.get("username").and_then(|u| u.as_str()) {
                return Ok(username.to_string());
            }
        }

        // Check for error
        if let Some(error) = first_item.get("error") {
            let hue_error: HueError = serde_json::from_value(error.clone())
                .map_err(|e| HueStatusError::JsonError { source: e })?;

            return Err(hue_error.into());
        }

        Err(HueStatusError::ApiError {
            message: "Unexpected response format".to_string(),
        })
    }

    /// Authenticate with callback for status updates
    pub async fn authenticate_with_callback<F>(
        &self,
        app_name: &str,
        instance_name: &str,
        callback: F,
    ) -> Result<AuthResult>
    where
        F: Fn(AuthStatus) + Send + Sync,
    {
        let device_type = format!("{app_name}#{instance_name}");

        callback(AuthStatus::WaitingForButton);

        if self.verbose {
            eprintln!(
                "🔑 Starting authentication with device type: {device_type}"
            );
        }

        let start_time = Instant::now();
        let mut poll_interval = interval(Duration::from_secs(1));

        loop {
            // Check if we've exceeded the timeout
            if start_time.elapsed() > self.timeout {
                callback(AuthStatus::Timeout);
                return Err(HueStatusError::TimeoutError {
                    operation: "Authentication".to_string(),
                });
            }

            // Wait for next poll interval
            poll_interval.tick().await;

            // Try to authenticate
            match self.try_authenticate(&device_type).await {
                Ok(username) => {
                    callback(AuthStatus::Success(username.clone()));

                    return Ok(AuthResult {
                        username,
                        device_type,
                        created_at: chrono::Utc::now(),
                    });
                }
                Err(HueStatusError::LinkButtonNotPressed) => {
                    // Continue polling - no callback needed as status hasn't changed
                    continue;
                }
                Err(e) => {
                    callback(AuthStatus::Error(e.to_string()));
                    return Err(e);
                }
            }
        }
    }

    /// Test if authentication credentials are valid
    pub async fn test_authentication(&self, username: &str) -> Result<()> {
        if self.verbose {
            eprintln!("🔍 Testing authentication for user: {username}");
        }

        let url = format!("http://{}/api/{}/config", self.bridge_ip, username);

        let response = timeout(Duration::from_secs(10), self.client.get(&url).send())
            .await
            .map_err(|_| HueStatusError::TimeoutError {
                operation: "Authentication test".to_string(),
            })?
            .map_err(|e| HueStatusError::NetworkError { source: e })?;

        if !response.status().is_success() {
            return Err(HueStatusError::AuthenticationFailed);
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| HueStatusError::NetworkError { source: e })?;

        // Check if response contains an error
        if let Ok(errors) = serde_json::from_value::<Vec<HueError>>(json.clone()) {
            if !errors.is_empty() {
                return Err(errors[0].clone().into());
            }
        }

        // Check if response contains bridge config (indicating success)
        if json.get("bridgeid").is_none() {
            return Err(HueStatusError::AuthenticationFailed);
        }

        if self.verbose {
            eprintln!("✅ Authentication test successful");
        }

        Ok(())
    }

    /// Get authentication status without trying to authenticate
    pub async fn get_auth_status(&self, username: &str) -> AuthStatus {
        match self.test_authentication(username).await {
            Ok(()) => AuthStatus::Success(username.to_string()),
            Err(HueStatusError::AuthenticationFailed) => {
                AuthStatus::Error("Invalid credentials".to_string())
            }
            Err(HueStatusError::LinkButtonNotPressed) => AuthStatus::WaitingForButton,
            Err(e) => AuthStatus::Error(e.to_string()),
        }
    }

    /// Create authenticated bridge client
    pub fn create_authenticated_client(&self, username: String) -> Result<BridgeClient> {
        BridgeClient::new(self.bridge_ip.clone())?
            .with_username(username)
            .with_verbose(self.verbose)
            .pipe(Ok)
    }

    /// Interactive authentication with user prompts
    pub async fn authenticate_interactive(
        &self,
        app_name: &str,
        instance_name: &str,
    ) -> Result<AuthResult> {
        println!("🔑 Authentication Required");
        println!();
        println!("To authenticate with your Hue bridge:");
        println!("1. Press the large button on top of your Hue bridge");
        println!("2. Wait for the button to start blinking");
        println!("3. Press Enter to continue");
        println!();
        print!("Press the bridge button now and then press Enter...");

        // Wait for user to press Enter
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| HueStatusError::IoError { source: e })?;

        println!("🔍 Attempting authentication...");

        // Give user a moment to press the button
        sleep(Duration::from_secs(1)).await;

        let result = self
            .authenticate_with_callback(app_name, instance_name, |status| match status {
                AuthStatus::WaitingForButton => {
                    print!("⏳ Waiting for button press...");
                    std::io::Write::flush(&mut std::io::stdout()).ok();
                }
                AuthStatus::Success(_) => {
                    println!("\n✅ Authentication successful!");
                }
                AuthStatus::Timeout => {
                    println!("\n⏰ Authentication timed out");
                }
                AuthStatus::Error(err) => {
                    println!("\n❌ Authentication failed: {err}");
                }
                _ => {}
            })
            .await;

        match &result {
            Ok(_) => println!("🎉 You can now use huestatus!"),
            Err(_) => println!("😞 Authentication failed. Please try again."),
        }

        result
    }

    /// Quick authentication attempt (single try)
    pub async fn quick_authenticate(
        &self,
        app_name: &str,
        instance_name: &str,
    ) -> Result<AuthResult> {
        let device_type = format!("{app_name}#{instance_name}");

        match self.try_authenticate(&device_type).await {
            Ok(username) => Ok(AuthResult {
                username,
                device_type,
                created_at: chrono::Utc::now(),
            }),
            Err(e) => Err(e),
        }
    }

    /// Check if bridge is accessible
    pub async fn check_bridge_accessibility(&self) -> Result<()> {
        let url = format!("http://{}/api/0/config", self.bridge_ip);

        if self.verbose {
            eprintln!("🔍 Checking bridge accessibility at {}", self.bridge_ip);
        }

        timeout(Duration::from_secs(5), self.client.get(&url).send())
            .await
            .map_err(|_| HueStatusError::TimeoutError {
                operation: "Bridge accessibility check".to_string(),
            })?
            .map_err(|e| HueStatusError::BridgeConnectionFailed {
                reason: e.to_string(),
            })?;

        if self.verbose {
            eprintln!("✅ Bridge is accessible");
        }

        Ok(())
    }
}

impl AuthResult {
    /// Get age of authentication
    pub fn age(&self) -> chrono::Duration {
        chrono::Utc::now().signed_duration_since(self.created_at)
    }

    /// Check if authentication is recent (within last hour)
    pub fn is_recent(&self) -> bool {
        self.age() < chrono::Duration::hours(1)
    }

    /// Check if authentication is old (older than 30 days)
    pub fn is_old(&self) -> bool {
        self.age() > chrono::Duration::days(30)
    }

    /// Get formatted age string
    pub fn age_string(&self) -> String {
        let age = self.age();

        if age < chrono::Duration::minutes(1) {
            "just now".to_string()
        } else if age < chrono::Duration::hours(1) {
            format!("{} minutes ago", age.num_minutes())
        } else if age < chrono::Duration::days(1) {
            format!("{} hours ago", age.num_hours())
        } else {
            format!("{} days ago", age.num_days())
        }
    }

    /// Get summary of authentication result
    pub fn summary(&self) -> String {
        format!(
            "User: {}, Device: {}, Created: {}",
            self.username,
            self.device_type,
            self.age_string()
        )
    }
}

impl std::fmt::Display for AuthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthStatus::WaitingForButton => write!(f, "Waiting for button press"),
            AuthStatus::ButtonPressed => write!(f, "Button pressed"),
            AuthStatus::Success(username) => write!(f, "Success ({username})"),
            AuthStatus::Timeout => write!(f, "Timeout"),
            AuthStatus::Error(err) => write!(f, "Error: {err}"),
        }
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
    fn test_bridge_auth_creation() {
        let auth = BridgeAuth::new("192.168.1.100".to_string());
        assert!(auth.is_ok());
    }

    #[test]
    fn test_auth_request_serialization() {
        let request = AuthRequest {
            devicetype: "huestatus#test".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("huestatus#test"));
    }

    #[test]
    fn test_auth_result_age() {
        let result = AuthResult {
            username: "test-user".to_string(),
            device_type: "huestatus#test".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::minutes(30),
        };

        assert!(result.is_recent()); // 30 minutes is within 1 hour
        assert!(!result.is_old());
        assert!(result.age_string().contains("minutes ago"));
    }

    #[test]
    fn test_auth_result_recent() {
        let result = AuthResult {
            username: "test-user".to_string(),
            device_type: "huestatus#test".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::minutes(10),
        };

        assert!(result.is_recent());
        assert!(!result.is_old());
    }

    #[test]
    fn test_auth_result_old() {
        let result = AuthResult {
            username: "test-user".to_string(),
            device_type: "huestatus#test".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::days(45),
        };

        assert!(!result.is_recent());
        assert!(result.is_old());
    }

    #[test]
    fn test_auth_status_display() {
        assert_eq!(
            AuthStatus::WaitingForButton.to_string(),
            "Waiting for button press"
        );
        assert_eq!(AuthStatus::Timeout.to_string(), "Timeout");
        assert_eq!(
            AuthStatus::Success("test-user".to_string()).to_string(),
            "Success (test-user)"
        );
        assert_eq!(
            AuthStatus::Error("Test error".to_string()).to_string(),
            "Error: Test error"
        );
    }

    #[test]
    fn test_auth_result_summary() {
        let result = AuthResult {
            username: "test-user".to_string(),
            device_type: "huestatus#test".to_string(),
            created_at: chrono::Utc::now(),
        };

        let summary = result.summary();
        assert!(summary.contains("test-user"));
        assert!(summary.contains("huestatus#test"));
    }
}
