pub mod bridge;
pub mod config;
pub mod error;
pub mod scenes;
pub mod setup;

pub use error::{HueStatusError, Result};

/// Application version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");

/// Application description
pub const APP_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
