# ADR-0009: Strong Typing Patterns with Comprehensive Enums and Structs

## Status

Accepted

## Date

2025-01-27T23:05:00Z

## Context

The `qop` migration tool handles complex state transitions, configuration variants, and command combinations that can lead to runtime errors if not properly modeled. Common sources of errors in migration tools include:

1. **Invalid State Combinations**: Applying operations in wrong states or with incompatible options
2. **Configuration Errors**: Mismatched configuration types or invalid combinations  
3. **Data Validation**: Ensuring migration IDs, paths, and other data meet requirements
4. **API Contracts**: Preventing invalid method calls or parameter combinations

Rust's type system provides powerful tools for encoding constraints and preventing invalid states through:
- Comprehensive enum modeling of all valid states
- Struct types with validation logic
- Generic constraints and trait bounds
- Serde integration for serialization safety

The goal is to make invalid states unrepresentable at the type level, moving error detection from runtime to compile time wherever possible.

## Decision

The codebase MUST leverage Rust's type system to prevent invalid states and operations through comprehensive enum modeling, strong struct typing, and validation constraints embedded in the type system.

### Type Safety Requirements

1. **Enum Modeling**: All finite state spaces MUST be modeled as enums
2. **Struct Validation**: Struct types MUST enforce their invariants through validation
3. **Generic Constraints**: Generic code MUST use appropriate trait bounds
4. **Serde Integration**: Serialization/deserialization MUST maintain type safety
5. **Error Prevention**: Invalid combinations MUST be unrepresentable when possible

## Consequences

### Positive

- **Compile-Time Safety**: Many error categories are caught at compile time
- **Self-Documenting Code**: Types serve as living documentation of valid states
- **Refactoring Safety**: Type system helps ensure correctness during changes
- **API Clarity**: Function signatures clearly communicate what values are valid
- **Runtime Reliability**: Fewer runtime errors due to invalid states
- **Developer Productivity**: IDE support and compiler help prevent mistakes

### Negative

- **Learning Curve**: Requires understanding of Rust's type system capabilities
- **Initial Complexity**: More upfront design work to model types correctly
- **Compilation Time**: Complex type hierarchies may increase compilation time
- **Boilerplate**: Some patterns require additional implementation code
- **Evolution Overhead**: Type changes may require updates across the codebase

## Implementation

### Comprehensive Enum Modeling

```rust
// Model all possible data sources as an enum rather than using strings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: DeserializeOwned"))]
pub enum DataSource<T: Serialize + DeserializeOwned> {
    Static(T),
    FromEnv(String),
}

// Model all possible subsystems with feature-conditional compilation
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Subsystem {
    #[cfg(feature = "sub+postgres")]
    Postgres(SubsystemPostgres),
    #[cfg(feature = "sub+sqlite")]
    Sqlite(SubsystemSqlite),
}

// Model all possible output formats instead of using strings
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Human,
    Json,
}

// Model privilege levels for experimental features
#[derive(Debug, Eq, PartialEq)]
pub enum Privilege {
    Normal,
    Experimental,
}
```

### Structured Command Modeling

```rust
// Hierarchical command modeling with type safety
#[derive(Debug)]
pub enum Command {
    Manual {
        path: PathBuf,           // Strong path type
        format: ManualFormat,    // Enum instead of string
    },
    Autocomplete {
        path: PathBuf,
        shell: clap_complete::Shell,  // Use library's enum type
    },
    Subsystem(Subsystem),        // Nested enum structure
}

// Subsystem-specific commands with precise parameter modeling
#[derive(Debug)]
pub enum MigrationApply {
    Up { 
        id: String, 
        timeout: Option<u64>,    // Optional with specific type
        dry: bool,               // Boolean rather than string flag
        yes: bool 
    },
    Down { 
        id: String, 
        timeout: Option<u64>, 
        remote: bool,
        dry: bool, 
        yes: bool, 
        unlock: bool 
    },
}
```

### Validated Struct Types

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MigrationMeta {
    pub comment: Option<String>,
    pub locked: Option<bool>,
}

impl MigrationMeta {
    /// Create with validation and defaults
    pub fn new_with_default_comment() -> Self {
        let username = whoami::username();
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let comment = format!("Created by {} at {}", username, timestamp);
        Self { 
            comment: Some(comment), 
            locked: None 
        }
    }
    
    /// Type-safe accessor with default behavior
    pub fn is_locked(&self) -> bool {
        self.locked.unwrap_or(false)
    }
}

impl Default for MigrationMeta {
    fn default() -> Self {
        Self { comment: None, locked: None }
    }
}
```

### Configuration Type Safety

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SubsystemPostgres {
    pub connection: DataSource<String>,    // Type-safe data source
    pub timeout: Option<u64>,              // Specific numeric type
    pub schema: String,                    // Required field
    pub table_prefix: String,              // Required field with validation
}

impl SubsystemPostgres {
    /// Type-safe table name generation
    pub fn migrations_table(&self) -> String {
        format!("{}_migrations", self.table_prefix)
    }
    
    pub fn log_table(&self) -> String {
        format!("{}_log", self.table_prefix)
    }
}

impl Default for SubsystemPostgres {
    fn default() -> Self {
        Self {
            connection: DataSource::Static(String::new()),
            timeout: None,
            schema: "public".to_string(),
            table_prefix: "__qop".to_string(),
        }
    }
}
```

### Generic Type Constraints

```rust
// Service layer uses generic constraints for type safety
pub struct MigrationService<R: MigrationRepository> {
    repo: R,
}

impl<R: MigrationRepository> MigrationService<R> {
    pub fn new(repo: R) -> Self { 
        Self { repo } 
    }
    
    // Methods can rely on trait constraints
    pub async fn up(&self, ...) -> Result<()> {
        // Compiler ensures R implements all required methods
        let applied = self.repo.fetch_applied_ids().await?;
        // ...
    }
}

// Repository trait defines exact contracts
#[async_trait::async_trait(?Send)]
pub trait MigrationRepository {
    async fn fetch_applied_ids(&self) -> Result<HashSet<String>>;
    async fn fetch_history(&self) -> Result<Vec<(String, NaiveDateTime, Option<String>, bool)>>;
    // Specific return types prevent implementation errors
}
```

### Validation Through Types

```rust
// Path handling with validation
impl ClapArgumentLoader {
    fn get_absolute_path(matches: &clap::ArgMatches, name: &str) -> Result<PathBuf> {
        let path_str: &String = matches.get_one(name).unwrap();
        let path = std::path::Path::new(path_str);
        if path.is_absolute() {
            Ok(path.to_path_buf().clean())  // Use path-clean for normalization
        } else {
            Ok(std::env::current_dir()?.join(path).clean())
        }
    }
}

// Version validation with type-safe parsing
impl WithVersion {
    pub fn validate(&self, cli: &str) -> Result<()> {
        // Use typed version parsing instead of string comparison
        let cli_version = Version::from_str(cli)
            .map_err(|e| anyhow::anyhow!("Invalid CLI version '{}': {}", cli, e))?;
        
        let version_specifier = VersionSpecifiers::from_str(&self.version)
            .map_err(|e| anyhow::anyhow!("Invalid version specification '{}': {}", self.version, e))?;

        if !version_specifier.contains(&cli_version) {
            anyhow::bail!("Version mismatch: expected '{}', got '{}'", self.version, cli);
        }

        Ok(())
    }
}
```

### Serde Integration for Serialization Safety

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]  // Consistent naming convention
pub struct Config {
    pub version: String,
    pub subsystem: Subsystem,
}

// Generic data source with proper serde bounds
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: DeserializeOwned"))]
pub enum DataSource<T: Serialize + DeserializeOwned> {
    Static(T),
    FromEnv(String),
}

// Feature-conditional serialization
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Subsystem {
    #[cfg(feature = "sub+postgres")]
    Postgres(SubsystemPostgres),
    #[cfg(feature = "sub+sqlite")]
    Sqlite(SubsystemSqlite),
}
```

### Error Type Safety

```rust
// Use Result<T> consistently with anyhow for error context
pub async fn apply_migration(&self, ...) -> Result<()> {
    let migration_dir = path.parent()
        .ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
    
    // Type-safe error propagation
    let (up_sql, down_sql, meta) = util::read_migration_with_meta(migration_dir, &target_id)?;
    
    Ok(())
}

// Validation functions return typed errors
fn normalize_migration_id(id: &str) -> String {
    if id.starts_with("id=") {
        id.strip_prefix("id=").unwrap().to_string()  // Safe unwrap due to check
    } else {
        id.to_string()
    }
}
```

## Design Guidelines

### Enum Usage Patterns

1. **Finite State Modeling**: Use enums for any finite set of possibilities
2. **Command Hierarchies**: Model command trees with nested enums
3. **Configuration Variants**: Use enums instead of string-based configuration
4. **Feature Flags**: Use conditional compilation with enums for optional features
5. **Error Categories**: Use enums for categorizing different error types

### Struct Design Principles  

1. **Validation in Constructors**: Validate invariants in constructors or factory methods
2. **Type-Safe Accessors**: Provide methods that encode business logic
3. **Comprehensive Defaults**: Implement `Default` with sensible values
4. **Serde Integration**: Use appropriate serde attributes for serialization
5. **Documentation**: Document type invariants and expected usage

### Generic Constraints

1. **Trait Bounds**: Use trait bounds to express requirements clearly
2. **Associated Types**: Use associated types when relationships exist between types
3. **Lifetime Parameters**: Use explicit lifetimes when ownership is complex
4. **Phantom Types**: Use phantom types to encode additional constraints when needed

## Anti-Patterns

```rust
// Don't: String-based modeling
pub struct Config {
    pub database_type: String,  // Should be enum
    pub output_format: String,  // Should be enum  
}

// Don't: Weak parameter types
pub fn execute_command(command: String, args: Vec<String>) -> Result<()> {
    // Loses type safety and validation
}

// Don't: Unvalidated constructors
pub struct MigrationId {
    pub id: String,  // Should validate format
}

impl MigrationId {
    pub fn new(id: String) -> Self {
        Self { id }  // No validation
    }
}

// Don't: Generic without bounds
pub struct Service<T> {
    repo: T,  // Should have trait bound
}
```

## Testing Type Safety

```rust
// Types enable comprehensive testing
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_meta_defaults() {
        let meta = MigrationMeta::default();
        assert_eq!(meta.is_locked(), false);  // Type-safe default behavior
    }

    #[test]
    fn test_config_serialization() {
        let config = Config {
            version: ">=0.1.0".to_string(),
            subsystem: Subsystem::Postgres(SubsystemPostgres::default()),
        };
        
        // Type-safe serialization round-trip
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        
        // Compiler ensures types match
        assert_eq!(config.version, deserialized.version);
    }
}
```

## References

- [Rust Book: Enums and Pattern Matching](https://doc.rust-lang.org/book/ch06-00-enums.html)
- [Making Invalid States Unrepresentable](https://geeklaunch.io/blog/make-invalid-states-unrepresentable/)
- [Serde Documentation](https://serde.rs/)
- [Type-Driven Development](https://blog.ploeh.dk/2015/08/10/type-driven-development/)
