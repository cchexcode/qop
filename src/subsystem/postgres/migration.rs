use {
    crate::config::{SubsystemPostgres, Config, DataSource, WithVersion},
    anyhow::{Context, Result},
    chrono::NaiveDateTime,
    pep440_rs::Version,
    sqlparser::{dialect::PostgreSqlDialect, parser::Parser},
    sqlx::{postgres::PgRow, Pool, Postgres, QueryBuilder, Row},
    std::{
        collections::{HashMap, HashSet},
        path::Path,
        str::FromStr,
    },
};

// Database utility functions
pub(crate) fn get_effective_timeout(config: &SubsystemPostgres, provided_timeout: Option<u64>) -> Option<u64> {
    provided_timeout.or(config.timeout)
}

pub(crate) fn build_table_query<'a>(base_sql: &'a str, schema: &str, table: &str) -> QueryBuilder<'a, Postgres> {
    let mut query = QueryBuilder::new(base_sql);
    query.push(format!("{}.{}", schema, table));
    query
}

pub(crate) async fn set_timeout_if_needed<'e, E>(executor: E, timeout_seconds: Option<u64>) -> Result<()>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    if let Some(seconds) = timeout_seconds {
        sqlx::query(&format!("SET LOCAL statement_timeout = '{}s'", seconds))
            .execute(executor)
            .await?;
    }
    Ok(())
}

pub(crate) async fn get_applied_migrations(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    schema: &str,
    table: &str,
) -> Result<HashSet<String>> {
    let mut query = build_table_query("SELECT id FROM ", schema, table);
    query.push(" ORDER BY id ASC");
    Ok(query.build()
        .fetch_all(&mut **tx)
        .await?
        .into_iter()
        .map(|row| row.get("id"))
        .collect())
}

pub(crate) async fn get_last_migration_id(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    schema: &str,
    table: &str,
) -> Result<Option<String>> {
    let mut query = build_table_query("SELECT id FROM ", schema, table);
    query.push(" ORDER BY id DESC LIMIT 1");
    Ok(query.build()
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| row.get("id")))
}

pub(crate) async fn insert_migration_record<'e, E>(
    executor: E,
    schema: &str,
    table: &str,
    id: &str,
    up_sql: &str,
    down_sql: &str,
    pre_migration_id: Option<&str>,
) -> Result<()>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    let mut query = build_table_query("INSERT INTO ", schema, table);
    query.push(" (id, version, up, down, pre) VALUES ($1, $2, $3, $4, $5)");
    query.build()
        .bind(id)
        .bind(env!("CARGO_PKG_VERSION"))
        .bind(up_sql)
        .bind(down_sql)
        .bind(pre_migration_id)
        .execute(executor)
        .await?;
    Ok(())
}

pub(crate) async fn delete_migration_record<'e, E>(
    executor: E,
    schema: &str,
    table: &str,
    id: &str,
) -> Result<()>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    let mut query = build_table_query("DELETE FROM ", schema, table);
    query.push(" WHERE id = $1");
    query.build().bind(id).execute(executor).await?;
    Ok(())
}

pub(crate) async fn get_migration_history(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    schema: &str,
    table: &str,
) -> Result<HashMap<String, NaiveDateTime>> {
    let mut query = build_table_query("SELECT id, created_at FROM ", schema, table);
    query.push(" ORDER BY id ASC");
    Ok(query.build()
        .fetch_all(&mut **tx)
        .await?
        .into_iter()
        .map(|row| (row.get("id"), row.get("created_at")))
        .collect())
}

pub(crate) async fn get_all_migration_data(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    schema: &str,
    table: &str,
) -> Result<Vec<PgRow>> {
    let mut query = build_table_query("SELECT id, up, down FROM ", schema, table);
    query.push(" ORDER BY id ASC");
    Ok(query.build().fetch_all(&mut **tx).await?)
}

pub(crate) fn normalize_migration_id(id: &str) -> String {
    if id.starts_with("id=") {
        id.to_string()
    } else {
        format!("id={}", id)
    }
}

pub(crate) async fn get_recent_migrations_for_revert(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    schema: &str,
    table: &str,
) -> Result<Vec<PgRow>> {
    let mut query = build_table_query("SELECT id, down FROM ", schema, table);
    query.push(" ORDER BY id DESC");
    Ok(query.build().fetch_all(&mut **tx).await?)
}

pub(crate) async fn get_migration_down_sql(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    schema: &str,
    table: &str,
    migration_id: &str,
) -> Result<String> {
    let mut query = build_table_query("SELECT down FROM ", schema, table);
    query.push(" WHERE id = $1");
    let row = query.build().bind(migration_id).fetch_one(&mut **tx).await?;
    Ok(row.get("down"))
}

pub(crate) async fn get_table_version(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    table: &str,
) -> Result<Option<String>> {
    let mut query = QueryBuilder::new("SELECT version FROM ");
    query.push(table);
    query.push(" ORDER BY id DESC LIMIT 1");
    Ok(query.build()
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| row.get("version")))
}

pub(crate) fn split_sql_statements(sql: &str) -> Result<Vec<String>> {
    let dialect = PostgreSqlDialect {};
    
    // Parse the SQL and expect it to succeed
    let statements = Parser::parse_sql(&dialect, sql)
        .with_context(|| "Failed to parse SQL migration - please check your SQL syntax")?;
    
    // Reconstruct each statement as a string
    let mut result = Vec::new();
    for statement in statements {
        let statement_str = format!("{};", statement);
        result.push(statement_str);
    }
    
    Ok(result)
}

pub(crate) async fn execute_sql_statements(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    sql: &str,
    migration_id: &str,
) -> Result<()> {
    let statements = split_sql_statements(sql)?;
    
    if statements.is_empty() {
        return Ok(());
    }

    for (i, statement) in statements.iter().enumerate() {
        match sqlx::query(statement).execute(&mut **tx).await {
            Ok(_) => {
                // Statement executed successfully
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to execute statement {} in migration {}: {}\nStatement: {}",
                    i + 1,
                    migration_id,
                    e,
                    statement
                ));
            }
        }
    }
    Ok(())
}

pub(crate) async fn get_db_assets(path: &Path, check_cli_version: bool) -> Result<(SubsystemPostgres, Pool<Postgres>)> {
    use {sqlx::postgres::PgPoolOptions, crate::config::Subsystem};
    
    let config_content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file at: {}", path.display()))?;

    let with_version: WithVersion = toml::from_str(&config_content)?;
    with_version.validate(env!("CARGO_PKG_VERSION"))?;

    let config: Config = toml::from_str(&config_content)
        .with_context(|| format!("Failed to parse config file at: {}", path.display()))?;

    let subsystem_config = match config.subsystem {
        | Subsystem::Postgres(postgres_config) => postgres_config,
        | Subsystem::Sqlite(_) => {
            anyhow::bail!("Expected PostgreSQL configuration, found SQLite configuration");
        },
    };

    let uri = match &subsystem_config.connection {
        | DataSource::Static(connection) => {
            connection.to_owned()
        },
        | DataSource::FromEnv(var) => {
            let v = std::env::var(var).unwrap();
            v.to_owned()
        },
    };
    
    let pool = PgPoolOptions::new().max_connections(10).connect(&uri).await?;
    let mut tx = pool.begin().await?;

    if check_cli_version {
        let last_migration_version = get_table_version(&mut tx, &subsystem_config.table).await?;
        match last_migration_version {
            | Some(version) => {
                let cli_version = Version::from_str(env!("CARGO_PKG_VERSION"))?;
                if cli_version.release() != &[0, 0, 0] {
                    let last_migration_version = Version::from_str(&version)?;
                    if last_migration_version > cli_version {
                        anyhow::bail!("Latest migration table version is older than the CLI version. Please run 'qop subsystem postgres history fix' to rename out-of-order migrations.");
                    }
                }
            },
            | None => (),
        };
    }

    tx.commit().await?;

    Ok((subsystem_config, pool))
}

pub(crate) fn get_local_migrations(path: &Path) -> Result<HashSet<String>> {
    let migration_dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
    Ok(std::fs::read_dir(migration_dir)
        .with_context(|| format!("Failed to read migration directory: {}", migration_dir.display()))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().ok()?.is_dir()
                && entry.file_name().to_string_lossy().starts_with("id=")
            {
                Some(entry.file_name().to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .collect())
}

// High-level command functions
pub async fn init(path: &Path) -> Result<()> {
    let (config, pool) = get_db_assets(path, false).await?;
    let schema = &config.schema;
    let table = &config.table;
    
    let mut tx = pool.begin().await?;
    {
        let mut query = build_table_query("CREATE TABLE IF NOT EXISTS ", &schema, &table);
        query.push(" (id VARCHAR PRIMARY KEY, version VARCHAR NOT NULL, up VARCHAR NOT NULL, down VARCHAR NOT NULL, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, pre VARCHAR)");
        query.build().execute(&mut *tx).await?
    };
    tx.commit().await?;
    println!("Initialized migration table.");
    Ok(())
}

pub async fn up(path: &Path, timeout: Option<u64>, count: Option<usize>, diff: bool, dry: bool) -> Result<()> {
    use {
        crate::migration_diff::{display_migration_diff, parse_migration_operations},
        std::io::{self, Write},
    };
    
    let (config, pool) = get_db_assets(path, true).await?;
    let migration_dir = path.parent().ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
    let local_migrations = get_local_migrations(path)?;
    let effective_timeout = get_effective_timeout(&config, timeout);
    let schema = &config.schema;
    let table = &config.table;

    let mut tx = pool.begin().await?;

    set_timeout_if_needed(&mut *tx, effective_timeout).await?;

    let applied_migrations = get_applied_migrations(&mut tx, &schema, &table).await?;
    let mut last_migration_id = get_last_migration_id(&mut tx, &schema, &table).await?;

    // Commit the initial query transaction
    tx.commit().await?;

    let mut migrations_to_apply: Vec<String> =
        local_migrations.difference(&applied_migrations).cloned().collect();

    migrations_to_apply.sort();

    let migrations_to_apply = if let Some(count) = count {
        migrations_to_apply.into_iter().take(count).collect()
    } else {
        migrations_to_apply
    };

    // Linear history enforcement: Check for out-of-order migrations
    if !applied_migrations.is_empty() && !migrations_to_apply.is_empty() {
        let max_applied_migration = applied_migrations.iter().max().cloned().unwrap_or_default();
        
        let out_of_order_migrations: Vec<&String> = migrations_to_apply
            .iter()
            .filter(|id| id.as_str() < max_applied_migration.as_str())
            .collect();

        if !out_of_order_migrations.is_empty() {
            println!("⚠️  Non-linear history detected!");
            println!("The following migrations would create a non-linear history:");
            for migration in &out_of_order_migrations {
                println!("  - {}", migration);
            }
            println!("Latest applied migration: {}", max_applied_migration);
            println!();
            println!("This could cause issues with database schema consistency.");
            println!("Alternatively, you can run 'qop migration history fix' to rename out-of-order migrations.");
            
            print!("Do you want to continue? [y/N]: ");
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();
            
            if input != "y" && input != "yes" {
                println!("Operation cancelled.");
                return Ok(());
            }
        }
    }

    if migrations_to_apply.is_empty() {
        println!("All migrations are up to date.");
    } else {
        // Show diff preview if --diff flag is specified
        if diff {
            let mut all_operations = Vec::new();
            
            println!("\n🔍 Analyzing {} migration(s) to be applied...", migrations_to_apply.len());
            
            for migration_id in &migrations_to_apply {
                let migration_path = migration_dir.join(migration_id);
                let up_sql_path = migration_path.join("up.sql");
                
                let up_sql = std::fs::read_to_string(&up_sql_path).with_context(
                    || format!("Failed to read up migration: {}", up_sql_path.display()),
                )?;
                
                match parse_migration_operations(&up_sql) {
                    Ok(operations) => {
                        display_migration_diff(migration_id, &operations);
                        all_operations.extend(operations);
                    }
                    Err(e) => {
                        println!("\n⚠️  Could not parse migration {}: {}", migration_id, e);
                        println!("📄 Raw SQL content will be executed as-is.");
                    }
                }
            }
            
            println!("\n📊 Migration Summary:");
            println!("  • {} migration(s) to apply", migrations_to_apply.len());
            println!("  • {} total operation(s) detected", all_operations.len());
            
            // Ask for confirmation when showing diff
            print!("\n❓ Do you want to apply these migrations? [y/N]: ");
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();
            
            if input != "y" && input != "yes" {
                println!("❌ Migration cancelled.");
                return Ok(());
            }
            
            if dry {
                println!("\n🧪 Running migrations in dry-run mode...");
            } else {
                println!("\n🚀 Applying migrations...");
            }
        } else if dry {
            println!("\n🧪 Running migrations in dry-run mode...");
        }
        
        // Apply each migration in its own transaction
        for migration_id in &migrations_to_apply {
            let migration_path = migration_dir.join(migration_id);
            if dry {
                println!("⏳ Testing migration: {}", migration_id);
            } else {
                println!("⏳ Applying migration: {}", migration_id);
            }
            let id = migration_id.as_str();

            let up_sql_path = migration_path.join("up.sql");
            let down_sql_path = migration_path.join("down.sql");

            let up_sql = std::fs::read_to_string(&up_sql_path).with_context(
                || format!("Failed to read up migration: {}", up_sql_path.display()),
            )?;
            let down_sql = std::fs::read_to_string(&down_sql_path).with_context(
                || {
                    format!(
                        "Failed to read down migration: {}",
                        down_sql_path.display()
                    )
                },
            )?;

            // Start a new transaction for this migration
            let mut migration_tx = pool.begin().await?;

            // Set timeout for this transaction if specified
            set_timeout_if_needed(&mut *migration_tx, effective_timeout).await?;

            // Execute the migration SQL
            execute_sql_statements(&mut migration_tx, &up_sql, id).await?;

            // Record the migration in the tracking table
            insert_migration_record(
                &mut *migration_tx,
                &schema,
                &table,
                id,
                &up_sql,
                &down_sql,
                last_migration_id.as_deref(),
            ).await?;

            // Commit or rollback based on dry-run mode
            if dry {
                migration_tx.rollback().await?;
                println!("🔄 Migration {} executed and rolled back (dry-run mode).", migration_id);
            } else {
                migration_tx.commit().await?;
                println!("✅ Migration {} applied successfully.", migration_id);
            }
            if !dry {
                last_migration_id = Some(id.to_string());
            }
        }

        if dry {
            println!("\n🎉 Successfully executed {} migration(s) in dry-run mode! (No changes were committed)", migrations_to_apply.len());
        } else {
            println!("\n🎉 Successfully applied {} migration(s)!", migrations_to_apply.len());
        }
    }

    Ok(())
}

pub async fn down(path: &Path, timeout: Option<u64>, count: Option<usize>, remote: bool, diff: bool, dry: bool) -> Result<()> {
    use {
        crate::migration_diff::{display_migration_diff, parse_migration_operations},
        std::io::{self, Write},
    };
    
    let (config, pool) = get_db_assets(path, true).await?;
    let migration_dir = path.parent().ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
    let effective_timeout = get_effective_timeout(&config, timeout);
    let schema = &config.schema;
    let table = &config.table;
    
    let mut tx = pool.begin().await?;

    set_timeout_if_needed(&mut *tx, effective_timeout).await?;

    let last_migrations = get_recent_migrations_for_revert(&mut tx, &schema, &table).await?;

    let migrations_to_revert: Vec<PgRow> = if let Some(count) = count {
        last_migrations.into_iter().take(count).collect()
    } else {
        last_migrations.into_iter().take(1).collect()
    };

    // Commit the initial query transaction
    tx.commit().await?;

    if migrations_to_revert.is_empty() {
        println!("No migrations to revert.");
    } else {
        // Show diff preview if --diff flag is specified
        if diff {
            println!("\n🔍 Analyzing {} migration(s) to be reverted...", migrations_to_revert.len());
            
            for row in &migrations_to_revert {
                let id: String = row.get("id");
                let down_sql: String = if remote {
                    row.get("down")
                } else {
                    let down_sql_path = migration_dir.join(&id).join("down.sql");
                    std::fs::read_to_string(&down_sql_path).with_context(|| {
                        format!(
                            "Failed to read down migration: {}",
                            down_sql_path.display()
                        )
                    })?
                };
                
                match parse_migration_operations(&down_sql) {
                    Ok(operations) => {
                        println!("\n📉 Migration {} (REVERT):", id);
                        display_migration_diff(&id, &operations);
                    }
                    Err(e) => {
                        println!("\n⚠️  Could not parse migration {}: {}", id, e);
                        println!("📄 Raw SQL content will be executed as-is.");
                    }
                }
            }
            
            println!("\n📊 Revert Summary:");
            println!("  • {} migration(s) to revert", migrations_to_revert.len());
            
            // Ask for confirmation when showing diff
            print!("\n❓ Do you want to revert these migrations? [y/N]: ");
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();
            
            if input != "y" && input != "yes" {
                println!("❌ Revert cancelled.");
                return Ok(());
            }
            
            println!("\n🔄 Reverting migrations...");
        }
        
        // Revert each migration in its own transaction
        for row in migrations_to_revert {
            let id: String = row.get("id");
            let down_sql: String = if remote {
                row.get("down")
            } else {
                let down_sql_path = migration_dir.join(&id).join("down.sql");
                std::fs::read_to_string(&down_sql_path).with_context(|| {
                    format!(
                        "Failed to read down migration: {}",
                        down_sql_path.display()
                    )
                })?
            };
            println!("Reverting migration: {}", id);

            // Start a new transaction for this migration revert
            let mut revert_tx = pool.begin().await?;

            // Set timeout for this transaction if specified
            set_timeout_if_needed(&mut *revert_tx, effective_timeout).await?;

            // Execute the down migration SQL
            execute_sql_statements(&mut revert_tx, &down_sql, &id).await?;

            // Remove the migration from the tracking table
            delete_migration_record(&mut *revert_tx, &schema, &table, &id).await?;

            // Commit or rollback based on dry-run mode
            if dry {
                revert_tx.rollback().await?;
                println!("🔄 Migration {} reverted and rolled back (dry-run mode).", id);
            } else {
                revert_tx.commit().await?;
                println!("✅ Migration {} reverted.", id);
            }
        }
    }

    Ok(())
}

pub async fn new_migration(path: &Path) -> Result<()> {
    use chrono::Utc;
    
    let id = Utc::now().timestamp_millis().to_string();
    let migration_path = path.parent().unwrap();
    let migration_id_path = migration_path.join(format!("id={}", id));
    std::fs::create_dir_all(&migration_id_path).with_context(|| {
        format!("Failed to create directory: {}", migration_id_path.display())
    })?;
    let up_path = migration_id_path.join("up.sql");
    let down_path = migration_id_path.join("down.sql");
    std::fs::write(&up_path, "-- SQL goes here").with_context(|| {
        format!("Failed to write up migration: {}", up_path.display())
    })?;
    std::fs::write(&down_path, "-- SQL goes here").with_context(|| {
        format!("Failed to write down migration: {}", down_path.display())
    })?;
    Ok(())
}

pub async fn apply_up(path: &Path, id: &str, timeout: Option<u64>, dry: bool) -> Result<()> {
    use std::io::{self, Write};
    
    let (config, pool) = get_db_assets(path, true).await?;
    let effective_timeout = get_effective_timeout(&config, timeout);
    let migration_dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
    let local_migrations = get_local_migrations(path)?;
    let schema = &config.schema;
    let table = &config.table;

    // Normalize the migration ID to include "id=" prefix if not present
    let target_migration_id = normalize_migration_id(&id);

    let mut tx = pool.begin().await?;

    // Get current applied migrations
    let applied_migrations = get_applied_migrations(&mut tx, &schema, &table).await?;

    tx.commit().await?;

    // Check if migration exists locally
    if !local_migrations.contains(&target_migration_id) {
        return Err(anyhow::anyhow!(
            "Migration {} does not exist locally",
            target_migration_id
        ));
    }

    // Check if migration is already applied
    if applied_migrations.contains(&target_migration_id) {
        println!("Migration {} is already applied.", target_migration_id);
        return Ok(());
    }

    // Check for non-linear history
    let mut needs_confirmation = false;
    if !applied_migrations.is_empty() {
        let max_applied_migration =
            applied_migrations.iter().max().cloned().unwrap_or_default();

        if target_migration_id.as_str() < max_applied_migration.as_str() {
            println!("⚠️  Non-linear history detected!");
            println!(
                "Applying migration {} would create a non-linear history.",
                target_migration_id
            );
            println!(
                "Latest applied migration: {}",
                max_applied_migration
            );
            println!();
            println!("This could cause issues with database schema consistency.");
            needs_confirmation = true;
        }
    }

    if needs_confirmation {
        print!("Do you want to continue? [y/N]: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        
        if input != "y" && input != "yes" {
            println!("Operation cancelled.");
            return Ok(());
        }
    }

    // Apply the migration
    let migration_path = migration_dir.join(&target_migration_id);
    let up_sql_path = migration_path.join("up.sql");
    let down_sql_path = migration_path.join("down.sql");

    let up_sql = std::fs::read_to_string(&up_sql_path).with_context(
        || format!("Failed to read up migration: {}", up_sql_path.display()),
    )?;
    let down_sql = std::fs::read_to_string(&down_sql_path).with_context(
        || {
            format!(
                "Failed to read down migration: {}",
                down_sql_path.display()
            )
        },
    )?;

    // Get the latest migration for the pre field
    let mut tx = pool.begin().await?;
    let last_migration_id = get_last_migration_id(&mut tx, &schema, &table).await?;
    tx.commit().await?;

    // Execute the migration
    let mut migration_tx = pool.begin().await?;

    set_timeout_if_needed(&mut *migration_tx, effective_timeout).await?;

    if dry {
        println!("Testing migration: {}", target_migration_id);
    } else {
        println!("Applying migration: {}", target_migration_id);
    }
    
    execute_sql_statements(&mut migration_tx, &up_sql, &target_migration_id).await?;

    insert_migration_record(
        &mut *migration_tx,
        &schema,
        &table,
        &target_migration_id,
        &up_sql,
        &down_sql,
        last_migration_id.as_deref(),
    ).await?;

    if dry {
        migration_tx.rollback().await?;
        println!("🔄 Migration {} executed and rolled back (dry-run mode).", target_migration_id);
    } else {
        migration_tx.commit().await?;
        println!("✅ Migration {} applied successfully.", target_migration_id);
    }

    Ok(())
}

pub async fn apply_down(path: &Path, id: &str, timeout: Option<u64>, remote: bool, dry: bool) -> Result<()> {
    use std::io::{self, Write};
    
    let (config, pool) = get_db_assets(path, true).await?;
    let effective_timeout = get_effective_timeout(&config, timeout);
    let migration_dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
    let schema = &config.schema;
    let table = &config.table;

    // Normalize the migration ID to include "id=" prefix if not present
    let target_migration_id = normalize_migration_id(&id);

    let mut tx = pool.begin().await?;

    // Get current applied migrations
    let applied_migrations = get_applied_migrations(&mut tx, &schema, &table).await?;

    tx.commit().await?;

    // Check if migration is applied
    if !applied_migrations.contains(&target_migration_id) {
        return Err(anyhow::anyhow!(
            "Migration {} is not currently applied",
            target_migration_id
        ));
    }

    // Check for non-linear history (reverting a migration that's not the latest)
    let mut needs_confirmation = false;
    if !applied_migrations.is_empty() {
        let max_applied_migration =
            applied_migrations.iter().max().cloned().unwrap_or_default();

        if target_migration_id != max_applied_migration {
            println!("⚠️  Non-linear history detected!");
            println!(
                "Reverting migration {} would create a non-linear history.",
                target_migration_id
            );
            println!(
                "Latest applied migration: {}",
                max_applied_migration
            );
            println!();
            println!("This could cause issues with database schema consistency.");
            needs_confirmation = true;
        }
    }

    if needs_confirmation {
        print!("Do you want to continue? [y/N]: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        
        if input != "y" && input != "yes" {
            println!("Operation cancelled.");
            return Ok(());
        }
    }

    // Get the down SQL from database or local file based on remote flag
    let down_sql: String = if remote {
        let mut tx = pool.begin().await?;
        let sql = get_migration_down_sql(&mut tx, &schema, &table, &target_migration_id).await?;
        tx.commit().await?;
        sql
    } else {
        let down_sql_path = migration_dir.join(&target_migration_id).join("down.sql");
        std::fs::read_to_string(&down_sql_path).with_context(|| {
            format!(
                "Failed to read down migration: {}",
                down_sql_path.display()
            )
        })?
    };

    // Execute the down migration
    let mut revert_tx = pool.begin().await?;

    set_timeout_if_needed(&mut *revert_tx, effective_timeout).await?;

    if dry {
        println!("Testing revert migration: {}", target_migration_id);
    } else {
        println!("Reverting migration: {}", target_migration_id);
    }
    
    execute_sql_statements(&mut revert_tx, &down_sql, &target_migration_id).await?;

    delete_migration_record(&mut *revert_tx, &schema, &table, &target_migration_id).await?;

    if dry {
        revert_tx.rollback().await?;
        println!("🔄 Migration {} reverted and rolled back (dry-run mode).", target_migration_id);
    } else {
        revert_tx.commit().await?;
        println!("✅ Migration {} reverted successfully.", target_migration_id);
    }

    Ok(())
}

pub async fn list(path: &Path) -> Result<()> {
    use {
        chrono::{Local, TimeZone, Utc},
        comfy_table::{
            modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, ContentArrangement, Table,
        },
        std::collections::BTreeMap,
    };
    
    let (config, pool) = get_db_assets(path, true).await?;
    let local_migrations = get_local_migrations(path)?;
    let schema = &config.schema;
    let table = &config.table;

    let mut tx = pool.begin().await?;

    let applied_migrations = get_migration_history(&mut tx, &schema, &table).await?;

    let mut all_migrations: BTreeMap<String, (Option<NaiveDateTime>, bool)> = BTreeMap::new();

    for id in &local_migrations {
        let entry = all_migrations.entry(id.clone()).or_default();
        entry.1 = true;
    }

    for (id, timestamp) in &applied_migrations {
        let entry = all_migrations.entry(id.clone()).or_default();
        entry.0 = Some(*timestamp);
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("ID"),
            Cell::new("Remote"),
            Cell::new("Local"),
        ]);

    if all_migrations.is_empty() {
        println!("No migrations found.");
    } else {
        for (id, (applied_at, is_local)) in all_migrations {
            let applied_str = if let Some(timestamp) = applied_at {
                // Convert naive datetime (assumed UTC) to local timezone
                let utc_datetime = Utc.from_utc_datetime(&timestamp);
                let local_datetime = utc_datetime.with_timezone(&Local);
                local_datetime.format("%Y-%m-%d %H:%M:%S %Z").to_string()
            } else {
                "❌".to_string()
            };
            let local_str = if is_local { "✅" } else { "❌" };
            table.add_row(vec![
                Cell::new(id),
                Cell::new(applied_str).set_alignment(comfy_table::CellAlignment::Center),
                Cell::new(local_str).set_alignment(comfy_table::CellAlignment::Center),
            ]);
        }
        println!("{table}");
    }

    tx.commit().await?;

    Ok(())
}

pub async fn history_fix(path: &Path) -> Result<()> {
    use chrono::Utc;
    
    let (config, pool) = get_db_assets(path, true).await?;
    let migration_dir = path.parent().ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
    let local_migrations = get_local_migrations(path)?;
    let schema = &config.schema;
    let table = &config.table;

    let mut tx = pool.begin().await?;

    let applied_migrations = get_applied_migrations(&mut tx, &schema, &table).await?;

    let max_applied_migration = applied_migrations.iter().max().cloned().unwrap_or_default();

    let max_applied_ts = applied_migrations
        .iter()
        .filter_map(|id| id.strip_prefix("id=").and_then(|s| s.parse::<i64>().ok()))
        .max()
        .unwrap_or(0);

    let mut next_ts = std::cmp::max(max_applied_ts, Utc::now().timestamp_millis());

    let out_of_order_migrations: Vec<String> = local_migrations
        .difference(&applied_migrations)
        .filter(|id| id.as_str() < max_applied_migration.as_str())
        .cloned()
        .collect();

    if out_of_order_migrations.is_empty() {
        println!("No out-of-order migrations to fix.");
    } else {
        for old_id in out_of_order_migrations {
            next_ts += 1;
            let new_id = format!("id={}", next_ts);
            let old_path = migration_dir.join(&old_id);
            let new_path = migration_dir.join(&new_id);

            std::fs::rename(&old_path, &new_path).with_context(|| {
                format!(
                    "Failed to shuffle migration from {} to {}",
                    old_path.display(),
                    new_path.display()
                )
            })?;

            println!("Shuffled migration {} to {}", old_id, new_id);
        }
    }

    tx.commit().await?;

    Ok(())
}

pub async fn history_sync(path: &Path) -> Result<()> {
    let (config, pool) = get_db_assets(path, true).await?;
    let migration_dir = path.parent().ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
    let schema = &config.schema;
    let table = &config.table;
    
    let mut tx = pool.begin().await?;

    let all_migrations = get_all_migration_data(&mut tx, &schema, &table).await?;

    if all_migrations.is_empty() {
        println!("No migrations to sync.");
    } else {
        for row in all_migrations {
            let id: String = row.get("id");
            let up_sql: String = row.get("up");
            let down_sql: String = row.get("down");

            let migration_id_path = migration_dir.join(&id);
            std::fs::create_dir_all(&migration_id_path).with_context(
                || {
                    format!(
                        "Failed to create directory: {}",
                        migration_id_path.display()
                    )
                },
            )?;

            let up_path = migration_id_path.join("up.sql");
            let down_path = migration_id_path.join("down.sql");

            std::fs::write(&up_path, up_sql).with_context(|| {
                format!("Failed to write up migration: {}", up_path.display())
            })?;
            std::fs::write(&down_path, down_sql).with_context(|| {
                format!("Failed to write down migration: {}", down_path.display())
            })?;

            println!("Synced migration: {}", id);
        }
    }

    tx.commit().await?;

    Ok(())
}

pub async fn diff(path: &Path) -> Result<()> {
    use crate::migration_diff::{display_migration_diff, parse_migration_operations};
    
    let (config, pool) = get_db_assets(path, true).await?;
    let migration_dir = path.parent().ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
    let local_migrations = get_local_migrations(path)?;
    let schema = &config.schema;
    let table = &config.table;

    let mut tx = pool.begin().await?;

    let applied_migrations = get_applied_migrations(&mut tx, &schema, &table).await?;

    tx.commit().await?;

    let mut migrations_to_apply: Vec<String> =
        local_migrations.difference(&applied_migrations).cloned().collect();

    migrations_to_apply.sort();

    if migrations_to_apply.is_empty() {
        println!("All migrations are up to date.");
    } else {
        println!("\n🔍 Analyzing {} migration(s) that would be applied...", migrations_to_apply.len());
        
        let mut all_operations = Vec::new();
        
        for migration_id in &migrations_to_apply {
            let migration_path = migration_dir.join(migration_id);
            let up_sql_path = migration_path.join("up.sql");
            
            let up_sql = std::fs::read_to_string(&up_sql_path).with_context(
                || format!("Failed to read up migration: {}", up_sql_path.display()),
            )?;
            
            match parse_migration_operations(&up_sql) {
                Ok(operations) => {
                    display_migration_diff(migration_id, &operations);
                    all_operations.extend(operations);
                }
                Err(e) => {
                    println!("\n⚠️  Could not parse migration {}: {}", migration_id, e);
                    println!("📄 Raw SQL content will be executed as-is.");
                }
            }
        }
        
        println!("\n📊 Migration Summary:");
        println!("  • {} migration(s) to apply", migrations_to_apply.len());
        println!("  • {} total operation(s) detected", all_operations.len());
    }

    Ok(())
}
