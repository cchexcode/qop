pub mod args;
pub mod reference;
pub mod config;
pub mod migration_diff;
pub mod subsystem;

use {
    crate::config::{Subsystem, Config, DataSource},
    anyhow::{Context, Result},
    args::ManualFormat,
};

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
                version: env!("CARGO_PKG_VERSION").to_string(),
                subsystem: Subsystem::Postgres {
                    connection: DataSource::Static("postgres://user:password@localhost:5432/postgres".to_string()),
                    timeout: Some(60),
                    table: "__qop".to_string(),
                    schema: "public".to_string(),
                },
            };
            let toml = toml::to_string(&config)?;
            std::fs::write(&path, toml)
                .with_context(|| format!("Failed to write config file to: {}", path.display()))?;
            Ok(())
        },
        | crate::args::Command::Migration(migration) => {
            let path = &migration.path;
            match migration.subsystem {
            | crate::args::Subsystem::Postgres(postgres_cmd) => match postgres_cmd {
            | crate::subsystem::postgres::commands::Command::Init => {
                crate::subsystem::postgres::migration::init(path).await
            },
            | crate::subsystem::postgres::commands::Command::Up { timeout, count, diff } => {
                crate::subsystem::postgres::migration::up(path, timeout, count, diff).await
            },
            | crate::subsystem::postgres::commands::Command::Down { timeout, count, remote, diff } => {
                crate::subsystem::postgres::migration::down(path, timeout, count, remote, diff).await
            },
            | crate::subsystem::postgres::commands::Command::Apply(apply_cmd) => {
                match apply_cmd {
                    | crate::subsystem::postgres::commands::MigrationApply::Up { id, timeout } => {
                        crate::subsystem::postgres::migration::apply_up(path, &id, timeout).await
                    },
                    | crate::subsystem::postgres::commands::MigrationApply::Down { id, timeout, remote } => {
                        crate::subsystem::postgres::migration::apply_down(path, &id, timeout, remote).await
                    },
                }
            },
            | crate::subsystem::postgres::commands::Command::List => {
                crate::subsystem::postgres::migration::list(path).await
            },
            | crate::subsystem::postgres::commands::Command::History(history_cmd) => {
                match history_cmd {
                    | crate::subsystem::postgres::commands::HistoryCommand::Fix => {
                        crate::subsystem::postgres::migration::history_fix(path).await
                    },
                    | crate::subsystem::postgres::commands::HistoryCommand::Sync => {
                        crate::subsystem::postgres::migration::history_sync(path).await
                    },
                }
            },
            | crate::subsystem::postgres::commands::Command::Diff => {
                crate::subsystem::postgres::migration::diff(path).await
            },
            | crate::subsystem::postgres::commands::Command::New => {
                crate::subsystem::postgres::migration::new_migration(path).await
            }
            }
        }
        },
    }
}
