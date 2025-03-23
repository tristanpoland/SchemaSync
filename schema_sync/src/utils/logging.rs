//! Logging utilities for SchemaSync
//!
//! This module provides logging setup and configuration.

use tracing::Level;
use tracing_subscriber::{fmt, EnvFilter};
use std::path::Path;
use std::fs::File;

use crate::config::LoggingConfig;
use crate::error::Result;

/// Initialize logging based on configuration
pub fn init_logging(config: &Option<LoggingConfig>) -> Result<()> {
    let config = match config {
        Some(cfg) => cfg,
        None => return Ok(()), // No logging configuration, use defaults
    };
    
    // Parse log level
    let level = match config.level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO, // Default to INFO
    };
    
    // Create filter for the level
    let env_filter = EnvFilter::from_default_env()
        .add_directive(format!("schema_sync={}", level).parse().unwrap());
    
    // Simple approach: just use the last specified output
    if let Some(file_path) = &config.file {
        // Ensure directory exists
        if let Some(parent) = Path::new(file_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Open log file
        let file = File::create(file_path)?;
        
        // Set up logging to file
        if config.format.to_lowercase() == "json" {
            // JSON to file
            let subscriber = fmt::Subscriber::builder()
                .json()
                .with_env_filter(env_filter)
                .with_writer(file)
                .finish();
                
            tracing::subscriber::set_global_default(subscriber)
                .map_err(|e| crate::error::Error::Unknown(e.to_string()))?;
        } else {
            // Text to file
            let subscriber = fmt::Subscriber::builder()
                .with_env_filter(env_filter)
                .with_writer(file)
                .finish();
                
            tracing::subscriber::set_global_default(subscriber)
                .map_err(|e| crate::error::Error::Unknown(e.to_string()))?;
        }
    } else if config.stdout {
        // Set up logging to stdout
        if config.format.to_lowercase() == "json" {
            // JSON to stdout
            let subscriber = fmt::Subscriber::builder()
                .json()
                .with_env_filter(env_filter)
                .finish();
                
            tracing::subscriber::set_global_default(subscriber)
                .map_err(|e| crate::error::Error::Unknown(e.to_string()))?;
        } else {
            // Text to stdout
            let subscriber = fmt::Subscriber::builder()
                .with_env_filter(env_filter)
                .finish();
                
            tracing::subscriber::set_global_default(subscriber)
                .map_err(|e| crate::error::Error::Unknown(e.to_string()))?;
        }
    }
    
    Ok(())
}

/// Log a message
#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)+) => {
        match $level {
            tracing::Level::ERROR => tracing::error!($($arg)+),
            tracing::Level::WARN => tracing::warn!($($arg)+),
            tracing::Level::INFO => tracing::info!($($arg)+),
            tracing::Level::DEBUG => tracing::debug!($($arg)+),
            tracing::Level::TRACE => tracing::trace!($($arg)+),
        }
    };
}