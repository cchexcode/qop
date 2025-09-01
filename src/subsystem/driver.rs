use anyhow::Context;
#[cfg(any(feature = "sub+postgres", feature = "sub+sqlite"))]
use crate::core::service::MigrationService;

/// Note: The old `MigrationDriver` trait and driver structs have been removed.

pub(crate) async fn dispatch(subsystem: crate::args::Subsystem) -> anyhow::Result<()> {
    match subsystem {
        #[cfg(feature = "sub+postgres")]
        crate::args::Subsystem::Postgres { path, config, command } => {
            // driver removed; construct repos directly per command
            match command {
                crate::subsystem::postgres::commands::Command::Init => {
                    let repo = super::postgres::repo::PostgresRepo::from_config(&path, config.clone(), false).await?;
                    let svc = MigrationService::new(repo);
                    svc.init().await
                }
                crate::subsystem::postgres::commands::Command::New { comment, locked } => {
                    let repo = super::postgres::repo::PostgresRepo::from_config(&path, config.clone(), true).await?;
                    let svc = MigrationService::new(repo);
                    svc.new_migration(&path, comment.as_deref(), locked).await
                }
                crate::subsystem::postgres::commands::Command::Up { timeout, count, diff: _, dry, yes } => {
                    let repo = super::postgres::repo::PostgresRepo::from_config(&path, config.clone(), true).await?;
                    let svc = MigrationService::new(repo);
                    svc.up(&path, timeout, count, yes, dry).await
                }
                crate::subsystem::postgres::commands::Command::Down { timeout, count, remote, diff: _, dry, yes, unlock } => {
                    let repo = super::postgres::repo::PostgresRepo::from_config(&path, config.clone(), true).await?;
                    let svc = MigrationService::new(repo);
                    svc.down(&path, timeout, count, remote, yes, dry, unlock).await
                }
                crate::subsystem::postgres::commands::Command::Apply(apply_cmd) => match apply_cmd {
                    crate::subsystem::postgres::commands::MigrationApply::Up { id, timeout, dry, yes } => {
                        let repo = super::postgres::repo::PostgresRepo::from_config(&path, config.clone(), true).await?;
                        let svc = MigrationService::new(repo);
                        svc.apply_up(&path, &id, timeout, yes, dry, false).await
                    }
                    crate::subsystem::postgres::commands::MigrationApply::Down { id, timeout, remote, dry, yes, unlock } => {
                        let repo = super::postgres::repo::PostgresRepo::from_config(&path, config.clone(), true).await?;
                        let svc = MigrationService::new(repo);
                        svc.apply_down(&path, &id, timeout, remote, yes, dry, unlock).await
                    }
                },
                crate::subsystem::postgres::commands::Command::List { output } => {
                    let out = match output {
                        super::postgres::commands::Output::Human => crate::core::service::OutputFormat::Human,
                        super::postgres::commands::Output::Json => crate::core::service::OutputFormat::Json,
                    };
                    let repo = super::postgres::repo::PostgresRepo::from_config(&path, config.clone(), true).await?;
                    let svc = MigrationService::new(repo);
                    svc.list(out).await
                }
                crate::subsystem::postgres::commands::Command::Config(cfg) => match cfg {
                    super::postgres::commands::ConfigCommand::Init { connection } => {
                        let cfg = super::postgres::build_sample(&connection);
                        let toml = toml::to_string(&cfg)?;
                        {
                            if let Some(parent) = path.parent() {
                                if !parent.as_os_str().is_empty() {
                                    std::fs::create_dir_all(parent)
                                        .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
                                }
                            }
                            std::fs::write(&path, &toml)
                                .with_context(|| format!("Failed to write config file to: {}", path.display()))?;
                        }
                        println!("Bootstrapped postgres config to {}", path.display());
                        Ok(())
                    }
                },
                crate::subsystem::postgres::commands::Command::History(history_cmd) => match history_cmd {
                    crate::subsystem::postgres::commands::HistoryCommand::Fix => {
                        let repo = super::postgres::repo::PostgresRepo::from_config(&path, config.clone(), true).await?;
                        super::postgres::migration::history_fix(&path, &repo.config.schema, &repo.config.tables.migrations, &repo.pool).await
                    }
                    crate::subsystem::postgres::commands::HistoryCommand::Sync => {
                        let repo = super::postgres::repo::PostgresRepo::from_config(&path, config.clone(), true).await?;
                        super::postgres::migration::history_sync(&path, &repo.config.schema, &repo.config.tables.migrations, &repo.pool).await
                    }
                },
                crate::subsystem::postgres::commands::Command::Diff => {
                    let repo = super::postgres::repo::PostgresRepo::from_config(&path, config.clone(), true).await?;
                    super::postgres::migration::diff(&path, &repo.config.schema, &repo.config.tables.migrations, &repo.pool).await
                },
            }
        }
        #[cfg(feature = "sub+sqlite")]
        crate::args::Subsystem::Sqlite { path, config, command } => {
            // driver removed; construct repos directly per command
            match command {
                crate::subsystem::sqlite::commands::Command::Init => {
                    let repo = super::sqlite::repo::SqliteRepo::from_config(&path, config.clone(), false).await?;
                    let svc = MigrationService::new(repo);
                    svc.init().await
                }
                crate::subsystem::sqlite::commands::Command::New { comment, locked } => {
                    let repo = super::sqlite::repo::SqliteRepo::from_config(&path, config.clone(), true).await?;
                    let svc = MigrationService::new(repo);
                    svc.new_migration(&path, comment.as_deref(), locked).await
                }
                crate::subsystem::sqlite::commands::Command::Up { timeout, count, diff: _, dry, yes } => {
                    let repo = super::sqlite::repo::SqliteRepo::from_config(&path, config.clone(), true).await?;
                    let svc = MigrationService::new(repo);
                    svc.up(&path, timeout, count, yes, dry).await
                }
                crate::subsystem::sqlite::commands::Command::Down { timeout, count, remote, diff: _, dry, yes, unlock } => {
                    let repo = super::sqlite::repo::SqliteRepo::from_config(&path, config.clone(), true).await?;
                    let svc = MigrationService::new(repo);
                    svc.down(&path, timeout, count, remote, yes, dry, unlock).await
                }
                crate::subsystem::sqlite::commands::Command::Apply(apply_cmd) => match apply_cmd {
                    crate::subsystem::sqlite::commands::MigrationApply::Up { id, timeout, dry, yes } => {
                        let repo = super::sqlite::repo::SqliteRepo::from_config(&path, config.clone(), true).await?;
                        let svc = MigrationService::new(repo);
                        svc.apply_up(&path, &id, timeout, yes, dry, false).await
                    }
                    crate::subsystem::sqlite::commands::MigrationApply::Down { id, timeout, remote, dry, yes, unlock } => {
                        let repo = super::sqlite::repo::SqliteRepo::from_config(&path, config.clone(), true).await?;
                        let svc = MigrationService::new(repo);
                        svc.apply_down(&path, &id, timeout, remote, yes, dry, unlock).await
                    }
                },
                crate::subsystem::sqlite::commands::Command::List { output } => {
                    let out = match output {
                        super::sqlite::commands::Output::Human => crate::core::service::OutputFormat::Human,
                        super::sqlite::commands::Output::Json => crate::core::service::OutputFormat::Json,
                    };
                    let repo = super::sqlite::repo::SqliteRepo::from_config(&path, config.clone(), true).await?;
                    let svc = MigrationService::new(repo);
                    svc.list(out).await
                }
                crate::subsystem::sqlite::commands::Command::Config(cfg) => match cfg {
                    super::sqlite::commands::ConfigCommand::Init { path: db_path } => {
                        let cfg = super::sqlite::build_sample_with_db_path(std::path::Path::new(&db_path));
                        let toml = toml::to_string(&cfg)?;
                        {
                            if let Some(parent) = path.parent() {
                                if !parent.as_os_str().is_empty() {
                                    std::fs::create_dir_all(parent)
                                        .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
                                }
                            }
                            std::fs::write(&path, &toml)
                                .with_context(|| format!("Failed to write config file to: {}", path.display()))?;
                        }
                        println!("Bootstrapped sqlite config to {}", path.display());
                        Ok(())
                    }
                },
                crate::subsystem::sqlite::commands::Command::History(history_cmd) => match history_cmd {
                    crate::subsystem::sqlite::commands::HistoryCommand::Fix => {
                        let repo = super::sqlite::repo::SqliteRepo::from_config(&path, config.clone(), true).await?;
                        super::sqlite::migration::history_fix(&path, &repo.config.tables.migrations, &repo.pool).await
                    }
                    crate::subsystem::sqlite::commands::HistoryCommand::Sync => {
                        let repo = super::sqlite::repo::SqliteRepo::from_config(&path, config.clone(), true).await?;
                        super::sqlite::migration::history_sync(&path, &repo.config.tables.migrations, &repo.pool).await
                    }
                },
                crate::subsystem::sqlite::commands::Command::Diff => {
                    let repo = super::sqlite::repo::SqliteRepo::from_config(&path, config.clone(), true).await?;
                    super::sqlite::migration::diff(&path, &repo.config.tables.migrations, &repo.pool).await
                },
            }
        }
    }
}
