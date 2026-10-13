use {
    super::model::{
        AppliedMigration,
        ApplyRequest,
        MigrationId,
        RevertRequest,
        ScriptFiles,
        StoredMigration,
    },
    anyhow::Result,
    std::{
        collections::BTreeSet,
        path::Path,
    },
};

#[async_trait::async_trait(?Send)]
pub trait MigrationRepository {
    async fn init_store(&self) -> Result<()>;
    async fn fetch_applied_ids(&self) -> Result<BTreeSet<MigrationId>>;
    async fn fetch_last_id(&self) -> Result<Option<MigrationId>>;
    async fn apply_migration(&self, request: &ApplyRequest) -> Result<()>;
    async fn revert_migration(&self, request: &RevertRequest) -> Result<()>;
    async fn fetch_history(&self) -> Result<Vec<AppliedMigration>>;
    async fn fetch_down_script(&self, id: &MigrationId) -> Result<String>;
    async fn fetch_stored(&self) -> Result<Vec<StoredMigration>>;
    fn config_path(&self) -> &Path;
    fn script_files(&self) -> ScriptFiles;
}
