pub mod commands;
pub mod migration;
#[cfg(feature = "sub+sqlite")]
pub mod repo;
pub mod config;

#[cfg(feature = "sub+sqlite")]
use crate::config::{Config, Subsystem, DataSource};
#[cfg(feature = "sub+sqlite")]
use crate::subsystem::sqlite::config::SubsystemSqlite;

#[cfg(feature = "sub+sqlite")]
pub fn build_sample_with_db_path(db_path: &std::path::Path) -> crate::config::Config {
    Config {
        version: env!("CARGO_PKG_VERSION").to_string(),
        subsystem: Subsystem::Sqlite(SubsystemSqlite {
            connection: DataSource::Static(db_path.to_string_lossy().to_string()),
            timeout: Some(60),
            table_prefix: "__qop".to_string(),
        }),
    }
}
