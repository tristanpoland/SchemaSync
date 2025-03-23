//! Database connection handling
//!
//! This module provides functionality to establish and manage database connections.

use sqlx::{
    mysql::MySqlPoolOptions,
    postgres::PgPoolOptions,
    sqlite::SqlitePoolOptions,
    Any, AnyPool, MySql, MySqlPool, Pool, Postgres, PgPool, Sqlite, SqlitePool,
};

use crate::config::DatabaseConfig;
use crate::error::{Error, Result};

/// Enumeration of supported database types
#[derive(Debug, Clone)]
pub enum DatabaseConnection {
    Postgres(Pool<Postgres>),
    MySql(Pool<MySql>),
    Sqlite(Pool<Sqlite>),
    Any(AnyPool),
}

impl DatabaseConnection {
    /// Create a new database connection from configuration
    pub async fn connect(config: &DatabaseConfig) -> Result<Self> {
        let pool_size = config.pool_size.unwrap_or(10) as u32;
        let timeout_seconds = config.timeout_seconds.unwrap_or(30);
        
        match config.driver.as_str() {
            "postgres" => {
                let pool = PgPoolOptions::new()
                    .max_connections(pool_size)
                    .acquire_timeout(std::time::Duration::from_secs(timeout_seconds))
                    .connect(&config.url)
                    .await?;
                    
                Ok(DatabaseConnection::Postgres(pool))
            }
            "mysql" => {
                let pool = MySqlPoolOptions::new()
                    .max_connections(pool_size)
                    .acquire_timeout(std::time::Duration::from_secs(timeout_seconds))
                    .connect(&config.url)
                    .await?;
                    
                Ok(DatabaseConnection::MySql(pool))
            }
            "sqlite" => {
                let pool = SqlitePoolOptions::new()
                    .max_connections(pool_size)
                    .acquire_timeout(std::time::Duration::from_secs(timeout_seconds))
                    .connect(&config.url)
                    .await?;
                    
                Ok(DatabaseConnection::Sqlite(pool))
            }
            _ => Err(Error::DatabaseError(format!(
                "Unsupported database driver: {}", config.driver
            ))),
        }
    }
    
    /// Get the schema name from the connection
    pub fn get_schema(&self) -> Option<&str> {
        None // In a real implementation, this would extract the schema from the connection
    }
    
    /// Execute a SQL query
    pub async fn execute(&self, sql: &str) -> Result<()> {
        match self {
            DatabaseConnection::Postgres(pool) => {
                sqlx::query(sql).execute(pool).await?;
                Ok(())
            }
            DatabaseConnection::MySql(pool) => {
                sqlx::query(sql).execute(pool).await?;
                Ok(())
            }
            DatabaseConnection::Sqlite(pool) => {
                sqlx::query(sql).execute(pool).await?;
                Ok(())
            }
            DatabaseConnection::Any(pool) => {
                sqlx::query(sql).execute(pool).await?;
                Ok(())
            }
        }
    }
}