#[cfg(not(any(feature = "sub+postgres", feature = "sub+sqlite")))]
compile_error!("At least one subsystem feature must be enabled: 'postgres' or 'sqlite'.");

#[cfg(feature = "sub+postgres")]
pub mod postgres;
#[cfg(feature = "sub+sqlite")]
pub mod sqlite;
pub mod driver;
pub mod prelude {
    pub use crate::core::{repo::MigrationRepository, service::MigrationService};
}