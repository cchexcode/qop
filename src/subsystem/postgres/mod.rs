pub mod commands;
pub mod migration;
pub mod repo;

#[cfg(feature = "postgres")]
pub fn build_sample() -> crate::config::Config {
    use crate::config::{Config, Subsystem, SubsystemPostgres, DataSource};
    Config {
        version: env!("CARGO_PKG_VERSION").to_string(),
        subsystem: Subsystem::Postgres(SubsystemPostgres {
            connection: DataSource::Static("postgres://user:password@localhost:5432/postgres".to_string()),
            timeout: Some(60),
            table: "__qop".to_string(),
            schema: "public".to_string(),
        }),
    }
}
