//! Database module for SchemaSync
//!
//! This module handles database connections and migrations.

pub mod connection;
pub mod executor;
pub mod migrations;

// Re-export key types
pub use connection::DatabaseConnection;