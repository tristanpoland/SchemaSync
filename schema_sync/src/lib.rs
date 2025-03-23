//! SchemaSync: A reverse-ORM for Rust that verifies and updates database schemas from structs
//!
//! SchemaSync allows you to define your database schema using Rust structs and automatically
//! generates and applies migrations to keep your database in sync with your code.

pub mod config;
pub mod db;
pub mod error;
pub mod models;
pub mod schema;
pub mod utils;

// Re-export main types for easier access
pub use config::Config;
pub use db::connection::DatabaseConnection;
pub use error::{Error, Result};
pub use schema_sync_macros::{schema_sync, SchemaSync};
pub use models::registry::ModelRegistry;
pub use schema::analyzer::SchemaAnalyzer;
pub use schema::diff::SchemaDiff;
pub use schema::generator::MigrationGenerator;

/// Initialize SchemaSync with the specified configuration file
pub async fn init(config_path: &str) -> Result<SchemaSyncClient> {
    let config = config::load_from_file(config_path)?;
    SchemaSyncClient::new(config).await
}

/// The main client for interacting with SchemaSync
pub struct SchemaSyncClient {
    config: Config,
    db_connection: DatabaseConnection,
    model_registry: ModelRegistry,
    schema_analyzer: SchemaAnalyzer,
}

impl SchemaSyncClient {
    /// Create a new SchemaSync client from configuration
    pub async fn new(config: Config) -> Result<Self> {
        let db_connection = DatabaseConnection::connect(&config.database).await?;
        let model_registry = ModelRegistry::new(&config.models);
        let schema_analyzer = SchemaAnalyzer::new(db_connection.clone());

        Ok(Self {
            config,
            db_connection,
            model_registry,
            schema_analyzer,
        })
    }

    /// Scan directories for model definitions and register them
    pub async fn register_models(&mut self) -> Result<()> {
        self.model_registry.scan_and_register(&self.config)?;
        Ok(())
    }

    /// Analyze the current database schema
    pub async fn analyze_database_schema(&self) -> Result<schema::types::DatabaseSchema> {
        self.schema_analyzer.analyze().await
    }

    /// Generate a schema diff between registered models and database
    pub async fn generate_schema_diff(&self) -> Result<SchemaDiff> {
        let db_schema = self.schema_analyzer.analyze().await?;
        let model_schema = self.model_registry.to_database_schema(&self.config)?;
        
        Ok(SchemaDiff::generate(db_schema, model_schema, &self.config.schema))
    }

    /// Generate migration SQL from schema diff
    pub async fn generate_migrations(&self, diff: &SchemaDiff) -> Result<Vec<String>> {
        let generator = MigrationGenerator::new(&self.config);
        generator.generate_migration_sql(diff).await
    }

    /// Apply migrations to database
    pub async fn apply_migrations(&self, migrations: Vec<String>) -> Result<()> {
        if self.config.migrations.dry_run {
            // Just log the migrations without applying
            for (i, migration) in migrations.iter().enumerate() {
                tracing::info!(migration_number = i + 1, sql = migration, "Migration SQL (dry run)");
            }
            return Ok(());
        }

        db::migrations::apply_migrations(
            &self.db_connection, 
            migrations, 
            &self.config.migrations
        ).await
    }

    /// Complete workflow: scan models, analyze db, generate and apply migrations
    pub async fn sync_database(&mut self) -> Result<()> {
        // Register all models
        self.register_models().await?;
        
        // Generate schema diff
        let diff = self.generate_schema_diff().await?;
        
        if diff.is_empty() {
            tracing::info!("Database schema is already in sync with models");
            return Ok(());
        }
        
        // Generate migrations
        let migrations = self.generate_migrations(&diff).await?;
        
        // Apply migrations
        self.apply_migrations(migrations).await
    }
}