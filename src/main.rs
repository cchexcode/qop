pub mod args;
pub mod reference;
pub mod config;
pub mod migration_diff;
pub mod subsystem;
pub mod config_init;
pub mod core;

use {
    crate::config::{Config, DataSource, Subsystem, SubsystemPostgres},
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
                subsystem: Subsystem::Postgres(SubsystemPostgres {
                    connection: DataSource::Static("postgres://user:password@localhost:5432/postgres".to_string()),
                    timeout: Some(60),
                    table: "__qop".to_string(),
                    schema: "public".to_string(),
                }),
            };
            let toml = toml::to_string(&config)?;
            std::fs::write(&path, toml)
                .with_context(|| format!("Failed to write config file to: {}", path.display()))?;
            Ok(())
        },
        | crate::args::Command::Subsystem(subsystem) => {
            crate::subsystem::driver::dispatch(subsystem).await
        },
        // If command parsing evolves to allow no subcommand, we could default to interactive here
    }
}
