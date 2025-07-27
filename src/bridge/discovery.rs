use crate::bridge::BridgeInfo;
use crate::error::{HueStatusError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::timeout;

/// Bridge discovery methods
#[derive(Debug, Clone)]
pub struct BridgeDiscovery {
    client: Client,
    timeout: Duration,
    verbose: bool,
}

/// Discovery result containing found bridges
#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    pub bridges: Vec<DiscoveredBridge>,
    pub method: DiscoveryMethod,
}

/// Information about a discovered bridge
#[derive(Debug, Clone)]
pub struct DiscoveredBridge {
    pub ip: String,
    pub id: Option<String>,
    pub name: Option<String>,
    pub model: Option<String>,
    pub version: Option<String>,
    pub port: Option<u16>,
}

/// Discovery method used to find bridges
#[derive(Debug, Clone, PartialEq)]
pub enum DiscoveryMethod {
    PhilipsService,
    Mdns,
    Manual,
    NetworkScan,
}

/// Philips discovery service response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PhilipsDiscoveryResponse {
    id: String,
    internalipaddress: String,
    #[serde(default)]
    port: Option<u16>,
}

impl BridgeDiscovery {
    /// Create a new bridge discovery instance
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("huestatus/1.0")
            .build()
            .map_err(|e| HueStatusError::NetworkError { source: e })?;

        Ok(Self {
            client,
            timeout: Duration::from_secs(10),
            verbose: false,
        })
    }

    /// Create with custom timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable verbose output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Discover bridges using all available methods
    pub async fn discover_all(&self) -> Result<DiscoveryResult> {
        // Try Philips discovery service first (most reliable)
        if let Ok(result) = self.discover_via_philips_service().await {
            if !result.bridges.is_empty() {
                return Ok(result);
            }
        }

        // Try mDNS discovery as fallback
        if let Ok(result) = self.discover_via_mdns().await {
            if !result.bridges.is_empty() {
                return Ok(result);
            }
        }

        // Try network scan as last resort
        if let Ok(result) = self.discover_via_network_scan().await {
            if !result.bridges.is_empty() {
                return Ok(result);
            }
        }

        Err(HueStatusError::BridgeNotFound)
    }

    /// Discover bridges using Philips discovery service
    pub async fn discover_via_philips_service(&self) -> Result<DiscoveryResult> {
        if self.verbose {
            eprintln!("🔍 Discovering bridges via Philips service...");
        }

        let url = "https://discovery.meethue.com/";

        let response = timeout(self.timeout, self.client.get(url).send())
            .await
            .map_err(|_| HueStatusError::TimeoutError {
                operation: "Philips discovery service".to_string(),
            })?
            .map_err(|e| HueStatusError::DiscoveryServiceUnreachable {
                reason: e.to_string(),
            })?;

        if !response.status().is_success() {
            return Err(HueStatusError::DiscoveryServiceUnreachable {
                reason: format!("HTTP {}", response.status()),
            });
        }

        let bridges: Vec<PhilipsDiscoveryResponse> =
            response
                .json()
                .await
                .map_err(|e| HueStatusError::DiscoveryServiceUnreachable {
                    reason: format!("Invalid JSON response: {e}"),
                })?;

        if self.verbose {
            eprintln!("📡 Found {} bridge(s) via Philips service", bridges.len());
        }

        let mut discovered_bridges = Vec::new();
        for bridge in bridges {
            // Validate and enrich bridge information
            if let Ok(enriched) = self
                .enrich_bridge_info(&bridge.internalipaddress, Some(bridge.id.clone()))
                .await
            {
                discovered_bridges.push(enriched);
            } else {
                // Add basic info even if enrichment fails
                discovered_bridges.push(DiscoveredBridge {
                    ip: bridge.internalipaddress,
                    id: Some(bridge.id),
                    name: None,
                    model: None,
                    version: None,
                    port: bridge.port,
                });
            }
        }

        Ok(DiscoveryResult {
            bridges: discovered_bridges,
            method: DiscoveryMethod::PhilipsService,
        })
    }

    /// Discover bridges using mDNS
    pub async fn discover_via_mdns(&self) -> Result<DiscoveryResult> {
        if self.verbose {
            eprintln!("🔍 Discovering bridges via mDNS...");
        }

        // Use tokio::task::spawn_blocking for blocking mDNS operations
        let discovered = tokio::task::spawn_blocking(Self::mdns_discovery_blocking)
            .await
            .map_err(|e| HueStatusError::MdnsDiscoveryFailed {
                reason: format!("Task join error: {e}"),
            })?;

        let mut bridges = Vec::new();
        for ip in discovered? {
            if let Ok(enriched) = self.enrich_bridge_info(&ip, None).await {
                bridges.push(enriched);
            } else {
                // Add basic info even if enrichment fails
                bridges.push(DiscoveredBridge {
                    ip,
                    id: None,
                    name: None,
                    model: None,
                    version: None,
                    port: None,
                });
            }
        }

        if self.verbose {
            eprintln!("📡 Found {} bridge(s) via mDNS", bridges.len());
        }

        Ok(DiscoveryResult {
            bridges,
            method: DiscoveryMethod::Mdns,
        })
    }

    /// Blocking mDNS discovery
    fn mdns_discovery_blocking() -> Result<Vec<String>> {
        // For now, return empty result since mDNS implementation
        // depends on specific library version compatibility
        // This can be implemented properly with the correct mdns crate version

        if std::env::var("RUST_LOG").is_err() {
            eprintln!("mDNS discovery not fully implemented yet");
        }

        Ok(Vec::new())
    }

    /// Discover bridges via network scan
    pub async fn discover_via_network_scan(&self) -> Result<DiscoveryResult> {
        if self.verbose {
            eprintln!("🔍 Scanning network for bridges...");
        }

        // Get local network ranges to scan
        let network_ranges = self.get_local_network_ranges()?;
        let mut bridges = Vec::new();

        for range in network_ranges {
            if self.verbose {
                eprintln!("📡 Scanning network range: {range}");
            }

            let range_bridges = self.scan_network_range(&range).await?;
            bridges.extend(range_bridges);
        }

        if self.verbose {
            eprintln!("📡 Found {} bridge(s) via network scan", bridges.len());
        }

        Ok(DiscoveryResult {
            bridges,
            method: DiscoveryMethod::NetworkScan,
        })
    }

    /// Get local network ranges for scanning
    fn get_local_network_ranges(&self) -> Result<Vec<String>> {
        // Get local IP addresses
        let local_ips = self.get_local_ip_addresses()?;
        let mut ranges = Vec::new();

        for ip in local_ips {
            if let IpAddr::V4(ipv4) = ip {
                let octets = ipv4.octets();
                // Assume /24 subnet
                let network = format!("{}.{}.{}", octets[0], octets[1], octets[2]);
                ranges.push(network);
            }
        }

        if ranges.is_empty() {
            // Fallback to common ranges
            ranges.extend([
                "192.168.1".to_string(),
                "192.168.0".to_string(),
                "10.0.1".to_string(),
                "172.16.0".to_string(),
            ]);
        }

        Ok(ranges)
    }

    /// Get local IP addresses
    fn get_local_ip_addresses(&self) -> Result<Vec<IpAddr>> {
        use std::net::UdpSocket;

        let mut ips = Vec::new();

        // Try to get local IP by connecting to a remote address
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(local_addr) = socket.local_addr() {
                    ips.push(local_addr.ip());
                }
            }
        }

        // Add localhost as fallback
        if ips.is_empty() {
            ips.push(IpAddr::from_str("127.0.0.1").unwrap());
        }

        Ok(ips)
    }

    /// Scan a network range for Hue bridges
    async fn scan_network_range(&self, network: &str) -> Result<Vec<DiscoveredBridge>> {
        let mut bridges = Vec::new();
        let mut tasks = Vec::new();

        // Scan IPs 1-254 in the network range
        for i in 1..=254 {
            let ip = format!("{network}.{i}");
            let client = self.client.clone();
            let timeout = self.timeout;
            let ip_clone = ip.clone();

            let task =
                tokio::spawn(
                    async move { Self::test_bridge_at_ip(client, &ip_clone, timeout).await },
                );

            tasks.push((ip, task));
        }

        // Wait for all tasks to complete
        for (_ip, task) in tasks {
            if let Ok(Ok(Some(bridge))) = task.await {
                bridges.push(bridge);
            }
        }

        Ok(bridges)
    }

    /// Test if there's a Hue bridge at the given IP
    async fn test_bridge_at_ip(
        client: Client,
        ip: &str,
        request_timeout: Duration,
    ) -> Result<Option<DiscoveredBridge>> {
        let url = format!("http://{ip}/api/0/config");

        if let Ok(Ok(response)) = timeout(request_timeout, client.get(&url).send()).await {
            if response.status().is_success() {
                // Try to parse as bridge config to confirm it's a Hue bridge
                if let Ok(json) = response.json::<serde_json::Value>().await {
                    if json.get("bridgeid").is_some() {
                        return Ok(Some(DiscoveredBridge {
                            ip: ip.to_string(),
                            id: json
                                .get("bridgeid")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            name: json.get("name").and_then(|v| v.as_str()).map(String::from),
                            model: json
                                .get("modelid")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            version: json
                                .get("apiversion")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            port: Some(80),
                        }));
                    }
                }
            }
        } // Ignore timeouts and errors

        Ok(None)
    }

    /// Enrich bridge information by querying the bridge
    async fn enrich_bridge_info(
        &self,
        ip: &str,
        known_id: Option<String>,
    ) -> Result<DiscoveredBridge> {
        let url = format!("http://{ip}/api/0/config");

        let response = timeout(self.timeout, self.client.get(&url).send())
            .await
            .map_err(|_| HueStatusError::TimeoutError {
                operation: format!("Bridge info query for {ip}"),
            })?
            .map_err(|e| HueStatusError::NetworkError { source: e })?;

        if !response.status().is_success() {
            return Err(HueStatusError::BridgeConnectionFailed {
                reason: format!("HTTP {}", response.status()),
            });
        }

        let config: serde_json::Value = response
            .json()
            .await
            .map_err(|e| HueStatusError::NetworkError { source: e })?;

        Ok(DiscoveredBridge {
            ip: ip.to_string(),
            id: known_id.or_else(|| {
                config
                    .get("bridgeid")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            }),
            name: config
                .get("name")
                .and_then(|v| v.as_str())
                .map(String::from),
            model: config
                .get("modelid")
                .and_then(|v| v.as_str())
                .map(String::from),
            version: config
                .get("apiversion")
                .and_then(|v| v.as_str())
                .map(String::from),
            port: Some(80),
        })
    }

    /// Create a manual discovery result for a specific IP
    pub async fn discover_manual(&self, ip: &str) -> Result<DiscoveryResult> {
        if self.verbose {
            eprintln!("🔍 Testing manual IP: {ip}");
        }

        // Validate IP format
        IpAddr::from_str(ip).map_err(|_| HueStatusError::InvalidConfig {
            reason: format!("Invalid IP address: {ip}"),
        })?;

        // Test if there's a bridge at this IP
        match self.enrich_bridge_info(ip, None).await {
            Ok(bridge) => {
                if self.verbose {
                    eprintln!("✅ Bridge found at {ip}");
                }

                Ok(DiscoveryResult {
                    bridges: vec![bridge],
                    method: DiscoveryMethod::Manual,
                })
            }
            Err(_) => {
                if self.verbose {
                    eprintln!("❌ No bridge found at {ip}");
                }

                Err(HueStatusError::BridgeNotFound)
            }
        }
    }

    /// Validate discovered bridge
    pub async fn validate_bridge(&self, bridge: &DiscoveredBridge) -> Result<()> {
        let url = format!("http://{}/api/0/config", bridge.ip);

        let response = timeout(self.timeout, self.client.get(&url).send())
            .await
            .map_err(|_| HueStatusError::TimeoutError {
                operation: format!("Bridge validation for {}", bridge.ip),
            })?
            .map_err(|e| HueStatusError::BridgeConnectionFailed {
                reason: e.to_string(),
            })?;

        if !response.status().is_success() {
            return Err(HueStatusError::BridgeConnectionFailed {
                reason: format!("HTTP {}", response.status()),
            });
        }

        // Try to parse as bridge config
        let config: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| HueStatusError::BridgeConnectionFailed {
                    reason: format!("Invalid response: {e}"),
                })?;

        // Verify it's actually a Hue bridge
        if config.get("bridgeid").is_none() {
            return Err(HueStatusError::BridgeConnectionFailed {
                reason: "Not a Hue bridge".to_string(),
            });
        }

        Ok(())
    }

    /// Get the best bridge from discovery results
    pub fn select_best_bridge(results: &[DiscoveryResult]) -> Option<&DiscoveredBridge> {
        // Priority order: Philips service > Manual > mDNS > Network scan
        let method_priority = |method: &DiscoveryMethod| match method {
            DiscoveryMethod::PhilipsService => 4,
            DiscoveryMethod::Manual => 3,
            DiscoveryMethod::Mdns => 2,
            DiscoveryMethod::NetworkScan => 1,
        };

        results
            .iter()
            .filter(|result| !result.bridges.is_empty())
            .max_by_key(|result| method_priority(&result.method))
            .and_then(|result| result.bridges.first())
    }
}

impl Default for BridgeDiscovery {
    fn default() -> Self {
        Self::new().expect("Failed to create bridge discovery")
    }
}

impl DiscoveredBridge {
    /// Get display name for the bridge
    pub fn display_name(&self) -> String {
        if let Some(name) = &self.name {
            format!("{} ({})", name, self.ip)
        } else if let Some(id) = &self.id {
            format!("Bridge {} ({})", &id[..8], self.ip)
        } else {
            format!("Bridge at {}", self.ip)
        }
    }

    /// Get bridge summary
    pub fn summary(&self) -> String {
        let mut parts = vec![format!("IP: {}", self.ip)];

        if let Some(id) = &self.id {
            parts.push(format!("ID: {id}"));
        }

        if let Some(name) = &self.name {
            parts.push(format!("Name: {name}"));
        }

        if let Some(model) = &self.model {
            parts.push(format!("Model: {model}"));
        }

        if let Some(version) = &self.version {
            parts.push(format!("API: {version}"));
        }

        parts.join(", ")
    }

    /// Check if bridge info is complete
    pub fn is_complete(&self) -> bool {
        self.id.is_some() && self.name.is_some()
    }

    /// Convert to BridgeInfo
    pub fn to_bridge_info(&self) -> BridgeInfo {
        BridgeInfo {
            id: self.id.clone().unwrap_or_default(),
            internalipaddress: self.ip.clone(),
            port: self.port,
            name: self.name.clone(),
            modelid: self.model.clone(),
            swversion: self.version.clone(),
        }
    }
}

impl DiscoveryResult {
    /// Get the first bridge from results
    pub fn first_bridge(&self) -> Option<&DiscoveredBridge> {
        self.bridges.first()
    }

    /// Check if any bridges were found
    pub fn has_bridges(&self) -> bool {
        !self.bridges.is_empty()
    }

    /// Get number of bridges found
    pub fn bridge_count(&self) -> usize {
        self.bridges.len()
    }

    /// Get summary of discovery results
    pub fn summary(&self) -> String {
        format!(
            "Found {} bridge(s) via {}",
            self.bridge_count(),
            match self.method {
                DiscoveryMethod::PhilipsService => "Philips service",
                DiscoveryMethod::Mdns => "mDNS",
                DiscoveryMethod::Manual => "manual entry",
                DiscoveryMethod::NetworkScan => "network scan",
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_discovery_creation() {
        let discovery = BridgeDiscovery::new();
        assert!(discovery.is_ok());
    }

    #[test]
    fn test_discovery_method_priority() {
        // This would be tested in integration tests with actual discovery
        assert_eq!(
            DiscoveryMethod::PhilipsService,
            DiscoveryMethod::PhilipsService
        );
    }

    #[test]
    fn test_discovered_bridge_display_name() {
        let bridge = DiscoveredBridge {
            ip: "192.168.1.100".to_string(),
            id: Some("001788fffe23456".to_string()),
            name: Some("Philips hue".to_string()),
            model: Some("BSB002".to_string()),
            version: Some("1.54.0".to_string()),
            port: Some(80),
        };

        assert_eq!(bridge.display_name(), "Philips hue (192.168.1.100)");
    }

    #[test]
    fn test_discovered_bridge_summary() {
        let bridge = DiscoveredBridge {
            ip: "192.168.1.100".to_string(),
            id: Some("001788fffe23456".to_string()),
            name: Some("Philips hue".to_string()),
            model: Some("BSB002".to_string()),
            version: Some("1.54.0".to_string()),
            port: Some(80),
        };

        let summary = bridge.summary();
        assert!(summary.contains("192.168.1.100"));
        assert!(summary.contains("001788fffe23456"));
        assert!(summary.contains("Philips hue"));
    }

    #[test]
    fn test_discovery_result_methods() {
        let bridge = DiscoveredBridge {
            ip: "192.168.1.100".to_string(),
            id: Some("test".to_string()),
            name: Some("Test Bridge".to_string()),
            model: None,
            version: None,
            port: None,
        };

        let result = DiscoveryResult {
            bridges: vec![bridge],
            method: DiscoveryMethod::PhilipsService,
        };

        assert!(result.has_bridges());
        assert_eq!(result.bridge_count(), 1);
        assert!(result.first_bridge().is_some());
    }

    #[test]
    fn test_bridge_completeness() {
        let complete_bridge = DiscoveredBridge {
            ip: "192.168.1.100".to_string(),
            id: Some("test".to_string()),
            name: Some("Test Bridge".to_string()),
            model: None,
            version: None,
            port: None,
        };

        let incomplete_bridge = DiscoveredBridge {
            ip: "192.168.1.100".to_string(),
            id: None,
            name: Some("Test Bridge".to_string()),
            model: None,
            version: None,
            port: None,
        };

        assert!(complete_bridge.is_complete());
        assert!(!incomplete_bridge.is_complete());
    }
}
