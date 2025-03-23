//! Utilities for SchemaSync
//!
//! This module provides utility functions used across the library.

pub mod naming;
pub mod logging;

// Re-export key utility functions
pub use naming::{
    apply_naming_convention, format_name, get_table_name, 
    get_column_name, get_index_name, get_foreign_key_name,
};  