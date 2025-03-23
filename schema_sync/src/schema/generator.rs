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
        
        // Handle index additions
        for (table_name, index_names) in &diff.indices_to_create {
            if let Some(table) = self.find_table_by_name(table_name, diff) {
                let indices: Vec<_> = table.indexes.iter()
                    .filter(|idx| index_names.contains(&idx.name))
                    .collect();
                    
                if !indices.is_empty() {
                    migrations.push(self.generate_create_indices_sql(table_name, &indices)?);
                }
            }
        }
        
        // Handle index deletions
        for (table_name, index_names) in &diff.indices_to_drop {
            migrations.push(self.generate_drop_indices_sql(table_name, index_names)?);
        }
        
        // Handle foreign key additions
        for (table_name, fk_names) in &diff.foreign_keys_to_create {
            if let Some(table) = self.find_table_by_name(table_name, diff) {
                let foreign_keys: Vec<_> = table.foreign_keys.iter()
                    .filter(|fk| fk_names.contains(&fk.name))
                    .collect();
                    
                if !foreign_keys.is_empty() {
                    migrations.push(self.generate_create_foreign_keys_sql(table_name, &foreign_keys)?);
                }
            }
        }
        
        // Handle foreign key deletions
        for (table_name, fk_names) in &diff.foreign_keys_to_drop {
            migrations.push(self.generate_drop_foreign_keys_sql(table_name, fk_names)?);
        }
        
        Ok(migrations)
    }
    
    /// Find a table by name in the diff
    fn find_table_by_name<'b>(&self, table_name: &str, diff: &'b SchemaDiff) -> Option<&'b Table> {
        diff.tables_to_create.iter().find(|t| t.name == table_name)
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
        
        // Add table comment if present
        if let Some(comment) = &table.comment {
            sql.push_str(&format!(
                "COMMENT ON TABLE {} IS '{}';\n",
                table.name,
                comment.replace('\'', "''")
            ));
        }
        
        // Add column comments if present
        for column in &table.columns {
            if let Some(comment) = &column.comment {
                sql.push_str(&format!(
                    "COMMENT ON COLUMN {}.{} IS '{}';\n",
                    table.name,
                    column.name,
                    comment.replace('\'', "''")
                ));
            }
        }
        
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
    
    /// Generate MySQL-specific table creation SQL
    fn generate_mysql_create_table_sql(&self, table: &Table) -> Result<String> {
        let mut sql = format!("CREATE TABLE IF NOT EXISTS `{}` (\n", table.name);
        
        // Add columns
        let mut column_defs = Vec::new();
        for column in &table.columns {
            let nullable = if column.nullable { "NULL" } else { "NOT NULL" };
            let default = if let Some(default_val) = &column.default {
                // Handle default values specifically for MySQL
                let mysql_default = match default_val.as_str() {
                    "CURRENT_TIMESTAMP" => "CURRENT_TIMESTAMP",
                    _ => &default_val
                };
                format!(" DEFAULT {}", mysql_default)
            } else {
                String::new()
            };
            
            // MySQL uses backticks for identifiers
            column_defs.push(format!(
                "  `{}` {}{} {}",
                column.name,
                self.translate_data_type_for_mysql(&column.data_type),
                default,
                nullable
            ));
            
            // Add column comment if present
            if let Some(comment) = &column.comment {
                column_defs.last_mut().unwrap().push_str(&format!(
                    " COMMENT '{}'",
                    comment.replace('\'', "''")
                ));
            }
        }
        
        // Add primary key
        if let Some(pk) = &table.primary_key {
            let pk_columns: Vec<String> = pk.columns.iter()
                .map(|col| format!("`{}`", col))
                .collect();
            
            column_defs.push(format!("  PRIMARY KEY ({})", pk_columns.join(", ")));
        }
        
        // Add keys for all unique constraints
        for index in table.indexes.iter().filter(|idx| idx.is_unique) {
            let index_columns: Vec<String> = index.columns.iter()
                .map(|col| format!("`{}`", col))
                .collect();
            
            column_defs.push(format!(
                "  UNIQUE KEY `{}` ({})",
                index.name,
                index_columns.join(", ")
            ));
        }
        
        // Add foreign keys
        for fk in &table.foreign_keys {
            let fk_columns: Vec<String> = fk.columns.iter()
                .map(|col| format!("`{}`", col))
                .collect();
                
            let ref_columns: Vec<String> = fk.ref_columns.iter()
                .map(|col| format!("`{}`", col))
                .collect();
                
            let on_delete = fk.on_delete.as_deref().unwrap_or("RESTRICT");
            let on_update = fk.on_update.as_deref().unwrap_or("RESTRICT");
            
            column_defs.push(format!(
                "  CONSTRAINT `{}` FOREIGN KEY ({}) REFERENCES `{}` ({}) ON DELETE {} ON UPDATE {}",
                fk.name,
                fk_columns.join(", "),
                fk.ref_table,
                ref_columns.join(", "),
                on_delete,
                on_update
            ));
        }
        
        sql.push_str(&column_defs.join(",\n"));
        
        // Add table options
        let mut table_options = Vec::new();
        
        // Default charset
        table_options.push("DEFAULT CHARACTER SET=utf8mb4".to_string());
        table_options.push("COLLATE=utf8mb4_unicode_ci".to_string());
        
        // Add table comment if present
        if let Some(comment) = &table.comment {
            let comment_option = format!("COMMENT='{}'", comment.replace('\'', "''"));
            table_options.push(comment_option);
        }
        
        if !table_options.is_empty() {
            sql.push_str(&format!("\n) {};\n", table_options.join(" ")));
        } else {
            sql.push_str("\n);\n");
        }
        
        // Create non-unique indices (MySQL doesn't include these in the CREATE TABLE)
        for index in table.indexes.iter().filter(|idx| !idx.is_unique) {
            let index_columns: Vec<String> = index.columns.iter()
                .map(|col| format!("`{}`", col))
                .collect();
            
            sql.push_str(&format!(
                "CREATE INDEX `{}` ON `{}` ({});\n",
                index.name,
                table.name,
                index_columns.join(", ")
            ));
        }
        
        Ok(sql)
    }
    
    /// Generate SQLite-specific table creation SQL
    fn generate_sqlite_create_table_sql(&self, table: &Table) -> Result<String> {
        let mut sql = format!("CREATE TABLE IF NOT EXISTS \"{}\" (\n", table.name);
        
        // Add columns
        let mut column_defs = Vec::new();
        for column in &table.columns {
            let nullable = if column.nullable { "" } else { "NOT NULL" };
            let default = if let Some(default_val) = &column.default {
                format!(" DEFAULT {}", default_val)
            } else {
                String::new()
            };
            
            let mut column_def = format!(
                "  \"{}\" {}{}",
                column.name,
                self.translate_data_type_for_sqlite(&column.data_type),
                default
            );
            
            // SQLite supports inline primary key for single-column primary keys
            if let Some(pk) = &table.primary_key {
                if pk.columns.len() == 1 && pk.columns[0] == column.name {
                    column_def.push_str(" PRIMARY KEY");
                    
                    // SQLite always has implicit rowid unless AUTOINCREMENT is specified
                    if column.data_type.to_lowercase().contains("int") {
                        column_def.push_str(" AUTOINCREMENT");
                    }
                }
            }
            
            if !nullable.is_empty() {
                column_def.push_str(&format!(" {}", nullable));
            }
            
            column_defs.push(column_def);
        }
        
        // Add multi-column primary key if needed
        if let Some(pk) = &table.primary_key {
            if pk.columns.len() > 1 {
                let pk_columns: Vec<String> = pk.columns.iter()
                    .map(|col| format!("\"{}\"", col))
                    .collect();
                
                column_defs.push(format!("  PRIMARY KEY ({})", pk_columns.join(", ")));
            }
        }
        
        // Add foreign key constraints (SQLite supports them in table definition)
        for fk in &table.foreign_keys {
            let fk_columns: Vec<String> = fk.columns.iter()
                .map(|col| format!("\"{}\"", col))
                .collect();
                
            let ref_columns: Vec<String> = fk.ref_columns.iter()
                .map(|col| format!("\"{}\"", col))
                .collect();
                
            let on_delete = if let Some(action) = &fk.on_delete {
                format!(" ON DELETE {}", action)
            } else {
                String::new()
            };
            
            let on_update = if let Some(action) = &fk.on_update {
                format!(" ON UPDATE {}", action)
            } else {
                String::new()
            };
            
            column_defs.push(format!(
                "  FOREIGN KEY ({}) REFERENCES \"{}\" ({}){}{}",
                fk_columns.join(", "),
                fk.ref_table,
                ref_columns.join(", "),
                on_delete,
                on_update
            ));
        }
        
        sql.push_str(&column_defs.join(",\n"));
        sql.push_str("\n);\n");
        
        // Create indices (SQLite doesn't include these in the CREATE TABLE)
        for index in &table.indexes {
            let unique = if index.is_unique { "UNIQUE " } else { "" };
            let index_columns: Vec<String> = index.columns.iter()
                .map(|col| format!("\"{}\"", col))
                .collect();
            
            sql.push_str(&format!(
                "CREATE {}INDEX IF NOT EXISTS \"{}\" ON \"{}\" ({});\n",
                unique,
                index.name,
                table.name,
                index_columns.join(", ")
            ));
        }
        
        Ok(sql)
    }
    
    /// Generate SQL to drop a table
    fn generate_drop_table_sql(&self, table_name: &str) -> Result<String> {
        let db_type = &self.config.database.driver;
        
        match db_type.as_str() {
            "postgres" => Ok(format!("DROP TABLE IF EXISTS {};", table_name)),
            "mysql" => Ok(format!("DROP TABLE IF EXISTS `{}`;", table_name)),
            "sqlite" => Ok(format!("DROP TABLE IF EXISTS \"{}\";", table_name)),
            _ => Err(crate::error::Error::MigrationError(format!(
                "Unsupported database type: {}", db_type
            ))),
        }
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
                    
                    // Add column comment if present
                    if let Some(comment) = &column.comment {
                        sql.push_str(&format!(
                            "COMMENT ON COLUMN {}.{} IS '{}';\n",
                            table_name,
                            column.name,
                            comment.replace('\'', "''")
                        ));
                    }
                }
                
                Ok(sql)
            }
            "mysql" => {
                let mut sql = String::new();
                
                for column in columns {
                    let nullable = if column.nullable { "NULL" } else { "NOT NULL" };
                    let default = if let Some(default_val) = &column.default {
                        format!(" DEFAULT {}", default_val)
                    } else {
                        String::new()
                    };
                    
                    let mut column_def = format!(
                        "ALTER TABLE `{}` ADD COLUMN `{}` {}{}",
                        table_name,
                        column.name,
                        self.translate_data_type_for_mysql(&column.data_type),
                        default
                    );
                    
                    if !nullable.is_empty() {
                        column_def.push_str(&format!(" {}", nullable));
                    }
                    
                    // Add column comment if present
                    if let Some(comment) = &column.comment {
                        column_def.push_str(&format!(
                            " COMMENT '{}'",
                            comment.replace('\'', "''")
                        ));
                    }
                    
                    sql.push_str(&format!("{};\n", column_def));
                }
                
                Ok(sql)
            }
            "sqlite" => {
                // SQLite does not directly support adding NOT NULL columns without defaults
                // We would need to use a transaction and rebuild table approach
                // For now, we'll handle the simple case only
                
                let mut sql = String::new();
                
                for column in columns {
                    // SQLite can only add nullable columns or columns with defaults
                    if !column.nullable && column.default.is_none() {
                        return Err(crate::error::Error::MigrationError(
                            format!("SQLite cannot add NOT NULL column '{}' without default value. \
                                     Consider rebuilding the entire table.", column.name)
                        ));
                    }
                    
                    let nullable = if column.nullable { "" } else { "NOT NULL" };
                    let default = if let Some(default_val) = &column.default {
                        format!(" DEFAULT {}", default_val)
                    } else {
                        String::new()
                    };
                    
                    let mut column_def = format!(
                        "ALTER TABLE \"{}\" ADD COLUMN \"{}\" {}{}",
                        table_name,
                        column.name,
                        self.translate_data_type_for_sqlite(&column.data_type),
                        default
                    );
                    
                    if !nullable.is_empty() {
                        column_def.push_str(&format!(" {}", nullable));
                    }
                    
                    sql.push_str(&format!("{};\n", column_def));
                }
                
                Ok(sql)
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
            "mysql" => {
                let mut sql = String::new();
                
                for column_name in column_names {
                    sql.push_str(&format!(
                        "ALTER TABLE `{}` DROP COLUMN `{}`;\n",
                        table_name,
                        column_name
                    ));
                }
                
                Ok(sql)
            }
            "sqlite" => {
                return Err(crate::error::Error::MigrationError(
                    "SQLite does not support dropping columns directly. \
                     You need to recreate the table without those columns.".to_string()
                ));
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
                    
                    // Alter comment
                    if change.from.comment != change.to.comment {
                        if let Some(comment) = &change.to.comment {
                            sql.push_str(&format!(
                                "COMMENT ON COLUMN {}.{} IS '{}';\n",
                                table_name,
                                change.column_name,
                                comment.replace('\'', "''")
                            ));
                        } else {
                            sql.push_str(&format!(
                                "COMMENT ON COLUMN {}.{} IS NULL;\n",
                                table_name,
                                change.column_name
                            ));
                        }
                    }
                }
                
                Ok(sql)
            }
            "mysql" => {
                let mut sql = String::new();
                
                for change in column_changes {
                    let nullable = if change.to.nullable { "NULL" } else { "NOT NULL" };
                    let default = if let Some(default_val) = &change.to.default {
                        format!(" DEFAULT {}", default_val)
                    } else {
                        String::new()
                    };
                    
                    let mut alter_sql = format!(
                        "ALTER TABLE `{}` MODIFY COLUMN `{}` {}{}",
                        table_name,
                        change.column_name,
                        self.translate_data_type_for_mysql(&change.to.data_type),
                        default
                    );
                    
                    if !nullable.is_empty() {
                        alter_sql.push_str(&format!(" {}", nullable));
                    }
                    
                    // Add column comment if present
                    if let Some(comment) = &change.to.comment {
                        alter_sql.push_str(&format!(
                            " COMMENT '{}'",
                            comment.replace('\'', "''")
                        ));
                    }
                    
                    sql.push_str(&format!("{};\n", alter_sql));
                }
                
                Ok(sql)
            }
            "sqlite" => {
                return Err(crate::error::Error::MigrationError(
                    "SQLite does not support altering column definitions directly. \
                     You need to recreate the table with the new column definitions.".to_string()
                ));
            }
            _ => Err(crate::error::Error::MigrationError(format!(
                "Unsupported database type: {}", db_type
            ))),
        }
    }
    
    /// Generate SQL to create indices
    fn generate_create_indices_sql(
        &self,
        table_name: &str,
        indices: &[&crate::schema::types::Index],
    ) -> Result<String> {
        let db_type = &self.config.database.driver;
        
        match db_type.as_str() {
            "postgres" => {
                let mut sql = String::new();
                
                for index in indices {
                    let unique = if index.is_unique { "UNIQUE " } else { "" };
                    let method = index.method.as_deref().unwrap_or("btree");
                    let columns = index.columns.join(", ");
                    
                    sql.push_str(&format!(
                        "CREATE {}INDEX IF NOT EXISTS {} ON {} USING {} ({});\n",
                        unique,
                        index.name,
                        table_name,
                        method,
                        columns
                    ));
                }
                
                Ok(sql)
            }
            "mysql" => {
                let mut sql = String::new();
                
                for index in indices {
                    let unique = if index.is_unique { "UNIQUE " } else { "" };
                    let index_columns: Vec<String> = index.columns.iter()
                        .map(|col| format!("`{}`", col))
                        .collect();
                    
                    sql.push_str(&format!(
                        "CREATE {}INDEX `{}` ON `{}` ({});\n",
                        unique,
                        index.name,
                        table_name,
                        index_columns.join(", ")
                    ));
                }
                
                Ok(sql)
            }
            "sqlite" => {
                let mut sql = String::new();
                
                for index in indices {
                    let unique = if index.is_unique { "UNIQUE " } else { "" };
                    let index_columns: Vec<String> = index.columns.iter()
                        .map(|col| format!("\"{}\"", col))
                        .collect();
                    
                    sql.push_str(&format!(
                        "CREATE {}INDEX IF NOT EXISTS \"{}\" ON \"{}\" ({});\n",
                        unique,
                        index.name,
                        table_name,
                        index_columns.join(", ")
                    ));
                }
                
                Ok(sql)
            }
            _ => Err(crate::error::Error::MigrationError(format!(
                "Unsupported database type: {}", db_type
            ))),
        }
    }
    
    /// Generate SQL to drop indices
    fn generate_drop_indices_sql(
        &self,
        table_name: &str,
        index_names: &[String],
    ) -> Result<String> {
        let db_type = &self.config.database.driver;
        
        match db_type.as_str() {
            "postgres" => {
                let mut sql = String::new();
                
                for index_name in index_names {
                    sql.push_str(&format!(
                        "DROP INDEX IF EXISTS {};\n",
                        index_name
                    ));
                }
                
                Ok(sql)
            }
            "mysql" => {
                let mut sql = String::new();
                
                for index_name in index_names {
                    sql.push_str(&format!(
                        "DROP INDEX `{}` ON `{}`;\n",
                        index_name,
                        table_name
                    ));
                }
                
                Ok(sql)
            }
            "sqlite" => {
                let mut sql = String::new();
                
                for index_name in index_names {
                    sql.push_str(&format!(
                        "DROP INDEX IF EXISTS \"{}\";\n",
                        index_name
                    ));
                }
                
                Ok(sql)
            }
            _ => Err(crate::error::Error::MigrationError(format!(
                "Unsupported database type: {}", db_type
            ))),
        }
    }
    
    /// Generate SQL to create foreign keys
    fn generate_create_foreign_keys_sql(
        &self,
        table_name: &str,
        foreign_keys: &[&crate::schema::types::ForeignKey],
    ) -> Result<String> {
        let db_type = &self.config.database.driver;
        
        match db_type.as_str() {
            "postgres" => {
                let mut sql = String::new();
                
                for fk in foreign_keys {
                    let columns = fk.columns.join(", ");
                    let ref_columns = fk.ref_columns.join(", ");
                    let on_delete = fk.on_delete.as_deref().unwrap_or("NO ACTION");
                    let on_update = fk.on_update.as_deref().unwrap_or("NO ACTION");
                    
                    sql.push_str(&format!(
                        "ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({}) ON DELETE {} ON UPDATE {};\n",
                        table_name,
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
            "mysql" => {
                let mut sql = String::new();
                
                for fk in foreign_keys {
                    let fk_columns: Vec<String> = fk.columns.iter()
                        .map(|col| format!("`{}`", col))
                        .collect();
                        
                    let ref_columns: Vec<String> = fk.ref_columns.iter()
                        .map(|col| format!("`{}`", col))
                        .collect();
                        
                    let on_delete = fk.on_delete.as_deref().unwrap_or("RESTRICT");
                    let on_update = fk.on_update.as_deref().unwrap_or("RESTRICT");
                    
                    sql.push_str(&format!(
                        "ALTER TABLE `{}` ADD CONSTRAINT `{}` FOREIGN KEY ({}) REFERENCES `{}` ({}) ON DELETE {} ON UPDATE {};\n",
                        table_name,
                        fk.name,
                        fk_columns.join(", "),
                        fk.ref_table,
                        ref_columns.join(", "),
                        on_delete,
                        on_update
                    ));
                }
                
                Ok(sql)
            }
            "sqlite" => {
                return Err(crate::error::Error::MigrationError(
                    "SQLite does not support adding foreign keys to existing tables. \
                     You need to recreate the table with the foreign key constraints.".to_string()
                ));
            }
            _ => Err(crate::error::Error::MigrationError(format!(
                "Unsupported database type: {}", db_type
            ))),
        }
    }
    
    /// Generate SQL to drop foreign keys
    fn generate_drop_foreign_keys_sql(
        &self,
        table_name: &str,
        fk_names: &[String],
    ) -> Result<String> {
        let db_type = &self.config.database.driver;
        
        match db_type.as_str() {
            "postgres" => {
                let mut sql = String::new();
                
                for fk_name in fk_names {
                    sql.push_str(&format!(
                        "ALTER TABLE {} DROP CONSTRAINT {};\n",
                        table_name,
                        fk_name
                    ));
                }
                
                Ok(sql)
            }
            "mysql" => {
                let mut sql = String::new();
                
                for fk_name in fk_names {
                    sql.push_str(&format!(
                        "ALTER TABLE `{}` DROP FOREIGN KEY `{}`;\n",
                        table_name,
                        fk_name
                    ));
                }
                
                Ok(sql)
            }
            "sqlite" => {
                return Err(crate::error::Error::MigrationError(
                    "SQLite does not support dropping foreign keys from existing tables. \
                     You need to recreate the table without the foreign key constraints.".to_string()
                ));
            }
            _ => Err(crate::error::Error::MigrationError(format!(
                "Unsupported database type: {}", db_type
            ))),
        }
    }
    
    /// Translate a PostgreSQL data type to MySQL
    fn translate_data_type_for_mysql(&self, pg_type: &str) -> String {
        let pg_type_lower = pg_type.to_lowercase();
        
        // Convert PostgreSQL types to equivalent MySQL types
        match pg_type_lower.as_str() {
            // Integer types
            "smallint" => "SMALLINT".to_string(),
            "integer" | "int" | "int4" => "INT".to_string(),
            "bigint" | "int8" => "BIGINT".to_string(),
            
            // Floating point types
            "real" | "float4" => "FLOAT".to_string(),
            "double precision" | "float8" => "DOUBLE".to_string(),
            
            // Character types
            t if t.starts_with("varchar") => {
                // Extract size if specified
                if let Some(start) = t.find('(') {
                    if let Some(end) = t.find(')') {
                        let size = &t[start..=end];
                        return format!("VARCHAR{}", size);
                    }
                }
                "VARCHAR(255)".to_string()
            }
            t if t.starts_with("char") => {
                // Extract size if specified
                if let Some(start) = t.find('(') {
                    if let Some(end) = t.find(')') {
                        let size = &t[start..=end];
                        return format!("CHAR{}", size);
                    }
                }
                "CHAR(1)".to_string()
            }
            "text" => "TEXT".to_string(),
            
            // Date/time types
            "date" => "DATE".to_string(),
            "timestamp" => "TIMESTAMP".to_string(),
            "timestamp with time zone" | "timestamptz" => "TIMESTAMP".to_string(),
            "time" => "TIME".to_string(),
            "time with time zone" | "timetz" => "TIME".to_string(),
            
            // Boolean type
            "boolean" | "bool" => "TINYINT(1)".to_string(),
            
            // Binary data
            "bytea" => "BLOB".to_string(),
            
            // JSON types
            "json" | "jsonb" => "JSON".to_string(),
            
            // UUID type
            "uuid" => "CHAR(36)".to_string(),
            
            // Numeric/decimal types
            t if t.starts_with("numeric") || t.starts_with("decimal") => {
                // Extract precision and scale if specified
                if let Some(start) = t.find('(') {
                    if let Some(end) = t.find(')') {
                        let params = &t[start..=end];
                        return format!("DECIMAL{}", params);
                    }
                }
                "DECIMAL(10,2)".to_string()
            }
            
            // Array types - MySQL doesn't have direct equivalent
            t if t.ends_with("[]") => "JSON".to_string(),
            
            // Use the type as-is if no mapping is found
            _ => pg_type.to_string(),
        }
    }
    
    /// Translate a PostgreSQL data type to SQLite
    fn translate_data_type_for_sqlite(&self, pg_type: &str) -> String {
        let pg_type_lower = pg_type.to_lowercase();
        
        // Convert PostgreSQL types to equivalent SQLite types
        match pg_type_lower.as_str() {
            // SQLite has only 5 storage classes: NULL, INTEGER, REAL, TEXT, and BLOB
            
            // Integer types
            "smallint" | "integer" | "int" | "int4" | "bigint" | "int8" | "serial" | "bigserial" => 
                "INTEGER".to_string(),
            
            // Floating point types
            "real" | "float4" | "double precision" | "float8" | "numeric" | "decimal" => 
                "REAL".to_string(),
            
            // Character types
            "char" | "varchar" | "text" | "character varying" | "character" => 
                "TEXT".to_string(),
            
            // Date/time types - SQLite doesn't have specific date/time types
            "date" | "timestamp" | "timestamp with time zone" | "timestamptz" | "time" | "time with time zone" | "timetz" => 
                "TEXT".to_string(),
            
            // Boolean type
            "boolean" | "bool" => "INTEGER".to_string(),
            
            // Binary data
            "bytea" => "BLOB".to_string(),
            
            // JSON types
            "json" | "jsonb" => "TEXT".to_string(),
            
            // UUID type
            "uuid" => "TEXT".to_string(),
            
            // Arrays - SQLite doesn't have arrays
            t if t.ends_with("[]") => "TEXT".to_string(),
            
            // If the type contains parentheses (like varchar(255)), extract the base type
            t if t.contains('(') => {
                let base_type = t.split('(').next().unwrap_or(t);
                self.translate_data_type_for_sqlite(base_type)
            }
            
            // Use TEXT as a default for unrecognized types
            _ => "TEXT".to_string(),
        }
    }
}