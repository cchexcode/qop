use {
    crate::{
        core::{
            model::{
                AppliedMigration,
                ApplyRequest,
                MigrationId,
                RevertRequest,
                ScriptFiles,
                StoredMigration,
            },
            repo::MigrationRepository,
        },
        subsystem::surrealdb::{
            config::SubsystemSurrealdb,
            migration as sd,
        },
    },
    anyhow::{
        Context,
        Result,
    },
    std::{
        collections::BTreeSet,
        path::{
            Path,
            PathBuf,
        },
    },
};

pub struct SurrealdbRepo {
    config: SubsystemSurrealdb,
    client: sd::SurrealdbClient,
    path: PathBuf,
}

impl SurrealdbRepo {
    pub async fn from_config(path: &Path, config: SubsystemSurrealdb, check_cli_version: bool) -> Result<Self> {
        let client = sd::build_client_from_config(path, &config, check_cli_version).await?;
        Ok(Self {
            config,
            client,
            path: path.to_path_buf(),
        })
    }

    fn parse_id(raw: String) -> Result<MigrationId> {
        MigrationId::parse(&raw).with_context(|| format!("Migration table contains invalid id '{raw}'"))
    }
}

#[async_trait::async_trait(?Send)]
impl MigrationRepository for SurrealdbRepo {
    async fn init_store(&self) -> Result<()> {
        sd::init_store(&self.client, &self.config).await
    }

    async fn fetch_applied_ids(&self) -> Result<BTreeSet<MigrationId>> {
        sd::get_applied_migrations(&self.client, &self.config.tables.migrations, self.config.timeout)
            .await?
            .into_iter()
            .map(Self::parse_id)
            .collect()
    }

    async fn fetch_last_id(&self) -> Result<Option<MigrationId>> {
        sd::get_last_migration_id(&self.client, &self.config.tables.migrations, self.config.timeout)
            .await?
            .map(Self::parse_id)
            .transpose()
    }

    async fn apply_migration(&self, request: &ApplyRequest) -> Result<()> {
        sd::apply_migration(&self.client, &self.config, request).await
    }

    async fn revert_migration(&self, request: &RevertRequest) -> Result<()> {
        sd::revert_migration(&self.client, &self.config, request).await
    }

    async fn fetch_history(&self) -> Result<Vec<AppliedMigration>> {
        let history =
            sd::get_migration_history(&self.client, &self.config.tables.migrations, self.config.timeout).await?;
        history
            .into_iter()
            .map(|(id, applied_at, comment, locked)| {
                Ok(AppliedMigration {
                    id: Self::parse_id(id)?,
                    applied_at,
                    comment,
                    locked,
                })
            })
            .collect()
    }

    async fn fetch_down_script(&self, id: &MigrationId) -> Result<String> {
        sd::get_migration_down_sql(
            &self.client,
            &self.config.tables.migrations,
            id.as_str(),
            self.config.timeout,
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("Migration {id} has no down script in the database"))
    }

    async fn fetch_stored(&self) -> Result<Vec<StoredMigration>> {
        let rows =
            sd::get_all_migration_data(&self.client, &self.config.tables.migrations, self.config.timeout).await?;
        rows.into_iter()
            .map(|(id, up, down, comment, locked)| {
                Ok(StoredMigration {
                    id: Self::parse_id(id)?,
                    up,
                    down,
                    comment,
                    locked,
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
