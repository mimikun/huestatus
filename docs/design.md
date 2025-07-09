# huestatus Design Specification

## Progress Checklist

### Core Design Sections
- [x] 1. Configuration File Location and Format
- [x] 2. Error Handling Strategy
- [x] 3. Bridge Discovery and API Discovery
- [x] 4. Initial Setup Flow
- [x] 5. CLI Arguments and Command Structure
- [x] 6. Project Structure and Module Design
- [x] 7. Configuration Decisions and Technical Specifications

### Design Decisions
- [x] Retry count/interval configurable (via config file and CLI flags)
- [x] Logging strategy defined (default quiet, --verbose for details)
- [x] File permissions automatically set to 600 on Unix systems
- [x] Environment variable support for CI/CD integration
- [x] Advanced scene management and diagnostics
- [x] Cross-platform support with platform-specific optimizations
- [x] Performance optimization with connection pooling and caching

## 1. Configuration File Location and Format

### Configuration File Location

**Decision:** Cross-platform support using `dirs` crate

```rust
// Linux/macOS: ~/.config/huestatus/config.json
// Windows: %APPDATA%/huestatus/config.json
let config_dir = dirs::config_dir()
    .ok_or("Configuration directory not found")?
    .join("huestatus");
```

### Configuration File Format

**Decision:** JSON format

**Rationale:**

- Simple and lightweight
- Hue API also uses JSON
- Easy to handle with `serde_json`
- Manual editing is rarely needed

### Configuration File Structure

```json
{
  "version": "1.0",
  "bridge": {
    "ip": "192.168.1.100",
    "application_key": "xxxx-xxxx-xxxx-xxxx",
    "last_verified": "2024-01-01T00:00:00Z"
  },
  "scenes": {
    "success": {
      "id": "12345678-1234-1234-1234-success-scene",
      "name": "huestatus-success",
      "auto_created": true
    },
    "failure": {
      "id": "12345678-1234-1234-1234-failure-scene",
      "name": "huestatus-failure",
      "auto_created": true
    }
  },
  "settings": {
    "timeout_seconds": 10,
    "retry_attempts": 3
  }
}
```

**Field Descriptions:**

- `version`: Configuration file version (for future compatibility)
- `last_verified`: Track bridge connection validity
- `auto_created`: Distinguish between auto-created and existing scenes
- `timeout_seconds`: API connection timeout (default 10 seconds)
- `retry_attempts`: Number of retry attempts (default 3)

## 2. Error Handling Strategy

### Network Error Handling

**Decision**: Adopted with standard retry pattern

```rust
// Retry up to 3 times with 1-second intervals
const MAX_RETRIES: usize = 3;
const RETRY_DELAY: Duration = Duration::from_secs(1);
```

**Error Patterns and Responses:**

#### Connection Error

- **Cause**: Invalid/offline bridge IP
- **Response**: Display reconfiguration suggestion message

#### Timeout

- **Cause**: Network delay
- **Response**: Use configurable timeout value (default 10 seconds)

#### Authentication Error

- **Cause**: Invalid application key
- **Response**: Execute re-authentication process

### Scene-Related Errors

**Decision**: Manual recovery with retry for execution failures

#### Scene Not Found

- **Cause**: Invalid scene ID in configuration file
- **Response**: Display error message and suggest running `--setup`

#### Scene Execution Failure

- **Cause**: API error
- **Response**: Retry up to 3 times, then display error message and exit

### Configuration File Errors

**Decision**: Explicit user action required for all configuration issues

#### File Not Found

- **Response**: Display message "Configuration not found. Run `huestatus --setup` to configure."

#### Invalid JSON

- **Response**: Display error "Configuration file corrupted. Run `huestatus --setup` to reconfigure."

#### Version Mismatch

- **Response**: Display error "Configuration version incompatible. Run `huestatus --setup` to update."

## 3. Bridge Discovery and API Discovery

### Bridge Discovery Methods

**Decision**: Implement multiple discovery methods with fallback priority

**Priority Order:**
1. **Philips Discovery Service** (Primary method - most reliable)
   - Endpoint: `https://discovery.meethue.com/`
   - Returns JSON with bridge IP addresses on local network
   - Advantage: Reliable, actively maintained by Philips
   - Disadvantage: Requires internet access

2. **mDNS Discovery** (Secondary method - unreliable in practice)
   - Service type: `_hue._tcp.local`
   - Note: Official Hue app doesn't use mDNS consistently
   - Advantage: Local network discovery, no external dependency
   - Disadvantage: May not work reliably on all network configurations

3. **Manual IP Entry** (User fallback)
   - Allow users to manually specify bridge IP
   - Useful for network configurations where discovery fails

### Authentication Process (Hue API v1)

**Decision**: Physical button press with 30-second timeout

**Authentication Flow:**
1. **Initial Request**: `POST /api` with devicetype
2. **Button Press Requirement**: User presses physical link button on bridge
3. **Timeout Window**: 30 seconds from button press to API call
4. **Success Response**: Bridge returns username (application key)
5. **Storage**: Save username for all future API calls

**Request Format:**
```json
POST /api
{
  "devicetype": "huestatus#cli"
}
```

**Success Response:**
```json
[{"success": {"username": "83b7780291a6ceffbe0bd049104df"}}]
```

**Error Response (Button Not Pressed):**
```json
{
  "error": {
    "type": 101,
    "address": "/config/linkbutton",
    "description": "link button not pressed"
  }
}
```

### Hue API v1 Specifications

**Decision**: Use HTTP-based API v1 with confirmed endpoints

**Base URL Format:**
```
http://<bridge_ip>/api/<username>
```

**Scene Management Endpoints:**

#### Scene List Retrieval
- **Method**: `GET /api/<username>/scenes`
- **Purpose**: Get all available scenes
- **Response**: JSON object with scene IDs and metadata

#### Scene Execution (Via Groups API)
- **Method**: `PUT /api/<username>/groups/0/action`
- **Purpose**: Execute scene on all lights (group 0 = all lights)
- **Request Body**: `{"scene": "<scene_id>"}`
- **Note**: Scene execution is done via Groups API, not Scenes API directly

#### Scene Creation
- **Method**: `POST /api/<username>/scenes`
- **Purpose**: Create new scene with specified lights and states
- **Request Body**: 
```json
{
  "name": "Scene Name",
  "lights": ["1", "2", "3"],
  "recycle": true,
  "lightstates": {
    "1": {"on": true, "bri": 254, "hue": 21845, "sat": 254},
    "2": {"on": true, "bri": 254, "hue": 21845, "sat": 254},
    "3": {"on": true, "bri": 254, "hue": 21845, "sat": 254}
  }
}
```

### Lighting Pattern Specifications

**Decision**: Define standard success/failure patterns within API v1 constraints

**Success Pattern (Green):**
```json
{
  "name": "huestatus-success",
  "lights": ["1", "2", "3"],
  "recycle": true,
  "lightstates": {
    "1": {"on": true, "bri": 254, "hue": 21845, "sat": 254},
    "2": {"on": true, "bri": 254, "hue": 21845, "sat": 254},
    "3": {"on": true, "bri": 254, "hue": 21845, "sat": 254}
  }
}
```

**Failure Pattern (Red):**
```json
{
  "name": "huestatus-failure",
  "lights": ["1", "2", "3"],
  "recycle": true,
  "lightstates": {
    "1": {"on": true, "bri": 254, "hue": 0, "sat": 254},
    "2": {"on": true, "bri": 254, "hue": 0, "sat": 254},
    "3": {"on": true, "bri": 254, "hue": 0, "sat": 254}
  }
}
```

**Color Parameters (Validated from Official Documentation):**
- **Hue Range**: 0-65535 (16-bit precision for 0.0055° accuracy)
  - Red: 0
  - Green: 21845 (120° × 65536/360°)
  - Blue: 43690 (240° × 65536/360°)
- **Saturation Range**: 0-254 (0 = white, 254 = full saturation)
- **Brightness Range**: 1-254 (1 = minimum, 254 = maximum, 0 is invalid)
- **On State**: true (lights turn on)

**Alternative Color Specification (XY Coordinates):**
- More accurate, hardware-independent color representation
- Example green: `"xy": [0.409, 0.518]` (within Gamut B)
- Consider implementing XY mode for better color consistency

### Technical Implementation Notes

**HTTP Client Requirements:**
- Protocol: HTTP (not HTTPS for API v1)
- Timeout: 10 seconds (configurable)
- Retry: 3 attempts with 1-second delay
- User-Agent: `huestatus/1.0`

**Error Handling Integration:**
- Bridge unreachable → Try next discovery method
- Error 101 (link button not pressed) → Display button press instructions
- Error 1 (unauthorized user) → Re-run authentication flow
- Scene not found → Suggest running `--setup`

**Bridge Limitations (From Capabilities API):**
- Maximum 200 scenes stored in bridge
- Maximum 2048 total scene lightstates
- Scenes marked with `recycle: true` can be auto-deleted by bridge
- Color gamut varies by light type (A, B, C)

**Capabilities Check:**
- **Endpoint**: `GET /api/<username>/capabilities`
- **Purpose**: Query bridge and light capabilities before scene creation
- **Response**: Available lights, scenes, sensors, and supported features

## 4. Initial Setup Flow

### Setup Process Overview

**Decision**: Interactive setup with validation and error recovery

**Setup Flow:**

1. **Configuration Directory Creation**
   - Create `~/.config/huestatus/` directory if it doesn't exist
   - Set appropriate permissions (755 for directory, 600 for config file)

2. **Bridge Discovery**
   - Try Philips Discovery Service first
   - Fall back to mDNS if discovery service fails
   - Allow manual IP entry if both methods fail

3. **Bridge Authentication**
   - Prompt user to press link button on bridge
   - Poll authentication endpoint for 30 seconds
   - Save application key on success

4. **Light Discovery**
   - Query available lights via `/api/<username>/lights`
   - Display light list to user for confirmation
   - Allow user to select subset of lights (optional)

5. **Scene Creation**
   - Create success scene (green) with selected lights
   - Create failure scene (red) with selected lights
   - Test scene execution to verify functionality

6. **Configuration Save**
   - Write complete configuration to JSON file
   - Set file permissions to 600 (read/write for owner only)
   - Display success message with next steps

### Setup Command Implementation

```bash
huestatus --setup
```

**Interactive Flow:**

```text
🔍 Discovering Philips Hue bridges...
✅ Found bridge at 192.168.1.100

🔑 Press the link button on your Hue bridge now.
   Waiting for button press... (30 seconds remaining)
✅ Authentication successful!

💡 Found 5 lights:
   1. Living Room Light 1
   2. Living Room Light 2
   3. Kitchen Light
   4. Bedroom Light
   5. Bathroom Light

❓ Use all lights for status indication? (Y/n): Y
✅ Using all 5 lights for status indication.

🎨 Creating status scenes...
✅ Created success scene (green)
✅ Created failure scene (red)

🧪 Testing scenes...
✅ Success scene works correctly
✅ Failure scene works correctly

💾 Configuration saved to ~/.config/huestatus/config.json
🎉 Setup complete! You can now use 'huestatus success' and 'huestatus failure'
```

### Error Recovery During Setup

**Bridge Discovery Failure:**
```text
❌ Bridge discovery failed. Please enter bridge IP manually:
IP Address: 192.168.1.100
🔍 Testing connection to 192.168.1.100...
✅ Bridge found at 192.168.1.100
```

**Authentication Timeout:**
```text
⏰ Authentication timed out. Please try again.
🔑 Press the link button on your Hue bridge now.
   Waiting for button press... (30 seconds remaining)
```

**Scene Creation Failure:**
```text
❌ Failed to create scenes. Retrying...
   Attempt 2/3...
✅ Scenes created successfully
```

### Setup Validation

**Pre-setup Checks:**
- Network connectivity
- Bridge reachability
- Sufficient bridge scene storage (check via capabilities API)

**Post-setup Validation:**
- Configuration file integrity
- Scene execution test
- Bridge connectivity test

### Force Reconfiguration

```bash
huestatus --setup --force
```

- Overwrite existing configuration without prompting
- Useful for automated deployments or troubleshooting

## 5. CLI Arguments and Command Structure

### Primary Commands

**Decision**: Simple, intuitive command structure with minimal arguments

#### Success Status
```bash
huestatus success
# Activates green scene to indicate success
```

#### Failure Status
```bash
huestatus failure
# Activates red scene to indicate failure
```

#### Setup Command
```bash
huestatus --setup
# Interactive setup process
```

### Optional Arguments

#### Configuration File Path
```bash
huestatus --config /path/to/config.json success
# Use custom configuration file location
```

#### Verbose Output
```bash
huestatus --verbose success
# Display detailed operation information
```

#### Force Reconfiguration
```bash
huestatus --setup --force
# Overwrite existing configuration
```

#### Timeout Override
```bash
huestatus --timeout 5 success
# Override default timeout (in seconds)
```

### Help and Version

```bash
huestatus --help
huestatus --version
```

### Command Structure Design

**Argument Parsing Priority:**
1. Global flags (--config, --verbose, --timeout)
2. Command (success, failure, --setup)
3. Command-specific flags (--force for setup)

**Error Handling:**
- Invalid command: Display help and exit with code 1
- Missing configuration: Suggest running `--setup`
- Network errors: Display error message and exit with code 2

### Shell Integration Examples

**CI/CD Integration:**
```bash
# GitHub Actions example
- name: Run tests
  run: |
    if npm test; then
      huestatus success
    else
      huestatus failure
    fi
```

**Build Script Integration:**
```bash
#!/bin/bash
set -e

echo "Building project..."
if cargo build --release; then
    echo "Build successful"
    huestatus success
else
    echo "Build failed"
    huestatus failure
    exit 1
fi
```

### Exit Codes

**Decision**: Standard Unix exit codes

- `0`: Success
- `1`: Invalid arguments or configuration error
- `2`: Network or API error
- `3`: Authentication error
- `4`: Scene execution error

## 6. Project Structure and Module Design

### Rust Project Structure

**Decision**: Modular design with clear separation of concerns

```
huestatus/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI argument parsing and main logic
│   ├── lib.rs               # Library interface
│   ├── config/
│   │   ├── mod.rs           # Configuration management
│   │   ├── file.rs          # Config file I/O
│   │   └── validation.rs    # Config validation
│   ├── bridge/
│   │   ├── mod.rs           # Bridge API client
│   │   ├── discovery.rs     # Bridge discovery methods
│   │   ├── auth.rs          # Authentication flow
│   │   └── client.rs        # HTTP client wrapper
│   ├── scenes/
│   │   ├── mod.rs           # Scene management
│   │   ├── create.rs        # Scene creation logic
│   │   └── execute.rs       # Scene execution logic
│   ├── setup/
│   │   ├── mod.rs           # Setup process orchestration
│   │   ├── interactive.rs   # Interactive setup flow
│   │   └── validation.rs    # Setup validation
│   └── error.rs             # Error types and handling
├── tests/
│   ├── integration/
│   │   ├── cli.rs           # CLI integration tests
│   │   └── config.rs        # Configuration tests
│   └── unit/
│       ├── bridge.rs        # Bridge API tests
│       └── scenes.rs        # Scene management tests
└── docs/
    ├── design.md            # This file
    └── api.md               # API documentation
```

### Module Responsibilities

#### `main.rs`
- CLI argument parsing using `clap`
- Command routing to appropriate modules
- Error handling and user feedback
- Exit code management

#### `config/` Module
- Configuration file location resolution
- JSON serialization/deserialization
- Configuration validation
- Migration between config versions

#### `bridge/` Module
- Bridge discovery (Philips service, mDNS, manual)
- HTTP client with retry logic
- Authentication flow management
- API error handling

#### `scenes/` Module
- Scene creation with light discovery
- Scene execution via Groups API
- Color calculation and validation
- Scene validation and testing

#### `setup/` Module
- Interactive setup flow
- User input handling
- Setup validation and error recovery
- Configuration file creation

#### `error.rs`
- Custom error types using `thiserror`
- Error context and chaining
- User-friendly error messages
- Error code mapping

### Dependencies

**Decision**: Minimal, well-maintained dependencies

```toml
[dependencies]
# CLI argument parsing
clap = { version = "4.0", features = ["derive"] }

# HTTP client
reqwest = { version = "0.11", features = ["json"] }

# JSON handling
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Cross-platform directory handling
dirs = "5.0"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Date/time handling
chrono = { version = "0.4", features = ["serde"] }

# Async runtime
tokio = { version = "1.0", features = ["rt", "macros"] }

# mDNS discovery (optional)
mdns = "3.0"
```

### Error Handling Strategy

**Decision**: Structured error types with user-friendly messages

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HueStatusError {
    #[error("Configuration file not found. Run 'huestatus --setup' to configure.")]
    ConfigNotFound,
    
    #[error("Invalid configuration: {reason}")]
    InvalidConfig { reason: String },
    
    #[error("Bridge not found. Check network connection and try again.")]
    BridgeNotFound,
    
    #[error("Authentication failed. Run 'huestatus --setup' to re-authenticate.")]
    AuthenticationFailed,
    
    #[error("Scene '{scene_name}' not found. Run 'huestatus --setup' to recreate scenes.")]
    SceneNotFound { scene_name: String },
    
    #[error("Network error: {source}")]
    NetworkError { #[from] source: reqwest::Error },
    
    #[error("API error: {message}")]
    ApiError { message: String },
}
```

### Testing Strategy

**Decision**: Comprehensive testing with mocking

#### Unit Tests
- Bridge discovery logic
- Scene creation and execution
- Configuration validation
- Error handling paths

#### Integration Tests
- CLI argument parsing
- End-to-end setup flow
- Configuration file handling
- API client behavior

#### Mock Testing
- HTTP responses for bridge API
- Network failure scenarios
- Authentication timeouts
- Scene execution failures

### Performance Considerations

**Decision**: Optimize for responsiveness and reliability

- **Async HTTP**: Use `tokio` for non-blocking network operations
- **Connection Pooling**: Reuse HTTP connections for multiple API calls
- **Timeout Management**: Configurable timeouts with sensible defaults
- **Error Recovery**: Fast failure and retry logic
- **Resource Usage**: Minimal memory footprint for CLI tool

## 7. Configuration Decisions and Technical Specifications

### 7.1 Retry Configuration

**Decision**: Make retry count and interval configurable

**Rationale**:
- Different network environments require different retry settings
- Corporate networks may need longer timeouts
- Maintain defaults while allowing customization

**Implementation**:
- Configuration file: `retry_attempts` and `retry_delay_seconds` fields
- Command line override: `--retry-attempts` and `--retry-delay` flags
- Default values: 3 attempts, 1 second delay

**Updated Configuration Schema**:
```json
{
  "settings": {
    "timeout_seconds": 10,
    "retry_attempts": 3,
    "retry_delay_seconds": 1
  }
}
```

### 7.2 Logging and Output Configuration

**Decision**: Basic logging by default, detailed logging with `--verbose` flag

**Rationale**:
- CLI tools should be quiet by default
- Detailed information needed for troubleshooting
- CI/CD environments prefer minimal output

**Implementation**:
- Default: Error messages and minimal necessary output
- `--verbose` flag: HTTP requests/responses, retry status, detailed timing
- `--quiet` flag: No output on success (errors only)

**Output Examples**:
```bash
# Default mode
$ huestatus success
# (no output on success)

# Verbose mode
$ huestatus --verbose success
🔍 Connecting to bridge at 192.168.1.100...
📡 PUT /api/username/groups/0/action
✅ Scene executed successfully (response time: 145ms)

# Quiet mode
$ huestatus --quiet success
# (absolutely no output unless error)
```

### 7.3 Security and File Permissions

**Decision**: Automatically set configuration file permissions to 600

**Rationale**:
- Application key is sensitive information
- Users may forget to set secure permissions manually
- Automatic security best practice enforcement

**Implementation**:
- Unix systems (Linux/macOS): Set 600 permissions automatically
- Windows: Skip permission setting due to different filesystem model
- Permission setting failure: Display warning but continue

**Security Implementation**:
```rust
#[cfg(unix)]
fn set_secure_permissions(path: &Path) -> Result<(), std::io::Error> {
    use std::os::unix::fs::PermissionsExt;
    let permissions = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(path, permissions)
}
```

### 7.4 Extended CLI Arguments

**Decision**: Add comprehensive CLI options for production use

**Additional Arguments**:

#### Retry Configuration
```bash
huestatus --retry-attempts 5 --retry-delay 2 success
# Override retry settings temporarily
```

#### Logging Options
```bash
huestatus --verbose success     # Detailed output
huestatus --quiet success       # No output on success
huestatus --log-level debug     # Debug logging
```

#### Configuration Management
```bash
huestatus --config /path/to/config.json success
huestatus --setup --config-dir /custom/path
```

#### Network Configuration
```bash
huestatus --timeout 15 success           # Override timeout
huestatus --bridge-ip 192.168.1.100     # Skip discovery
```

### 7.5 Advanced Scene Management

**Decision**: Support scene validation and refresh

**Additional Commands**:
```bash
huestatus --validate              # Test current configuration
huestatus --refresh-scenes        # Recreate scenes if needed
huestatus --list-scenes          # Show available scenes
```

### 7.6 Environment Variable Support

**Decision**: Support environment variables for CI/CD integration

**Environment Variables**:
- `HUESTATUS_CONFIG`: Configuration file path
- `HUESTATUS_BRIDGE_IP`: Bridge IP address
- `HUESTATUS_TIMEOUT`: API timeout in seconds
- `HUESTATUS_VERBOSE`: Enable verbose output
- `HUESTATUS_QUIET`: Enable quiet mode

**Usage Example**:
```bash
# CI/CD environment
export HUESTATUS_CONFIG=/app/config/hue.json
export HUESTATUS_QUIET=1
huestatus success
```

### 7.7 Configuration Migration Strategy

**Decision**: Implement version-aware configuration migration

**Migration Implementation**:
- Check `version` field in configuration file
- Automatic migration for minor version changes
- Prompt user for major version changes
- Backup old configuration before migration

**Version Compatibility**:
```rust
pub enum ConfigVersion {
    V1_0,  // Initial version
    V1_1,  // Added retry settings
    V1_2,  // Added logging preferences
}
```

### 7.8 Error Recovery and Diagnostics

**Decision**: Implement comprehensive diagnostic tools

**Diagnostic Commands**:
```bash
huestatus --doctor                # Run full system diagnostics
huestatus --test-connection       # Test bridge connectivity
huestatus --check-scenes         # Validate scene configuration
```

**Diagnostic Output**:
```text
🔍 huestatus System Diagnostics

Network Connectivity:
✅ Internet connection available
✅ Bridge reachable at 192.168.1.100
✅ Bridge API responding (v1.54.0)

Authentication:
✅ Application key valid
✅ Bridge authorization confirmed

Scene Configuration:
✅ Success scene exists (ID: abc123)
✅ Failure scene exists (ID: def456)
✅ All lights responsive

Configuration:
✅ Config file valid (/home/user/.config/huestatus/config.json)
✅ File permissions secure (600)
✅ All required fields present

🎉 All systems operational
```

### 7.9 Performance Optimization

**Decision**: Implement connection pooling and async operations

**Performance Features**:
- HTTP connection reuse across API calls
- Async scene execution for better responsiveness
- Local scene caching to reduce API calls
- Bridge capability caching

**Implementation Details**:
```rust
// Connection pool configuration
pub struct BridgeClient {
    client: reqwest::Client,
    base_url: String,
    username: String,
    scene_cache: Arc<Mutex<HashMap<String, Scene>>>,
}
```

### 7.10 Cross-Platform Considerations

**Decision**: Full cross-platform support with platform-specific optimizations

**Platform Support**:
- **Linux**: Full feature support, systemd integration
- **macOS**: Full feature support, launchd integration
- **Windows**: Full feature support, Windows Service integration
- **FreeBSD/OpenBSD**: Basic support

**Platform-Specific Features**:
```rust
#[cfg(target_os = "linux")]
mod linux_specific {
    // systemd integration
    pub fn install_systemd_service() -> Result<(), Error> { ... }
}

#[cfg(target_os = "macos")]
mod macos_specific {
    // launchd integration
    pub fn install_launchd_service() -> Result<(), Error> { ... }
}
```

## Configuration Schema Final Version

```json
{
  "version": "1.2",
  "bridge": {
    "ip": "192.168.1.100",
    "application_key": "xxxx-xxxx-xxxx-xxxx",
    "last_verified": "2024-01-01T00:00:00Z",
    "capabilities_cache": {
      "max_scenes": 200,
      "lights_count": 10,
      "cached_at": "2024-01-01T00:00:00Z"
    }
  },
  "scenes": {
    "success": {
      "id": "12345678-1234-1234-1234-success-scene",
      "name": "huestatus-success",
      "auto_created": true,
      "last_validated": "2024-01-01T00:00:00Z"
    },
    "failure": {
      "id": "12345678-1234-1234-1234-failure-scene",
      "name": "huestatus-failure",
      "auto_created": true,
      "last_validated": "2024-01-01T00:00:00Z"
    }
  },
  "settings": {
    "timeout_seconds": 10,
    "retry_attempts": 3,
    "retry_delay_seconds": 1,
    "verbose_logging": false,
    "quiet_mode": false,
    "auto_refresh_scenes": true,
    "validate_scenes_on_startup": false
  },
  "advanced": {
    "connection_pool_size": 5,
    "cache_duration_minutes": 30,
    "scene_validation_interval_hours": 24
  }
}
```

