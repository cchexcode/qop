#[cfg(not(any(feature = "sub+postgres", feature = "sub+sqlite", feature = "sub+surrealdb")))]
compile_error!("At least one subsystem feature must be enabled: 'sub+postgres', 'sub+sqlite', or 'sub+surrealdb'.");

pub(crate) mod cli;
pub mod driver;
#[cfg(feature = "sub+postgres")]
pub mod postgres;
#[cfg(feature = "sub+sqlite")]
pub mod sqlite;
#[cfg(feature = "sub+surrealdb")]
pub mod surrealdb;
