//! Models module for SchemaSync
//!
//! This module handles model registration and discovery.

pub mod registry;

// Re-export key types
pub use registry::{ModelInfo, ModelRegistry, SchemaSyncModel};