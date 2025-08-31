# ADR-0007: Repository-Service Architecture Pattern

## Status

Accepted

## Date

2025-01-27T22:55:00Z

## Context

The `qop` migration tool has complex business logic around migration operations (applying migrations, handling rollbacks, validation, user interaction) that needs to be separated from database-specific implementation details. The application supports multiple database backends with different connection patterns, SQL dialects, and operational characteristics.

Key concerns that need architectural separation:
1. **Business Logic**: Migration sequencing, validation, user prompts, dry-run functionality
2. **Data Access**: Database connections, SQL execution, transaction management
3. **Testability**: Business logic should be testable without database dependencies
4. **Maintainability**: Changes to business logic shouldn't require changes to database layers
5. **Extensibility**: New database backends shouldn't require changes to business logic

The Repository pattern provides data access abstraction, while the Service pattern encapsulates business logic and coordinates between different layers.

## Decision

The codebase MUST implement a clear Repository-Service architecture where business logic is encapsulated in generic services that operate on repository abstractions, with database-specific implementations providing the concrete repository behavior.

### Architecture Layers

1. **Service Layer** (`core::service`): Contains business logic, operates on repository abstractions
2. **Repository Layer** (`core::repo`): Defines data access contracts via traits  
3. **Repository Implementation Layer** (`subsystem::<backend>::repo`): Database-specific implementations
4. **Coordination Layer** (`subsystem::driver`): Wires together services and repositories

### Separation of Concerns

- **Services**: Business logic, validation, user interaction, migration sequencing
- **Repositories**: Data persistence, database transactions, SQL execution
- **Implementations**: Database-specific connection handling, query building, error mapping

## Consequences

### Positive

- **Testability**: Business logic can be tested with mock repositories
- **Maintainability**: Clear separation between business logic and data access
- **Extensibility**: New database backends only require new repository implementations  
- **Reusability**: Business logic is shared across all database backends
- **Single Responsibility**: Each layer has a focused, well-defined responsibility
- **Dependency Inversion**: High-level modules don't depend on low-level database details

### Negative

- **Abstraction Overhead**: Additional layers may impact performance
- **Indirection Complexity**: More layers to understand and navigate
- **Interface Evolution**: Changes to repository contracts affect all implementations
- **Boilerplate**: Each backend requires repository implementation code

## Implementation

### Repository Trait Definition

```rust
#[async_trait::async_trait(?Send)]
pub trait MigrationRepository {
    // Core operations
    async fn init_store(&self) -> Result<()>;
    async fn fetch_applied_ids(&self) -> Result<HashSet<String>>;
    async fn fetch_last_id(&self) -> Result<Option<String>>;
    
    // Migration operations
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
    
    async fn revert_migration(
        &self, 
        id: &str, 
        down_sql: &str, 
        timeout: Option<u64>, 
        dry_run: bool, 
        unlock: bool
    ) -> Result<()>;
    
    // History and metadata
    async fn fetch_history(&self) -> Result<Vec<(String, NaiveDateTime, Option<String>, bool)>>;
    async fn fetch_down_sql(&self, id: &str) -> Result<Option<String>>;
    async fn fetch_all_migrations(&self) -> Result<Vec<(String, String, String, Option<String>)>>;
    
    // Utility
    fn get_path(&self) -> &Path;
}
```

### Service Layer Implementation

```rust
pub struct MigrationService<R: MigrationRepository> {
    repo: R,
}

impl<R: MigrationRepository> MigrationService<R> {
    pub fn new(repo: R) -> Self { 
        Self { repo } 
    }

    /// Apply pending migrations with business logic validation
    pub async fn up(&self, path: &Path, timeout: Option<u64>, count: Option<usize>, yes: bool, dry_run: bool) -> Result<()> {
        // 1. Gather data from filesystem and database
        let local = util::get_local_migrations(path)?;
        let applied = self.repo.fetch_applied_ids().await?;

        // 2. Apply business logic: determine what to apply
        let mut to_apply: Vec<String> = local.difference(&applied).cloned().collect();
        to_apply.sort();
        if let Some(c) = count { to_apply.truncate(c); }

        if to_apply.is_empty() {
            println!("All migrations are up to date.");
            return Ok(())
        }

        // 3. Business validation: check for non-linear migration history
        let out_of_order = util::check_non_linear_history(&applied, &to_apply);
        if !out_of_order.is_empty() {
            let max_applied = applied.iter().max().cloned().unwrap_or_default();
            if !util::handle_non_linear_warning(&out_of_order, &max_applied)? { 
                println!("Operation cancelled.");
                return Ok(())
            }
        }

        // 4. User interaction and confirmation
        println!("\n📋 About to apply {} migration(s):", to_apply.len());
        for id in &to_apply { println!("  - {}", id); }
        
        let migration_dir = path.parent().ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
        let diff_fn = /* ... preview function ... */;
        if !util::prompt_for_confirmation_with_diff("❓ Do you want to proceed?", yes, diff_fn)? {
            println!("❌ Migration cancelled.");
            return Ok(())
        }

        // 5. Execute via repository: business logic coordinates, repository executes
        let mut previous: Option<String> = self.repo.fetch_last_id().await?;
        let mut applied_count = 0usize;
        for id in to_apply {
            let (up_sql, down_sql, meta) = util::read_migration_with_meta(migration_dir, &id)?;
            self.repo.apply_migration(&id, &up_sql, &down_sql, 
                                     meta.comment.as_deref(), 
                                     previous.as_deref(), 
                                     timeout, dry_run, 
                                     meta.is_locked()).await?;
            previous = Some(id.clone());
            applied_count += 1;
        }

        util::print_migration_results(applied_count, "applied");
        Ok(())
    }
}
```

### Repository Implementation Pattern

```rust
pub struct PostgresRepo {
    pub config: SubsystemPostgres,
    pub pool: Pool<Postgres>,
    pub path: PathBuf,
}

#[async_trait::async_trait(?Send)]
impl MigrationRepository for PostgresRepo {
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
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        
        // Database-specific transaction and SQL execution
        if let Some(timeout) = timeout {
            sqlx::query(&format!("SET statement_timeout = '{}'s", timeout))
                .execute(&mut *tx).await?;
        }
        
        // Execute migration SQL
        execute_sql_statements(&mut tx, up_sql, id).await?;
        
        // Update migration tracking table with database-specific SQL
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
        
        // Handle dry run vs real execution
        if dry_run {
            tx.rollback().await?;
            println!("🔄 Dry run completed - changes rolled back");
        } else {
            tx.commit().await?;
        }
        
        Ok(())
    }
}
```

### Service-Repository Coordination

```rust
// In subsystem::driver::dispatch
match command {
    Command::Up { timeout, count, dry, yes, .. } => {
        // 1. Create database-specific repository
        let repo = PostgresRepo::from_config(&path, config.clone(), true).await?;
        
        // 2. Wrap in generic service (business logic)
        let svc = MigrationService::new(repo);
        
        // 3. Execute business operation
        svc.up(&path, timeout, count, yes, dry).await
    }
    // ... other commands follow same pattern
}
```

## Design Guidelines

### Service Layer Responsibilities
- Migration sequencing and validation logic
- User interaction and confirmation prompts
- File system operations (reading migration files)
- Business rule enforcement (migration locking, non-linear history)
- Output formatting and user feedback

### Repository Layer Responsibilities  
- Database connection management
- SQL execution and transaction handling
- Database-specific query building
- Error mapping from database errors to application errors
- Database schema management (migrations table, logging table)

### Separation Principles

1. **Services are database-agnostic**: No SQL or database-specific logic in services
2. **Repositories are business-agnostic**: No validation or user interaction in repositories
3. **Clear interface contracts**: Repository traits define exact contracts between layers
4. **Error boundaries**: Each layer handles its own error types and context
5. **Resource management**: Repositories own database resources, services coordinate operations

### Testing Strategies

```rust
// Service layer can be tested with mock repositories
struct MockRepository {
    applied_ids: HashSet<String>,
    // ... other mock state
}

#[async_trait::async_trait(?Send)]
impl MigrationRepository for MockRepository {
    async fn fetch_applied_ids(&self) -> Result<HashSet<String>> {
        Ok(self.applied_ids.clone())
    }
    // ... implement other methods
}

#[tokio::test]
async fn test_migration_service_logic() {
    let mock_repo = MockRepository::new();
    let service = MigrationService::new(mock_repo);
    
    // Test business logic without database dependencies
    let result = service.up(path, None, None, true, false).await;
    assert!(result.is_ok());
}
```

## Anti-Patterns

```rust
// Don't: Business logic in repository implementations
impl MigrationRepository for PostgresRepo {
    async fn apply_migration(&self, ...) -> Result<()> {
        // BAD: User interaction in repository
        if !confirm("Apply migration?") { return Ok(()); }
        
        // BAD: Business validation in repository
        if self.check_migration_conflicts() { ... }
    }
}

// Don't: Database-specific logic in services
impl<R: MigrationRepository> MigrationService<R> {
    async fn up(&self, ...) -> Result<()> {
        // BAD: SQL construction in service
        let query = format!("INSERT INTO {}.migrations ...", schema);
        
        // BAD: Database-specific error handling
        match postgres_error.code() { ... }
    }
}
```

## References

- [Repository Pattern](https://martinfowler.com/eaaCatalog/repository.html)
- [Service Layer Pattern](https://martinfowler.com/eaaCatalog/serviceLayer.html)
- [Domain-Driven Design: Repository](https://www.domainlanguage.com/wp-content/uploads/2016/05/DDD_Reference_2015-03.pdf)
