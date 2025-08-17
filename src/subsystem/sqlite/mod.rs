pub mod commands;
pub mod migration;
#[cfg(feature = "sqlite")]
pub mod repo;

#[cfg(feature = "sqlite")]
pub fn build_sample(path: &std::path::Path) -> crate::config::Config {
    use crate::config::{Config, Subsystem, SubsystemSqlite, DataSource};
    let db_path = path.parent().unwrap_or_else(|| std::path::Path::new(".")).join("test.db");
    Config {
        version: env!("CARGO_PKG_VERSION").to_string(),
        subsystem: Subsystem::Sqlite(SubsystemSqlite {
            connection: DataSource::Static(db_path.to_string_lossy().to_string()),
            timeout: Some(60),
            table: "__qop".to_string(),
        }),
    }
}