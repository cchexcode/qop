pub mod args;
pub mod reference;
pub mod config;

use {
    crate::config::{Backend, Config},
    anyhow::Result,
    args::ManualFormat,
    chrono::Utc,
    sqlx::{postgres::PgRow, Pool, Postgres, Row},
    std::path::PathBuf,
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
            | crate::args::Migration::Up { path, timeout } => {
                let (config, pool) = get_db_assets(&path, timeout).await?;
                match config.backend {
                    | Backend::Postgres { schema, table, .. } => {
                        let mut tx = pool.begin().await?;
                        let p = PathBuf::from(&path);
                        let migration_dir = p.parent().unwrap();
                        let migrations: Vec<String> = std::fs::read_dir(migration_dir)?
                            .filter_map(|entry| {
                                let entry = entry.ok()?;
                                if entry.file_type().ok()?.is_dir()
                                    && entry.file_name().to_string_lossy().starts_with("id=")
                                {
                                    Some(entry.path().to_string_lossy().into_owned())
                                } else {
                                    None
                                }
                            })
                            .collect();

                        let last_migration: Option<String> =
                            sqlx::query(&format!("SELECT id FROM {}.{} ORDER BY id DESC LIMIT 1", schema, table))
                                .fetch_optional(&mut *tx)
                                .await?
                                .map(|row| row.get("id"));

                        let mut migrations_to_apply = migrations;
                        if let Some(last_id) = last_migration.as_deref() {
                            migrations_to_apply.retain(|m| {
                                let p = PathBuf::from(m);
                                let dirname = p.file_name().unwrap().to_str().unwrap();
                                dirname > last_id
                            });
                        }

                        migrations_to_apply.sort();

                        for migration_path in &migrations_to_apply {
                            println!("Applying migration: {}", migration_path);
                            let p = PathBuf::from(migration_path);
                            let id = p.file_name().unwrap().to_str().unwrap();

                            let up_sql_path = p.join("up.sql");
                            let down_sql_path = p.join("down.sql");

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

                        tx.commit().await?;

                        println!("Applied {} migrations.", migrations_to_apply.len());

                        Ok(())
                    },
                }
            },
            | crate::args::Migration::Down { path, timeout } => {
                let (config, pool) = get_db_assets(&path, timeout).await?;
                match config.backend {
                    | Backend::Postgres { schema, table, .. } => {
                        let mut tx = pool.begin().await?;

                        let last_migration: Option<PgRow> =
                            sqlx::query(&format!("SELECT id, down FROM {}.{} ORDER BY id DESC LIMIT 1", schema, table))
                                .fetch_optional(&mut *tx)
                                .await?;

                        if let Some(row) = last_migration {
                            let id: String = row.get("id");
                            let down_sql: String = row.get("down");
                            println!("Reverting migration: {}", id);
                            sqlx::query(&down_sql).execute(&mut *tx).await?;
                            sqlx::query(&format!("DELETE FROM {}.{} WHERE id = $1", schema, table))
                                .bind(&id)
                                .execute(&mut *tx)
                                .await?;
                            println!("Migration {} reverted.", id);
                        } else {
                            println!("No migrations to revert.");
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
