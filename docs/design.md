# huestatus Design Specification

## Progress Checklist

### Core Design Sections
- [x] 1. Configuration File Location and Format
- [x] 2. Error Handling Strategy
- [ ] 3. Bridge Discovery and API Discovery
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

## Undecided Items

- Should retry count/interval be configurable?
- Is detailed log output necessary?
- Should configuration file permissions (600 recommended) be set automatically?

