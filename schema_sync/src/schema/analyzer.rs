//! Database schema analyzer
//!
//! This module provides functionality to analyze an existing database schema.

use async_trait::async_trait;
use sqlx::{Any, Row, FromRow, MySql, Pool, Postgres, Sqlite};
use std::collections::HashMap;

use crate::db::connection::DatabaseConnection;
use crate::error::Result;
use crate::schema::types::{Column, DatabaseSchema, ForeignKey, Index, PrimaryKey, Table, View};


/// Schema analyzer trait
#[async_trait]
pub trait Analyzer {
    /// Analyze the database schema
    async fn analyze_schema(&self, schema_name: Option<&str>) -> Result<DatabaseSchema>;

    /// Analyze table definitions
    async fn analyze_tables(&self, schema_name: Option<&str>) -> Result<HashMap<String, Table>>;

    /// Analyze view definitions
    async fn analyze_views(&self, schema_name: Option<&str>) -> Result<HashMap<String, View>>;
}

/// Schema analyzer for database schema introspection
pub struct SchemaAnalyzer {
    connection: DatabaseConnection,
}

impl SchemaAnalyzer {
    /// Create a new schema analyzer
    pub fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }

    /// Analyze the current database schema
    pub async fn analyze(&self) -> Result<DatabaseSchema> {
        match &self.connection {
            DatabaseConnection::Postgres(pool) => {
                PostgresAnalyzer { pool }
                    .analyze_schema(self.connection.get_schema())
                    .await
            }
            DatabaseConnection::MySql(pool) => {
                MySqlAnalyzer { pool }
                    .analyze_schema(self.connection.get_schema())
                    .await
            }
            DatabaseConnection::Sqlite(pool) => {
                SqliteAnalyzer { pool }
                    .analyze_schema(self.connection.get_schema())
                    .await
            }
            _ => Err(crate::error::Error::SchemaAnalysisError(
                "Unsupported database type".to_string(),
            )),
        }
    }
}

// Row types for PostgreSQL queries
#[derive(FromRow)]
struct TableRow {
    table_name: String,
}

#[derive(FromRow)]
struct ColumnRow {
    column_name: String,
    data_type: String,
    is_nullable: String,
    column_default: Option<String>,
    character_maximum_length: Option<i64>,
}

#[derive(FromRow)]
struct PrimaryKeyRow {
    constraint_name: String,
    column_name: String,
}

#[derive(FromRow)]
struct IndexRow {
    index_name: String,
    column_name: String,
    is_unique: Option<bool>,
    index_method: String,
}

#[derive(FromRow)]
struct ForeignKeyRow {
    constraint_name: String,
    column_name: String,
    ref_table: String,
    ref_column: String,
    delete_rule: String,
    update_rule: String,
}

#[derive(FromRow)]
struct ViewRow {
    table_name: String,
    view_definition: Option<String>,
    is_updatable: Option<String>,
}

#[derive(FromRow)]
struct MatViewRow {
    matviewname: String,
    definition: Option<String>,
}

/// PostgreSQL schema analyzer
struct PostgresAnalyzer<'a> {
    pool: &'a Pool<Postgres>,
}

#[async_trait]
impl<'a> Analyzer for PostgresAnalyzer<'a> {
    async fn analyze_schema(&self, schema_name: Option<&str>) -> Result<DatabaseSchema> {
        let schema = schema_name.unwrap_or("public");
        let mut db_schema = DatabaseSchema::new(Some(schema.to_string()));

        // Get tables
        db_schema.tables = self.analyze_tables(Some(schema)).await?;

        // Get views
        db_schema.views = self.analyze_views(Some(schema)).await?;

        Ok(db_schema)
    }

    async fn analyze_tables(&self, schema_name: Option<&str>) -> Result<HashMap<String, Table>> {
        let schema = schema_name.unwrap_or("public");
        let mut tables = HashMap::new();

        // Query to get table names
        let sql = r#"
            SELECT table_name 
            FROM information_schema.tables 
            WHERE table_schema = $1 AND table_type = 'BASE TABLE'
        "#;
        
        let table_rows = sqlx::query_as::<_, TableRow>(sql)
            .bind(schema)
            .fetch_all(self.pool)
            .await?;

        for row in table_rows {
            let table_name = row.table_name;
            let mut table = Table::new(&table_name);

            // Get columns
            let sql = r#"
                SELECT 
                    column_name, 
                    data_type, 
                    is_nullable, 
                    column_default,
                    character_maximum_length
                FROM information_schema.columns 
                WHERE table_schema = $1 AND table_name = $2
                ORDER BY ordinal_position
            "#;
            
            let column_rows = sqlx::query_as::<_, ColumnRow>(sql)
                .bind(schema)
                .bind(&table_name)
                .fetch_all(self.pool)
                .await?;

            for col in column_rows {
                let mut data_type = col.data_type;
                if let Some(max_length) = col.character_maximum_length {
                    if data_type == "character varying" {
                        data_type = format!("varchar({})", max_length);
                    }
                }

                let column = Column {
                    name: col.column_name,
                    data_type,
                    nullable: col.is_nullable == "YES",
                    default: col.column_default,
                    comment: None,
                    is_unique: false, // Will be updated when checking constraints
                    is_generated: false,
                    generation_expression: None,
                };

                table.add_column(column);
            }

            // Get primary key
            let sql = r#"
                SELECT
                    tc.constraint_name,
                    kcu.column_name
                FROM
                    information_schema.table_constraints tc
                JOIN information_schema.key_column_usage kcu
                    ON tc.constraint_name = kcu.constraint_name
                    AND tc.table_schema = kcu.table_schema
                WHERE
                    tc.constraint_type = 'PRIMARY KEY'
                    AND tc.table_schema = $1
                    AND tc.table_name = $2
                ORDER BY kcu.ordinal_position
            "#;
            
            let pk_rows = sqlx::query_as::<_, PrimaryKeyRow>(sql)
                .bind(schema)
                .bind(&table_name)
                .fetch_all(self.pool)
                .await?;

            if !pk_rows.is_empty() {
                let pk_name = pk_rows[0].constraint_name.clone();
                let pk_columns = pk_rows.iter().map(|r| r.column_name.clone()).collect();

                table.set_primary_key(PrimaryKey {
                    name: Some(pk_name),
                    columns: pk_columns,
                });
            }

            // Get indexes
            let sql = r#"
                SELECT
                    i.relname as index_name,
                    a.attname as column_name,
                    ix.indisunique as is_unique,
                    am.amname as index_method
                FROM
                    pg_index ix
                JOIN pg_class i ON i.oid = ix.indexrelid
                JOIN pg_class t ON t.oid = ix.indrelid
                JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey)
                JOIN pg_namespace n ON n.oid = t.relnamespace
                JOIN pg_am am ON am.oid = i.relam
                WHERE
                    t.relname = $1
                    AND n.nspname = $2
                    AND NOT ix.indisprimary
                ORDER BY i.relname, a.attnum
            "#;
            
            let index_rows = sqlx::query_as::<_, IndexRow>(sql)
                .bind(&table_name)
                .bind(schema)
                .fetch_all(self.pool)
                .await?;

            let mut indexes = HashMap::new();
            for row in index_rows {
                let index_name = row.index_name;
                let column_name = row.column_name;
                let is_unique = row.is_unique.unwrap_or(false);
                let method = row.index_method;

                indexes
                    .entry(index_name.clone())
                    .or_insert_with(|| Index {
                        name: index_name.clone(),
                        columns: Vec::new(),
                        is_unique,
                        method: Some(method),
                    })
                    .columns
                    .push(column_name);
            }

            table.indexes = indexes.into_values().collect();

            // Get foreign keys
            let sql = r#"
                SELECT
                    tc.constraint_name,
                    kcu.column_name,
                    ccu.table_name AS ref_table,
                    ccu.column_name AS ref_column,
                    rc.delete_rule,
                    rc.update_rule
                FROM
                    information_schema.table_constraints tc
                JOIN information_schema.key_column_usage kcu
                    ON tc.constraint_name = kcu.constraint_name
                    AND tc.table_schema = kcu.table_schema
                JOIN information_schema.constraint_column_usage ccu
                    ON ccu.constraint_name = tc.constraint_name
                    AND ccu.table_schema = tc.table_schema
                JOIN information_schema.referential_constraints rc
                    ON tc.constraint_name = rc.constraint_name
                    AND tc.table_schema = rc.constraint_schema
                WHERE
                    tc.constraint_type = 'FOREIGN KEY'
                    AND tc.table_schema = $1
                    AND tc.table_name = $2
                ORDER BY tc.constraint_name, kcu.ordinal_position
            "#;
            
            let fk_rows = sqlx::query_as::<_, ForeignKeyRow>(sql)
                .bind(schema)
                .bind(&table_name)
                .fetch_all(self.pool)
                .await?;

            let mut foreign_keys = HashMap::new();
            for row in fk_rows {
                let fk_name = row.constraint_name;
                let column_name = row.column_name;
                let ref_table = row.ref_table;
                let ref_column = row.ref_column;
                let on_delete = row.delete_rule;
                let on_update = row.update_rule;

                let entry_key = fk_name.clone();
                foreign_keys
                    .entry(entry_key.clone())
                    .or_insert_with(|| ForeignKey {
                        name: fk_name,
                        columns: Vec::new(),
                        ref_table,
                        ref_columns: Vec::new(),
                        on_delete: Some(on_delete),
                        on_update: Some(on_update),
                    })
                    .columns
                    .push(column_name);

                foreign_keys
                    .get_mut(&entry_key)
                    .unwrap()
                    .ref_columns
                    .push(ref_column);
            }

            table.foreign_keys = foreign_keys.into_values().collect();

            tables.insert(table_name, table);
        }

        Ok(tables)
    }

    async fn analyze_views(&self, schema_name: Option<&str>) -> Result<HashMap<String, View>> {
        let schema = schema_name.unwrap_or("public");
        let mut views = HashMap::new();

        // Query to get view definitions
        let sql = r#"
            SELECT table_name, view_definition, is_updatable
            FROM information_schema.views
            WHERE table_schema = $1
        "#;
        
        let view_rows = sqlx::query_as::<_, ViewRow>(sql)
            .bind(schema)
            .fetch_all(self.pool)
            .await?;

        for row in view_rows {
            let view_name = row.table_name;
            let view_definition = row.view_definition.unwrap_or_default();

            // Get view columns
            let sql = r#"
                SELECT 
                    column_name, 
                    data_type, 
                    is_nullable
                FROM information_schema.columns 
                WHERE table_schema = $1 AND table_name = $2
                ORDER BY ordinal_position
            "#;
            
            let column_rows = sqlx::query_as::<_, ColumnRow>(sql)
                .bind(schema)
                .bind(&view_name)
                .fetch_all(self.pool)
                .await?;

            let columns = column_rows
                .into_iter()
                .map(|col| Column {
                    name: col.column_name,
                    data_type: col.data_type,
                    nullable: col.is_nullable == "YES",
                    default: None,
                    comment: None,
                    is_unique: false,
                    is_generated: false,
                    generation_expression: None,
                })
                .collect();

            let view = View {
                name: view_name.clone(),
                definition: view_definition,
                columns,
                is_materialized: false, // Need separate query for materialized views
            };

            views.insert(view_name, view);
        }

        // Add materialized views
        let sql = r#"
            SELECT matviewname, definition
            FROM pg_matviews
            WHERE schemaname = $1
        "#;
        
        let mat_view_rows = sqlx::query_as::<_, MatViewRow>(sql)
            .bind(schema)
            .fetch_all(self.pool)
            .await?;

        for row in mat_view_rows {
            let view_name = row.matviewname;
            let view_definition = row.definition.unwrap_or_default();

            // Get view columns
            let sql = r#"
                SELECT 
                    column_name, 
                    data_type, 
                    is_nullable 
                FROM information_schema.columns 
                WHERE table_schema = $1 AND table_name = $2
                ORDER BY ordinal_position
            "#;
            
            let column_rows = sqlx::query_as::<_, ColumnRow>(sql)
                .bind(schema)
                .bind(&view_name)
                .fetch_all(self.pool)
                .await?;

            let columns = column_rows
                .into_iter()
                .map(|col| Column {
                    name: col.column_name,
                    data_type: col.data_type,
                    nullable: col.is_nullable == "YES",
                    default: None,
                    comment: None,
                    is_unique: false,
                    is_generated: false,
                    generation_expression: None,
                })
                .collect();

            let view = View {
                name: view_name.clone(),
                definition: view_definition,
                columns,
                is_materialized: true,
            };

            views.insert(view_name, view);
        }

        Ok(views)
    }
}

// Similar implementations for MySQL and SQLite analyzers
// (abbreviated here for brevity - would implement specific versions for each database type)

struct MySqlAnalyzer<'a> {
    pool: &'a Pool<MySql>,
}
#[async_trait]
impl<'a> Analyzer for MySqlAnalyzer<'a> {
    async fn analyze_schema(&self, schema_name: Option<&str>) -> Result<DatabaseSchema> {
        let schema = schema_name.unwrap_or("public");
        let mut db_schema = DatabaseSchema::new(Some(schema.to_string()));
        db_schema.tables = self.analyze_tables(Some(schema)).await?;
        db_schema.views = self.analyze_views(Some(schema)).await?;
        Ok(db_schema)
    }

    async fn analyze_tables(&self, schema_name: Option<&str>) -> Result<HashMap<String, Table>> {
        let schema = schema_name.unwrap_or("public");
        let mut tables = HashMap::new();

        // Get tables
        let sql = r#"
            SELECT table_name
            FROM information_schema.tables
            WHERE table_schema = ?
              AND table_type = 'BASE TABLE'
        "#;

        let table_rows = sqlx::query_as::<_, TableRow>(sql)
            .bind(schema)
            .fetch_all(self.pool)
            .await?;

        for row in table_rows {
            let table_name = row.table_name;
            let mut table = Table::new(&table_name);

            // Columns
            let sql = r#"
                SELECT column_name, data_type, is_nullable, column_default, character_maximum_length
                FROM information_schema.columns
                WHERE table_schema = ? AND table_name = ?
                ORDER BY ordinal_position
            "#;

            let column_rows = sqlx::query_as::<_, ColumnRow>(sql)
                .bind(schema)
                .bind(&table_name)
                .fetch_all(self.pool)
                .await?;

            for col in column_rows {
                let mut data_type = col.data_type;
                if let Some(max_length) = col.character_maximum_length {
                    if data_type == "varchar" {
                        data_type = format!("varchar({})", max_length);
                    }
                }

                let column = Column {
                    name: col.column_name,
                    data_type,
                    nullable: col.is_nullable == "YES",
                    default: col.column_default,
                    comment: None,
                    is_unique: false,
                    is_generated: false,
                    generation_expression: None,
                };

                table.add_column(column);
            }

            // Primary key
            let sql = r#"
                SELECT k.constraint_name, k.column_name
                FROM information_schema.table_constraints t
                JOIN information_schema.key_column_usage k
                ON t.constraint_name = k.constraint_name
                WHERE t.table_schema = ? AND t.table_name = ?
                AND t.constraint_type = 'PRIMARY KEY'
            "#;

            let pk_rows = sqlx::query_as::<_, PrimaryKeyRow>(sql)
                .bind(schema)
                .bind(&table_name)
                .fetch_all(self.pool)
                .await?;

            if !pk_rows.is_empty() {
                let pk_name = pk_rows[0].constraint_name.clone();
                let pk_columns = pk_rows.into_iter().map(|r| r.column_name).collect();
                table.set_primary_key(PrimaryKey {
                    name: Some(pk_name),
                    columns: pk_columns,
                });
            }

            // Foreign keys
            let sql = r#"
                SELECT
                    rc.constraint_name,
                    kcu.column_name,
                    kcu.referenced_table_name as ref_table,
                    kcu.referenced_column_name as ref_column,
                    rc.delete_rule,
                    rc.update_rule
                FROM information_schema.referential_constraints rc
                JOIN information_schema.key_column_usage kcu
                  ON rc.constraint_name = kcu.constraint_name
                WHERE rc.constraint_schema = ? AND kcu.table_name = ?
            "#;

            let fk_rows = sqlx::query_as::<_, ForeignKeyRow>(sql)
                .bind(schema)
                .bind(&table_name)
                .fetch_all(self.pool)
                .await?;

            let mut foreign_keys = HashMap::new();
            for row in fk_rows {
                let key = row.constraint_name.clone();
                foreign_keys
                    .entry(key.clone())
                    .or_insert(ForeignKey {
                        name: Some(row.constraint_name),
                        columns: vec![],
                        ref_table: row.ref_table,
                        ref_columns: vec![],
                        on_delete: Some(row.delete_rule),
                        on_update: Some(row.update_rule),
                    })
                    .columns
                    .push(row.column_name);
                foreign_keys
                    .get_mut(&key)
                    .unwrap()
                    .ref_columns
                    .push(row.ref_column);
            }

            table.foreign_keys = foreign_keys.into_values().collect();

            tables.insert(table_name, table);
        }

        Ok(tables)
    }

    async fn analyze_views(&self, schema_name: Option<&str>) -> Result<HashMap<String, View>> {
        let schema = schema_name.unwrap_or("public");
        let mut views = HashMap::new();

        let sql = r#"
            SELECT table_name, view_definition, 'NO' as is_updatable
            FROM information_schema.views
            WHERE table_schema = ?
        "#;

        let view_rows = sqlx::query_as::<_, ViewRow>(sql)
            .bind(schema)
            .fetch_all(self.pool)
            .await?;

        for row in view_rows {
            let view_name = row.table_name;
            let sql = r#"
                SELECT column_name, data_type, is_nullable
                FROM information_schema.columns
                WHERE table_schema = ? AND table_name = ?
                ORDER BY ordinal_position
            "#;

            let column_rows = sqlx::query_as::<_, ColumnRow>(sql)
                .bind(schema)
                .bind(&view_name)
                .fetch_all(self.pool)
                .await?;

            let columns = column_rows
                .into_iter()
                .map(|col| Column {
                    name: col.column_name,
                    data_type: col.data_type,
                    nullable: col.is_nullable == "YES",
                    default: None,
                    comment: None,
                    is_unique: false,
                    is_generated: false,
                    generation_expression: None,
                })
                .collect();

            let view = View {
                name: view_name.clone(),
                definition: row.view_definition.unwrap_or_default(),
                columns,
                is_materialized: false,
            };

            views.insert(view_name, view);
        }

        Ok(views)
    }
}

struct SqliteAnalyzer<'a> {
    pool: &'a Pool<Sqlite>,
}
#[async_trait]
impl<'a> Analyzer for SqliteAnalyzer<'a> {
    async fn analyze_schema(&self, schema_name: Option<&str>) -> Result<DatabaseSchema> {
        let mut db_schema = DatabaseSchema::new(None);
        db_schema.tables = self.analyze_tables(schema_name).await?;
        db_schema.views = self.analyze_views(schema_name).await?;
        Ok(db_schema)
    }

    async fn analyze_tables(&self, _schema_name: Option<&str>) -> Result<HashMap<String, Table>> {
        let mut tables = HashMap::new();

        let sql = r#"SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'"#;
        let table_rows = sqlx::query_as::<_, TableRow>(sql)
            .fetch_all(self.pool)
            .await?;

        for row in table_rows {
            let table_name = row.table_name.clone();
            let mut table = Table::new(&table_name);

            let pragma = format!("PRAGMA table_info({})", table_name);
            let columns = sqlx::query(&pragma).fetch_all(self.pool).await?;

            for col in columns {
                let name: String = col.try_get("name")?;
                let data_type: String = col.try_get("type")?;
                let notnull: i64 = col.try_get("notnull")?;
                let dflt_value: Option<String> = col.try_get("dflt_value")?;
                let pk: i64 = col.try_get("pk")?;

                let column = Column {
                    name,
                    data_type,
                    nullable: notnull == 0,
                    default: dflt_value,
                    comment: None,
                    is_unique: false,
                    is_generated: false,
                    generation_expression: None,
                };

                table.add_column(column);

                if pk > 0 {
                    table.set_primary_key(PrimaryKey {
                        name: None,
                        columns: vec![column.name.clone()],
                    });
                }
            }

            tables.insert(table_name, table);
        }

        Ok(tables)
    }

    async fn analyze_views(&self, _schema_name: Option<&str>) -> Result<HashMap<String, View>> {
        let mut views = HashMap::new();

        let sql = r#"SELECT name, sql FROM sqlite_master WHERE type = 'view'"#;
        let rows = sqlx::query(sql).fetch_all(self.pool).await?;

        for row in rows {
            let view_name: String = row.try_get("name")?;
            let definition: String = row.try_get("sql")?;

            let pragma = format!("PRAGMA table_info({})", view_name);
            let columns_info = sqlx::query(&pragma).fetch_all(self.pool).await?;

            let columns = columns_info
                .into_iter()
                .map(|col| Column {
                    name: col.get("name"),
                    data_type: col.get("type"),
                    nullable: col.get::<i64, _>("notnull") == 0,
                    default: col.get("dflt_value"),
                    comment: None,
                    is_unique: false,
                    is_generated: false,
                    generation_expression: None,
                })
                .collect();

            views.insert(
                view_name.clone(),
                View {
                    name: view_name,
                    definition,
                    columns,
                    is_materialized: false,
                },
            );
        }

        Ok(views)
    }
}
