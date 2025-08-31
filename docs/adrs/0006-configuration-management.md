# ADR-0006: Configuration Management with TOML and Environment Variables

## Status

Accepted

## Date

2025-01-27T22:50:00Z

## Context

The `qop` migration tool requires flexible configuration management to support various deployment scenarios:

1. **Development**: Simple static configuration for local development
2. **Production**: Secure handling of credentials via environment variables
3. **CI/CD**: Configuration that can be easily automated and templated
4. **Multi-environment**: Support for different database connections per environment

Configuration needs to be:
- Human-readable and maintainable
- Secure for sensitive data (connection strings, credentials)
- Validatable to catch configuration errors early
- Versionable to ensure compatibility between CLI and config formats

TOML provides a balance between human readability and machine parsing, while environment variable support enables secure credential management.

## Decision

The application MUST use TOML files for configuration with support for environment variable substitution and comprehensive validation.

### Configuration Structure

1. **Primary Format**: TOML configuration files (`qop.toml`)
2. **Environment Integration**: Support both static values and environment variable references
3. **Version Validation**: Include version specifications for compatibility checking
4. **Subsystem Isolation**: Each database backend has its own configuration section
5. **Serde Integration**: Use Serde for deserialization with appropriate attributes

### Configuration Schema

```toml
# Version compatibility specification
version = ">=0.1.0"

[subsystem.postgres]
connection = { static = "postgresql://user:pass@localhost:5432/db" }
# OR
connection = { from_env = "DATABASE_URL" }
schema = "public"
table_prefix = "__qop"
timeout = 30

[subsystem.sqlite]
connection = { static = "sqlite:///path/to/database.db" }
# OR  
connection = { from_env = "SQLITE_DATABASE_URL" }
table_prefix = "__qop"
timeout = 30
```

## Consequences

### Positive

- **Human Readable**: TOML is easy to read and write by developers
- **Environment Flexibility**: Supports both static and environment-based configuration
- **Security**: Sensitive data can be kept out of version control via environment variables
- **Validation**: Type-safe deserialization with comprehensive error messages
- **Version Safety**: Configuration compatibility is validated at runtime
- **IDE Support**: TOML has good tooling and syntax highlighting support

### Negative

- **Parsing Overhead**: TOML parsing adds runtime overhead compared to hardcoded configuration
- **File Dependency**: Applications require external configuration files
- **Environment Complexity**: Multiple configuration sources can make debugging difficult
- **Schema Evolution**: Configuration format changes require careful versioning

## Implementation

### Core Configuration Types

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    pub version: String,
    pub subsystem: Subsystem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: DeserializeOwned"))]
pub enum DataSource<T: Serialize + DeserializeOwned> {
    Static(T),
    FromEnv(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Subsystem {
    #[cfg(feature = "sub+postgres")]
    Postgres(SubsystemPostgres),
    #[cfg(feature = "sub+sqlite")]
    Sqlite(SubsystemSqlite),
}
```

### Subsystem Configuration Pattern

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SubsystemPostgres {
    pub connection: DataSource<String>,
    pub timeout: Option<u64>,
    pub schema: String,
    pub table_prefix: String,
}

impl SubsystemPostgres {
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

### Configuration Loading Pattern

```rust
// Load and parse configuration
let config_content = std::fs::read_to_string(&path)
    .with_context(|| format!("Failed to read config file: {}", path.display()))?;

let config: Config = toml::from_str(&config_content)
    .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

// Validate version compatibility
config.validate_version(env!("CARGO_PKG_VERSION"))?;
```

### Environment Variable Resolution

```rust
pub fn resolve_connection_string(source: &DataSource<String>, config_path: &Path) -> Result<String> {
    match source {
        DataSource::Static(connection) => Ok(connection.clone()),
        DataSource::FromEnv(var) => {
            std::env::var(var).with_context(|| {
                format!(
                    "Missing environment variable '{}' referenced in config {}",
                    var,
                    config_path.display()
                )
            })
        }
    }
}
```

### Version Validation Pattern

```rust
impl WithVersion {
    pub fn validate(&self, cli: &str) -> Result<()> {
        let cli_version = Version::from_str(cli)
            .map_err(|e| anyhow::anyhow!("Invalid CLI version '{}': {}", cli, e))?;
        
        let version_specifier = VersionSpecifiers::from_str(&self.version)
            .map_err(|e| anyhow::anyhow!("Invalid version specification '{}': {}", self.version, e))?;

        if !version_specifier.contains(&cli_version) {
            anyhow::bail!(
                "Version mismatch: Config requires CLI version '{}', but current version is '{}'", 
                self.version, 
                cli
            );
        }

        Ok(())
    }
}
```

### Configuration Generation Pattern

```rust
pub fn build_sample_config(connection: &str) -> Config {
    Config {
        version: ">=0.1.0".to_string(),
        subsystem: Subsystem::Postgres(SubsystemPostgres {
            connection: DataSource::Static(connection.to_string()),
            timeout: Some(30),
            schema: "public".to_string(),
            table_prefix: "__qop".to_string(),
        }),
    }
}
```

## Configuration Guidelines

1. **Sensitive Data**: Connection strings with credentials MUST use environment variables
2. **Defaults**: All configuration structures MUST implement sensible defaults
3. **Validation**: Configuration MUST be validated immediately after loading
4. **Error Context**: Configuration errors MUST include file paths and specific field information
5. **Version Compatibility**: All configurations MUST include version specifications

### Environment Variable Naming

- Use descriptive, uppercase names: `DATABASE_URL`, `SQLITE_DATABASE_URL`
- Include service/component context when needed
- Follow conventional patterns for the deployment environment

### File Organization

```
project/
├── qop.toml              # Main configuration file
├── environments/
│   ├── development.toml  # Development-specific config
│   ├── staging.toml      # Staging-specific config
│   └── production.toml   # Production-specific config (uses env vars)
```

### Security Best Practices

```toml
# Good: Use environment variables for sensitive data
[subsystem.postgres]
connection = { from_env = "DATABASE_URL" }
schema = "public"
table_prefix = "__qop"

# Bad: Hardcode credentials in configuration files
[subsystem.postgres]
connection = { static = "postgresql://user:password@localhost:5432/db" }
```

## Example Configurations

### Development Configuration
```toml
version = ">=0.1.0"

[subsystem.sqlite]
connection = { static = "sqlite:///./dev.db" }
table_prefix = "__qop"
timeout = 30
```

### Production Configuration
```toml
version = ">=0.4.0"

[subsystem.postgres]
connection = { from_env = "DATABASE_URL" }
schema = "public" 
table_prefix = "__qop"
timeout = 60
```

## References

- [TOML Specification](https://toml.io/)
- [Serde TOML Documentation](https://docs.rs/toml/)
- [PEP 440 Version Specification](https://peps.python.org/pep-0440/)
- [Twelve-Factor App: Config](https://12factor.net/config)
