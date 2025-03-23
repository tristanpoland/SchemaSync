//! Migration management
//!
//! This module handles the execution and tracking of database migrations.

use chrono::Utc;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::config::MigrationsConfig;
use crate::db::connection::DatabaseConnection;
use crate::error::{Error, Result};

/// Apply migrations to the database
pub async fn apply_migrations(
    connection: &DatabaseConnection,
    migrations: Vec<String>,
    config: &MigrationsConfig,
) -> Result<()> {
    // Create migrations directory if it doesn't exist
    fs::create_dir_all(&config.directory)?;

    // Create migration history table if it doesn't exist
    ensure_migration_history_table(connection, &config.history_table).await?;

    for (i, migration_sql) in migrations.iter().enumerate() {
        let migration_id = generate_migration_id(i);
        let filename = format!("{}_{}.sql", migration_id, "schema_sync_migration");
        let filepath = Path::new(&config.directory).join(&filename);

        // Write migration to file
        let mut file = File::create(&filepath)?;
        file.write_all(migration_sql.as_bytes())?;

        // Apply migration
        if !config.dry_run {
            tracing::info!(migration_id = migration_id, "Applying migration");

            if config.transaction_per_migration {
                apply_migration_in_transaction(connection, migration_sql).await?;
            } else {
                connection.execute(migration_sql).await?;
            }

            // Record migration in history table
            record_migration(connection, &config.history_table, &migration_id, &filename).await?;

            tracing::info!(
                migration_id = migration_id,
                "Migration applied successfully"
            );
        }
    }

    Ok(())
}

/// Ensure the migration history table exists
async fn ensure_migration_history_table(
    connection: &DatabaseConnection,
    table_name: &str,
) -> Result<()> {
    let create_table_sql = format!(
        "CREATE TABLE IF NOT EXISTS {} (
            id SERIAL PRIMARY KEY,
            migration_id VARCHAR(255) NOT NULL,
            name VARCHAR(255) NOT NULL,
            applied_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
            checksum VARCHAR(64) NULL,
            execution_time_ms INTEGER NULL
        )",
        table_name
    );

    connection.execute(&create_table_sql).await
}

/// Apply a migration within a transaction
async fn apply_migration_in_transaction(
    connection: &DatabaseConnection,
    migration_sql: &str,
) -> Result<()> {
    // Start transaction SQL depends on database type
    let start_transaction = "BEGIN;";
    let commit_transaction = "COMMIT;";

    // Execute start transaction
    connection.execute(start_transaction).await?;

    // Execute migration SQL
    match connection.execute(migration_sql).await {
        Ok(_) => {
            // Commit transaction
            connection.execute(commit_transaction).await?;
            Ok(())
        }
        Err(e) => {
            // Rollback transaction
            let rollback_transaction = "ROLLBACK;";
            let _ = connection.execute(rollback_transaction).await;
            Err(e)
        }
    }
}

/// Record a migration in the history table
async fn record_migration(
    connection: &DatabaseConnection,
    table_name: &str,
    migration_id: &str,
    filename: &str,
) -> Result<()> {
    let sql = format!(
        "INSERT INTO {} (migration_id, name, applied_at) VALUES ('{}', '{}', CURRENT_TIMESTAMP)",
        table_name, migration_id, filename
    );

    connection.execute(&sql).await
}

/// Generate a migration ID based on timestamp
fn generate_migration_id(sequence: usize) -> String {
    let now = Utc::now();
    format!("{}_{:04}", now.format("%Y%m%d%H%M%S"), sequence)
}
