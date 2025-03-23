//! Naming utilities for SchemaSync
//!
//! This module provides utilities for naming conventions and transformations.

use inflector::Inflector;

/// Apply a naming convention to a string
pub fn apply_naming_convention(name: &str, convention: &str) -> String {
    match convention {
        "snake_case" => name.to_snake_case(),
        "camel_case" => name.to_camel_case(),
        "pascal_case" => name.to_pascal_case(),
        "kebab_case" => name.to_kebab_case(),
        "screaming_snake_case" => name.to_screaming_snake_case(),
        _ => name.to_string(),
    }
}

/// Format a name according to a pattern
pub fn format_name(pattern: &str, replacements: &[(&str, &str)]) -> String {
    let mut result = pattern.to_string();
    
    for (placeholder, value) in replacements {
        result = result.replace(&format!("{{{}}}", placeholder), value);
    }
    
    result
}

/// Get table name from a model name
pub fn get_table_name(
    model_name: &str,
    style: &str,
    pluralize: bool,
) -> String {
    let name = apply_naming_convention(model_name, style);
    
    if pluralize {
        name.to_plural()
    } else {
        name
    }
}

/// Get column name from a field name
pub fn get_column_name(field_name: &str, style: &str) -> String {
    apply_naming_convention(field_name, style)
}

/// Get index name from table and columns
pub fn get_index_name(
    pattern: &str,
    table_name: &str,
    columns: &[String],
) -> String {
    let columns_str = columns.join("_");
    
    format_name(pattern, &[
        ("table", table_name),
        ("columns", &columns_str),
    ])
}

/// Get foreign key constraint name
pub fn get_foreign_key_name(
    pattern: &str,
    table_name: &str,
    column_name: &str,
) -> String {
    format_name(pattern, &[
        ("table", table_name),
        ("column", column_name),
    ])
}

/// Sanitize identifiers for SQL
pub fn sanitize_identifier(name: &str) -> String {
    // Remove or replace characters not allowed in SQL identifiers
    let mut sanitized = name.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
    
    // Ensure identifier doesn't start with a number
    if sanitized.chars().next().map_or(false, |c| c.is_numeric()) {
        sanitized = format!("_{}", sanitized);
    }
    
    sanitized
}

/// Check for name conflicts
pub fn check_identifier_conflicts(
    names: &[String],
    ignore_case: bool,
) -> Option<(String, String)> {
    let mut seen = std::collections::HashMap::<String, String>::new();
    
    for name in names {
        let key = if ignore_case { name.to_lowercase() } else { name.clone() };
        
        if let Some(existing) = seen.get(&key) {
            if name != existing {
                return Some((existing.clone(), name.clone()));
            }
        } else {
            seen.insert(key, name.clone());
        }
    }
    
    None
}

/// Truncate identifier to database limit
pub fn truncate_identifier(name: &str, max_length: usize) -> String {
    if name.len() <= max_length {
        name.to_string()
    } else {
        // Use first part and hash of full name to ensure uniqueness
        let hash = format!("{:x}", md5::compute(name.as_bytes()));
        let prefix_len = max_length - 9; // 8 chars for hash + 1 for underscore
        
        format!("{}_{}", &name[0..prefix_len], &hash[0..8])
    }
}