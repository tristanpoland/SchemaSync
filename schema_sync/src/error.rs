//! Error types for SchemaSync

use thiserror::Error;

/// Result type for SchemaSync operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for SchemaSync
#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Schema analysis error: {0}")]
    SchemaAnalysisError(String),
    
    #[error("Migration error: {0}")]
    MigrationError(String),
    
    #[error("Model registration error: {0}")]
    ModelRegistrationError(String),
    
    #[error("Type mapping error: {0}")]
    TypeMappingError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("SQLx error: {0}")]
    SqlxError(#[from] sqlx::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Syntax error: {0}")]
    SyntaxError(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

// The #[from] attribute on SqlxError already implements this conversion
// so we don't need a separate implementation

/// Convert Serde JSON errors to SchemaSync errors
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::SerializationError(error.to_string())
    }
}

/// Convert TOML deserialization errors to SchemaSync errors
impl From<toml::de::Error> for Error {
    fn from(error: toml::de::Error) -> Self {
        Error::ConfigError(error.to_string())
    }
}