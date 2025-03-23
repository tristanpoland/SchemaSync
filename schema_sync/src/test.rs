//! Tests for SchemaSync
//!
//! This file contains unit and integration tests for the SchemaSync library.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::fs;
    use std::collections::HashMap;
    use tempfile::tempdir;
    use rstest::*;
    use pretty_assertions::assert_eq;
    
    use schema_sync::{
        Config, DatabaseConnection, ModelRegistry, SchemaAnalyzer, 
        SchemaDiff, MigrationGenerator, Error
    };
    use schema_sync::schema::types::{
        Column, DatabaseSchema, FieldDefinition, ForeignKey, 
        Index, PrimaryKey, Table, View
    };
    use schema_sync::models::SchemaSyncModel;
    use schema_sync::utils::naming;

    // Helper function to create a test configuration
    fn test_config() -> Config {
        let config_str = r###"
        [database]
        driver = "postgres"
        url = "postgres://postgres:password@localhost:5432/schema_sync_test"
        pool_size = 5
        timeout_seconds = 10
        schema = "public"
        enable_ssl = false

        [migrations]
        directory = "./test_migrations"
        naming = "timestamp_description"
        auto_generate = true
        auto_apply = false
        transaction_per_migration = true
        dry_run = true
        backup_before_migrate = false
        history_table = "schema_sync_history"

        [models]
        paths = ["./tests/models"]
        exclude_paths = []
        attributes = ["#[schema_sync]"]
        recursive_scan = true
        derive_macros = ["Serialize", "Deserialize"]

        [schema]
        strict_mode = true
        allow_column_removal = false
        allow_table_removal = false
        default_nullable = false
        index_foreign_keys = true
        unique_constraints_as_indices = true
        add_updated_at_column = true
        add_created_at_column = true

        [naming]
        table_style = "snake_case"
        column_style = "snake_case"
        index_pattern = "ix_{table}_{columns}"
        constraint_pattern = "fk_{table}_{column}"
        pluralize_tables = true
        ignore_case_conflicts = false

        [type_mapping]
        custom = [
          { rust_type = "chrono::DateTime<chrono::Utc>", db_type = "TIMESTAMP WITH TIME ZONE" },
          { rust_type = "uuid::Uuid", db_type = "UUID" }
        ]
        "###;
        
        toml::from_str(config_str).expect("Failed to parse test config")
    }

    #[test]
    fn test_naming_conventions() {
        assert_eq!(naming::apply_naming_convention("UserProfile", "snake_case"), "user_profile");
        assert_eq!(naming::apply_naming_convention("user_profile", "camel_case"), "userProfile");
        assert_eq!(naming::apply_naming_convention("user_profile", "pascal_case"), "UserProfile");
        assert_eq!(naming::apply_naming_convention("UserProfile", "kebab_case"), "user-profile");
        assert_eq!(naming::apply_naming_convention("UserProfile", "screaming_snake_case"), "USER_PROFILE");
        
        assert_eq!(
            naming::get_table_name("UserProfile", "snake_case", true),
            "user_profiles"
        );
        
        assert_eq!(
            naming::get_column_name("firstName", "snake_case"),
            "first_name"
        );
        
        assert_eq!(
            naming::get_index_name("ix_{table}_{columns}", "users", &vec!["email".to_string()]),
            "ix_users_email"
        );
        
        assert_eq!(
            naming::get_foreign_key_name("fk_{table}_{column}", "posts", "author_id"),
            "fk_posts_author_id"
        );
    }
    
    #[test]
    fn test_config_loading() {
        let config = test_config();
        
        assert_eq!(config.database.driver, "postgres");
        assert_eq!(config.migrations.history_table, "schema_sync_history");
        assert_eq!(config.schema.add_created_at_column, true);
        assert_eq!(config.naming.pluralize_tables, true);
    }
    
    #[test]
    fn test_type_mapping() {
        struct TestModel;
        
        impl SchemaSyncModel for TestModel {
            fn get_table_name() -> String {
                "test_models".to_string()
            }
            
            fn get_field_definitions() -> Vec<FieldDefinition> {
                vec![
                    FieldDefinition {
                        name: "id".to_string(),
                        rust_type: "i32".to_string(),
                        db_type: None,
                        nullable: false,
                        primary_key: true,
                        unique: false,
                        default: None,
                        foreign_key: None,
                        comment: None,
                        attributes: HashMap::new(),
                    },
                    FieldDefinition {
                        name: "name".to_string(),
                        rust_type: "String".to_string(),
                        db_type: None,
                        nullable: false,
                        primary_key: false,
                        unique: true,
                        default: None,
                        foreign_key: None,
                        comment: None,
                        attributes: HashMap::new(),
                    },
                ]
            }
            
            fn register_with_schema_sync() {}
        }
        
        let config = test_config();
        let registry = ModelRegistry::new(&config.models);
        
        // Test default mappings
        assert_eq!(
            registry.map_type_to_db_type("i32", &config).unwrap(),
            "INTEGER"
        );
        assert_eq!(
            registry.map_type_to_db_type("String", &config).unwrap(),
            "VARCHAR(255)"
        );
        assert_eq!(
            registry.map_type_to_db_type("bool", &config).unwrap(),
            "BOOLEAN"
        );
        
        // Test custom mappings
        assert_eq!(
            registry.map_type_to_db_type("chrono::DateTime<chrono::Utc>", &config).unwrap(),
            "TIMESTAMP WITH TIME ZONE"
        );
        assert_eq!(
            registry.map_type_to_db_type("uuid::Uuid", &config).unwrap(),
            "UUID"
        );
    }
    
    #[test]
    fn test_schema_diff_generation() {
        // Create current schema
        let mut current_schema = DatabaseSchema::new(Some("public".to_string()));
        
        let mut users_table = Table::new("users");
        users_table.add_column(Column {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            default: None,
            comment: None,
            is_unique: false,
            is_generated: false,
            generation_expression: None,
        });
        users_table.add_column(Column {
            name: "name".to_string(),
            data_type: "VARCHAR(255)".to_string(),
            nullable: false,
            default: None,
            comment: None,
            is_unique: false,
            is_generated: false,
            generation_expression: None,
        });
        users_table.set_primary_key(PrimaryKey {
            name: Some("pk_users".to_string()),
            columns: vec!["id".to_string()],
        });
        
        current_schema.add_table(users_table);
        
        // Create target schema (with changes)
        let mut target_schema = DatabaseSchema::new(Some("public".to_string()));
        
        let mut users_table = Table::new("users");
        users_table.add_column(Column {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            default: None,
            comment: None,
            is_unique: false,
            is_generated: false,
            generation_expression: None,
        });
        users_table.add_column(Column {
            name: "name".to_string(),
            data_type: "VARCHAR(255)".to_string(),
            nullable: false,
            default: None,
            comment: None,
            is_unique: false,
            is_generated: false,
            generation_expression: None,
        });
        // New column
        users_table.add_column(Column {
            name: "email".to_string(),
            data_type: "VARCHAR(255)".to_string(),
            nullable: false,
            default: None,
            comment: None,
            is_unique: true,
            is_generated: false,
            generation_expression: None,
        });
        users_table.set_primary_key(PrimaryKey {
            name: Some("pk_users".to_string()),
            columns: vec!["id".to_string()],
        });
        
        target_schema.add_table(users_table);
        
        // New table
        let mut posts_table = Table::new("posts");
        posts_table.add_column(Column {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            default: None,
            comment: None,
            is_unique: false,
            is_generated: false,
            generation_expression: None,
        });
        posts_table.add_column(Column {
            name: "title".to_string(),
            data_type: "VARCHAR(255)".to_string(),
            nullable: false,
            default: None,
            comment: None,
            is_unique: false,
            is_generated: false,
            generation_expression: None,
        });
        posts_table.add_column(Column {
            name: "user_id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            default: None,
            comment: None,
            is_unique: false,
            is_generated: false,
            generation_expression: None,
        });
        posts_table.set_primary_key(PrimaryKey {
            name: Some("pk_posts".to_string()),
            columns: vec!["id".to_string()],
        });
        posts_table.foreign_keys.push(ForeignKey {
            name: "fk_posts_user_id".to_string(),
            columns: vec!["user_id".to_string()],
            ref_table: "users".to_string(),
            ref_columns: vec!["id".to_string()],
            on_delete: Some("CASCADE".to_string()),
            on_update: Some("CASCADE".to_string()),
        });
        
        target_schema.add_table(posts_table);
        
        // Generate diff
        let config = test_config();
        let diff = SchemaDiff::generate(current_schema, target_schema, &config.schema);
        
        // Verify diff
        assert_eq!(diff.tables_to_create.len(), 1);
        assert_eq!(diff.tables_to_create[0].name, "posts");
        
        assert_eq!(diff.tables_to_drop.len(), 0); // No table removal allowed
        
        assert_eq!(diff.columns_to_add.len(), 1);
        assert!(diff.columns_to_add.contains_key("users"));
        assert_eq!(diff.columns_to_add["users"].len(), 1);
        assert_eq!(diff.columns_to_add["users"][0].name, "email");
        
        assert_eq!(diff.columns_to_drop.len(), 0); // No column removal allowed
    }
    
    #[test]
    fn test_migration_generator() {
        // Create a simple schema diff
        let mut diff = SchemaDiff {
            tables_to_create: Vec::new(),
            tables_to_drop: Vec::new(),
            columns_to_add: HashMap::new(),
            columns_to_drop: HashMap::new(),
            columns_to_alter: HashMap::new(),
            indices_to_create: HashMap::new(),
            indices_to_drop: HashMap::new(),
            foreign_keys_to_create: HashMap::new(),
            foreign_keys_to_drop: HashMap::new(),
        };
        
        // Add a table to create
        let mut users_table = Table::new("users");
        users_table.add_column(Column {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            default: None,
            comment: None,
            is_unique: false,
            is_generated: false,
            generation_expression: None,
        });
        users_table.add_column(Column {
            name: "name".to_string(),
            data_type: "VARCHAR(255)".to_string(),
            nullable: false,
            default: None,
            comment: None,
            is_unique: false,
            is_generated: false,
            generation_expression: None,
        });
        users_table.set_primary_key(PrimaryKey {
            name: Some("pk_users".to_string()),
            columns: vec!["id".to_string()],
        });
        
        diff.tables_to_create.push(users_table);
        
        // Add a column to add
        let email_column = Column {
            name: "email".to_string(),
            data_type: "VARCHAR(255)".to_string(),
            nullable: false,
            default: None,
            comment: None,
            is_unique: true,
            is_generated: false,
            generation_expression: None,
        };
        
        diff.columns_to_add.insert("users".to_string(), vec![email_column]);
        
        // Generate migrations
        let config = test_config();
        let generator = MigrationGenerator::new(&config);
        
        #[cfg(feature = "tokio")]
        {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let migrations = runtime.block_on(generator.generate_migration_sql(&diff)).unwrap();
            
            assert_eq!(migrations.len(), 2);
            assert!(migrations[0].contains("CREATE TABLE IF NOT EXISTS users"));
            assert!(migrations[1].contains("ALTER TABLE users ADD COLUMN email VARCHAR(255) NOT NULL"));
        }
    }
    
    #[rstest]
    #[case("snake_case", "UserProfile", "user_profile")]
    #[case("camel_case", "user_profile", "userProfile")]
    #[case("pascal_case", "user_profile", "UserProfile")]
    #[case("kebab_case", "UserProfile", "user-profile")]
    fn test_naming_convention_variations(#[case] style: &str, #[case] input: &str, #[case] expected: &str) {
        assert_eq!(naming::apply_naming_convention(input, style), expected);
    }
    
    #[rstest]
    #[case(true, "user", "users")]
    #[case(false, "user", "user")]
    #[case(true, "activity", "activities")]
    #[case(true, "category", "categories")]
    fn test_pluralization(#[case] pluralize: bool, #[case] input: &str, #[case] expected: &str) {
        assert_eq!(naming::get_table_name(input, "snake_case", pluralize), expected);
    }
    
    #[test]
    fn test_identifier_conflicts() {
        let names = vec![
            "User".to_string(),
            "user".to_string(),
            "admin".to_string(),
        ];
        
        // With case sensitivity
        assert!(naming::check_identifier_conflicts(&names, false).is_none());
        
        // Without case sensitivity
        let conflict = naming::check_identifier_conflicts(&names, true);
        assert!(conflict.is_some());
        let (name1, name2) = conflict.unwrap();
        assert!(name1 == "User" || name1 == "user");
        assert!(name2 == "User" || name2 == "user");
        assert_ne!(name1, name2);
    }
    
    #[test]
    fn test_sanitize_identifier() {
        assert_eq!(naming::sanitize_identifier("user-name"), "user_name");
        assert_eq!(naming::sanitize_identifier("123user"), "_123user");
        assert_eq!(naming::sanitize_identifier("user.name"), "user_name");
        assert_eq!(naming::sanitize_identifier("user@name"), "user_name");
    }
    
    #[test]
    fn test_truncate_identifier() {
        let long_name = "this_is_a_very_long_identifier_that_exceeds_database_limits";
        let truncated = naming::truncate_identifier(long_name, 30);
        
        assert_eq!(truncated.len(), 30);
        assert!(truncated.starts_with("this_is_a_very_long_identi"));
        assert!(truncated.contains("_"));
    }
    
    // Integration tests that require a database connection
    #[cfg(feature = "integration_tests")]
    mod integration_tests {
        use super::*;
        use schema_sync::{SchemaSyncClient, init};
        
        // These tests require a PostgreSQL database
        // They are only run when the "integration_tests" feature is enabled
        
        #[test]
        fn test_schema_analyzer() {
            let config = test_config();
            
            #[cfg(feature = "tokio")]
            {
                let runtime = tokio::runtime::Runtime::new().unwrap();
                
                runtime.block_on(async {
                    let conn = DatabaseConnection::connect(&config.database).await.unwrap();
                    let analyzer = SchemaAnalyzer::new(conn);
                    
                    let schema = analyzer.analyze().await.unwrap();
                    
                    // Verify schema contains expected tables
                    // This depends on the test database setup
                    assert!(schema.tables.contains_key("schema_sync_history"));
                });
            }
        }
        
        #[test]
        fn test_end_to_end_workflow() {
            // Create temporary directory for test models
            let temp_dir = tempdir().unwrap();
            let models_dir = temp_dir.path().join("models");
            fs::create_dir_all(&models_dir).unwrap();
            
            // Create test model file
            let model_content = r#"
            use serde::{Serialize, Deserialize};
            use chrono::{DateTime, Utc};
            use uuid::Uuid;
            
            #[derive(Serialize, Deserialize)]
            #[schema_sync]
            pub struct TestModel {
                #[schema_sync_field(primary_key = true)]
                pub id: i32,
                
                pub name: String,
                
                #[schema_sync_field(unique = true)]
                pub code: String,
                
                #[schema_sync_field(nullable = true)]
                pub description: Option<String>,
            }
            "#;
            
            fs::write(models_dir.join("test_model.rs"), model_content).unwrap();
            
            // Create test config that points to our temp directory
            let mut config = test_config();
            config.models.paths = vec![models_dir.to_str().unwrap().to_string()];
            config.migrations.dry_run = true; // Don't actually apply migrations
            
            #[cfg(feature = "tokio")]
            {
                let runtime = tokio::runtime::Runtime::new().unwrap();
                
                runtime.block_on(async {
                    // Create client
                    let mut client = SchemaSyncClient::new(config).await.unwrap();
                    
                    // Run sync workflow
                    let result = client.sync_database().await;
                    
                    // Should succeed with migrations (in dry run mode)
                    assert!(result.is_ok());
                });
            }
        }
    }
}