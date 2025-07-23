pub mod args;
pub mod reference;
pub mod config;

use {
    crate::config::{Backend, Config},
    anyhow::Result,
    args::ManualFormat,
    chrono::{NaiveDateTime, Utc},
    comfy_table::{
        modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, ContentArrangement, Table,
    },
    sqlx::{postgres::PgRow, Pool, Postgres, Row},
    std::{
        collections::{BTreeMap, HashMap, HashSet},
        path::{Path, PathBuf},
    },
};

async fn get_db_assets(path: &str, timeout: Option<u64>) -> Result<(Config, Pool<Postgres>)> {
    let config: Config = toml::from_str(&std::fs::read_to_string(path)?)?;
    let pool = match &config.backend {
        | Backend::Postgres { host, port, username, password, database, .. } => {
            let mut uri = format!("postgres://{}:{}@{}:{}/{}", username, password, host, port, database);
            if let Some(seconds) = timeout {
                uri.push_str(&format!("?statement_timeout={}", seconds * 1000));
            }
            sqlx::postgres::PgPoolOptions::new().max_connections(10).connect(&uri).await?
        },
    };
    Ok((config, pool))
}

fn get_local_migrations(path: &Path) -> Result<HashSet<String>> {
    let migration_dir = path.parent().ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
    Ok(std::fs::read_dir(migration_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().ok()?.is_dir() && entry.file_name().to_string_lossy().starts_with("id=") {
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
            let out_path = PathBuf::from(path);
            std::fs::create_dir_all(&out_path)?;
            match format {
                | ManualFormat::Manpages => {
                    reference::build_manpages(&out_path)?;
                },
                | ManualFormat::Markdown => {
                    reference::build_markdown(&out_path)?;
                },
            }
            Ok(())
        },
        | crate::args::Command::Autocomplete { path, shell } => {
            let out_path = PathBuf::from(path);
            std::fs::create_dir_all(&out_path)?;
            reference::build_shell_completion(&out_path, &shell)?;
            Ok(())
        },
        | crate::args::Command::Init { path } => {
            let p = PathBuf::from(path);
            let parent = p.parent().ok_or_else(|| anyhow::anyhow!("invalid path"))?;
            std::fs::create_dir_all(parent)?;
            let config = Config {
                backend: Backend::Postgres {
                    host: "localhost".to_string(),
                    port: 5432,
                    username: "postgres".to_string(),
                    password: "postgres".to_string(),
                    database: "postgres".to_string(),
                    schema: "public".to_string(),
                    table: "__qop".to_string(),
                },
            };
            let toml = toml::to_string(&config)?;
            std::fs::write(p, toml)?;
            Ok(())
        },
        | crate::args::Command::Migration(migration) => match migration {
            | crate::args::Migration::Init { path } => {
                let (config, pool) = get_db_assets(&path, None).await?;
                match config.backend {
                    | Backend::Postgres { schema, table, .. } => {
                        let mut tx = pool.begin().await?;
                        sqlx::query(
                            &format!(
                                "CREATE TABLE IF NOT EXISTS {}.{} (id VARCHAR PRIMARY KEY, version VARCHAR NOT NULL, up VARCHAR NOT NULL, down VARCHAR NOT NULL, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, pre VARCHAR)",
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
            | crate::args::Migration::Up { path, timeout, count } => {
                let (config, pool) = get_db_assets(&path, timeout).await?;
                let p = PathBuf::from(&path);
                let migration_dir = p.parent().ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", p.display()))?;
                let local_migrations = get_local_migrations(&p)?;

                match config.backend {
                    | Backend::Postgres { schema, table, .. } => {
                        let mut tx = pool.begin().await?;

                        let applied_migrations: HashSet<String> =
                            sqlx::query(&format!("SELECT id FROM {}.{} ORDER BY id ASC", schema, table))
                                .fetch_all(&mut *tx)
                                .await?
                                .into_iter()
                                .map(|row| row.get("id"))
                                .collect();

                        let mut migrations_to_apply: Vec<String> =
                            local_migrations.difference(&applied_migrations).cloned().collect();

                        migrations_to_apply.sort();

                        let migrations_to_apply = if let Some(count) = count {
                            migrations_to_apply.into_iter().take(count).collect()
                        } else {
                            migrations_to_apply
                        };

                        if migrations_to_apply.is_empty() {
                            println!("All migrations are up to date.");
                        } else {
                            for migration_id in &migrations_to_apply {
                                let migration_path = migration_dir.join(migration_id);
                                println!("Applying migration: {}", migration_path.display());
                                let id = migration_id.as_str();

                                let up_sql_path = migration_path.join("up.sql");
                                let down_sql_path = migration_path.join("down.sql");

                                let up_sql = std::fs::read_to_string(up_sql_path)?;
                                let down_sql = std::fs::read_to_string(down_sql_path)?;

                                sqlx::query(&up_sql).execute(&mut *tx).await?;

                                sqlx::query(&format!(
                                    "INSERT INTO {}.{} (id, version, up, down) VALUES ($1, $2, $3, $4)",
                                    schema, table
                                ))
                                .bind(id)
                                .bind(env!("CARGO_PKG_VERSION"))
                                .bind(up_sql)
                                .bind(down_sql)
                                .execute(&mut *tx)
                                .await?;
                            }

                            println!("Applied {} migrations.", migrations_to_apply.len());
                        }

                        tx.commit().await?;

                        Ok(())
                    },
                }
            },
            | crate::args::Migration::Down { path, timeout, count } => {
                let (config, pool) = get_db_assets(&path, timeout).await?;
                match config.backend {
                    | Backend::Postgres { schema, table, .. } => {
                        let mut tx = pool.begin().await?;

                        let last_migrations: Vec<PgRow> = sqlx::query(&format!(
                            "SELECT id, down FROM {}.{} ORDER BY id DESC",
                            schema, table
                        ))
                        .fetch_all(&mut *tx)
                        .await?;

                        let migrations_to_revert: Vec<PgRow> = if let Some(count) = count {
                            last_migrations.into_iter().take(count).collect()
                        } else {
                            last_migrations.into_iter().take(1).collect()
                        };

                        if migrations_to_revert.is_empty() {
                            println!("No migrations to revert.");
                        } else {
                            for row in migrations_to_revert {
                                let id: String = row.get("id");
                                let down_sql: String = row.get("down");
                                println!("Reverting migration: {}", id);
                                sqlx::query(&down_sql).execute(&mut *tx).await?;
                                sqlx::query(&format!("DELETE FROM {}.{} WHERE id = $1", schema, table))
                                    .bind(&id)
                                    .execute(&mut *tx)
                                    .await?;
                                println!("Migration {} reverted.", id);
                            }
                        }

                        tx.commit().await?;

                        Ok(())
                    },
                }
            },
            | crate::args::Migration::List { path } => {
                let (config, pool) = get_db_assets(&path, None).await?;
                let p = PathBuf::from(&path);
                let local_migrations = get_local_migrations(&p)?;

                match config.backend {
                    | Backend::Postgres { schema, table, .. } => {
                        let mut tx = pool.begin().await?;

                        let applied_migrations: HashMap<String, NaiveDateTime> = sqlx::query(&format!(
                            "SELECT id, created_at FROM {}.{} ORDER BY id ASC",
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
                                Cell::new("Migration ID"),
                                Cell::new("Applied At"),
                                Cell::new("Local"),
                            ]);

                        if all_migrations.is_empty() {
                            println!("No migrations found.");
                        } else {
                            for (id, (applied_at, is_local)) in all_migrations {
                                let applied_str = if let Some(timestamp) = applied_at {
                                    timestamp.format("%Y-%m-%d %H:%M:%S").to_string()
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
            | crate::args::Migration::New { path } => {
                let config: Config = toml::from_str(&std::fs::read_to_string(&path)?)?;
                match config.backend {
                    | Backend::Postgres { .. } => {
                        let id = Utc::now().timestamp_millis().to_string();
                        let p = PathBuf::from(path);
                        let migration_path = p.parent().unwrap();
                        let migration_id_path = migration_path.join(format!("id={}", id));
                        std::fs::create_dir_all(&migration_id_path)?;
                        let up_path = migration_id_path.join("up.sql");
                        let down_path = migration_id_path.join("down.sql");
                        std::fs::write(up_path, "-- SQL goes here")?;
                        std::fs::write(down_path, "-- SQL goes here")?;
                        Ok(())
                    },
                }
            },
        },
    }
}
