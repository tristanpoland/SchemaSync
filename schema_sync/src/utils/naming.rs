//! Naming utilities for SchemaSync
//!
//! This module provides utilities for naming conventions and transformations.

use inflector::Inflector;
use std::collections::HashMap;

/// Apply a naming convention to a string
pub fn apply_naming_convention(name: &str, convention: &str) -> String {
    match convention {
        "snake_case" => name.to_snake_case(),
        "camel_case" => name.to_camel_case(),
        "pascal_case" => name.to_pascal_case(),
        "kebab_case" => name.to_kebab_case(),
        "screaming_snake_case" => name.to_screaming_snake_case(),
        "title_case" => name.to_title_case(),
        "sentence_case" => name.to_sentence_case(),
        _ => name.to_string(), // Default: keep as is
    }
}

/// Format a name according to a pattern with placeholders
pub fn format_name(pattern: &str, replacements: &[(&str, &str)]) -> String {
    let mut result = pattern.to_string();
    
    for (placeholder, value) in replacements {
        result = result.replace(&format!("{{{}}}", placeholder), value);
    }
    
    result
}

/// Get table name from a model name according to convention
pub fn get_table_name(
    model_name: &str,
    style: &str,
    pluralize: bool,
) -> String {
    let name = apply_naming_convention(model_name, style);
    
    if pluralize {
        // Handle special pluralization cases that the inflector might not handle correctly
        match name.to_lowercase().as_str() {
            "person" => "people".to_string(),
            "child" => "children".to_string(),
            "man" => "men".to_string(),
            "woman" => "women".to_string(),
            "foot" => "feet".to_string(),
            "tooth" => "teeth".to_string(),
            "goose" => "geese".to_string(),
            "mouse" => "mice".to_string(),
            _ => name.to_plural()
        }
    } else {
        name
    }
}

/// Get column name from a field name according to convention
pub fn get_column_name(field_name: &str, style: &str) -> String {
    apply_naming_convention(field_name, style)
}

/// Get index name from table and columns according to pattern
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

/// Get foreign key constraint name according to pattern
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

/// Create a constraint name based on multiple fields and pattern
pub fn get_constraint_name(
    pattern: &str,
    table_name: &str,
    constraint_type: &str,
    columns: &[String],
) -> String {
    let columns_str = columns.join("_");
    
    format_name(pattern, &[
        ("table", table_name),
        ("type", constraint_type),
        ("columns", &columns_str),
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

/// Check for name conflicts in a list of identifiers
pub fn check_identifier_conflicts(
    names: &[String],
    ignore_case: bool,
) -> Option<(String, String)> {
    let mut seen = HashMap::<String, String>::new();
    
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

/// Truncate an identifier to fit database limits
pub fn truncate_identifier(name: &str, max_length: usize) -> String {
    if name.len() <= max_length {
        name.to_string()
    } else {
        // Calculate how much of the original name we can keep
        // We need space for the hash (8 chars) and the underscore (1 char)
        let keep_length = max_length - 9;
        
        // Generate hash of the full name for uniqueness
        let hash = format!("{:x}", md5::compute(name.as_bytes()));
        
        // Make sure we don't try to slice beyond the string length
        let prefix = if keep_length < name.len() {
            &name[0..keep_length]
        } else {
            name
        };
        
        format!("{}_{}", prefix, &hash[0..8])
    }
}

/// Get maximum identifier length for specific database
pub fn get_max_identifier_length(db_type: &str) -> usize {
    match db_type.to_lowercase().as_str() {
        "postgres" => 63,  // PostgreSQL limit
        "mysql" => 64,     // MySQL limit
        "sqlite" => 2048,  // SQLite essentially unlimited
        "oracle" => 30,    // Oracle older versions (12c and earlier have 30 char limit)
        "oracle_12c" => 128, // Oracle 12c and later
        "mssql" => 128,    // SQL Server
        _ => 63,           // Default to PostgreSQL limit
    }
}

/// Convert a singular name to plural
pub fn pluralize(name: &str) -> String {
    // Handle special cases first
    match name.to_lowercase().as_str() {
        "person" => "people".to_string(),
        "child" => "children".to_string(),
        "man" => "men".to_string(),
        "woman" => "women".to_string(),
        "foot" => "feet".to_string(),
        "tooth" => "teeth".to_string(),
        "goose" => "geese".to_string(),
        "mouse" => "mice".to_string(),
        _ => name.to_plural()
    }
}

/// Convert a plural name to singular
pub fn singularize(name: &str) -> String {
    // Handle special cases first
    match name.to_lowercase().as_str() {
        "people" => "person".to_string(),
        "children" => "child".to_string(),
        "men" => "man".to_string(),
        "women" => "woman".to_string(),
        "feet" => "foot".to_string(),
        "teeth" => "tooth".to_string(),
        "geese" => "goose".to_string(),
        "mice" => "mouse".to_string(),
        _ => name.to_singular()
    }
}

/// Generate a unique name with a suffix if name exists in the list
pub fn generate_unique_name(name: &str, existing_names: &[String]) -> String {
    if !existing_names.contains(&name.to_string()) {
        return name.to_string();
    }
    
    let mut counter = 1;
    loop {
        let new_name = format!("{}_{}", name, counter);
        if !existing_names.contains(&new_name) {
            return new_name;
        }
        counter += 1;
    }
}

/// Generate a combined name from multiple parts with a separator
pub fn combine_names(parts: &[&str], separator: &str) -> String {
    parts.join(separator)
}

/// Format SQL identifier according to database style (quoted, backticks, etc.)
pub fn format_sql_identifier(name: &str, db_type: &str) -> String {
    match db_type.to_lowercase().as_str() {
        "postgres" => format!("\"{}\"", name),
        "mysql" => format!("`{}`", name),
        "sqlite" => format!("\"{}\"", name),
        "oracle" => format!("\"{}\"", name),
        "mssql" => format!("[{}]", name),
        _ => name.to_string(), // Default: no quoting
    }
}

/// Format name as a valid file name (for migrations, etc.)
pub fn format_file_name(name: &str) -> String {
    // Replace spaces and special characters that may cause issues in filenames
    let sanitized = name
        .replace(' ', "_")
        .replace('/', "_")
        .replace('\\', "_")
        .replace(':', "_")
        .replace('*', "_")
        .replace('?', "_")
        .replace('"', "_")
        .replace('<', "_")
        .replace('>', "_")
        .replace('|', "_");
    
    sanitized.to_lowercase()
}

/// Create a timestamp-based migration name
pub fn create_migration_name(description: &str, timestamp: bool) -> String {
    let clean_description = format_file_name(description);
    
    if timestamp {
        use chrono::Utc;
        let now = Utc::now();
        format!("{}_{}", now.format("%Y%m%d%H%M%S"), clean_description)
    } else {
        clean_description
    }
}

/// Split a compound name (camelCase, snake_case, etc.) into words
pub fn split_into_words(name: &str) -> Vec<String> {
    // First, handle snake_case and kebab-case
    if name.contains('_') || name.contains('-') {
        return name
            .replace('-', "_")
            .split('_')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
    }
    
    // Then handle camelCase and PascalCase
    let mut words = Vec::new();
    let mut current_word = String::new();
    
    for (i, c) in name.char_indices() {
        if i > 0 && c.is_uppercase() {
            if !current_word.is_empty() {
                words.push(current_word);
                current_word = String::new();
            }
        }
        current_word.push(c);
    }
    
    if !current_word.is_empty() {
        words.push(current_word);
    }
    
    // Convert all words to lowercase
    words.iter().map(|w| w.to_lowercase()).collect()
}

/// Check if a name is a reserved SQL keyword
pub fn is_sql_keyword(name: &str) -> bool {
    // Common SQL keywords across databases
    const SQL_KEYWORDS: &[&str] = &[
        "add", "all", "alter", "and", "any", "as", "asc", "backup", "begin", "between",
        "by", "case", "check", "column", "constraint", "create", "database", "default",
        "delete", "desc", "distinct", "drop", "else", "end", "except", "exec", "exists",
        "foreign", "from", "full", "group", "having", "in", "index", "inner", "insert",
        "intersect", "into", "is", "join", "key", "left", "like", "limit", "not",
        "null", "on", "or", "order", "outer", "primary", "procedure", "right",
        "rownum", "select", "set", "table", "top", "truncate", "union", "unique",
        "update", "values", "view", "where", "with"
    ];
    
    SQL_KEYWORDS.contains(&name.to_lowercase().as_str())
}

/// Escape a SQL keyword if needed
pub fn escape_sql_keyword(name: &str, db_type: &str) -> String {
    if is_sql_keyword(name) {
        format_sql_identifier(name, db_type)
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_apply_naming_convention() {
        assert_eq!(apply_naming_convention("UserProfile", "snake_case"), "user_profile");
        assert_eq!(apply_naming_convention("user_profile", "camel_case"), "userProfile");
        assert_eq!(apply_naming_convention("user_profile", "pascal_case"), "UserProfile");
        assert_eq!(apply_naming_convention("UserProfile", "kebab_case"), "user-profile");
        assert_eq!(apply_naming_convention("UserProfile", "screaming_snake_case"), "USER_PROFILE");
    }
    
    #[test]
    fn test_format_name() {
        assert_eq!(
            format_name("ix_{table}_{columns}", &[("table", "users"), ("columns", "email")]),
            "ix_users_email"
        );
        
        assert_eq!(
            format_name("{type}_{table}_{action}", &[
                ("type", "trigger"),
                ("table", "users"),
                ("action", "before_insert")
            ]),
            "trigger_users_before_insert"
        );
    }
    
    #[test]
    fn test_table_name() {
        assert_eq!(get_table_name("UserProfile", "snake_case", true), "user_profiles");
        assert_eq!(get_table_name("UserProfile", "snake_case", false), "user_profile");
        assert_eq!(get_table_name("Person", "camel_case", true), "people");
    }
    
    #[test]
    fn test_column_name() {
        assert_eq!(get_column_name("firstName", "snake_case"), "first_name");
        assert_eq!(get_column_name("user_id", "camel_case"), "userId");
        assert_eq!(get_column_name("date_of_birth", "pascal_case"), "DateOfBirth");
    }
    
    #[test]
    fn test_index_name() {
        assert_eq!(
            get_index_name("ix_{table}_{columns}", "users", &vec!["email".to_string()]),
            "ix_users_email"
        );
        
        assert_eq!(
            get_index_name("idx_{table}_{columns}", "orders", &vec!["customer_id".to_string(), "order_date".to_string()]),
            "idx_orders_customer_id_order_date"
        );
    }
    
    #[test]
    fn test_foreign_key_name() {
        assert_eq!(
            get_foreign_key_name("fk_{table}_{column}", "posts", "author_id"),
            "fk_posts_author_id"
        );
        
        assert_eq!(
            get_foreign_key_name("fk_{column}_to_{table}", "users", "created_by"),
            "fk_created_by_to_users"
        );
    }
    
    #[test]
    fn test_sanitize_identifier() {
        assert_eq!(sanitize_identifier("user-name"), "user_name");
        assert_eq!(sanitize_identifier("123user"), "_123user");
        assert_eq!(sanitize_identifier("user.name"), "user_name");
        assert_eq!(sanitize_identifier("user@name"), "user_name");
    }
    
    #[test]
    fn test_identifier_conflicts() {
        let names = vec![
            "User".to_string(),
            "user".to_string(),
            "admin".to_string()
        ];
        
        // With case sensitivity
        assert!(check_identifier_conflicts(&names, false).is_none());
        
        // Without case sensitivity
        let conflict = check_identifier_conflicts(&names, true);
        assert!(conflict.is_some());
        let (name1, name2) = conflict.unwrap();
        assert!(name1 == "User" || name1 == "user");
        assert!(name2 == "User" || name2 == "user");
        assert_ne!(name1, name2);
    }
    
    #[test]
    fn test_truncate_identifier() {
        let long_name = "this_is_a_very_long_identifier_that_exceeds_database_limits";
        let truncated = truncate_identifier(long_name, 30);
        
        assert_eq!(truncated.len(), 30);
        assert!(truncated.starts_with("this_is_a_very_long"));
        assert!(truncated.contains('_'));
    }
    
    #[test]
    fn test_generate_unique_name() {
        let existing = vec![
            "user".to_string(),
            "user_1".to_string(),
            "customer".to_string()
        ];
        
        assert_eq!(generate_unique_name("profile", &existing), "profile");
        assert_eq!(generate_unique_name("user", &existing), "user_2");
    }
    
    #[test]
    fn test_split_into_words() {
        assert_eq!(
            split_into_words("camelCaseText"),
            vec!["camel".to_string(), "case".to_string(), "text".to_string()]
        );
        
        assert_eq!(
            split_into_words("snake_case_text"),
            vec!["snake".to_string(), "case".to_string(), "text".to_string()]
        );
        
        assert_eq!(
            split_into_words("PascalCaseText"),
            vec!["pascal".to_string(), "case".to_string(), "text".to_string()]
        );
    }
    
    #[test]
    fn test_is_sql_keyword() {
        assert!(is_sql_keyword("SELECT"));
        assert!(is_sql_keyword("from"));
        assert!(is_sql_keyword("JOIN"));
        assert!(!is_sql_keyword("username"));
    }
    
    #[test]
    fn test_escape_sql_keyword() {
        assert_eq!(escape_sql_keyword("select", "postgres"), "\"select\"");
        assert_eq!(escape_sql_keyword("from", "mysql"), "`from`");
        assert_eq!(escape_sql_keyword("username", "postgres"), "username");
    }
}