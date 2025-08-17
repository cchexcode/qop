use anyhow::Result;
use std::path::PathBuf;
use crate::core::service::MigrationService;

/// Common driver API for migration subsystems
#[async_trait::async_trait(?Send)]
pub trait MigrationDriver {
    async fn init(&self, path: PathBuf) -> Result<()>;
    async fn new_migration(&self, path: PathBuf) -> Result<()>;
    async fn up(&self, path: PathBuf, timeout: Option<u64>, count: Option<usize>, diff: bool, dry: bool, yes: bool) -> Result<()>;
    async fn down(&self, path: PathBuf, timeout: Option<u64>, count: Option<usize>, remote: bool, diff: bool, dry: bool, yes: bool) -> Result<()>;
    async fn apply_up(&self, path: PathBuf, id: String, timeout: Option<u64>, dry: bool, yes: bool) -> Result<()>;
    async fn apply_down(&self, path: PathBuf, id: String, timeout: Option<u64>, remote: bool, dry: bool, yes: bool) -> Result<()>;
    async fn list(&self, path: PathBuf, output: crate::core::service::OutputFormat) -> Result<()>;
    async fn history_fix(&self, path: PathBuf) -> Result<()>;
    async fn history_sync(&self, path: PathBuf) -> Result<()>;
    async fn diff(&self, path: PathBuf) -> Result<()>;
}

#[cfg(feature = "postgres")]
pub struct PostgresDriver;

#[async_trait::async_trait(?Send)]
#[cfg(feature = "postgres")]
impl MigrationDriver for PostgresDriver {
    async fn init(&self, path: PathBuf) -> Result<()> { 
        let repo = super::postgres::repo::PostgresRepo::from_path(&path).await?;
        let svc = MigrationService::new(repo);
        svc.init().await
    }
    async fn new_migration(&self, path: PathBuf) -> Result<()> { 
        let repo = super::postgres::repo::PostgresRepo::from_path(&path).await?;
        let svc = MigrationService::new(repo);
        svc.new_migration(&path).await
    }
    async fn up(&self, path: PathBuf, timeout: Option<u64>, count: Option<usize>, _diff: bool, _dry: bool, yes: bool) -> Result<()> { 
        let repo = super::postgres::repo::PostgresRepo::from_path(&path).await?;
        let svc = MigrationService::new(repo);
        svc.up(&path, timeout, count, yes).await
    }
    async fn down(&self, path: PathBuf, timeout: Option<u64>, count: Option<usize>, remote: bool, _diff: bool, _dry: bool, yes: bool) -> Result<()> { 
        let repo = super::postgres::repo::PostgresRepo::from_path(&path).await?;
        let svc = MigrationService::new(repo);
        svc.down(&path, timeout, count, remote, yes).await
    }
    async fn apply_up(&self, path: PathBuf, id: String, timeout: Option<u64>, _dry: bool, yes: bool) -> Result<()> { 
        let repo = super::postgres::repo::PostgresRepo::from_path(&path).await?;
        let svc = MigrationService::new(repo);
        svc.apply_up(&path, &id, timeout, yes).await
    }
    async fn apply_down(&self, path: PathBuf, id: String, timeout: Option<u64>, remote: bool, _dry: bool, yes: bool) -> Result<()> { 
        let repo = super::postgres::repo::PostgresRepo::from_path(&path).await?;
        let svc = MigrationService::new(repo);
        svc.apply_down(&path, &id, timeout, remote, yes).await
    }
    async fn list(&self, path: PathBuf, output: crate::core::service::OutputFormat) -> Result<()> { 
        let repo = super::postgres::repo::PostgresRepo::from_path(&path).await?;
        let svc = MigrationService::new(repo);
        svc.list(output).await
    }
    async fn history_fix(&self, path: PathBuf) -> Result<()> { super::postgres::migration::history_fix(&path).await }
    async fn history_sync(&self, path: PathBuf) -> Result<()> { super::postgres::migration::history_sync(&path).await }
    async fn diff(&self, path: PathBuf) -> Result<()> { super::postgres::migration::diff(&path).await }
}

#[cfg(feature = "sqlite")]
pub struct SqliteDriver;

#[async_trait::async_trait(?Send)]
#[cfg(feature = "sqlite")]
impl MigrationDriver for SqliteDriver {
    async fn init(&self, path: PathBuf) -> Result<()> { 
        let repo = super::sqlite::repo::SqliteRepo::from_path(&path).await?;
        let svc = MigrationService::new(repo);
        svc.init().await
    }
    async fn new_migration(&self, path: PathBuf) -> Result<()> { 
        let repo = super::sqlite::repo::SqliteRepo::from_path(&path).await?;
        let svc = MigrationService::new(repo);
        svc.new_migration(&path).await
    }
    async fn up(&self, path: PathBuf, timeout: Option<u64>, count: Option<usize>, _diff: bool, _dry: bool, yes: bool) -> Result<()> { 
        let repo = super::sqlite::repo::SqliteRepo::from_path(&path).await?;
        let svc = MigrationService::new(repo);
        svc.up(&path, timeout, count, yes).await
    }
    async fn down(&self, path: PathBuf, timeout: Option<u64>, count: Option<usize>, remote: bool, _diff: bool, _dry: bool, yes: bool) -> Result<()> { 
        let repo = super::sqlite::repo::SqliteRepo::from_path(&path).await?;
        let svc = MigrationService::new(repo);
        svc.down(&path, timeout, count, remote, yes).await
    }
    async fn apply_up(&self, path: PathBuf, id: String, timeout: Option<u64>, _dry: bool, yes: bool) -> Result<()> { 
        let repo = super::sqlite::repo::SqliteRepo::from_path(&path).await?;
        let svc = MigrationService::new(repo);
        svc.apply_up(&path, &id, timeout, yes).await
    }
    async fn apply_down(&self, path: PathBuf, id: String, timeout: Option<u64>, remote: bool, _dry: bool, yes: bool) -> Result<()> { 
        let repo = super::sqlite::repo::SqliteRepo::from_path(&path).await?;
        let svc = MigrationService::new(repo);
        svc.apply_down(&path, &id, timeout, remote, yes).await
    }
    async fn list(&self, path: PathBuf, output: crate::core::service::OutputFormat) -> Result<()> { 
        let repo = super::sqlite::repo::SqliteRepo::from_path(&path).await?;
        let svc = MigrationService::new(repo);
        svc.list(output).await
    }
    async fn history_fix(&self, path: PathBuf) -> Result<()> { super::sqlite::migration::history_fix(&path).await }
    async fn history_sync(&self, path: PathBuf) -> Result<()> { super::sqlite::migration::history_sync(&path).await }
    async fn diff(&self, path: PathBuf) -> Result<()> { super::sqlite::migration::diff(&path).await }
}

pub(crate) async fn dispatch(subsystem: crate::args::Subsystem) -> anyhow::Result<()> {
    match subsystem {
        #[cfg(feature = "postgres")]
        crate::args::Subsystem::Postgres { path, config: _cfg, command } => {
            let driver = PostgresDriver;
            match command {
                crate::subsystem::postgres::commands::Command::Init => driver.init(path).await,
                crate::subsystem::postgres::commands::Command::New => driver.new_migration(path).await,
                crate::subsystem::postgres::commands::Command::Up { timeout, count, diff, dry, yes } => driver.up(path, timeout, count, diff, dry, yes).await,
                crate::subsystem::postgres::commands::Command::Down { timeout, count, remote, diff, dry, yes } => driver.down(path, timeout, count, remote, diff, dry, yes).await,
                crate::subsystem::postgres::commands::Command::Apply(apply_cmd) => match apply_cmd {
                    crate::subsystem::postgres::commands::MigrationApply::Up { id, timeout, dry, yes } => driver.apply_up(path, id, timeout, dry, yes).await,
                    crate::subsystem::postgres::commands::MigrationApply::Down { id, timeout, remote, dry, yes } => driver.apply_down(path, id, timeout, remote, dry, yes).await,
                },
                crate::subsystem::postgres::commands::Command::List { output } => {
                    let out = match output {
                        super::postgres::commands::Output::Human => crate::core::service::OutputFormat::Human,
                        super::postgres::commands::Output::Json => crate::core::service::OutputFormat::Json,
                    };
                    driver.list(path, out).await
                }
                crate::subsystem::postgres::commands::Command::Config(cfg) => match cfg {
                    super::postgres::commands::ConfigCommand::Init => {
                        #[cfg(feature = "postgres")]
                        {
                            let cfg = super::postgres::build_sample();
                            let toml = toml::to_string(&cfg)?;
                            crate::config_init::write_config(&path, &toml)?;
                            println!("Wrote sample Postgres config to {}", path.display());
                            Ok(())
                        }
                        #[cfg(not(feature = "postgres"))]
                        {
                            anyhow::bail!("postgres feature disabled")
                        }
                    }
                },
                crate::subsystem::postgres::commands::Command::History(history_cmd) => match history_cmd {
                    crate::subsystem::postgres::commands::HistoryCommand::Fix => driver.history_fix(path).await,
                    crate::subsystem::postgres::commands::HistoryCommand::Sync => driver.history_sync(path).await,
                },
                crate::subsystem::postgres::commands::Command::Diff => driver.diff(path).await,
            }
        }
        #[cfg(feature = "sqlite")]
        crate::args::Subsystem::Sqlite { path, config: _cfg, command } => {
            let driver = SqliteDriver;
            match command {
                
                crate::subsystem::sqlite::commands::Command::Init => driver.init(path).await,
                crate::subsystem::sqlite::commands::Command::New => driver.new_migration(path).await,
                crate::subsystem::sqlite::commands::Command::Up { timeout, count, diff, dry, yes } => driver.up(path, timeout, count, diff, dry, yes).await,
                crate::subsystem::sqlite::commands::Command::Down { timeout, count, remote, diff, dry, yes } => driver.down(path, timeout, count, remote, diff, dry, yes).await,
                crate::subsystem::sqlite::commands::Command::Apply(apply_cmd) => match apply_cmd {
                    crate::subsystem::sqlite::commands::MigrationApply::Up { id, timeout, dry, yes } => driver.apply_up(path, id, timeout, dry, yes).await,
                    crate::subsystem::sqlite::commands::MigrationApply::Down { id, timeout, remote, dry, yes } => driver.apply_down(path, id, timeout, remote, dry, yes).await,
                },
                crate::subsystem::sqlite::commands::Command::List { output } => {
                    let out = match output {
                        super::sqlite::commands::Output::Human => crate::core::service::OutputFormat::Human,
                        super::sqlite::commands::Output::Json => crate::core::service::OutputFormat::Json,
                    };
                    driver.list(path, out).await
                }
                crate::subsystem::sqlite::commands::Command::Config(cfg) => match cfg {
                    super::sqlite::commands::ConfigCommand::Init => {
                        #[cfg(feature = "sqlite")]
                        {
                            let cfg = super::sqlite::build_sample(&path);
                            let toml = toml::to_string(&cfg)?;
                            crate::config_init::write_config(&path, &toml)?;
                            println!("Wrote sample SQLite config to {}", path.display());
                            Ok(())
                        }
                        #[cfg(not(feature = "sqlite"))]
                        {
                            anyhow::bail!("sqlite feature disabled")
                        }
                    }
                },
                crate::subsystem::sqlite::commands::Command::History(history_cmd) => match history_cmd {
                    crate::subsystem::sqlite::commands::HistoryCommand::Fix => driver.history_fix(path).await,
                    crate::subsystem::sqlite::commands::HistoryCommand::Sync => driver.history_sync(path).await,
                },
                crate::subsystem::sqlite::commands::Command::Diff => driver.diff(path).await,
            }
        }
    }
}
