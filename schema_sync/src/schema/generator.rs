//! Migration generator
//!
//! This module generates SQL migration statements from schema diffs

use crate::config::Config;
use crate::error::Result;
use crate::schema::diff::{ColumnChange, SchemaDiff};
use crate::schema::types::{Column, Table};

/// Migration SQL generator
pub struct MigrationGenerator<'a> {
    config: &'a Config,
}

impl<'a> MigrationGenerator<'a> {
    /// Create a new migration generator
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }
    
    /// Generate migration SQL from a schema diff
    pub async fn generate_migration_sql(&self, diff: &SchemaDiff) -> Result<Vec<String>> {
        let mut migrations = Vec::new();
        
        // Handle table creation
        for table in &diff.tables_to_create {
            migrations.push(self.generate_create_table_sql(table)?);
        }
        
        // Handle table deletion
        for table_name in &diff.tables_to_drop {
            migrations.push(self.generate_drop_table_sql(table_name)?);
        }
        
        // Handle column additions
        for (table_name, columns) in &diff.columns_to_add {
            migrations.push(self.generate_add_columns_sql(table_name, columns)?);
        }
        
        // Handle column deletions
        for (table_name, column_names) in &diff.columns_to_drop {
            migrations.push(self.generate_drop_columns_sql(table_name, column_names)?);
        }
        
        // Handle column modifications
        for (table_name, column_changes) in &diff.columns_to_alter {
            migrations.push(self.generate_alter_columns_sql(table_name, column_changes)?);
        }
        
        // TODO: Handle index and foreign key changes
        
        Ok(migrations)
    }
    
    /// Generate SQL to create a table
    fn generate_create_table_sql(&self, table: &Table) -> Result<String> {
        let db_type = &self.config.database.driver;
        
        match db_type.as_str() {
            "postgres" => self.generate_postgres_create_table_sql(table),
            "mysql" => self.generate_mysql_create_table_sql(table),
            "sqlite" => self.generate_sqlite_create_table_sql(table),
            _ => Err(crate::error::Error::MigrationError(format!(
                "Unsupported database type: {}", db_type
            ))),
        }
    }
    
    /// Generate PostgreSQL-specific table creation SQL
    fn generate_postgres_create_table_sql(&self, table: &Table) -> Result<String> {
        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", table.name);
        
        // Add columns
        let mut column_defs = Vec::new();
        for column in &table.columns {
            let nullable = if column.nullable { "NULL" } else { "NOT NULL" };
            let default = if let Some(default_val) = &column.default {
                format!(" DEFAULT {}", default_val)
            } else {
                String::new()
            };
            
            column_defs.push(format!(
                "  {} {}{} {}",
                column.name,
                column.data_type,
                default,
                nullable
            ));
        }
        
        // Add primary key
        if let Some(pk) = &table.primary_key {
            let columns = pk.columns.join(", ");
            column_defs.push(format!("  PRIMARY KEY ({})", columns));
        }
        
        sql.push_str(&column_defs.join(",\n"));
        sql.push_str("\n);\n");
        
        // Add indices
        for index in &table.indexes {
            let unique = if index.is_unique { "UNIQUE " } else { "" };
            let method = index.method.as_deref().unwrap_or("btree");
            let columns = index.columns.join(", ");
            
            sql.push_str(&format!(
                "CREATE {}INDEX {} ON {} USING {} ({});\n",
                unique,
                index.name,
                table.name,
                method,
                columns
            ));
        }
        
        // Add foreign keys
        for fk in &table.foreign_keys {
            let columns = fk.columns.join(", ");
            let ref_columns = fk.ref_columns.join(", ");
            let on_delete = fk.on_delete.as_deref().unwrap_or("NO ACTION");
            let on_update = fk.on_update.as_deref().unwrap_or("NO ACTION");
            
            sql.push_str(&format!(
                "ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({}) ON DELETE {} ON UPDATE {};\n",
                table.name,
                fk.name,
                columns,
                fk.ref_table,
                ref_columns,
                on_delete,
                on_update
            ));
        }
        
        Ok(sql)
    }
    
    // Generate MySQL-specific table creation SQL
    fn generate_mysql_create_table_sql(&self, table: &Table) -> Result<String> {
        // MySQL implementation
        // (abbreviated for brevity)
        todo!("Implement MySQL table creation SQL")
    }
    
    // Generate SQLite-specific table creation SQL
    fn generate_sqlite_create_table_sql(&self, table: &Table) -> Result<String> {
        // SQLite implementation
        // (abbreviated for brevity)
        todo!("Implement SQLite table creation SQL")
    }
    
    /// Generate SQL to drop a table
    fn generate_drop_table_sql(&self, table_name: &str) -> Result<String> {
        Ok(format!("DROP TABLE IF EXISTS {};", table_name))
    }
    
    /// Generate SQL to add columns to a table
    fn generate_add_columns_sql(&self, table_name: &str, columns: &[Column]) -> Result<String> {
        let db_type = &self.config.database.driver;
        
        match db_type.as_str() {
            "postgres" => {
                let mut sql = String::new();
                
                for column in columns {
                    let nullable = if column.nullable { "NULL" } else { "NOT NULL" };
                    let default = if let Some(default_val) = &column.default {
                        format!(" DEFAULT {}", default_val)
                    } else {
                        String::new()
                    };
                    
                    sql.push_str(&format!(
                        "ALTER TABLE {} ADD COLUMN {} {}{} {};\n",
                        table_name,
                        column.name,
                        column.data_type,
                        default,
                        nullable
                    ));
                }
                
                Ok(sql)
            }
            "mysql" | "sqlite" => {
                // Abbreviated for brevity
                todo!("Implement for MySQL/SQLite")
            }
            _ => Err(crate::error::Error::MigrationError(format!(
                "Unsupported database type: {}", db_type
            ))),
        }
    }
    
    /// Generate SQL to drop columns from a table
    fn generate_drop_columns_sql(&self, table_name: &str, column_names: &[String]) -> Result<String> {
        let db_type = &self.config.database.driver;
        
        match db_type.as_str() {
            "postgres" => {
                let mut sql = String::new();
                
                for column_name in column_names {
                    sql.push_str(&format!(
                        "ALTER TABLE {} DROP COLUMN {};\n",
                        table_name,
                        column_name
                    ));
                }
                
                Ok(sql)
            }
            "mysql" | "sqlite" => {
                // Abbreviated for brevity
                todo!("Implement for MySQL/SQLite")
            }
            _ => Err(crate::error::Error::MigrationError(format!(
                "Unsupported database type: {}", db_type
            ))),
        }
    }
    
    /// Generate SQL to alter columns in a table
    fn generate_alter_columns_sql(
        &self,
        table_name: &str,
        column_changes: &[ColumnChange],
    ) -> Result<String> {
        let db_type = &self.config.database.driver;
        
        match db_type.as_str() {
            "postgres" => {
                let mut sql = String::new();
                
                for change in column_changes {
                    // Alter column type
                    if change.from.data_type != change.to.data_type {
                        sql.push_str(&format!(
                            "ALTER TABLE {} ALTER COLUMN {} TYPE {} USING {}::{};\n",
                            table_name,
                            change.column_name,
                            change.to.data_type,
                            change.column_name,
                            change.to.data_type
                        ));
                    }
                    
                    // Alter nullability
                    if change.from.nullable != change.to.nullable {
                        if change.to.nullable {
                            sql.push_str(&format!(
                                "ALTER TABLE {} ALTER COLUMN {} DROP NOT NULL;\n",
                                table_name,
                                change.column_name
                            ));
                        } else {
                            sql.push_str(&format!(
                                "ALTER TABLE {} ALTER COLUMN {} SET NOT NULL;\n",
                                table_name,
                                change.column_name
                            ));
                        }
                    }
                    
                    // Alter default value
                    if change.from.default != change.to.default {
                        if let Some(default_val) = &change.to.default {
                            sql.push_str(&format!(
                                "ALTER TABLE {} ALTER COLUMN {} SET DEFAULT {};\n",
                                table_name,
                                change.column_name,
                                default_val
                            ));
                        } else {
                            sql.push_str(&format!(
                                "ALTER TABLE {} ALTER COLUMN {} DROP DEFAULT;\n",
                                table_name,
                                change.column_name
                            ));
                        }
                    }
                }
                
                Ok(sql)
            }
            "mysql" | "sqlite" => {
                // Abbreviated for brevity
                todo!("Implement for MySQL/SQLite")
            }
            _ => Err(crate::error::Error::MigrationError(format!(
                "Unsupported database type: {}", db_type
            ))),
        }
    }
}