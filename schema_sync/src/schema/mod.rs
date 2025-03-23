//! Schema module for SchemaSync
//!
//! This module handles database schema analysis, comparison, and generation.

pub mod analyzer;
pub mod diff;
pub mod generator;
pub mod types;

// Re-export key types
pub use analyzer::SchemaAnalyzer;
pub use diff::{ColumnChange, SchemaDiff};
pub use generator::MigrationGenerator;
pub use types::{
    Column, Constraint, DatabaseSchema, FieldDefinition, ForeignKey, 
    ForeignKeyDefinition, Index, PrimaryKey, Table, View,
};