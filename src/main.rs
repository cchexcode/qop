pub mod args;
pub mod reference;
pub mod config;

use {
    crate::config::{Backend, Config},
    anyhow::{Context, Result},
    args::ManualFormat,
    chrono::{NaiveDateTime, Utc, Local, TimeZone},
    comfy_table::{
        modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, ContentArrangement, Table,
    },
    sqlx::{postgres::PgRow, Pool, Postgres, Row},
    std::{
        collections::{BTreeMap, HashMap, HashSet},
        io::{self, Write},
        path::Path,
    },
};

async fn get_db_assets(path: &Path, timeout: Option<u64>) -> Result<(Config, Pool<Postgres>)> {
    let config_content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file at: {}", path.display()))?;
    let config: Config = toml::from_str(&config_content)
        .with_context(|| format!("Failed to parse config file at: {}", path.display()))?;
    let pool = match &config.backend {
        | Backend::Postgres { host, port, username, password, database, .. } => {
            let mut uri = format!("postgres://");
            if let (Some(username), Some(password)) = (username, password) {
                uri.push_str(&format!("{}:{}@", username, password));
            }
            uri.push_str(&format!("{}:{}/{}", host, port, database));
            if let Some(seconds) = timeout {
                uri.push_str(&format!("?statement_timeout={}", seconds * 1000));
            }
            sqlx::postgres::PgPoolOptions::new().max_connections(10).connect(&uri).await?
        },
    };
    Ok((config, pool))
}

fn get_local_migrations(path: &Path) -> Result<HashSet<String>> {
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

#[tokio::main]
async fn main() -> Result<()> {
    let cmd = crate::args::ClapArgumentLoader::load()?;

    match cmd.command {
        | crate::args::Command::Manual { path, format } => {
            std::fs::create_dir_all(&path)
                .with_context(|| format!("Failed to create directory: {}", path.display()))?;
            match format {
                | ManualFormat::Manpages => {
                    reference::build_manpages(&path)?;
                },
                | ManualFormat::Markdown => {
                    reference::build_markdown(&path)?;
                },
            }
            Ok(())
        },
        | crate::args::Command::Autocomplete { path, shell } => {
            std::fs::create_dir_all(&path)
                .with_context(|| format!("Failed to create directory: {}", path.display()))?;
            reference::build_shell_completion(&path, &shell)?;
            Ok(())
        },
        | crate::args::Command::Init { path } => {
            let parent = path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("invalid path"))?;
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
            let config = Config {
                backend: Backend::Postgres {
                    host: "localhost".to_string(),
                    port: 5432,
                    username: Some("postgres".to_string()),
                    password: Some("postgres".to_string()),
                    database: "postgres".to_string(),
                    schema: "public".to_string(),
                    table: "__qop".to_string(),
                },
            };
            let toml = toml::to_string(&config)?;
            std::fs::write(&path, toml)
                .with_context(|| format!("Failed to write config file to: {}", path.display()))?;
            Ok(())
        },
        | crate::args::Command::Migration(migration) => {
            let path = &migration.path;
            match migration.command {
            | crate::args::MigrationCommand::Init => {
                let (config, pool) = get_db_assets(path, None).await?;
                match config.backend {
                    | Backend::Postgres { schema, table, .. } => {
                        let mut tx = pool.begin().await?;
                        sqlx::query(
                            &format!(
                                "CREATE TABLE IF NOT EXISTS {}.{} (id VARCHAR PRIMARY KEY, version VARCHAR NOT NULL, up VARCHAR NOT NULL, down VARCHAR NOT NULL, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, pre VARCHAR);",
                                schema, table
                            )
                        )
                        .execute(&mut *tx)
                        .await?;
                        tx.commit().await?;
                        println!("Initialized migration table.");
                        Ok(())
                    },
                }
            },
            | crate::args::MigrationCommand::Up { timeout, count } => {
                let (config, pool) = get_db_assets(path, None).await?;
                let migration_dir = path.parent().ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
                let local_migrations = get_local_migrations(path)?;

                match config.backend {
                    | Backend::Postgres { schema, table, .. } => {
                        let mut tx = pool.begin().await?;

                        if let Some(seconds) = timeout {
                            sqlx::query(&format!("SET LOCAL statement_timeout = '{}s';", seconds))
                                .execute(&mut *tx)
                                .await?;
                        }

                        let applied_migrations: HashSet<String> =
                            sqlx::query(&format!("SELECT id FROM {}.{} ORDER BY id ASC;", schema, table))
                                .fetch_all(&mut *tx)
                                .await?
                                .into_iter()
                                .map(|row| row.get("id"))
                                .collect();

                        let mut last_migration_id: Option<String> =
                            sqlx::query(&format!("SELECT id FROM {}.{} ORDER BY id DESC LIMIT 1;", schema, table))
                                .fetch_optional(&mut *tx)
                                .await?
                                .map(|row| row.get("id"));

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
                                println!("Alternatively, you can run 'qop migration fix' to rename out-of-order migrations.");
                                
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
                            // Apply each migration in its own transaction
                            for migration_id in &migrations_to_apply {
                                let migration_path = migration_dir.join(migration_id);
                                println!("Applying migration: {}", migration_path.display());
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
                                if let Some(seconds) = timeout {
                                    sqlx::query(&format!("SET LOCAL statement_timeout = '{}s';", seconds))
                                        .execute(&mut *migration_tx)
                                        .await?;
                                }

                                // Execute the migration SQL
                                sqlx::query(&up_sql).execute(&mut *migration_tx).await?;

                                // Record the migration in the tracking table
                                sqlx::query(&format!(
                                    "INSERT INTO {}.{} (id, version, up, down, pre) VALUES ($1, $2, $3, $4, $5);",
                                    schema, table
                                ))
                                .bind(id)
                                .bind(env!("CARGO_PKG_VERSION"))
                                .bind(up_sql)
                                .bind(down_sql)
                                .bind(last_migration_id.as_deref())
                                .execute(&mut *migration_tx)
                                .await?;

                                // Commit this migration's transaction
                                migration_tx.commit().await?;
                                
                                last_migration_id = Some(id.to_string());
                            }

                            println!("Applied {} migrations.", migrations_to_apply.len());
                        }

                        Ok(())
                    },
                }
            },
            | crate::args::MigrationCommand::Down { timeout, count, remote } => {
                let (config, pool) = get_db_assets(path, None).await?;
                let migration_dir = path.parent().ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
                match config.backend {
                    | Backend::Postgres { schema, table, .. } => {
                        let mut tx = pool.begin().await?;

                        if let Some(seconds) = timeout {
                            sqlx::query(&format!("SET LOCAL statement_timeout = '{}s';", seconds))
                                .execute(&mut *tx)
                                .await?;
                        }

                        let last_migrations: Vec<PgRow> = sqlx::query(&format!(
                            "SELECT id, down FROM {}.{} ORDER BY id DESC;",
                            schema, table
                        ))
                        .fetch_all(&mut *tx)
                        .await?;

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
                                if let Some(seconds) = timeout {
                                    sqlx::query(&format!("SET LOCAL statement_timeout = '{}s';", seconds))
                                        .execute(&mut *revert_tx)
                                        .await?;
                                }

                                // Execute the down migration SQL
                                sqlx::query(&down_sql).execute(&mut *revert_tx).await?;

                                // Remove the migration from the tracking table
                                sqlx::query(&format!("DELETE FROM {}.{} WHERE id = $1;", schema, table))
                                    .bind(&id)
                                    .execute(&mut *revert_tx)
                                    .await?;

                                // Commit this migration revert's transaction
                                revert_tx.commit().await?;

                                println!("Migration {} reverted.", id);
                            }
                        }

                        Ok(())
                    },
                }
            },
            | crate::args::MigrationCommand::Apply(apply_cmd) => {
                match apply_cmd {
                    | crate::args::MigrationApply::Up { id, timeout } => {
                        let (config, pool) = get_db_assets(path, None).await?;
                        let migration_dir = path
                            .parent()
                            .ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
                        let local_migrations = get_local_migrations(path)?;

                        // Normalize the migration ID to include "id=" prefix if not present
                        let target_migration_id = if id.starts_with("id=") {
                            id.clone()
                        } else {
                            format!("id={}", id)
                        };

                        match config.backend {
                            | Backend::Postgres { schema, table, .. } => {
                                let mut tx = pool.begin().await?;

                                // Get current applied migrations
                                let applied_migrations: HashSet<String> = sqlx::query(&format!(
                                    "SELECT id FROM {}.{} ORDER BY id ASC;",
                                    schema, table
                                ))
                                .fetch_all(&mut *tx)
                                .await?
                                .into_iter()
                                .map(|row| row.get("id"))
                                .collect();

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
                                let last_migration_id: Option<String> = sqlx::query(&format!(
                                    "SELECT id FROM {}.{} ORDER BY id DESC LIMIT 1;",
                                    schema, table
                                ))
                                .fetch_optional(&mut *tx)
                                .await?
                                .map(|row| row.get("id"));
                                tx.commit().await?;

                                // Execute the migration
                                let mut migration_tx = pool.begin().await?;

                                if let Some(seconds) = timeout {
                                    sqlx::query(&format!("SET LOCAL statement_timeout = '{}s';", seconds))
                                        .execute(&mut *migration_tx)
                                        .await?;
                                }

                                println!("Applying migration: {}", target_migration_id);
                                sqlx::query(&up_sql).execute(&mut *migration_tx).await?;

                                sqlx::query(&format!(
                                    "INSERT INTO {}.{} (id, version, up, down, pre) VALUES ($1, $2, $3, $4, $5);",
                                    schema, table
                                ))
                                .bind(&target_migration_id)
                                .bind(env!("CARGO_PKG_VERSION"))
                                .bind(up_sql)
                                .bind(down_sql)
                                .bind(last_migration_id.as_deref())
                                .execute(&mut *migration_tx)
                                .await?;

                                migration_tx.commit().await?;
                                println!("Migration {} applied successfully.", target_migration_id);

                                Ok(())
                            },
                        }
                    },
                    | crate::args::MigrationApply::Down { id, timeout, remote } => {
                        let (config, pool) = get_db_assets(path, None).await?;
                        let migration_dir = path
                            .parent()
                            .ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;

                        // Normalize the migration ID to include "id=" prefix if not present
                        let target_migration_id = if id.starts_with("id=") {
                            id.clone()
                        } else {
                            format!("id={}", id)
                        };

                        match config.backend {
                            | Backend::Postgres { schema, table, .. } => {
                                let mut tx = pool.begin().await?;

                                // Get current applied migrations
                                let applied_migrations: HashSet<String> = sqlx::query(&format!(
                                    "SELECT id FROM {}.{} ORDER BY id ASC;",
                                    schema, table
                                ))
                                .fetch_all(&mut *tx)
                                .await?
                                .into_iter()
                                .map(|row| row.get("id"))
                                .collect();

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
                                    let migration_row = sqlx::query(&format!(
                                        "SELECT down FROM {}.{} WHERE id = $1;",
                                        schema, table
                                    ))
                                    .bind(&target_migration_id)
                                    .fetch_one(&mut *tx)
                                    .await?;

                                    let sql: String = migration_row.get("down");
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

                                if let Some(seconds) = timeout {
                                    sqlx::query(&format!("SET LOCAL statement_timeout = '{}s';", seconds))
                                        .execute(&mut *revert_tx)
                                        .await?;
                                }

                                println!("Reverting migration: {}", target_migration_id);
                                sqlx::query(&down_sql).execute(&mut *revert_tx).await?;

                                sqlx::query(&format!("DELETE FROM {}.{} WHERE id = $1;", schema, table))
                                    .bind(&target_migration_id)
                                    .execute(&mut *revert_tx)
                                    .await?;

                                revert_tx.commit().await?;
                                println!("Migration {} reverted successfully.", target_migration_id);

                                Ok(())
                            },
                        }
                    },
                }
            },
            | crate::args::MigrationCommand::Fix { .. } => {
                let (config, pool) = get_db_assets(path, None).await?;
                let migration_dir = path.parent().ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
                let local_migrations = get_local_migrations(path)?;

                match config.backend {
                    | Backend::Postgres { schema, table, .. } => {
                        let mut tx = pool.begin().await?;

                        let applied_migrations: HashSet<String> =
                            sqlx::query(&format!("SELECT id FROM {}.{} ORDER BY id ASC;", schema, table))
                                .fetch_all(&mut *tx)
                                .await?
                                .into_iter()
                                .map(|row| row.get("id"))
                                .collect();

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
                    },
                }
            },
            | crate::args::MigrationCommand::Sync { .. } => {
                let (config, pool) = get_db_assets(path, None).await?;
                let migration_dir = path.parent().ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
                match config.backend {
                    | Backend::Postgres { schema, table, .. } => {
                        let mut tx = pool.begin().await?;

                        let all_migrations: Vec<PgRow> =
                            sqlx::query(&format!("SELECT id, up, down FROM {}.{} ORDER BY id ASC;", schema, table))
                                .fetch_all(&mut *tx)
                                .await?;

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
                    },
                }
            },
            | crate::args::MigrationCommand::List { .. } => {
                let (config, pool) = get_db_assets(path, None).await?;
                let local_migrations = get_local_migrations(path)?;

                match config.backend {
                    | Backend::Postgres { schema, table, .. } => {
                        let mut tx = pool.begin().await?;

                        let applied_migrations: HashMap<String, NaiveDateTime> = sqlx::query(&format!(
                            "SELECT id, created_at FROM {}.{} ORDER BY id ASC;",
                            schema, table
                        ))
                        .fetch_all(&mut *tx)
                        .await?
                        .into_iter()
                        .map(|row| (row.get("id"), row.get("created_at")))
                        .collect();

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
                    },
                }
            },
            | crate::args::MigrationCommand::New { .. } => {
                let config: Config = toml::from_str(
                    &std::fs::read_to_string(path)
                        .with_context(|| format!("Failed to read config file at: {}", path.display()))?,
                )
                .with_context(|| format!("Failed to parse config file at: {}", path.display()))?;
                match config.backend {
                    | Backend::Postgres { .. } => {
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
                    },
                }
            }
        }
        },
    }
}
