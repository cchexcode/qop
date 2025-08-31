pub mod commands;
pub mod migration;
pub mod repo;
pub mod config;

#[cfg(feature = "sub+postgres")]
use crate::config::{Config, Subsystem, DataSource};
#[cfg(feature = "sub+postgres")]
use crate::subsystem::postgres::config::SubsystemPostgres;

#[cfg(feature = "sub+postgres")]
pub fn build_sample(connection: &str) -> crate::config::Config {
    Config {
        version: env!("CARGO_PKG_VERSION").to_string(),
        subsystem: Subsystem::Postgres(SubsystemPostgres {
            connection: DataSource::Static(connection.to_string()),
            timeout: Some(60),
            table_prefix: "__qop".to_string(),
            schema: "public".to_string(),
        }),
    }
}
