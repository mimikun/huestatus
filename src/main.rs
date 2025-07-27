use clap::{Arg, Command};
use console::style;
use huestatus::{
    bridge::BridgeClient,
    config::Config,
    error::{HueStatusError, Result},
    scenes::SceneManager,
    setup::{SetupOptions, SetupProcess},
    APP_DESCRIPTION, APP_NAME, VERSION,
};
use std::process;

/// CLI application entry point
#[tokio::main]
async fn main() {
    // Set up panic handler for better error reporting
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("💥 Internal error: {panic_info}");
        eprintln!("Please report this issue at: https://github.com/mimikun/huestatus/issues");
        process::exit(1);
    }));

    // Parse command line arguments
    let matches = create_cli().get_matches();

    // Extract global options
    let verbose = matches.get_flag("verbose");
    let quiet = matches.get_flag("quiet");
    let _config_path = matches.get_one::<String>("config").cloned();
    let timeout = matches.get_one::<u64>("timeout").copied().unwrap_or(10);
    let retry_attempts = matches
        .get_one::<usize>("retry-attempts")
        .copied()
        .unwrap_or(3);
    let retry_delay = matches.get_one::<u64>("retry-delay").copied().unwrap_or(1);

    // Run the appropriate command
    let result = match matches.subcommand() {
        Some(("success", _)) => {
            execute_status_command(
                "success",
                verbose,
                quiet,
                timeout,
                retry_attempts,
                retry_delay,
            )
            .await
        }
        Some(("failure", _)) => {
            execute_status_command(
                "failure",
                verbose,
                quiet,
                timeout,
                retry_attempts,
                retry_delay,
            )
            .await
        }
        Some(("setup", setup_matches)) => {
            let force = setup_matches.get_flag("force");
            let interactive = !setup_matches.get_flag("non-interactive");
            let test_scenes = setup_matches.get_flag("test");

            execute_setup_command(SetupOptions {
                force,
                interactive,
                verbose,
                test_scenes,
                ..SetupOptions::default()
            })
            .await
        }
        Some(("validate", _)) => execute_validate_command(verbose).await,
        Some(("doctor", _)) => execute_doctor_command().await,
        _ => {
            // No subcommand provided, show help
            let mut cmd = create_cli();
            cmd.print_help().unwrap();
            println!();
            Ok(())
        }
    };

    // Handle result and exit
    match result {
        Ok(()) => process::exit(0),
        Err(e) => {
            if !quiet {
                eprintln!("{}", format_error(&e));

                if verbose {
                    eprintln!("\nDebug information:");
                    eprintln!("Error type: {e:?}");
                    eprintln!("Exit code: {}", e.exit_code());
                }

                // Show helpful suggestions
                show_error_suggestions(&e);
            }
            process::exit(e.exit_code());
        }
    }
}

/// Create CLI command structure
fn create_cli() -> Command {
    Command::new(APP_NAME)
        .version(VERSION)
        .about(APP_DESCRIPTION)
        .author("mimikun <mimikun@users.noreply.github.com>")
        .long_about("A CLI tool for displaying build status using Philips Hue lights.\n\nUse your Hue lights to show success (green) or failure (red) status for CI/CD pipelines, builds, tests, and more.")
        .arg_required_else_help(true)
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::SetTrue)
                .help("Enable verbose output")
                .global(true),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .action(clap::ArgAction::SetTrue)
                .help("Suppress all output except errors")
                .global(true)
                .conflicts_with("verbose"),
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Use custom configuration file")
                .global(true),
        )
        .arg(
            Arg::new("timeout")
                .short('t')
                .long("timeout")
                .value_name("SECONDS")
                .value_parser(clap::value_parser!(u64))
                .help("API timeout in seconds [default: 10]")
                .global(true),
        )
        .arg(
            Arg::new("retry-attempts")
                .long("retry-attempts")
                .value_name("COUNT")
                .value_parser(clap::value_parser!(usize))
                .help("Number of retry attempts [default: 3]")
                .global(true),
        )
        .arg(
            Arg::new("retry-delay")
                .long("retry-delay")
                .value_name("SECONDS")
                .value_parser(clap::value_parser!(u64))
                .help("Delay between retries in seconds [default: 1]")
                .global(true),
        )
        .subcommand(
            Command::new("success")
                .about("Show success status (green lights)")
                .long_about("Activate the success scene to display green lights, indicating a successful build, test, or operation."),
        )
        .subcommand(
            Command::new("failure")
                .about("Show failure status (red lights)")
                .long_about("Activate the failure scene to display red lights, indicating a failed build, test, or operation."),
        )
        .subcommand(
            Command::new("setup")
                .about("Configure huestatus")
                .long_about("Interactive setup process to discover your Hue bridge, authenticate, and create status scenes.")
                .arg(
                    Arg::new("force")
                        .short('f')
                        .long("force")
                        .action(clap::ArgAction::SetTrue)
                        .help("Overwrite existing configuration"),
                )
                .arg(
                    Arg::new("non-interactive")
                        .long("non-interactive")
                        .action(clap::ArgAction::SetTrue)
                        .help("Run setup without interactive prompts"),
                )
                .arg(
                    Arg::new("test")
                        .long("test")
                        .action(clap::ArgAction::SetTrue)
                        .help("Test scene execution after setup"),
                ),
        )
        .subcommand(
            Command::new("validate")
                .about("Validate current configuration")
                .long_about("Check if your current configuration is valid and all components are working correctly."),
        )
        .subcommand(
            Command::new("doctor")
                .about("Run diagnostic checks")
                .long_about("Perform comprehensive diagnostic checks to identify and help resolve any issues."),
        )
}

/// Execute status command (success or failure)
async fn execute_status_command(
    status_type: &str,
    verbose: bool,
    quiet: bool,
    timeout: u64,
    retry_attempts: usize,
    retry_delay: u64,
) -> Result<()> {
    // Load configuration
    let config = Config::load().map_err(|e| match e {
        HueStatusError::ConfigNotFound => HueStatusError::ConfigNotFound,
        _ => e,
    })?;

    // Apply command-line overrides
    let effective_timeout = timeout;
    let effective_verbose = verbose || config.effective_verbose();
    let effective_quiet = quiet || config.effective_quiet();

    if effective_verbose && !effective_quiet {
        eprintln!("🔍 Executing {status_type} status...");
        eprintln!("📍 Bridge: {}", config.bridge.ip);
    }

    // Create bridge client
    let client = BridgeClient::with_config(
        config.bridge.ip.clone(),
        effective_timeout,
        retry_attempts,
        retry_delay,
        effective_verbose && !effective_quiet,
    )?
    .with_username(config.bridge.application_key.clone());

    // Create scene manager
    let scene_manager =
        SceneManager::new(client).with_verbose(effective_verbose && !effective_quiet);

    // Execute the status scene
    let result = scene_manager
        .execute_status_scene(status_type, &config)
        .await?;

    if !effective_quiet {
        if effective_verbose {
            println!(
                "✅ {} status displayed successfully ({}ms)",
                style(status_type).bold(),
                result.execution_time_ms
            );
        } else {
            // Silent success for non-verbose, non-quiet mode
        }
    }

    Ok(())
}

/// Execute setup command
async fn execute_setup_command(options: SetupOptions) -> Result<()> {
    let mut setup = SetupProcess::new().with_options(options.verbose, options.force, None);

    let result = setup.run(&options).await?;

    if options.verbose {
        println!("Setup result: {}", result.summary());
    }

    Ok(())
}

/// Execute validate command
async fn execute_validate_command(verbose: bool) -> Result<()> {
    if verbose {
        println!("🔍 Validating configuration...");
    }

    // Load and validate configuration
    let config = Config::load()?;
    config.validate()?;

    if verbose {
        println!("✅ Configuration is valid");
        println!(
            "📍 Bridge: {} ({})",
            config.bridge.ip,
            config.bridge.last_verified.format("%Y-%m-%d %H:%M UTC")
        );
    }

    // Test bridge connection
    let client = BridgeClient::new(config.bridge.ip.clone())?
        .with_username(config.bridge.application_key.clone())
        .with_verbose(verbose);

    client.test_connection().await?;

    if verbose {
        println!("✅ Bridge connection successful");
    }

    // Validate scenes
    let scene_manager = SceneManager::new(client).with_verbose(verbose);
    let validation_results = scene_manager.validate_status_scenes(&config).await?;

    let mut total_issues = 0;
    for result in validation_results {
        if !result.is_valid {
            total_issues += result.issues.len();
            if verbose {
                println!("❌ Scene '{}' has issues:", result.scene_name);
                for issue in &result.issues {
                    println!("  • {issue}");
                }
            }
        } else if verbose {
            println!("✅ Scene '{}' is valid", result.scene_name);
        }
    }

    if total_issues == 0 {
        if !verbose {
            println!("✅ All validations passed");
        }
    } else {
        return Err(HueStatusError::ValidationFailed {
            reason: format!("Found {total_issues} validation issues"),
        });
    }

    Ok(())
}

/// Execute doctor command
async fn execute_doctor_command() -> Result<()> {
    let setup = SetupProcess::new();
    setup.run_diagnostics().await
}

/// Format error message for display
fn format_error(error: &HueStatusError) -> String {
    let emoji = match error {
        HueStatusError::ConfigNotFound => "📁",
        HueStatusError::BridgeNotFound => "🔍",
        HueStatusError::AuthenticationFailed => "🔑",
        HueStatusError::SceneNotFound { .. } => "🎬",
        HueStatusError::NetworkError { .. } => "🌐",
        HueStatusError::TimeoutError { .. } => "⏰",
        _ => "❌",
    };

    format!("{} {}", emoji, error.user_message())
}

/// Show helpful suggestions based on error type
fn show_error_suggestions(error: &HueStatusError) {
    println!();

    match error {
        HueStatusError::ConfigNotFound => {
            println!("💡 {}", style("Try running:").bold());
            println!("   huestatus setup");
        }
        HueStatusError::BridgeNotFound => {
            println!("💡 {}", style("Suggestions:").bold());
            println!("   • Ensure your Hue bridge is connected and powered on");
            println!("   • Check that your device is on the same network as the bridge");
            println!("   • Try running: huestatus setup");
        }
        HueStatusError::AuthenticationFailed => {
            println!("💡 {}", style("Try running:").bold());
            println!("   huestatus setup --force");
        }
        HueStatusError::SceneNotFound { .. } => {
            println!("💡 {}", style("Try running:").bold());
            println!("   huestatus setup --force");
        }
        HueStatusError::NetworkError { .. } | HueStatusError::TimeoutError { .. } => {
            println!("💡 {}", style("Suggestions:").bold());
            println!("   • Check your network connection");
            println!("   • Verify the bridge IP address is correct");
            println!("   • Try increasing the timeout with --timeout <seconds>");
        }
        _ => {
            if error.is_recoverable_with_setup() {
                println!("💡 {}", style("Try running:").bold());
                println!("   huestatus setup --force");
            }
        }
    }

    println!();
    println!("For more help: https://github.com/mimikun/huestatus");
}
