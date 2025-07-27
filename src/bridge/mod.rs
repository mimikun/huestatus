use crate::error::{HueStatusError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod auth;
pub mod client;
pub mod discovery;

pub use auth::*;
pub use client::*;
pub use discovery::*;

/// Hue API response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HueResponse<T> {
    Success(T),
    Error(HueError),
    ErrorArray(Vec<HueError>),
}

/// Hue API error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HueError {
    pub error: HueErrorDetails,
}

/// Hue API error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HueErrorDetails {
    #[serde(rename = "type")]
    pub error_type: u16,
    pub address: String,
    pub description: String,
}

/// Bridge information from discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeInfo {
    pub id: String,
    pub internalipaddress: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub modelid: Option<String>,
    #[serde(default)]
    pub swversion: Option<String>,
}

/// Bridge capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeCapabilities {
    pub lights: CapabilityLimits,
    pub sensors: CapabilityLimits,
    pub groups: CapabilityLimits,
    pub scenes: CapabilityLimits,
    pub rules: CapabilityLimits,
    pub schedules: CapabilityLimits,
    pub resourcelinks: CapabilityLimits,
    pub streaming: Option<StreamingCapabilities>,
    pub timezones: Vec<String>,
}

/// Capability limits for different resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityLimits {
    pub available: usize,
    pub total: usize,
}

/// Streaming capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingCapabilities {
    pub available: usize,
    pub total: usize,
    pub channels: usize,
}

/// Light information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Light {
    pub name: String,
    pub state: LightState,
    #[serde(rename = "type")]
    pub light_type: String,
    pub modelid: String,
    pub manufacturername: String,
    pub productname: Option<String>,
    pub capabilities: Option<LightCapabilities>,
    pub config: Option<LightConfig>,
    pub swversion: Option<String>,
    pub swconfigid: Option<String>,
    pub productid: Option<String>,
}

/// Light state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightState {
    pub on: bool,
    pub bri: Option<u8>,
    pub hue: Option<u16>,
    pub sat: Option<u8>,
    pub effect: Option<String>,
    pub xy: Option<[f64; 2]>,
    pub ct: Option<u16>,
    pub alert: Option<String>,
    pub colormode: Option<String>,
    pub mode: Option<String>,
    pub reachable: Option<bool>,
}

/// Light capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightCapabilities {
    pub certified: bool,
    pub control: LightControl,
    pub streaming: Option<StreamingCapability>,
}

/// Light control capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightControl {
    pub mindimlevel: Option<u16>,
    pub maxlumen: Option<u16>,
    pub colorgamuttype: Option<String>,
    pub colorgamut: Option<[[f64; 2]; 3]>,
    pub ct: Option<ColorTemperatureCapability>,
}

/// Color temperature capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorTemperatureCapability {
    pub min: u16,
    pub max: u16,
}

/// Streaming capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingCapability {
    pub renderer: bool,
    pub proxy: bool,
}

/// Light configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightConfig {
    pub archetype: String,
    pub function: String,
    pub direction: String,
    pub startup: Option<StartupConfig>,
}

/// Startup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupConfig {
    pub mode: String,
    pub configured: bool,
    pub customsettings: Option<serde_json::Value>,
}

/// Scene information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
    pub lights: Vec<String>,
    pub owner: String,
    pub recycle: bool,
    pub locked: bool,
    pub appdata: Option<serde_json::Value>,
    pub picture: Option<String>,
    pub image: Option<String>,
    pub lastupdated: String,
    pub version: u8,
    pub lightstates: Option<HashMap<String, LightState>>,
}

/// Scene creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSceneRequest {
    pub name: String,
    pub lights: Vec<String>,
    pub recycle: bool,
    pub lightstates: HashMap<String, LightState>,
}

/// Scene action request (for executing scenes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneActionRequest {
    pub scene: String,
}

/// Group information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub lights: Vec<String>,
    #[serde(rename = "type")]
    pub group_type: String,
    pub state: GroupState,
    pub recycle: bool,
    pub action: GroupAction,
    pub sensors: Option<Vec<String>>,
}

/// Group state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupState {
    pub all_on: bool,
    pub any_on: bool,
}

/// Group action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupAction {
    pub on: Option<bool>,
    pub bri: Option<u8>,
    pub hue: Option<u16>,
    pub sat: Option<u8>,
    pub effect: Option<String>,
    pub xy: Option<[f64; 2]>,
    pub ct: Option<u16>,
    pub alert: Option<String>,
    pub colormode: Option<String>,
    pub scene: Option<String>,
}

/// Bridge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfiguration {
    pub name: String,
    pub zigbeechannel: u8,
    pub bridgeid: String,
    pub mac: String,
    pub dhcp: bool,
    pub ipaddress: String,
    pub netmask: String,
    pub gateway: String,
    pub proxyaddress: String,
    pub proxyport: u16,
    #[serde(rename = "UTC")]
    pub utc: String,
    pub localtime: String,
    pub timezone: String,
    pub modelid: String,
    pub datastoreversion: String,
    pub swversion: String,
    pub apiversion: String,
    pub swupdate: SwUpdate,
    pub swupdate2: SwUpdate2,
    pub linkbutton: bool,
    pub portalservices: bool,
    pub portalconnection: String,
    pub portalstate: PortalState,
    pub internetservices: InternetServices,
    pub factorynew: bool,
    pub replacesbridgeid: Option<String>,
    pub backup: Backup,
    pub starterkitid: String,
    pub whitelist: HashMap<String, WhitelistEntry>,
}

/// Software update information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwUpdate {
    pub updatestate: u8,
    pub checkforupdate: bool,
    pub devicetypes: DeviceTypes,
    pub url: String,
    pub text: String,
    pub notify: bool,
}

/// Software update 2 information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwUpdate2 {
    pub checkforupdate: bool,
    pub lastchange: String,
    pub bridge: BridgeUpdateInfo,
    pub state: String,
    pub autoinstall: AutoInstall,
}

/// Device types for updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTypes {
    pub bridge: bool,
    pub lights: Vec<String>,
    pub sensors: Vec<String>,
}

/// Bridge update information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeUpdateInfo {
    pub state: String,
    pub lastinstall: String,
}

/// Auto install configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoInstall {
    pub updatetime: String,
    pub on: bool,
}

/// Portal state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalState {
    pub signedon: bool,
    pub incoming: bool,
    pub outgoing: bool,
    pub communication: String,
}

/// Internet services configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternetServices {
    pub internet: String,
    pub remoteaccess: String,
    pub time: String,
    pub swupdate: String,
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    pub status: String,
    pub errorcode: u8,
}

/// Whitelist entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhitelistEntry {
    #[serde(rename = "last use date")]
    pub last_use_date: String,
    #[serde(rename = "create date")]
    pub create_date: String,
    pub name: String,
}

impl<T> HueResponse<T> {
    /// Check if response is successful
    pub fn is_success(&self) -> bool {
        matches!(self, HueResponse::Success(_))
    }

    /// Check if response is an error
    pub fn is_error(&self) -> bool {
        matches!(self, HueResponse::Error(_) | HueResponse::ErrorArray(_))
    }

    /// Extract success value or convert to error
    pub fn into_result(self) -> Result<T> {
        match self {
            HueResponse::Success(value) => Ok(value),
            HueResponse::Error(error) => Err(error.into()),
            HueResponse::ErrorArray(errors) => {
                if let Some(error) = errors.into_iter().next() {
                    Err(error.into())
                } else {
                    Err(HueStatusError::ApiError {
                        message: "Unknown API error".to_string(),
                    })
                }
            }
        }
    }

    /// Extract success value or return None
    pub fn success(self) -> Option<T> {
        match self {
            HueResponse::Success(value) => Some(value),
            _ => None,
        }
    }

    /// Extract error or return None
    pub fn error(self) -> Option<HueError> {
        match self {
            HueResponse::Error(error) => Some(error),
            HueResponse::ErrorArray(errors) => errors.into_iter().next(),
            _ => None,
        }
    }
}

impl From<HueError> for HueStatusError {
    fn from(error: HueError) -> Self {
        match error.error.error_type {
            1 => HueStatusError::AuthenticationFailed,
            101 => HueStatusError::LinkButtonNotPressed,
            3 => HueStatusError::InvalidConfig {
                reason: format!("Resource not available: {}", error.error.description),
            },
            4 => HueStatusError::InvalidConfig {
                reason: format!("Method not available: {}", error.error.description),
            },
            5 => HueStatusError::InvalidConfig {
                reason: format!("Missing parameter: {}", error.error.description),
            },
            6 => HueStatusError::InvalidConfig {
                reason: format!("Parameter not available: {}", error.error.description),
            },
            7 => HueStatusError::InvalidConfig {
                reason: format!("Invalid value: {}", error.error.description),
            },
            8 => HueStatusError::InvalidConfig {
                reason: format!("Parameter not modifiable: {}", error.error.description),
            },
            11 => HueStatusError::ApiError {
                message: "Too many items in list".to_string(),
            },
            12 => portal_connection_required_error(),
            _ => HueStatusError::ApiError {
                message: format!(
                    "API error {}: {}",
                    error.error.error_type, error.error.description
                ),
            },
        }
    }
}

impl HueError {
    /// Check if error is related to authentication
    pub fn is_auth_error(&self) -> bool {
        matches!(self.error.error_type, 1 | 101)
    }

    /// Check if error is related to link button
    pub fn is_link_button_error(&self) -> bool {
        self.error.error_type == 101
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(self.error.error_type, 101 | 11)
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self.error.error_type {
            1 => "Authentication failed. Please run 'huestatus --setup' to re-authenticate."
                .to_string(),
            101 => "Link button not pressed. Press the button on your Hue bridge and try again."
                .to_string(),
            3 => format!("Resource not available: {}", self.error.description),
            4 => format!("Method not available: {}", self.error.description),
            5 => format!("Missing parameter: {}", self.error.description),
            6 => format!("Parameter not available: {}", self.error.description),
            7 => format!("Invalid value: {}", self.error.description),
            8 => format!("Parameter not modifiable: {}", self.error.description),
            11 => "Too many items in list".to_string(),
            12 => "Portal connection required".to_string(),
            _ => format!(
                "API error {}: {}",
                self.error.error_type, self.error.description
            ),
        }
    }
}

impl Light {
    /// Check if light supports color
    pub fn supports_color(&self) -> bool {
        self.capabilities
            .as_ref()
            .and_then(|c| c.control.colorgamut.as_ref())
            .is_some()
    }

    /// Check if light supports color temperature
    pub fn supports_color_temperature(&self) -> bool {
        self.capabilities
            .as_ref()
            .and_then(|c| c.control.ct.as_ref())
            .is_some()
    }

    /// Check if light is reachable
    pub fn is_reachable(&self) -> bool {
        self.state.reachable.unwrap_or(false)
    }

    /// Check if light is on
    pub fn is_on(&self) -> bool {
        self.state.on
    }

    /// Get light brightness (0-254)
    pub fn brightness(&self) -> u8 {
        self.state.bri.unwrap_or(0)
    }

    /// Get light hue (0-65535)
    pub fn hue(&self) -> u16 {
        self.state.hue.unwrap_or(0)
    }

    /// Get light saturation (0-254)
    pub fn saturation(&self) -> u8 {
        self.state.sat.unwrap_or(0)
    }

    /// Get light XY coordinates
    pub fn xy(&self) -> Option<[f64; 2]> {
        self.state.xy
    }

    /// Get light color temperature
    pub fn color_temperature(&self) -> Option<u16> {
        self.state.ct
    }

    /// Check if light is suitable for status indication
    pub fn is_suitable_for_status(&self) -> bool {
        self.is_reachable() && (self.supports_color() || self.supports_color_temperature())
    }

    /// Check if the light supports a specific effect
    pub fn supports_effect(&self, effect: &str) -> bool {
        if let Some(_capabilities) = &self.capabilities {
            match effect {
                "colorloop" => self.supports_color(),
                "none" => true,
                _ => false,
            }
        } else {
            false
        }
    }
}

impl Scene {
    /// Check if scene is recycle-able
    pub fn is_recyclable(&self) -> bool {
        self.recycle
    }

    /// Check if scene is locked
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Get scene light count
    pub fn light_count(&self) -> usize {
        self.lights.len()
    }

    /// Check if scene has lightstates
    pub fn has_lightstates(&self) -> bool {
        self.lightstates.is_some()
    }

    /// Get scene lightstates
    pub fn get_lightstates(&self) -> Option<&HashMap<String, LightState>> {
        self.lightstates.as_ref()
    }

    /// Check if scene is suitable for status indication
    pub fn is_suitable_for_status(&self) -> bool {
        !self.is_locked() && self.light_count() > 0
    }
}

impl CreateSceneRequest {
    /// Create a new scene request for success status (green)
    pub fn new_success_scene(name: String, lights: Vec<String>) -> Self {
        let mut lightstates = HashMap::new();

        for light_id in &lights {
            lightstates.insert(
                light_id.clone(),
                LightState {
                    on: true,
                    bri: Some(254),
                    hue: Some(21845), // Green: 120° × 65536/360°
                    sat: Some(254),
                    effect: None,
                    xy: None,
                    ct: None,
                    alert: None,
                    colormode: Some("hs".to_string()),
                    mode: None,
                    reachable: None,
                },
            );
        }

        Self {
            name,
            lights,
            recycle: true,
            lightstates,
        }
    }

    /// Create a new scene request for failure status (red)
    pub fn new_failure_scene(name: String, lights: Vec<String>) -> Self {
        let mut lightstates = HashMap::new();

        for light_id in &lights {
            lightstates.insert(
                light_id.clone(),
                LightState {
                    on: true,
                    bri: Some(254),
                    hue: Some(0), // Red: 0°
                    sat: Some(254),
                    effect: None,
                    xy: None,
                    ct: None,
                    alert: None,
                    colormode: Some("hs".to_string()),
                    mode: None,
                    reachable: None,
                },
            );
        }

        Self {
            name,
            lights,
            recycle: true,
            lightstates,
        }
    }

    /// Create a new scene request with custom color
    pub fn new_custom_scene(name: String, lights: Vec<String>, hue: u16, sat: u8, bri: u8) -> Self {
        let mut lightstates = HashMap::new();

        for light_id in &lights {
            lightstates.insert(
                light_id.clone(),
                LightState {
                    on: true,
                    bri: Some(bri),
                    hue: Some(hue),
                    sat: Some(sat),
                    effect: None,
                    xy: None,
                    ct: None,
                    alert: None,
                    colormode: Some("hs".to_string()),
                    mode: None,
                    reachable: None,
                },
            );
        }

        Self {
            name,
            lights,
            recycle: true,
            lightstates,
        }
    }

    /// Validate scene request
    pub fn validate(&self) -> Result<()> {
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

        if self.lightstates.is_empty() {
            return Err(HueStatusError::InvalidSceneData {
                reason: "Scene must have light states".to_string(),
            });
        }

        // Validate that all lights have corresponding lightstates
        for light_id in &self.lights {
            if !self.lightstates.contains_key(light_id) {
                return Err(HueStatusError::InvalidSceneData {
                    reason: format!("Light {light_id} has no corresponding light state"),
                });
            }
        }

        Ok(())
    }
}

impl SceneActionRequest {
    /// Create a new scene action request
    pub fn new(scene_id: String) -> Self {
        Self { scene: scene_id }
    }
}

impl LightState {
    /// Create a new light state for success status (green)
    pub fn new_success_state() -> Self {
        Self {
            on: true,
            bri: Some(254),
            hue: Some(21845), // Green
            sat: Some(254),
            effect: None,
            xy: None,
            ct: None,
            alert: None,
            colormode: Some("hs".to_string()),
            mode: None,
            reachable: None,
        }
    }

    /// Create a new light state for failure status (red)
    pub fn new_failure_state() -> Self {
        Self {
            on: true,
            bri: Some(254),
            hue: Some(0), // Red
            sat: Some(254),
            effect: None,
            xy: None,
            ct: None,
            alert: None,
            colormode: Some("hs".to_string()),
            mode: None,
            reachable: None,
        }
    }

    /// Create a new light state with custom color
    pub fn new_custom_state(hue: u16, sat: u8, bri: u8) -> Self {
        Self {
            on: true,
            bri: Some(bri),
            hue: Some(hue),
            sat: Some(sat),
            effect: None,
            xy: None,
            ct: None,
            alert: None,
            colormode: Some("hs".to_string()),
            mode: None,
            reachable: None,
        }
    }

    /// Validate light state
    pub fn validate(&self) -> Result<()> {
        if let Some(bri) = self.bri {
            if bri == 0 {
                return Err(HueStatusError::InvalidSceneData {
                    reason: "Brightness cannot be 0 (use on: false instead)".to_string(),
                });
            }
        }

        // Hue validation is implicit since u16 max is 65535

        if let Some(sat) = self.sat {
            if sat > 254 {
                return Err(HueStatusError::InvalidSceneData {
                    reason: "Saturation value must be between 0 and 254".to_string(),
                });
            }
        }

        Ok(())
    }
}

// Add missing error variant to HueStatusError
/// Helper function to create PortalConnectionRequired error
pub fn portal_connection_required_error() -> HueStatusError {
    HueStatusError::ApiError {
        message: "Portal connection required".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hue_response_success() {
        let response: HueResponse<String> = HueResponse::Success("test".to_string());
        assert!(response.is_success());
        assert!(!response.is_error());
        assert_eq!(response.success(), Some("test".to_string()));
    }

    #[test]
    fn test_hue_response_error() {
        let error = HueError {
            error: HueErrorDetails {
                error_type: 1,
                address: "/test".to_string(),
                description: "Test error".to_string(),
            },
        };
        let response: HueResponse<String> = HueResponse::Error(error);
        assert!(!response.is_success());
        assert!(response.is_error());
        assert!(response.error().is_some());
    }

    #[test]
    fn test_create_scene_request_success() {
        let lights = vec!["1".to_string(), "2".to_string()];
        let scene = CreateSceneRequest::new_success_scene("test-scene".to_string(), lights.clone());

        assert_eq!(scene.name, "test-scene");
        assert_eq!(scene.lights, lights);
        assert!(scene.recycle);
        assert_eq!(scene.lightstates.len(), 2);

        // Check that all lights have green color
        for light_id in &lights {
            let state = scene.lightstates.get(light_id).unwrap();
            assert_eq!(state.hue, Some(21845)); // Green
            assert_eq!(state.sat, Some(254));
            assert_eq!(state.bri, Some(254));
            assert!(state.on);
        }
    }

    #[test]
    fn test_create_scene_request_failure() {
        let lights = vec!["1".to_string(), "2".to_string()];
        let scene = CreateSceneRequest::new_failure_scene("test-scene".to_string(), lights.clone());

        assert_eq!(scene.name, "test-scene");
        assert_eq!(scene.lights, lights);
        assert!(scene.recycle);
        assert_eq!(scene.lightstates.len(), 2);

        // Check that all lights have red color
        for light_id in &lights {
            let state = scene.lightstates.get(light_id).unwrap();
            assert_eq!(state.hue, Some(0)); // Red
            assert_eq!(state.sat, Some(254));
            assert_eq!(state.bri, Some(254));
            assert!(state.on);
        }
    }

    #[test]
    fn test_light_state_validation() {
        let mut state = LightState::new_success_state();
        assert!(state.validate().is_ok());

        state.bri = Some(0);
        assert!(state.validate().is_err());

        state.bri = Some(254);
        state.hue = Some(65535);
        assert!(state.validate().is_ok());

        state.hue = Some(0);
        state.sat = Some(255);
        assert!(state.validate().is_err());
    }

    #[test]
    fn test_scene_request_validation() {
        let lights = vec!["1".to_string()];
        let scene = CreateSceneRequest::new_success_scene("test".to_string(), lights);
        assert!(scene.validate().is_ok());

        let empty_scene = CreateSceneRequest::new_success_scene("".to_string(), vec![]);
        assert!(empty_scene.validate().is_err());
    }

    #[test]
    fn test_hue_error_types() {
        let auth_error = HueError {
            error: HueErrorDetails {
                error_type: 1,
                address: "/test".to_string(),
                description: "Unauthorized".to_string(),
            },
        };
        assert!(auth_error.is_auth_error());

        let button_error = HueError {
            error: HueErrorDetails {
                error_type: 101,
                address: "/test".to_string(),
                description: "Link button not pressed".to_string(),
            },
        };
        assert!(button_error.is_link_button_error());
        assert!(button_error.is_recoverable());
    }
}
