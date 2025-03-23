//! Type definitions for database schema objects

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a complete database schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSchema {
    pub tables: HashMap<String, Table>,
    pub views: HashMap<String, View>,
    pub schema_name: Option<String>,
}

impl DatabaseSchema {
    /// Create a new empty database schema
    pub fn new(schema_name: Option<String>) -> Self {
        Self {
            tables: HashMap::new(),
            views: HashMap::new(),
            schema_name,
        }
    }
    
    /// Add a table to the schema
    pub fn add_table(&mut self, table: Table) {
        self.tables.insert(table.name.clone(), table);
    }
    
    /// Add a view to the schema
    pub fn add_view(&mut self, view: View) {
        self.views.insert(view.name.clone(), view);
    }
}

/// Represents a database table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub primary_key: Option<PrimaryKey>,
    pub indexes: Vec<Index>,
    pub foreign_keys: Vec<ForeignKey>,
    pub constraints: Vec<Constraint>,
    pub comment: Option<String>,
}

impl Table {
    /// Create a new table with the given name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            columns: Vec::new(),
            primary_key: None,
            indexes: Vec::new(),
            foreign_keys: Vec::new(),
            constraints: Vec::new(),
            comment: None,
        }
    }
    
    /// Add a column to the table
    pub fn add_column(&mut self, column: Column) {
        self.columns.push(column);
    }
    
    /// Set the primary key for the table
    pub fn set_primary_key(&mut self, pk: PrimaryKey) {
        self.primary_key = Some(pk);
    }
    
    /// Add an index to the table
    pub fn add_index(&mut self, index: Index) {
        self.indexes.push(index);
    }
    
    /// Add a foreign key to the table
    pub fn add_foreign_key(&mut self, fk: ForeignKey) {
        self.foreign_keys.push(fk);
    }
}

/// Represents a database column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default: Option<String>,
    pub comment: Option<String>,
    pub is_unique: bool,
    pub is_generated: bool,
    pub generation_expression: Option<String>,
}

impl Column {
    /// Create a new column with the given name and type
    pub fn new(name: &str, data_type: &str) -> Self {
        Self {
            name: name.to_string(),
            data_type: data_type.to_string(),
            nullable: false,
            default: None,
            comment: None,
            is_unique: false,
            is_generated: false,
            generation_expression: None,
        }
    }
    
    /// Set whether the column is nullable
    pub fn nullable(mut self, nullable: bool) -> Self {
        self.nullable = nullable;
        self
    }
    
    /// Set a default value for the column
    pub fn default(mut self, default: &str) -> Self {
        self.default = Some(default.to_string());
        self
    }
}

/// Represents a primary key constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimaryKey {
    pub name: Option<String>,
    pub columns: Vec<String>,
}

/// Represents an index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub method: Option<String>,
}

/// Represents a foreign key constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKey {
    pub name: String,
    pub columns: Vec<String>,
    pub ref_table: String,
    pub ref_columns: Vec<String>,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

/// Represents a general constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub name: String,
    pub definition: String,
    pub constraint_type: String,
}

/// Represents a database view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct View {
    pub name: String,
    pub definition: String,
    pub columns: Vec<Column>,
    pub is_materialized: bool,
}

/// Represents a field definition from a Rust model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    pub rust_type: String,
    pub db_type: Option<String>,
    pub nullable: bool,
    pub primary_key: bool,
    pub unique: bool,
    pub default: Option<String>,
    pub foreign_key: Option<ForeignKeyDefinition>,
    pub comment: Option<String>,
    pub attributes: HashMap<String, String>,
}

/// Represents a foreign key definition from a Rust model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyDefinition {
    pub ref_table: String,
    pub ref_column: String,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}