# ADR-0005: Async/Await Patterns and Tokio Runtime

## Status

Accepted

## Date

2025-01-27T22:45:00Z

## Context

The `qop` migration tool performs significant I/O operations including:
1. Database connections and queries (potentially long-running)
2. File system operations for reading migration files
3. Network operations for database connectivity

Modern database drivers (like `sqlx`) are built with async/await patterns to provide better performance and resource utilization. The application needs a consistent approach to handle asynchronous operations throughout the codebase while maintaining simplicity and avoiding common async pitfalls.

The choice of async runtime is critical as it affects performance, ecosystem compatibility, and development patterns throughout the application.

## Decision

The codebase MUST use async/await patterns with the Tokio runtime for all I/O operations, following established async best practices and patterns.

### Runtime Configuration

1. **Tokio Runtime**: The application MUST use Tokio as the async runtime
2. **Runtime Features**: Enable multi-threaded runtime with necessary features
3. **Main Function**: Use `#[tokio::main]` for the application entry point
4. **Async Traits**: Use `async-trait` crate for trait methods that need to be async

### Tokio Configuration

```toml
tokio = { version = "1.47.1", features = [
    "rt",              # Basic runtime
    "rt-multi-thread", # Multi-threaded runtime
    "macros",          # #[tokio::main] and other macros
    "process",         # Process spawning (if needed)
    "io-util",         # I/O utilities
    "time",            # Time utilities
    "sync",            # Synchronization primitives
] }
```

### Async Patterns Requirements

1. **Repository Layer**: All database operations MUST be async
2. **Service Layer**: Business logic operations that involve I/O MUST be async
3. **Error Propagation**: Async functions MUST use `?` operator with `anyhow::Result`
4. **Resource Management**: Database connections MUST use connection pooling
5. **Transaction Handling**: Database transactions MUST be properly scoped and handled

## Consequences

### Positive

- **Performance**: Non-blocking I/O provides better resource utilization
- **Scalability**: Better handling of concurrent database operations
- **Ecosystem Compatibility**: Works seamlessly with modern database drivers (`sqlx`)
- **Resource Efficiency**: Lower memory and CPU usage compared to blocking operations
- **Cancellation**: Natural support for operation cancellation and timeouts

### Negative

- **Complexity**: Async/await adds complexity to the codebase
- **Learning Curve**: Developers must understand async patterns and potential pitfalls
- **Runtime Dependency**: Application is tightly coupled to Tokio runtime
- **Debugging**: Async stack traces can be more difficult to debug
- **Compilation Time**: Async code may increase compilation time

## Implementation

### Main Function Pattern
```rust
#[tokio::main]
async fn main() -> Result<()> {
    let cmd = crate::args::ClapArgumentLoader::load()?;
    
    match cmd.command {
        Command::Subsystem(subsystem) => {
            crate::subsystem::driver::dispatch(subsystem).await
        },
        // ... other commands
    }
}
```

### Repository Pattern
```rust
#[async_trait::async_trait(?Send)]
pub trait MigrationRepository {
    async fn init_store(&self) -> Result<()>;
    async fn fetch_applied_ids(&self) -> Result<HashSet<String>>;
    async fn apply_migration(
        &self, 
        id: &str, 
        up_sql: &str, 
        down_sql: &str,
        comment: Option<&str>,
        pre: Option<&str>,
        timeout: Option<u64>,
        dry_run: bool,
        locked: bool
    ) -> Result<()>;
}
```

### Service Layer Pattern
```rust
impl<R: MigrationRepository> MigrationService<R> {
    pub async fn up(&self, path: &Path, timeout: Option<u64>, count: Option<usize>, yes: bool, dry_run: bool) -> Result<()> {
        let local = util::get_local_migrations(path)?;  // Sync file operation
        let applied = self.repo.fetch_applied_ids().await?;  // Async database operation
        
        for id in to_apply {
            let (up_sql, down_sql, meta) = util::read_migration_with_meta(migration_dir, &id)?;  // Sync
            self.repo.apply_migration(&id, &up_sql, &down_sql, 
                                     meta.comment.as_deref(), 
                                     previous.as_deref(), 
                                     timeout, dry_run, 
                                     meta.is_locked()).await?;  // Async
            previous = Some(id.clone());
        }
        
        Ok(())
    }
}
```

### Database Connection Pattern
```rust
impl PostgresRepo {
    pub async fn from_config(path: &Path, config: SubsystemPostgres, check_cli_version: bool) -> Result<Self> {
        let pool = build_pool_from_config(path, &config, check_cli_version).await?;
        Ok(Self { config, pool, path: path.to_path_buf() })
    }
}

async fn build_pool_from_config(path: &Path, config: &SubsystemPostgres, check_cli_version: bool) -> Result<Pool<Postgres>> {
    let uri = resolve_connection_string(&config.connection, path)?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&uri).await?;
    Ok(pool)
}
```

### Transaction Handling Pattern
```rust
async fn apply_migration(&self, id: &str, up_sql: &str, ...) -> Result<()> {
    let mut tx = self.pool.begin().await?;
    
    // Execute the migration
    execute_sql_statements(&mut tx, up_sql, id).await?;
    
    // Update migration tracking
    let query = sqlx::query(&format!(
        "INSERT INTO {}.{} (id, version, up, down, pre, comment, locked) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        self.config.schema, self.config.migrations_table()
    ));
    
    query.bind(id)
         .bind(env!("CARGO_PKG_VERSION"))
         .bind(up_sql)
         .bind(down_sql)
         .bind(pre)
         .bind(comment)
         .bind(locked)
         .execute(&mut *tx).await?;
    
    // Commit or rollback based on dry_run
    if dry_run {
        tx.rollback().await?;
        println!("🔄 Dry run completed - changes rolled back");
    } else {
        tx.commit().await?;
    }
    
    Ok(())
}
```

## Guidelines

1. **Async Propagation**: If a function calls async code, it MUST be async itself
2. **Non-Send Bounds**: Use `?Send` bounds for async traits when appropriate: `#[async_trait::async_trait(?Send)]`
3. **Resource Cleanup**: Use proper scoping and RAII patterns for resource cleanup
4. **Connection Pooling**: Always use connection pools for database access
5. **Transaction Scope**: Keep transaction scopes as narrow as possible
6. **Error Propagation**: Use `?` operator consistently with async functions

## Anti-Patterns

```rust
// Don't: Blocking in async context
async fn bad_example() -> Result<()> {
    let result = std::thread::sleep(Duration::from_secs(1)); // Blocks executor
    Ok(())
}

// Don't: Unnecessary async
async fn unnecessary_async() -> Result<String> {
    Ok("hello".to_string()) // No I/O, doesn't need to be async
}

// Don't: Forgetting .await
async fn forgot_await() -> Result<()> {
    some_async_function(); // Missing .await
    Ok(())
}
```

## Performance Considerations

1. **Connection Pooling**: Use appropriate pool sizes (typically 5-10 connections for CLI tools)
2. **Transaction Batching**: Batch related operations in single transactions when possible
3. **Resource Limits**: Set appropriate timeouts and connection limits
4. **Memory Usage**: Be mindful of large result sets and streaming when appropriate

## Backend-specific guidance

### PostgreSQL
- Pool: `PgPoolOptions::new().max_connections(10)`
- Timeout: `SET LOCAL statement_timeout = <ms>` applied per migration transaction
- Execution: `sqlx::raw_sql(sql)` inside transaction

### SQLite
- Pool: `SqlitePoolOptions::new().max_connections(1)`
- Timeout: `PRAGMA busy_timeout = <ms>` applied per migration transaction
- Execution: `sqlx::raw_sql(sql)` inside transaction

## References

- [Tokio Documentation](https://tokio.rs/)
- [async-trait crate](https://docs.rs/async-trait/)
- [SQLx Documentation](https://docs.rs/sqlx/)
- [Rust Async Book](https://rust-lang.github.io/async-book/)
