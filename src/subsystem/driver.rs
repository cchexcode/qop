use {
    crate::args::Subsystem,
    anyhow::Result,
};

pub(crate) async fn dispatch(subsystem: Subsystem) -> Result<()> {
    match subsystem {
        #[cfg(feature = "sub+postgres")]
        | Subsystem::Postgres(invocation) => super::postgres::commands::execute(invocation).await,
        #[cfg(feature = "sub+sqlite")]
        | Subsystem::Sqlite(invocation) => super::sqlite::commands::execute(invocation).await,
        #[cfg(feature = "sub+surrealdb")]
        | Subsystem::Surrealdb(invocation) => super::surrealdb::commands::execute(invocation).await,
    }
}
