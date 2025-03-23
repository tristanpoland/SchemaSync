//! Model registry for SchemaSync
//!
//! This module manages the registration and discovery of model structs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use regex::Regex;
use syn::{parse_file, Attribute, Fields, Item, ItemStruct};
use quote::ToTokens;

use crate::config::{Config, ModelsConfig};
use crate::error::{Error, Result};
use crate::schema::types::{DatabaseSchema, FieldDefinition, Table};
use crate::utils::naming::apply_naming_convention;

/// A model that can be synchronized with the database
pub trait SchemaSyncModel {
    /// Get the table name for this model
    fn get_table_name() -> String;
    
    /// Get field definitions for this model
    fn get_field_definitions() -> Vec<FieldDefinition>;
    
    /// Register this model with SchemaSync
    fn register_with_schema_sync();
}

/// Registry for SchemaSync models
pub struct ModelRegistry {
    models: HashMap<String, ModelInfo>,
    config: ModelsConfig,
}

/// Information about a registered model
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub file_path: PathBuf,
    pub table_name: String,
    pub fields: Vec<FieldDefinition>,
    pub attributes: HashMap<String, String>,
}

impl ModelRegistry {
    /// Create a new model registry
    pub fn new(config: &ModelsConfig) -> Self {
        Self {
            models: HashMap::new(),
            config: config.clone(),
        }
    }
    
    /// Scan directories for model definitions and register them
    pub fn scan_and_register(&mut self, config: &Config) -> Result<()> {
        let attribute_patterns: Vec<Regex> = self.config.attributes
            .iter()
            .map(|attr| {
                Regex::new(&format!(r"#\[{}.*\]", attr.trim_start_matches("#[").trim_end_matches("]")))
                    .map_err(|e| Error::ModelRegistrationError(format!("Invalid attribute regex: {}", e)))
            })
            .collect::<Result<Vec<Regex>>>()?;
        
        // Create a copy of the paths to avoid borrowing self
        let paths = self.config.paths.clone();
        let recursive_scan = self.config.recursive_scan;
        let exclude_paths = self.config.exclude_paths.clone().unwrap_or_default();
        
        for path in &paths {
            let base_path = Path::new(path);
            
            if !base_path.exists() {
                return Err(Error::ModelRegistrationError(
                    format!("Path does not exist: {}", path)
                ));
            }
            
            // Walk directory and find Rust files
            for entry in WalkDir::new(base_path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                
                // Skip if path is in excluded paths
                if exclude_paths.iter().any(|exclude| path.starts_with(exclude)) {
                    continue;
                }
                
                // Only process .rs files
                if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
                    self.process_file(path, &attribute_patterns, config)?;
                }
                
                // If not recursive, don't go into subdirectories
                if !recursive_scan && path.is_dir() && path != base_path {
                    continue;
                }
            }
        }
        
        Ok(())
    }
    
    /// Process a Rust file and extract model definitions
    fn process_file(
        &mut self,
        file_path: &Path,
        attribute_patterns: &[Regex],
        config: &Config,
    ) -> Result<()> {
        let file_content = std::fs::read_to_string(file_path)?;
        let syntax = parse_file(&file_content)
            .map_err(|e| Error::SyntaxError(format!("Failed to parse file: {}", e)))?;
        
        for item in syntax.items {
            if let Item::Struct(item_struct) = item {
                // Check if struct has one of the required attributes
                if self.has_schema_sync_attribute(&item_struct.attrs, attribute_patterns) {
                    self.register_model(file_path, item_struct, config)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if a struct has a SchemaSync attribute
    fn has_schema_sync_attribute(&self, attrs: &[Attribute], patterns: &[Regex]) -> bool {
        for attr in attrs {
            let attr_str = attr.to_token_stream().to_string();
            if patterns.iter().any(|pattern| pattern.is_match(&attr_str)) {
                return true;
            }
        }
        
        false
    }
    
    /// Register a model from a struct definition
    fn register_model(
        &mut self,
        file_path: &Path,
        item_struct: ItemStruct,
        config: &Config,
    ) -> Result<()> {
        let struct_name = item_struct.ident.to_string();
        
        // Extract table name from attribute or apply naming convention
        let table_name = self.extract_table_name(&item_struct, &struct_name, &config.naming)?;
        
        // Extract field definitions
        let fields = match item_struct.fields {
            Fields::Named(named_fields) => {
                named_fields
                    .named
                    .into_iter()
                    .filter_map(|field| {
                        let field_name = field.ident?.to_string();
                        let field_type = field.ty.to_token_stream().to_string();
                        
                        // Extract field attributes for additional properties
                        let mut attributes = HashMap::new();
                        let mut primary_key = false;
                        let mut nullable = false;
                        let mut unique = false;
                        let mut default = None;
                        let mut foreign_key = None;
                        let mut comment = None;
                        let mut db_type = None;
                        
                        for attr in &field.attrs {
                            if attr.path().is_ident("schema_sync_field") {
                                let attr_str = attr.to_token_stream().to_string();
                                
                                // Parse schema_sync_field attributes
                                if attr_str.contains("primary_key") {
                                    primary_key = attr_str.contains("primary_key = true");
                                }
                                
                                if attr_str.contains("nullable") {
                                    nullable = attr_str.contains("nullable = true");
                                }
                                
                                if attr_str.contains("unique") {
                                    unique = attr_str.contains("unique = true");
                                }
                                
                                if attr_str.contains("default") {
                                    // Extract default value between quotes
                                    if let Some(start) = attr_str.find("default = \"") {
                                        if let Some(end) = attr_str[start + 11..].find('"') {
                                            default = Some(attr_str[start + 11..start + 11 + end].to_string());
                                        }
                                    }
                                }
                                
                                if attr_str.contains("comment") {
                                    // Extract comment value between quotes
                                    if let Some(start) = attr_str.find("comment = \"") {
                                        if let Some(end) = attr_str[start + 11..].find('"') {
                                            comment = Some(attr_str[start + 11..start + 11 + end].to_string());
                                        }
                                    }
                                }
                                
                                if attr_str.contains("db_type") {
                                    // Extract db_type value between quotes
                                    if let Some(start) = attr_str.find("db_type = \"") {
                                        if let Some(end) = attr_str[start + 11..].find('"') {
                                            db_type = Some(attr_str[start + 11..start + 11 + end].to_string());
                                        }
                                    }
                                }
                                
                                if attr_str.contains("foreign_key") {
                                    // Extract foreign_key value between quotes
                                    if let Some(start) = attr_str.find("foreign_key = \"") {
                                        if let Some(end) = attr_str[start + 15..].find('"') {
                                            let fk_value = attr_str[start + 15..start + 15 + end].to_string();
                                            
                                            // Parse foreign key reference (table.column)
                                            if let Some(dot_pos) = fk_value.find('.') {
                                                let ref_table = fk_value[..dot_pos].to_string();
                                                let ref_column = fk_value[dot_pos + 1..].to_string();
                                                
                                                foreign_key = Some(crate::schema::types::ForeignKeyDefinition {
                                                    ref_table,
                                                    ref_column,
                                                    on_delete: None,
                                                    on_update: None,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Determine nullability from Option<T> type if not explicitly set
                        if !nullable && field_type.starts_with("Option < ") {
                            nullable = true;
                        }
                        
                        Some(FieldDefinition {
                            name: field_name,
                            rust_type: field_type,
                            db_type,
                            nullable,
                            primary_key,
                            unique,
                            default,
                            foreign_key,
                            comment,
                            attributes,
                        })
                    })
                    .collect()
            }
            _ => {
                return Err(Error::ModelRegistrationError(
                    format!("Only named fields are supported in struct: {}", struct_name)
                ));
            }
        };
        
        // Register the model
        let model_info = ModelInfo {
            name: struct_name.clone(),
            file_path: file_path.to_owned(),
            table_name,
            fields,
            attributes: HashMap::new(),
        };
        
        self.models.insert(struct_name, model_info);
        
        Ok(())
    }
    
    /// Extract table name from struct or attributes
    fn extract_table_name(
        &self,
        item_struct: &ItemStruct,
        struct_name: &str,
        naming_config: &crate::config::NamingConfig,
    ) -> Result<String> {
        // Check for explicit table name in attributes
        for attr in &item_struct.attrs {
            if attr.path().is_ident("schema_sync") {
                let attr_str = attr.to_token_stream().to_string();
                
                if attr_str.contains("table =") {
                    // Extract table name between quotes
                    if let Some(start) = attr_str.find("table = \"") {
                        if let Some(end) = attr_str[start + 9..].find('"') {
                            return Ok(attr_str[start + 9..start + 9 + end].to_string());
                        }
                    }
                }
            }
        }
        
        // Apply naming convention
        let table_name = apply_naming_convention(struct_name, &naming_config.table_style);
        
        // Apply pluralization if configured
        let final_name = if naming_config.pluralize_tables {
            use inflector::Inflector;
            table_name.to_plural()
        } else {
            table_name
        };
        
        Ok(final_name)
    }
    
    /// Convert registered models to database schema
    pub fn to_database_schema(&self, config: &Config) -> Result<DatabaseSchema> {
        let mut schema = DatabaseSchema::new(config.database.schema.clone());
        
        for (_, model_info) in &self.models {
            let mut table = Table::new(&model_info.table_name);
            
            // Convert fields to columns
            for field in &model_info.fields {
                // Map Rust type to database type
                let db_type = match &field.db_type {
                    Some(t) => t.clone(),
                    None => self.map_type_to_db_type(&field.rust_type, config)?,
                };
                
                let column = crate::schema::types::Column {
                    name: field.name.clone(),
                    data_type: db_type,
                    nullable: field.nullable,
                    default: field.default.clone(),
                    comment: field.comment.clone(),
                    is_unique: field.unique,
                    is_generated: false,
                    generation_expression: None,
                };
                
                table.add_column(column);
            }
            
            // Set primary key if defined
            let pk_fields: Vec<&FieldDefinition> = model_info.fields
                .iter()
                .filter(|f| f.primary_key)
                .collect();
                
            if !pk_fields.is_empty() {
                let pk_columns = pk_fields.iter().map(|f| f.name.clone()).collect();
                table.set_primary_key(crate::schema::types::PrimaryKey {
                    name: Some(format!("pk_{}", model_info.table_name)),
                    columns: pk_columns,
                });
            }
            
            // Add created_at and updated_at columns if configured
            if config.schema.add_created_at_column {
                let column_exists = table.columns.iter().any(|c| c.name == "created_at");
                
                if !column_exists {
                    table.add_column(crate::schema::types::Column {
                        name: "created_at".to_string(),
                        data_type: "TIMESTAMP WITH TIME ZONE".to_string(),
                        nullable: false,
                        default: Some("CURRENT_TIMESTAMP".to_string()),
                        comment: Some("Record creation timestamp".to_string()),
                        is_unique: false,
                        is_generated: false,
                        generation_expression: None,
                    });
                }
            }
            
            if config.schema.add_updated_at_column {
                let column_exists = table.columns.iter().any(|c| c.name == "updated_at");
                
                if !column_exists {
                    table.add_column(crate::schema::types::Column {
                        name: "updated_at".to_string(),
                        data_type: "TIMESTAMP WITH TIME ZONE".to_string(),
                        nullable: false,
                        default: Some("CURRENT_TIMESTAMP".to_string()),
                        comment: Some("Record last update timestamp".to_string()),
                        is_unique: false,
                        is_generated: false,
                        generation_expression: None,
                    });
                }
            }
            
            // Add indexes for unique and foreign key columns
            for field in &model_info.fields {
                // Add unique constraints
                if field.unique {
                    let index_name = format!("ix_{}_{}",
                        model_info.table_name,
                        field.name
                    );
                    
                    table.add_index(crate::schema::types::Index {
                        name: index_name,
                        columns: vec![field.name.clone()],
                        is_unique: true,
                        method: Some("btree".to_string()),
                    });
                }
                
                // Add foreign key constraints
                if let Some(fk) = &field.foreign_key {
                    // Generate foreign key name
                    let fk_name = crate::utils::get_foreign_key_name(
                        &config.naming.constraint_pattern,
                        &model_info.table_name,
                        &field.name,
                    );
                    
                    table.foreign_keys.push(crate::schema::types::ForeignKey {
                        name: fk_name,
                        columns: vec![field.name.clone()],
                        ref_table: fk.ref_table.clone(),
                        ref_columns: vec![fk.ref_column.clone()],
                        on_delete: fk.on_delete.clone(),
                        on_update: fk.on_update.clone(),
                    });
                    
                    // Add index for foreign key if configured
                    if config.schema.index_foreign_keys {
                        let index_name = format!("ix_{}_{}",
                            model_info.table_name,
                            field.name
                        );
                        
                        table.add_index(crate::schema::types::Index {
                            name: index_name,
                            columns: vec![field.name.clone()],
                            is_unique: false,
                            method: Some("btree".to_string()),
                        });
                    }
                }
            }
            
            schema.add_table(table);
        }
        
        Ok(schema)
    }
    
    /// Map Rust type to database type
    pub fn map_type_to_db_type(&self, rust_type: &str, config: &Config) -> Result<String> {
        // First check for custom type mappings
        if let Some(custom_mappings) = &config.type_mapping.custom {
            for mapping in custom_mappings {
                if mapping.rust_type == rust_type {
                    return Ok(mapping.db_type.clone());
                }
            }
        }
        
        // Then check for overrides
        if let Some(overrides) = &config.type_mapping.override_ {
            if let Some(db_type) = overrides.get(rust_type) {
                return Ok(db_type.clone());
            }
        }
        
        // Default mappings
        match rust_type {
            "String" | "&str" => Ok("VARCHAR(255)".to_string()),
            "i8" => Ok("SMALLINT".to_string()),
            "i16" => Ok("SMALLINT".to_string()),
            "i32" => Ok("INTEGER".to_string()),
            "i64" => Ok("BIGINT".to_string()),
            "u8" | "u16" | "u32" => Ok("INTEGER".to_string()),
            "u64" => Ok("BIGINT".to_string()),
            "f32" => Ok("REAL".to_string()),
            "f64" => Ok("DOUBLE PRECISION".to_string()),
            "bool" => Ok("BOOLEAN".to_string()),
            t if t.contains("Vec<u8>") => Ok("BYTEA".to_string()),
            t if t.contains("DateTime") => Ok("TIMESTAMP WITH TIME ZONE".to_string()),
            t if t.contains("NaiveDateTime") => Ok("TIMESTAMP".to_string()),
            t if t.contains("NaiveDate") => Ok("DATE".to_string()),
            t if t.contains("Uuid") => Ok("UUID".to_string()),
            t if t.contains("Decimal") => Ok("NUMERIC(20,6)".to_string()),
            t if t.contains("Json") || t.contains("Value") => Ok("JSONB".to_string()),
            _ => Err(Error::TypeMappingError(format!(
                "No mapping found for Rust type: {}", rust_type
            ))),
        }
    }
    
    /// Get all registered models
    pub fn get_models(&self) -> &HashMap<String, ModelInfo> {
        &self.models
    }
    
    /// Get a specific model by name
    pub fn get_model(&self, name: &str) -> Option<&ModelInfo> {
        self.models.get(name)
    }
}