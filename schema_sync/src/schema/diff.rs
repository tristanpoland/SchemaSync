//! Schema difference calculator
//!
//! This module compares two database schemas and calculates the differences

use std::collections::{HashMap, HashSet};

use crate::config::SchemaConfig;
use crate::error::Result;
use crate::schema::types::{Column, DatabaseSchema, Table};

/// Represents changes needed to synchronize two schemas
#[derive(Debug, Clone)]
pub struct SchemaDiff {
    pub tables_to_create: Vec<Table>,
    pub tables_to_drop: Vec<String>,
    pub columns_to_add: HashMap<String, Vec<Column>>,
    pub columns_to_drop: HashMap<String, Vec<String>>,
    pub columns_to_alter: HashMap<String, Vec<ColumnChange>>,
    pub indices_to_create: HashMap<String, Vec<String>>,
    pub indices_to_drop: HashMap<String, Vec<String>>,
    pub foreign_keys_to_create: HashMap<String, Vec<String>>,
    pub foreign_keys_to_drop: HashMap<String, Vec<String>>,
}

impl SchemaDiff {
    /// Generate a schema diff between two database schemas
    pub fn generate(
        current_schema: DatabaseSchema, 
        target_schema: DatabaseSchema, 
        schema_config: &SchemaConfig
    ) -> Self {
        // Tables to create (in target but not in current)
        let tables_to_create = target_schema
            .tables
            .values()
            .filter(|table| !current_schema.tables.contains_key(&table.name))
            .cloned()
            .collect();
            
        // Tables to drop (in current but not in target)
        let tables_to_drop = if schema_config.allow_table_removal {
            current_schema
                .tables
                .keys()
                .filter(|&name| !target_schema.tables.contains_key(name))
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        
        // Process tables that exist in both schemas for column changes
        let mut columns_to_add = HashMap::new();
        let mut columns_to_drop = HashMap::new();
        let mut columns_to_alter = HashMap::new();
        
        for (table_name, target_table) in &target_schema.tables {
            if let Some(current_table) = current_schema.tables.get(table_name) {
                // Map columns by name for easier comparison
                let current_columns: HashMap<String, &Column> = current_table
                    .columns
                    .iter()
                    .map(|col| (col.name.clone(), col))
                    .collect();
                
                let target_columns: HashMap<String, &Column> = target_table
                    .columns
                    .iter()
                    .map(|col| (col.name.clone(), col))
                    .collect();
                
                // Columns to add (in target but not in current)
                let add_columns: Vec<Column> = target_table
                    .columns
                    .iter()
                    .filter(|col| !current_columns.contains_key(&col.name))
                    .cloned()
                    .collect();
                
                if !add_columns.is_empty() {
                    columns_to_add.insert(table_name.clone(), add_columns);
                }
                
                // Columns to drop (in current but not in target)
                if schema_config.allow_column_removal {
                    let drop_columns: Vec<String> = current_table
                        .columns
                        .iter()
                        .filter(|col| !target_columns.contains_key(&col.name))
                        .map(|col| col.name.clone())
                        .collect();
                    
                    if !drop_columns.is_empty() {
                        columns_to_drop.insert(table_name.clone(), drop_columns);
                    }
                }
                
                // Columns to alter (different definition in target)
                let alter_columns: Vec<ColumnChange> = target_table
                    .columns
                    .iter()
                    .filter_map(|target_col| {
                        if let Some(current_col) = current_columns.get(&target_col.name) {
                            if Self::column_needs_alteration(current_col, target_col, schema_config) {
                                Some(ColumnChange {
                                    column_name: target_col.name.clone(),
                                    from: (*current_col).clone(),
                                    to: target_col.clone(),
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                
                if !alter_columns.is_empty() {
                    columns_to_alter.insert(table_name.clone(), alter_columns);
                }
            }
        }
        
        // TODO: Implement index and foreign key diff logic
        
        Self {
            tables_to_create,
            tables_to_drop,
            columns_to_add,
            columns_to_drop,
            columns_to_alter,
            indices_to_create: HashMap::new(),
            indices_to_drop: HashMap::new(),
            foreign_keys_to_create: HashMap::new(),
            foreign_keys_to_drop: HashMap::new(),
        }
    }
    
    /// Check if a column needs to be altered
    fn column_needs_alteration(
        current: &Column, 
        target: &Column, 
        schema_config: &SchemaConfig
    ) -> bool {
        // Type different
        if current.data_type != target.data_type {
            return true;
        }
        
        // Nullability different
        if current.nullable != target.nullable {
            return true;
        }
        
        // Default value different
        if current.default != target.default {
            return true;
        }
        
        // Handle uniqueness changes
        if current.is_unique != target.is_unique {
            return true;
        }
        
        false
    }
    
    /// Check if the diff is empty (no changes needed)
    pub fn is_empty(&self) -> bool {
        self.tables_to_create.is_empty()
            && self.tables_to_drop.is_empty()
            && self.columns_to_add.is_empty()
            && self.columns_to_drop.is_empty()
            && self.columns_to_alter.is_empty()
            && self.indices_to_create.is_empty()
            && self.indices_to_drop.is_empty()
            && self.foreign_keys_to_create.is_empty()
            && self.foreign_keys_to_drop.is_empty()
    }
}

/// Represents a column change
#[derive(Debug, Clone)]
pub struct ColumnChange {
    pub column_name: String,
    pub from: Column,
    pub to: Column,
}