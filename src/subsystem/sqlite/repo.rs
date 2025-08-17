use {
    crate::core::repo::MigrationRepository,
    crate::subsystem::sqlite::migration as sq,
    anyhow::Result,
    chrono::NaiveDateTime,
    sqlx::{Pool, Sqlite},
    sqlx::sqlite::SqliteRow,
    sqlx::Row,
    std::collections::HashSet,
};

pub struct SqliteRepo {
    pub config: crate::subsystem::sqlite::config::SubsystemSqlite,
    pub pool: Pool<Sqlite>,
    pub path: std::path::PathBuf,
}

impl SqliteRepo {
    pub async fn from_path(path: &std::path::Path) -> Result<Self> {
        let (config, pool) = sq::get_db_assets(path, true).await?;
        Ok(Self { config, pool, path: path.to_path_buf() })
    }
}

#[async_trait::async_trait(?Send)]
impl MigrationRepository for SqliteRepo {
    async fn init_store(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        {
            let mut query = sq::build_table_query("CREATE TABLE IF NOT EXISTS ", &self.config.table);
            query.push(" (id TEXT PRIMARY KEY, version TEXT NOT NULL, up TEXT NOT NULL, down TEXT NOT NULL, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, pre TEXT)");
            query.build().execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn fetch_applied_ids(&self) -> Result<HashSet<String>> {
        let mut tx = self.pool.begin().await?;
        let ids = sq::get_applied_migrations(&mut tx, &self.config.table).await?;
        tx.commit().await?;
        Ok(ids)
    }

    async fn fetch_last_id(&self) -> Result<Option<String>> {
        let mut tx = self.pool.begin().await?;
        let id = sq::get_last_migration_id(&mut tx, &self.config.table).await?;
        tx.commit().await?;
        Ok(id)
    }

    async fn apply_migration(&self, id: &str, up_sql: &str, down_sql: &str, pre: Option<&str>, timeout: Option<u64>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sq::set_timeout_if_needed(&mut *tx, timeout).await?;
        sq::execute_sql_statements(&mut tx, up_sql, id).await?;
        sq::insert_migration_record(&mut *tx, &self.config.table, id, up_sql, down_sql, pre).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn revert_migration(&self, id: &str, down_sql: &str, timeout: Option<u64>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sq::set_timeout_if_needed(&mut *tx, timeout).await?;
        sq::execute_sql_statements(&mut tx, down_sql, id).await?;
        sq::delete_migration_record(&mut *tx, &self.config.table, id).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn fetch_history(&self) -> Result<Vec<(String, NaiveDateTime)>> {
        let mut tx = self.pool.begin().await?;
        let map = sq::get_migration_history(&mut tx, &self.config.table).await?;
        tx.commit().await?;
        let mut v: Vec<(String, NaiveDateTime)> = map.into_iter().collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(v)
    }

    async fn fetch_recent_for_revert_remote(&self) -> Result<Vec<(String, String)>> {
        let mut tx = self.pool.begin().await?;
        let rows: Vec<SqliteRow> = sq::get_recent_migrations_for_revert(&mut tx, &self.config.table).await?;
        tx.commit().await?;
        Ok(rows.into_iter().map(|row| (row.get("id"), row.get("down"))).collect())
    }

    async fn fetch_down_sql(&self, id: &str) -> Result<Option<String>> {
        // fetch by reading file in local mode; SQLite path stores down text in table too but no single get function provided
        let mut tx = self.pool.begin().await?;
        let mut q = sqlx::QueryBuilder::new("SELECT down FROM ");
        q.push(&self.config.table);
        q.push(" WHERE id = ?");
        let row = q.build().bind(id).fetch_optional(&mut *tx).await?;
        tx.commit().await?;
        Ok(row.map(|r| r.get("down")))
    }

    fn get_path(&self) -> &std::path::Path { &self.path }
}


