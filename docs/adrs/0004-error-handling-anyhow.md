# ADR-0004: Unified Error Handling with `anyhow`

## Status

Accepted

## Date

2025-01-27T22:40:00Z

## Context

The `qop` migration tool interfaces with multiple external systems (databases, file systems, configuration files) and performs complex operations that can fail in various ways. Error handling needs to be:

1. Consistent across all subsystems and operations
2. User-friendly with meaningful error messages
3. Developer-friendly for debugging and maintenance
4. Efficient to implement without excessive boilerplate
5. Compatible with async/await patterns

Traditional Rust error handling with custom error types requires significant boilerplate and can lead to inconsistent error patterns across different modules. The `anyhow` crate provides a unified approach to error handling that balances simplicity with functionality.

## Decision

All functions that can fail MUST use `anyhow::Result<T>` as their return type, and the codebase MUST adopt `anyhow` patterns for consistent error handling throughout all layers.

### Error Handling Requirements

1. **Unified Return Type**: All fallible functions MUST return `anyhow::Result<T>`
2. **Context Addition**: Errors MUST include context using `.with_context()` or `.context()`
3. **Error Propagation**: The `?` operator MUST be used for error propagation
4. **Custom Error Creation**: Use `anyhow::anyhow!()` or `anyhow::bail!()` for custom errors
5. **Async Compatibility**: Error handling MUST work seamlessly with async/await

### Error Context Patterns

1. **File Operations**:
   ```rust
   std::fs::read_to_string(&path)
       .with_context(|| format!("Failed to read file: {}", path.display()))?
   ```

2. **Database Operations**:
   ```rust
   query.execute(&mut *tx).await
       .with_context(|| format!("Failed to execute migration {}", migration_id))?
   ```

3. **Configuration Validation**:
   ```rust
   Version::from_str(cli)
       .map_err(|e| anyhow::anyhow!("Invalid CLI version '{}': {}", cli, e))?
   ```

4. **Early Returns**:
   ```rust
   if !version_specifier.contains(&cli_version) {
       anyhow::bail!("Version mismatch: expected '{}', got '{}'", 
                     self.version, cli);
   }
   ```

## Consequences

### Positive

- **Consistency**: All error handling follows the same patterns across the codebase
- **Minimal Boilerplate**: No need to define custom error types or implement conversions
- **Rich Context**: Error chains provide detailed information about failure causes
- **User Experience**: Meaningful error messages help users understand and resolve issues
- **Development Velocity**: Less time spent on error handling implementation
- **Async Compatibility**: Works seamlessly with async/await patterns

### Negative

- **Type Erasure**: Specific error types are erased, making programmatic error handling difficult
- **Library Dependency**: Adds `anyhow` as a fundamental dependency throughout the codebase
- **Performance Overhead**: Error chains may have slight performance impact compared to simple errors
- **Less Granular Handling**: Cannot easily pattern match on specific error types

## Implementation

### Function Signatures
```rust
// Correct
pub async fn apply_migration(&self, id: &str, ...) -> Result<()> { ... }

// Incorrect
pub async fn apply_migration(&self, id: &str, ...) -> std::result::Result<(), MyError> { ... }
```

### Context Addition
```rust
// Good - provides context about what operation failed
let config: Config = toml::from_str(&content)
    .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

// Better - includes both operation and location context
std::fs::create_dir_all(&path)
    .with_context(|| format!("Failed to create directory: {}", path.display()))?;
```

### Error Creation
```rust
// For validation errors
if targets.is_empty() {
    anyhow::bail!("No migrations found to revert");
}

// For complex error scenarios
let migration_dir = path.parent()
    .ok_or_else(|| anyhow::anyhow!("Invalid migration path: {}", path.display()))?;
```

### Error Propagation in Async Functions
```rust
pub async fn up(&self, path: &Path, timeout: Option<u64>) -> Result<()> {
    let local = util::get_local_migrations(path)?;  // File I/O error
    let applied = self.repo.fetch_applied_ids().await?;  // Database error
    
    for id in to_apply {
        self.repo.apply_migration(&id, &up_sql, &down_sql, 
                                 meta.comment.as_deref(), 
                                 previous.as_deref(), 
                                 timeout, dry_run, 
                                 meta.is_locked()).await?;  // Database error with context
    }
    
    Ok(())
}
```

## Guidelines

1. **Always Add Context**: Every error should include context about what operation was being performed
2. **Include Relevant Data**: Error messages should include file paths, IDs, or other relevant identifiers
3. **Use Consistent Formatting**: Error messages should follow consistent formatting patterns
4. **Avoid Error Swallowing**: Always propagate errors using `?` or handle them appropriately
5. **Main Function**: The main function should use `anyhow::Result<()>` for consistent error reporting

## Anti-Patterns

```rust
// Don't: Swallow errors without context
let _ = some_operation();

// Don't: Use unwrap() in library code
let value = risky_operation().unwrap();

// Don't: Generic error messages
.context("Something went wrong")?

// Don't: Mixed error types
fn mixed_errors() -> Result<String, Box<dyn std::error::Error>> { ... }
```

## References

- [`anyhow` crate documentation](https://docs.rs/anyhow/)
- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Error Handling Patterns in Rust](https://rust-lang.github.io/api-guidelines/errors.html)
