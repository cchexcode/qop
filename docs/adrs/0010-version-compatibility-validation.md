# ADR-0010: Version Compatibility Validation Pattern

## Status

Accepted

## Date

2025-01-27T23:10:00Z

## Context

The `qop` migration tool evolves over time with new features, configuration schema changes, and database operation improvements. Configuration files created with older versions of the tool may not be compatible with newer versions, and vice versa. This creates several challenges:

1. **Configuration Evolution**: Configuration schema changes over time require migration or validation
2. **Feature Compatibility**: New CLI features may not work with old configuration files
3. **Database Schema Changes**: Internal migration table schemas may evolve between versions
4. **User Experience**: Users need clear error messages when version mismatches occur
5. **Development Workflow**: Developers need to ensure compatibility during upgrades

Without proper version validation:
- Users may encounter cryptic errors from configuration mismatches
- Silent failures may occur when incompatible versions interact
- Database corruption could result from schema assumption mismatches
- Development and production environments may become inconsistent

The solution requires a robust version compatibility system that validates CLI-config compatibility at runtime and provides clear feedback to users.

## Decision

The application MUST implement comprehensive version compatibility validation using PEP 440-style version specifications in configuration files, with validation performed at configuration load time.

### Version Compatibility Requirements

1. **Version Specifications**: Configuration files MUST include PEP 440-compatible version specifications
2. **Runtime Validation**: Version compatibility MUST be validated when configuration is loaded
3. **Clear Error Messages**: Version mismatches MUST produce actionable error messages
4. **Semantic Versioning**: The CLI MUST follow semantic versioning principles
5. **Flexible Specifications**: Configuration files MUST support flexible version ranges

### Implementation Pattern

```toml
# Configuration files specify required CLI version ranges
version = ">=0.4.0,<1.0.0"  # PEP 440 specification

[subsystem.postgres]
# ... rest of configuration
```

## Consequences

### Positive

- **Compatibility Safety**: Prevents incompatible CLI-configuration combinations
- **Clear Error Messages**: Users receive actionable feedback for version mismatches
- **Development Safety**: Prevents accidental usage of incompatible versions
- **Future-Proofing**: Establishes patterns for handling future version evolution
- **Operational Reliability**: Reduces risk of silent failures in production environments
- **Documentation**: Configuration files self-document their compatibility requirements

### Negative

- **Configuration Overhead**: Users must maintain version specifications in config files
- **Version Maintenance**: Developers must carefully manage version compatibility ranges
- **Learning Curve**: Users need to understand PEP 440 version specification syntax
- **Migration Complexity**: Version changes may require configuration file updates
- **Development Workflow**: Additional validation step adds complexity to development process

## Implementation

### Configuration Schema

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WithVersion {
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    pub version: String,        // PEP 440 version specification
    pub subsystem: Subsystem,
}
```

### Version Validation Logic

```rust
use pep440_rs::{Version, VersionSpecifiers};
use std::str::FromStr;

impl WithVersion {
    pub fn validate(&self, cli: &str) -> Result<(), anyhow::Error> {
        // Parse CLI version (e.g., "0.4.2")
        let cli_version = Version::from_str(cli)
            .map_err(|e| anyhow::anyhow!("Invalid CLI version '{}': {}", cli, e))?;
        
        // Parse version specification from config (e.g., ">=0.4.0,<1.0.0")
        let version_specifier = VersionSpecifiers::from_str(&self.version)
            .map_err(|e| anyhow::anyhow!("Invalid version specification '{}': {}", self.version, e))?;

        // Check compatibility
        if !version_specifier.contains(&cli_version) {
            return Err(anyhow::anyhow!(
                "Version mismatch: Config requires CLI version '{}', but current CLI version is '{}'", 
                self.version, 
                cli
            ));
        }

        Ok(())
    }
}
```

### Integration with Configuration Loading

```rust
impl ClapArgumentLoader {
    pub fn load() -> Result<CallArgs> {
        // ... command parsing logic ...
        
        // Load configuration file when needed
        let config_content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&config_content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        // Validate version compatibility immediately after parsing
        config.validate(env!("CARGO_PKG_VERSION"))
            .with_context(|| format!("Version compatibility check failed for config: {}", path.display()))?;

        // ... continue with validated configuration ...
    }
}
```

### Repository Integration

```rust
impl PostgresRepo {
    pub async fn from_config(
        path: &Path, 
        config: SubsystemPostgres, 
        check_cli_version: bool
    ) -> Result<Self> {
        let pool = build_pool_from_config(path, &config, check_cli_version).await?;
        Ok(Self { config, pool, path: path.to_path_buf() })
    }
}

async fn build_pool_from_config(
    path: &Path, 
    config: &SubsystemPostgres, 
    check_cli_version: bool
) -> Result<Pool<Postgres>> {
    let uri = resolve_connection_string(&config.connection, path)?;
    let pool = PgPoolOptions::new().max_connections(5).connect(&uri).await?;
    
    // Optional: Additional version checks against database schema
    if check_cli_version {
        let mut tx = pool.begin().await?;
        // Check if migration table schema is compatible with current CLI version
        let version_check_result = sqlx::query(
            "SELECT version FROM information_schema.tables WHERE table_name = $1 LIMIT 1"
        )
        .bind(&config.migrations_table())
        .fetch_optional(&mut *tx)
        .await?;
        
        tx.commit().await?;
    }
    
    Ok(pool)
}
```

### Configuration Generation with Version

```rust
pub fn build_sample_postgres_config(connection: &str) -> Config {
    Config {
        version: ">=0.4.0".to_string(),  // Current version requirement
        subsystem: Subsystem::Postgres(SubsystemPostgres {
            connection: DataSource::Static(connection.to_string()),
            timeout: Some(30),
            schema: "public".to_string(),
            table_prefix: "__qop".to_string(),
        }),
    }
}

pub fn build_sample_sqlite_config(db_path: &Path) -> Config {
    Config {
        version: ">=0.4.0".to_string(),
        subsystem: Subsystem::Sqlite(SubsystemSqlite {
            connection: DataSource::Static(
                format!("sqlite:///{}", db_path.display())
            ),
            timeout: Some(30),
            table_prefix: "__qop".to_string(),
        }),
    }
}
```

### Error Messages and User Guidance

```rust
// Enhanced error context with suggestions
impl WithVersion {
    pub fn validate(&self, cli: &str) -> Result<()> {
        let cli_version = Version::from_str(cli)
            .map_err(|e| anyhow::anyhow!("Invalid CLI version '{}': {}", cli, e))?;
        
        let version_specifier = VersionSpecifiers::from_str(&self.version)
            .map_err(|e| anyhow::anyhow!(
                "Invalid version specification '{}' in config file: {}\n\
                 Valid examples: '>=0.4.0', '>=0.4.0,<1.0.0', '==0.4.2'",
                self.version, e
            ))?;

        if !version_specifier.contains(&cli_version) {
            return Err(anyhow::anyhow!(
                "Version mismatch:\n\
                 • Config file requires CLI version: {}\n\
                 • Current CLI version: {}\n\
                 \n\
                 Solutions:\n\
                 • Update CLI: cargo install qop --force\n\
                 • Update config version specification\n\
                 • Use compatible CLI version",
                self.version, 
                cli
            ));
        }

        Ok(())
    }
}
```

## Version Specification Guidelines

### Recommended Patterns

1. **Minimum Version**: `>=0.4.0` - Requires at least version 0.4.0
2. **Version Range**: `>=0.4.0,<1.0.0` - Compatible with 0.4.x but not 1.x
3. **Patch Range**: `>=0.4.2,<0.5.0` - Compatible with 0.4.2+ but not 0.5.x
4. **Exact Version**: `==0.4.2` - Requires exactly version 0.4.2 (not recommended)

### Configuration Examples

```toml
# Conservative: Only allow current major version
version = ">=0.4.0,<0.5.0"

[subsystem.postgres]
connection = { from_env = "DATABASE_URL" }
schema = "public"
table_prefix = "__qop"
timeout = 30
```

```toml
# Permissive: Allow current and future minor versions
version = ">=0.4.0,<1.0.0"

[subsystem.sqlite]
connection = { static = "sqlite:///app.db" }
table_prefix = "__qop"
timeout = 30
```

```toml
# Development: Allow any version (not recommended for production)
version = ">=0.0.0"

[subsystem.postgres]
connection = { static = "postgresql://localhost:5432/dev" }
schema = "public"
table_prefix = "__qop_dev"
```

### Version Evolution Strategy

1. **Semantic Versioning**: Follow semantic versioning for CLI releases
2. **Breaking Changes**: Major version bumps for breaking configuration changes
3. **Feature Addition**: Minor version bumps for new features
4. **Bug Fixes**: Patch version bumps for bug fixes
5. **Configuration Migration**: Provide tools or documentation for config migration

## Development Guidelines

### Version Compatibility Matrix

| CLI Version | Config Version Spec | Compatibility |
|-------------|-------------------|---------------|
| 0.4.0       | `>=0.4.0`         | ✅ Compatible |
| 0.4.2       | `>=0.4.0,<0.5.0`  | ✅ Compatible |
| 0.5.0       | `>=0.4.0,<0.5.0`  | ❌ Incompatible |
| 1.0.0       | `>=0.4.0,<1.0.0`  | ❌ Incompatible |

### Testing Version Compatibility

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility() {
        let config = WithVersion {
            version: ">=0.4.0,<1.0.0".to_string(),
        };

        // Should accept compatible versions
        assert!(config.validate("0.4.0").is_ok());
        assert!(config.validate("0.4.5").is_ok());
        assert!(config.validate("0.9.9").is_ok());

        // Should reject incompatible versions
        assert!(config.validate("0.3.9").is_err());
        assert!(config.validate("1.0.0").is_err());
    }

    #[test]
    fn test_invalid_version_specs() {
        let config = WithVersion {
            version: "invalid-spec".to_string(),
        };

        assert!(config.validate("0.4.0").is_err());
    }
}
```

### Release Process Integration

1. **Version Bump**: Update `Cargo.toml` version following semantic versioning
2. **Config Generation**: Update sample config generation to require new version
3. **Migration Guide**: Document any configuration changes required
4. **Compatibility Testing**: Test against various config version specifications
5. **Release Notes**: Document version compatibility requirements

## References

- [PEP 440: Version Identification and Dependency Specification](https://peps.python.org/pep-0440/)
- [Semantic Versioning](https://semver.org/)
- [pep440_rs Crate Documentation](https://docs.rs/pep440_rs/)
- [Cargo Book: Specifying Dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html)
