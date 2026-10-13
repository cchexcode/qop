use {
    crate::{
        config::ensure_store_compatible,
        core::{
            model::{
                AppliedMigration,
                ApplyRequest,
                MigrationId,
                RevertRequest,
                RunMode,
                ScriptFiles,
                StoredMigration,
            },
            outcome::blocked,
            repo::MigrationRepository,
        },
        subsystem::postgres::config::SubsystemPostgres,
    },
    anyhow::{
        Context,
        Result,
    },
    sqlx::{
        Pool,
        Postgres,
        QueryBuilder,
        Row,
        postgres::PgPoolOptions,
    },
    std::{
        collections::BTreeSet,
        path::{
            Path,
            PathBuf,
        },
    },
};

pub struct PostgresRepo {
    config: SubsystemPostgres,
    pool: Pool<Postgres>,
    path: PathBuf,
}

impl PostgresRepo {
    pub async fn from_config(path: &Path, config: SubsystemPostgres, check_cli_version: bool) -> Result<Self> {
        let context = format!("[subsystem.postgres].connection in {}", path.display());
        let uri = config.connection.resolve(&context)?;
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&uri)
            .await
            .with_context(|| format!("Failed to connect to PostgreSQL using {}", path.display()))?;
        let repo = Self {
            config,
            pool,
            path: path.to_path_buf(),
        };
        if check_cli_version {
            repo.ensure_store_version().await?;
        }
        Ok(repo)
    }

    fn timeout(&self, provided: Option<u64>) -> Option<u64> {
        provided.or(self.config.timeout)
    }

    fn qualified(&self, table: &str) -> Result<String> {
        Ok(format!(
            "{}.{}",
            super::sql::quote_ident(&self.config.schema)?,
            super::sql::quote_ident(table)?
        ))
    }

    fn migrations(&self) -> Result<String> {
        self.qualified(&self.config.tables.migrations)
    }

    fn log_table(&self) -> Result<String> {
        self.qualified(&self.config.tables.log)
    }

    async fn ensure_store_version(&self) -> Result<()> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema = $1 AND table_name = $2)",
        )
        .bind(&self.config.schema)
        .bind(&self.config.tables.migrations)
        .fetch_one(&self.pool)
        .await
        .context("Failed to check whether the migrations table exists")?;
        if !exists {
            return Ok(());
        }
        let table = self.migrations()?;
        let version: Option<String> =
            sqlx::query_scalar(&format!("SELECT version FROM {table} ORDER BY id DESC LIMIT 1"))
                .fetch_optional(&self.pool)
                .await
                .context("Failed to read the migration store version")?;
        if let Some(version) = version {
            ensure_store_compatible(&version)?;
        }
        Ok(())
    }

    async fn set_timeout(tx: &mut sqlx::Transaction<'_, Postgres>, seconds: Option<u64>) -> Result<()> {
        let Some(seconds) = seconds else {
            return Ok(());
        };
        let millis = super::sql::timeout_millis(seconds)?;
        // SET does not accept bind parameters; millis is a validated integer.
        sqlx::query(&format!("SET LOCAL statement_timeout = {millis}"))
            .execute(&mut **tx)
            .await
            .context("Failed to set statement_timeout")?;
        Ok(())
    }

    async fn execute_sql(tx: &mut sqlx::Transaction<'_, Postgres>, sql: &str, id: &MigrationId) -> Result<()> {
        sqlx::raw_sql(sql)
            .execute(&mut **tx)
            .await
            .with_context(|| format!("Failed to execute statements in migration {id}"))?;
        Ok(())
    }

    async fn insert_log(
        tx: &mut sqlx::Transaction<'_, Postgres>,
        table: &str,
        id: &MigrationId,
        operation: &str,
        sql_command: &str,
    ) -> Result<()> {
        let mut query = QueryBuilder::new("INSERT INTO ");
        query.push(table);
        query.push(" (id, migration_id, operation, sql_command) VALUES (");
        query.push_bind(uuid::Uuid::now_v7().to_string());
        query.push(", ");
        query.push_bind(id.as_str());
        query.push(", ");
        query.push_bind(operation);
        query.push(", ");
        query.push_bind(sql_command);
        query.push(")");
        query
            .build()
            .execute(&mut **tx)
            .await
            .with_context(|| format!("Failed to write audit log for migration {id}"))?;
        Ok(())
    }

    async fn is_locked(&self, tx: &mut sqlx::Transaction<'_, Postgres>, id: &MigrationId) -> Result<bool> {
        let mut query = QueryBuilder::new("SELECT locked FROM ");
        query.push(self.migrations()?);
        query.push(" WHERE id = ");
        query.push_bind(id.as_str());
        let locked = query
            .build()
            .fetch_optional(&mut **tx)
            .await
            .with_context(|| format!("Failed to read lock status for migration {id}"))?
            .map(|row| row.get::<bool, _>("locked"))
            .unwrap_or(false);
        Ok(locked)
    }

    fn parse_id(raw: String) -> Result<MigrationId> {
        MigrationId::parse(&raw).with_context(|| format!("Migration table contains invalid id '{raw}'"))
    }
}

#[async_trait::async_trait(?Send)]
impl MigrationRepository for PostgresRepo {
    async fn init_store(&self) -> Result<()> {
        let mut tx = self.pool.begin().await.context("Failed to begin transaction")?;
        let migrations = self.migrations()?;
        let log = self.log_table()?;
        sqlx::query(&format!(
            "CREATE TABLE IF NOT EXISTS {migrations} (id VARCHAR PRIMARY KEY, version VARCHAR NOT NULL, up VARCHAR \
             NOT NULL, down VARCHAR NOT NULL, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, pre VARCHAR, \
             comment VARCHAR, locked BOOLEAN NOT NULL DEFAULT FALSE)"
        ))
        .execute(&mut *tx)
        .await
        .context("Failed to create the migrations table")?;
        sqlx::query(&format!(
            "CREATE TABLE IF NOT EXISTS {log} (id VARCHAR PRIMARY KEY, migration_id VARCHAR NOT NULL, operation \
             VARCHAR NOT NULL, sql_command TEXT NOT NULL, executed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP)"
        ))
        .execute(&mut *tx)
        .await
        .context("Failed to create the log table")?;
        tx.commit().await.context("Failed to commit transaction")?;
        Ok(())
    }

    async fn fetch_applied_ids(&self) -> Result<BTreeSet<MigrationId>> {
        let table = self.migrations()?;
        let rows: Vec<String> = sqlx::query_scalar(&format!("SELECT id FROM {table} ORDER BY id ASC"))
            .fetch_all(&self.pool)
            .await
            .context("Failed to read applied migrations")?;
        rows.into_iter().map(Self::parse_id).collect()
    }

    async fn fetch_last_id(&self) -> Result<Option<MigrationId>> {
        let table = self.migrations()?;
        let id: Option<String> = sqlx::query_scalar(&format!("SELECT id FROM {table} ORDER BY id DESC LIMIT 1"))
            .fetch_optional(&self.pool)
            .await
            .context("Failed to read the latest migration id")?;
        id.map(Self::parse_id).transpose()
    }

    async fn apply_migration(&self, request: &ApplyRequest) -> Result<()> {
        let mut tx = self.pool.begin().await.context("Failed to begin transaction")?;
        Self::set_timeout(&mut tx, self.timeout(request.timeout)).await?;
        Self::execute_sql(&mut tx, &request.scripts.up, &request.id).await?;
        let table = self.migrations()?;
        let mut query = QueryBuilder::new("INSERT INTO ");
        query.push(&table);
        query.push(" (id, version, up, down, comment, pre, locked) VALUES (");
        query.push_bind(request.id.as_str());
        query.push(", ");
        query.push_bind(env!("CARGO_PKG_VERSION"));
        query.push(", ");
        query.push_bind(&request.scripts.up);
        query.push(", ");
        query.push_bind(&request.scripts.down);
        query.push(", ");
        query.push_bind(request.comment.as_deref());
        query.push(", ");
        query.push_bind(request.previous.as_ref().map(MigrationId::as_str));
        query.push(", ");
        query.push_bind(request.locked);
        query.push(")");
        query
            .build()
            .execute(&mut *tx)
            .await
            .with_context(|| format!("Failed to record migration {}", request.id))?;
        Self::insert_log(&mut tx, &self.log_table()?, &request.id, "up", &request.scripts.up).await?;
        finish(tx, request.mode).await
    }

    async fn revert_migration(&self, request: &RevertRequest) -> Result<()> {
        let mut tx = self.pool.begin().await.context("Failed to begin transaction")?;
        Self::set_timeout(&mut tx, self.timeout(request.timeout)).await?;
        if self.is_locked(&mut tx, &request.id).await? && !request.unlock {
            tx.rollback().await.context("Failed to roll back transaction")?;
            return Err(blocked(format!(
                "Migration {} is locked and cannot be reverted without --unlock",
                request.id
            )));
        }
        Self::execute_sql(&mut tx, &request.down, &request.id).await?;
        let mut query = QueryBuilder::new("DELETE FROM ");
        query.push(self.migrations()?);
        query.push(" WHERE id = ");
        query.push_bind(request.id.as_str());
        query
            .build()
            .execute(&mut *tx)
            .await
            .with_context(|| format!("Failed to delete migration {}", request.id))?;
        Self::insert_log(&mut tx, &self.log_table()?, &request.id, "down", &request.down).await?;
        finish(tx, request.mode).await
    }

    async fn fetch_history(&self) -> Result<Vec<AppliedMigration>> {
        let table = self.migrations()?;
        let rows = sqlx::query(&format!(
            "SELECT id, created_at, comment, locked FROM {table} ORDER BY id ASC"
        ))
        .fetch_all(&self.pool)
        .await
        .context("Failed to read migration history")?;
        rows.into_iter()
            .map(|row| {
                Ok(AppliedMigration {
                    id: Self::parse_id(row.get("id"))?,
                    applied_at: row.get("created_at"),
                    comment: row.get("comment"),
                    locked: row.get("locked"),
                })
            })
            .collect()
    }

    async fn fetch_down_script(&self, id: &MigrationId) -> Result<String> {
        let table = self.migrations()?;
        let mut query = QueryBuilder::new("SELECT down FROM ");
        query.push(table);
        query.push(" WHERE id = ");
        query.push_bind(id.as_str());
        let row = query
            .build()
            .fetch_optional(&self.pool)
            .await
            .with_context(|| format!("Failed to read the down script for migration {id}"))?;
        row.map(|row| row.get("down"))
            .ok_or_else(|| anyhow::anyhow!("Migration {id} has no down script in the database"))
    }

    async fn fetch_stored(&self) -> Result<Vec<StoredMigration>> {
        let table = self.migrations()?;
        let rows = sqlx::query(&format!(
            "SELECT id, up, down, comment, locked FROM {table} ORDER BY id ASC"
        ))
        .fetch_all(&self.pool)
        .await
        .context("Failed to read stored migrations")?;
        rows.into_iter()
            .map(|row| {
                Ok(StoredMigration {
                    id: Self::parse_id(row.get("id"))?,
                    up: row.get("up"),
                    down: row.get("down"),
                    comment: row.get("comment"),
                    locked: row.get("locked"),
                })
            })
            .collect()
    }

    fn config_path(&self) -> &Path {
        &self.path
    }

    fn script_files(&self) -> ScriptFiles {
        super::SCRIPTS
    }
}

async fn finish(tx: sqlx::Transaction<'_, Postgres>, mode: RunMode) -> Result<()> {
    match mode {
        | RunMode::DryRun => tx.rollback().await.context("Failed to roll back dry-run")?,
        | RunMode::Apply => tx.commit().await.context("Failed to commit transaction")?,
    }
    Ok(())
}
