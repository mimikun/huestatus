use std::process::Command;
use tempfile::TempDir;

/// Test CLI help output
#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("huestatus"));
    assert!(stdout.contains("success"));
    assert!(stdout.contains("failure"));
    assert!(stdout.contains("setup"));
}

/// Test CLI version output
#[test]
fn test_cli_version() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

/// Test CLI without config fails gracefully
#[test]
fn test_cli_no_config() {
    let temp_dir = TempDir::new().unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "success"])
        .env("HOME", temp_dir.path())
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Configuration") || stderr.contains("setup"));
}

/// Test invalid command
#[test]
fn test_invalid_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "invalid-command"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
}