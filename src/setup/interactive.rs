use crate::bridge::DiscoveredBridge;
use crate::error::{HueStatusError, Result};
use console::{style, Term};
use std::io::{self, Write};

/// Interactive user interface for setup
pub struct InteractiveSetup {
    #[allow(dead_code)]
    term: Term,
}

impl InteractiveSetup {
    pub fn new() -> Self {
        Self {
            term: Term::stdout(),
        }
    }

    /// Confirm bridge selection with user
    pub fn confirm_bridge(&self, bridge: &DiscoveredBridge) -> Result<bool> {
        println!("Found Hue bridge:");
        println!("  • Name: {}", bridge.name.as_deref().unwrap_or("Unknown"));
        println!("  • IP: {}", bridge.ip);
        if let Some(model) = &bridge.model {
            println!("  • Model: {}", model);
        }
        println!();

        self.ask_yes_no("Use this bridge?", true)
    }

    /// Ask yes/no question
    pub fn ask_yes_no(&self, question: &str, default: bool) -> Result<bool> {
        let default_text = if default { "(Y/n)" } else { "(y/N)" };

        loop {
            print!("{} {}: ", question, style(default_text).dim());
            io::stdout()
                .flush()
                .map_err(|e| HueStatusError::IoError { source: e })?;

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| HueStatusError::IoError { source: e })?;

            match input.trim().to_lowercase().as_str() {
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                "" => return Ok(default),
                _ => println!("Please enter 'y' or 'n'"),
            }
        }
    }

    /// Get manual IP from user
    pub fn get_manual_ip(&self) -> Result<String> {
        loop {
            print!("Enter bridge IP address: ");
            io::stdout()
                .flush()
                .map_err(|e| HueStatusError::IoError { source: e })?;

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| HueStatusError::IoError { source: e })?;

            let ip = input.trim();
            if !ip.is_empty() {
                return Ok(ip.to_string());
            }

            println!("Please enter a valid IP address.");
        }
    }

    /// Show authentication instructions
    pub fn show_auth_instructions(&self, bridge_ip: &str) {
        println!();
        println!("{}", style("Authentication Required").bold().cyan());
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        println!("To connect to your Hue bridge at {}:", bridge_ip);
        println!("1. Press the large round button on top of your bridge");
        println!("2. The button will start blinking");
        println!("3. Press Enter within 30 seconds");
        println!();
        print!("Press the bridge button now, then press Enter...");
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
    }
}

impl Default for InteractiveSetup {
    fn default() -> Self {
        Self::new()
    }
}
