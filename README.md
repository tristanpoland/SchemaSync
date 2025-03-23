# SchemaSync Documentation

## A Rust-based ORM for Schema-Driven Database Management

SchemaSync is a reverse-ORM library for Rust that allows developers to define database schemas using Rust structs and automatically generates migrations to keep databases in sync with your code. 

---

## Key Features

- **Code-First Approach**: Define your database schema using standard Rust structs
- **Automatic Migration Generation**: Create SQL migrations from schema differences
- **Multiple Database Support**: Works with PostgreSQL, MySQL, and SQLite
- **Customizable Type Mappings**: Map Rust types to database types
- **Schema Analysis**: Analyze existing database schemas
- **CLI Interface**: Command-line tools for easy integration

---

## Getting Started

### Installation

Add SchemaSync to your Cargo.toml:

```toml
[dependencies]
schema_sync = "0.1.0"
```

### Basic Usage

1. Define your model structs with the `#[schema_sync]` attribute:

```rust
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[schema_sync]
pub struct User {
    #[schema_sync_field(primary_key = true)]
    pub id: Uuid,
    
    #[schema_sync_field(unique = true)]
    pub email: String,
    
    pub name: String,
    
    #[schema_sync_field(nullable = true)]
    pub bio: Option<String>,
    
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

2. Initialize SchemaSync with a configuration file:

```rust
use schema_sync::{init, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize with config file
    let mut client = init("schema_sync.toml").await?;
    
    // Synchronize database
    client.sync_database().await?;
    
    Ok(())
}
```

3. Run the synchronization process to generate and apply migrations.

---

## Configuration

SchemaSync is configured using a TOML file. Here's an example configuration:

```toml
[database]
driver = "postgres"
url = "postgres://username:password@localhost:5432/dbname"
pool_size = 10
timeout_seconds = 30
schema = "public"
enable_ssl = true

[migrations]
directory = "./migrations"
naming = "timestamp_description"
auto_generate = true
auto_apply = false
transaction_per_migration = true
dry_run = false
backup_before_migrate = true
history_table = "schema_sync_history"

[models]
paths = ["./src/models"]
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
```

---

## Command-Line Interface

SchemaSync provides a command-line interface for common operations:

```bash
# Initialize a new project
schema_sync init --name my_project

# Analyze database schema
schema_sync analyze --format json --output schema.json

# Generate migrations (dry run)
schema_sync generate --dry-run

# Apply migrations
schema_sync apply

# Complete workflow: analyze, generate, and apply
schema_sync sync
```

---

## Field Attributes

Use field attributes to customize column properties:

- `#[schema_sync_field(primary_key = true)]` - Define primary key
- `#[schema_sync_field(nullable = true)]` - Make column nullable
- `#[schema_sync_field(unique = true)]` - Add unique constraint
- `#[schema_sync_field(default = "value")]` - Set default value
- `#[schema_sync_field(comment = "description")]` - Add column comment
- `#[schema_sync_field(db_type = "VARCHAR(100)")]` - Override database type
- `#[schema_sync_field(foreign_key = "table.column")]` - Define foreign key

---

## API Reference

### Main Types

- `SchemaSyncClient` - Main client for interacting with SchemaSync
- `DatabaseConnection` - Database connection handler
- `ModelRegistry` - Registry for model structs
- `SchemaAnalyzer` - Database schema analyzer
- `SchemaDiff` - Schema difference calculator
- `MigrationGenerator` - SQL migration generator

### Key Methods

- `init(config_path)` - Initialize with configuration
- `register_models()` - Scan and register model structs
- `analyze_database_schema()` - Analyze current database
- `generate_schema_diff()` - Compare model and database schemas
- `generate_migrations(diff)` - Generate migration SQL
- `apply_migrations(migrations)` - Apply migrations to database
- `sync_database()` - Complete workflow: register, analyze, generate, apply

---

## Type Mappings

SchemaSync provides default type mappings:

| Rust Type | Database Type |
|-----------|---------------|
| `String` | `VARCHAR(255)` |
| `i8`, `i16` | `SMALLINT` |
| `i32` | `INTEGER` |
| `i64` | `BIGINT` |
| `u8`, `u16`, `u32` | `INTEGER` |
| `u64` | `BIGINT` |
| `f32` | `REAL` |
| `f64` | `DOUBLE PRECISION` |
| `bool` | `BOOLEAN` |
| `Vec<u8>` | `BYTEA` |
| `DateTime<Utc>` | `TIMESTAMP WITH TIME ZONE` |
| `NaiveDateTime` | `TIMESTAMP` |
| `NaiveDate` | `DATE` |
| `Uuid` | `UUID` |
| `Decimal` | `NUMERIC(20,6)` |
| `Json`, `Value` | `JSONB` |

Custom mappings can be defined in the configuration file.

---

## License

SchemaSync is licensed under MIT

---

## Contributing

Contributions are welcome! Please feel free to submit pull requests.

---

*Documentation built for SchemaSync v0.1.0*
