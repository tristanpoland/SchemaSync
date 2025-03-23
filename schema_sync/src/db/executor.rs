//! SQL executor
//!
//! This module provides SQL execution functionality.

use crate::db::connection::DatabaseConnection;
use crate::error::Result;

/// SQL executor for running queries
pub struct SqlExecutor {
    connection: DatabaseConnection,
}

impl SqlExecutor {
    /// Create a new SQL executor
    pub fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }
    
    /// Execute a single SQL statement
    pub async fn execute(&self, sql: &str) -> Result<()> {
        self.connection.execute(sql).await
    }
    
    /// Execute multiple SQL statements in order
    pub async fn execute_batch(&self, statements: &[String]) -> Result<()> {
        for statement in statements {
            self.execute(statement).await?;
        }
        
        Ok(())
    }
    
    /// Execute multiple SQL statements in a transaction
    pub async fn execute_in_transaction(&self, statements: &[String]) -> Result<()> {
        // Start transaction
        self.execute("BEGIN;").await?;
        
        // Execute statements
        match self.execute_batch(statements).await {
            Ok(_) => {
                // Commit transaction
                self.execute("COMMIT;").await
            }
            Err(e) => {
                // Rollback transaction
                let _ = self.execute("ROLLBACK;").await;
                Err(e)
            }
        }
    }
    
    /// Get database connection
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }
}