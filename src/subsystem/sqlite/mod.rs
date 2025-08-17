pub mod commands;
pub mod migration;
#[cfg(feature = "sqlite")]
pub mod repo;
pub mod config;

#[cfg(feature = "sqlite")]
use crate::config::{Config, Subsystem, DataSource};
#[cfg(feature = "sqlite")]
use crate::subsystem::sqlite::config::SubsystemSqlite;

#[cfg(feature = "sqlite")]
pub fn build_sample_with_db_path(db_path: &std::path::Path) -> crate::config::Config {
    Config {
        version: env!("CARGO_PKG_VERSION").to_string(),
        subsystem: Subsystem::Sqlite(SubsystemSqlite {
            connection: DataSource::Static(db_path.to_string_lossy().to_string()),
            timeout: Some(60),
            table: "__qop".to_string(),
        }),
    }
}