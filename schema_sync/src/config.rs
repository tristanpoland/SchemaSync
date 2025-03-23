//! Configuration handling for SchemaSync

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::error::{Error, Result};

/// Load configuration from a TOML file
pub fn load_from_file(path: &str) -> Result<Config> {
    let config_str = fs::read_to_string(path)
        .map_err(|e| Error::ConfigError(format!("Failed to read config file: {}", e)))?;
    
    let config: Config = toml::from_str(&config_str)
        .map_err(|e| Error::ConfigError(format!("Failed to parse config file: {}", e)))?;
    
    Ok(config)
}

/// Represents the complete SchemaSync configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub database: DatabaseConfig,
    pub migrations: MigrationsConfig,
    pub models: ModelsConfig,
    pub schema: SchemaConfig,
    pub naming: NamingConfig,
    pub type_mapping: TypeMappingConfig,
    pub logging: Option<LoggingConfig>,
    pub hooks: Option<HooksConfig>,
    pub output: Option<OutputConfig>,
    pub security: Option<SecurityConfig>,
    pub performance: Option<PerformanceConfig>,
}

/// Database connection configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub driver: String,
    pub url: String,
    pub pool_size: Option<u32>,
    pub timeout_seconds: Option<u64>,
    pub schema: Option<String>,
    pub enable_ssl: Option<bool>,
}

/// Migration settings configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MigrationsConfig {
    pub directory: String,
    pub naming: String,
    pub auto_generate: bool,
    pub auto_apply: bool,
    pub transaction_per_migration: bool,
    pub dry_run: bool,
    pub backup_before_migrate: bool,
    pub history_table: String,
}

/// Model discovery configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelsConfig {
    pub paths: Vec<String>,
    pub exclude_paths: Option<Vec<String>>,
    pub attributes: Vec<String>,
    pub recursive_scan: bool,
    pub derive_macros: Option<Vec<String>>,
}

/// Schema generation behavior configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SchemaConfig {
    pub strict_mode: bool,
    pub allow_column_removal: bool,
    pub allow_table_removal: bool,
    pub default_nullable: bool,
    pub index_foreign_keys: bool,
    pub unique_constraints_as_indices: bool,
    pub add_updated_at_column: bool,
    pub add_created_at_column: bool,
}

/// Naming conventions configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NamingConfig {
    pub table_style: String,
    pub column_style: String,
    pub index_pattern: String,
    pub constraint_pattern: String,
    pub pluralize_tables: bool,
    pub ignore_case_conflicts: bool,
}

/// Type mapping configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TypeMappingConfig {
    pub custom: Option<Vec<CustomTypeMapping>>,
    pub override_: Option<std::collections::HashMap<String, String>>,
}

/// Custom type mapping
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomTypeMapping {
    pub rust_type: String,
    pub db_type: String,
}

/// Logging configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub file: Option<String>,
    pub format: String,
    pub stdout: bool,
    pub include_timestamps: bool,
}

/// Hooks configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HooksConfig {
    pub before_migration: Option<Vec<String>>,
    pub after_migration: Option<Vec<String>>,
    pub on_schema_change: Option<String>,
    pub on_error: Option<String>,
}

/// Output generation configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutputConfig {
    pub generate_documentation: bool,
    pub documentation_format: String,
    pub generate_diagrams: bool,
    pub diagram_format: String,
    pub output_directory: String,
}

/// Security configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecurityConfig {
    pub encrypt_sensitive_columns: bool,
    pub sensitive_column_attributes: Vec<String>,
    pub mask_logs: bool,
    pub audit_schema_changes: bool,
}

/// Performance configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PerformanceConfig {
    pub analyze_after_migration: bool,
    pub chunk_size: usize,
    pub parallel_migrations: bool,
    pub index_concurrently: bool,
}