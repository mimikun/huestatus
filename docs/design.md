# huestatus Design Specification

## Progress Checklist

### Core Design Sections
- [x] 1. Configuration File Location and Format
- [x] 2. Error Handling Strategy
- [x] 3. Bridge Discovery and API Discovery
- [ ] 4. Initial Setup Flow
- [ ] 5. CLI Arguments and Command Structure
- [ ] 6. Project Structure and Module Design

### Remaining Decisions
- [ ] Should retry count/interval be configurable?
- [ ] Is detailed log output necessary?
- [ ] Should configuration file permissions (600 recommended) be set automatically?

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

## Undecided Items

- Should retry count/interval be configurable?
- Is detailed log output necessary?
- Should configuration file permissions (600 recommended) be set automatically?

