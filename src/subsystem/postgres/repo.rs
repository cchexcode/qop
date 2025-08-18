use {
    crate::core::repo::MigrationRepository,
    crate::subsystem::postgres::migration as pg,
    anyhow::Result,
    chrono::NaiveDateTime,
    sqlx::{Pool, Postgres, Row},
    std::collections::HashSet,
};

pub struct PostgresRepo {
    pub config: crate::subsystem::postgres::config::SubsystemPostgres,
    pub pool: Pool<Postgres>,
    pub path: std::path::PathBuf,
}

impl PostgresRepo {
    pub async fn from_path(path: &std::path::Path) -> Result<Self> {
        let config_content = std::fs::read_to_string(path)?;
        let with_version: crate::config::WithVersion = toml::from_str(&config_content)?;
        with_version.validate(env!("CARGO_PKG_VERSION"))?;
        let cfg: crate::config::Config = toml::from_str(&config_content)?;
        let subsystem = match cfg.subsystem { crate::config::Subsystem::Postgres(c) => c };
        let pool = pg::build_pool_from_config(path, &subsystem, true).await?;
        Ok(Self { config: subsystem, pool, path: path.to_path_buf() })
    }

    pub async fn from_config(path: &std::path::Path, config: crate::subsystem::postgres::config::SubsystemPostgres) -> Result<Self> {
        let pool = pg::build_pool_from_config(path, &config, true).await?;
        Ok(Self { config, pool, path: path.to_path_buf() })
    }
}

#[async_trait::async_trait(?Send)]
impl MigrationRepository for PostgresRepo {
    async fn init_store(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        {
            let mut query = pg::build_table_query("CREATE TABLE IF NOT EXISTS ", &self.config.schema, &self.config.table);
            query.push(" (id VARCHAR PRIMARY KEY, version VARCHAR NOT NULL, up VARCHAR NOT NULL, down VARCHAR NOT NULL, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, pre VARCHAR)");
            query.build().execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn fetch_applied_ids(&self) -> Result<HashSet<String>> {
        let mut tx = self.pool.begin().await?;
        let ids = pg::get_applied_migrations(&mut tx, &self.config.schema, &self.config.table).await?;
        tx.commit().await?;
        Ok(ids)
    }

    async fn fetch_last_id(&self) -> Result<Option<String>> {
        let mut tx = self.pool.begin().await?;
        let id = pg::get_last_migration_id(&mut tx, &self.config.schema, &self.config.table).await?;
        tx.commit().await?;
        Ok(id)
    }

    async fn apply_migration(&self, id: &str, up_sql: &str, down_sql: &str, pre: Option<&str>, timeout: Option<u64>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        pg::set_timeout_if_needed(&mut *tx, timeout).await?;

        pg::execute_sql_statements(&mut tx, up_sql, id).await?;
        pg::insert_migration_record(&mut *tx, &self.config.schema, &self.config.table, id, up_sql, down_sql, pre).await?;

        tx.commit().await?;
        Ok(())
    }

    async fn revert_migration(&self, id: &str, down_sql: &str, timeout: Option<u64>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        pg::set_timeout_if_needed(&mut *tx, timeout).await?;
        pg::execute_sql_statements(&mut tx, down_sql, id).await?;
        pg::delete_migration_record(&mut *tx, &self.config.schema, &self.config.table, id).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn fetch_history(&self) -> Result<Vec<(String, NaiveDateTime)>> {
        let mut tx = self.pool.begin().await?;
        let map = pg::get_migration_history(&mut tx, &self.config.schema, &self.config.table).await?;
        tx.commit().await?;
        let mut v: Vec<(String, NaiveDateTime)> = map.into_iter().collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(v)
    }

    async fn fetch_recent_for_revert_remote(&self) -> Result<Vec<(String, String)>> {
        let mut tx = self.pool.begin().await?;
        let rows = pg::get_recent_migrations_for_revert(&mut tx, &self.config.schema, &self.config.table).await?;
        tx.commit().await?;
        Ok(rows.into_iter().map(|row| (row.get("id"), row.get("down"))).collect())
    }

    async fn fetch_down_sql(&self, id: &str) -> Result<Option<String>> {
        let mut tx = self.pool.begin().await?;
        let sql = pg::get_migration_down_sql(&mut tx, &self.config.schema, &self.config.table, id).await.ok();
        tx.commit().await?;
        Ok(sql)
    }

    async fn fetch_all_migrations(&self) -> Result<Vec<(String, String, String)>> {
        let mut tx = self.pool.begin().await?;
        let rows = pg::get_all_migration_data(&mut tx, &self.config.schema, &self.config.table).await?;
        tx.commit().await?;
        Ok(rows.into_iter().map(|row| (row.get("id"), row.get("up"), row.get("down"))).collect())
    }

    fn get_path(&self) -> &std::path::Path { &self.path }
}


