use {
    super::{
        command::{
            ApplyOperation,
            DownOptions,
            HistoryOperation,
            Operation,
            UpOptions,
        },
        console::Console,
        migration::MigrationTree,
        model::{
            ApplyRequest,
            Direction,
            MigrationId,
            MigrationStatus,
            OutputFormat,
            RevertRequest,
            RunMode,
            ScriptSource,
            out_of_order,
        },
        outcome::blocked,
        repo::MigrationRepository,
    },
    anyhow::{
        Context,
        Result,
    },
    chrono::{
        TimeZone,
        Utc,
    },
    std::collections::BTreeSet,
};

pub struct MigrationService<R: MigrationRepository> {
    repo: R,
}

impl<R: MigrationRepository> MigrationService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, command: Operation) -> Result<()> {
        match command {
            | Operation::Init => self.init().await,
            | Operation::New { .. } => anyhow::bail!("creating a migration file does not use a database connection"),
            | Operation::Up(options) => self.up(options).await,
            | Operation::Down(options) => self.down(options).await,
            | Operation::Apply(operation) => self.apply(operation).await,
            | Operation::List { output } => self.list(output).await,
            | Operation::History(HistoryOperation::Sync) => self.history_sync().await,
            | Operation::History(HistoryOperation::Fix) => self.history_fix().await,
            | Operation::Diff => self.diff().await,
        }
    }

    async fn init(&self) -> Result<()> {
        self.repo.init_store().await?;
        println!("Initialized migration tables.");
        Ok(())
    }

    async fn up(&self, options: UpOptions) -> Result<()> {
        let tree = self.tree()?;
        let local = tree.list_ids()?;
        let applied = self.repo.fetch_applied_ids().await?;
        let mut pending: Vec<MigrationId> = local.difference(&applied).cloned().collect();
        if let Some(count) = options.count {
            pending.truncate(count);
        }
        if pending.is_empty() {
            println!("All migrations are up to date.");
            return Ok(());
        }
        self.confirm_linear(&applied, &pending, options.yes)?;

        let kind = self.repo.script_files();
        let mut planned = Vec::with_capacity(pending.len());
        for id in &pending {
            let (scripts, meta) = tree.read(id, kind)?;
            planned.push((id.clone(), scripts, meta));
        }
        let preview = || -> Result<()> {
            for (id, scripts, _) in &planned {
                Console::preview_script(id, &scripts.up, Direction::Up)?;
            }
            Ok(())
        };
        self.confirm_batch(options.preview, options.yes, &pending, "apply", &preview)?;

        let mut previous = self.repo.fetch_last_id().await?;
        let mut applied_count = 0usize;
        for (id, scripts, meta) in planned {
            self.repo
                .apply_migration(&ApplyRequest {
                    previous: previous.clone(),
                    comment: meta.comment,
                    locked: meta.locked,
                    timeout: options.timeout,
                    mode: options.mode,
                    id: id.clone(),
                    scripts,
                })
                .await?;
            previous = Some(id);
            applied_count += 1;
        }
        Console::report(applied_count, "applied", options.mode);
        Ok(())
    }

    async fn down(&self, options: DownOptions) -> Result<()> {
        let applied = self.repo.fetch_applied_ids().await?;
        if applied.is_empty() {
            println!("No migrations applied.");
            return Ok(());
        }
        let targets: Vec<MigrationId> = applied.iter().rev().take(options.count).cloned().collect();
        if targets.is_empty() {
            println!("Nothing to revert.");
            return Ok(());
        }
        let scripts = self.down_scripts(&targets, options.source).await?;
        let preview = || -> Result<()> {
            for (id, sql) in &scripts {
                Console::preview_script(id, sql, Direction::Down)?;
            }
            Ok(())
        };
        self.confirm_batch(options.preview, options.yes, &targets, "revert", &preview)?;
        for (id, down) in scripts {
            self.repo
                .revert_migration(&RevertRequest {
                    timeout: options.timeout,
                    mode: options.mode,
                    unlock: options.unlock,
                    id,
                    down,
                })
                .await?;
        }
        Console::report(targets.len(), "reverted", options.mode);
        Ok(())
    }

    async fn apply(&self, operation: ApplyOperation) -> Result<()> {
        match operation {
            | ApplyOperation::Up {
                id,
                timeout,
                mode,
                yes,
                locked,
            } => self.apply_up(id, timeout, mode, yes, locked).await,
            | ApplyOperation::Down {
                id,
                timeout,
                source,
                mode,
                yes,
                unlock,
            } => self.apply_down(id, timeout, source, mode, yes, unlock).await,
        }
    }

    async fn apply_up(
        &self,
        id: MigrationId,
        timeout: Option<u64>,
        mode: RunMode,
        yes: bool,
        locked: bool,
    ) -> Result<()> {
        let tree = self.tree()?;
        let applied = self.repo.fetch_applied_ids().await?;
        if applied.contains(&id) {
            anyhow::bail!("Migration {id} is already applied");
        }
        if !tree.list_ids()?.contains(&id) {
            anyhow::bail!("Migration {id} does not exist locally");
        }
        self.confirm_linear(&applied, std::slice::from_ref(&id), yes)?;
        let (scripts, meta) = tree.read(&id, self.repo.script_files())?;
        let preview = || Console::preview_script(&id, &scripts.up, Direction::Up);
        self.confirm_one(yes, &format!("❓ Do you want to apply migration '{id}'?"), &preview)?;
        let previous = self.repo.fetch_last_id().await?;
        self.repo
            .apply_migration(&ApplyRequest {
                locked: locked || meta.locked,
                comment: meta.comment,
                id: id.clone(),
                scripts,
                previous,
                timeout,
                mode,
            })
            .await?;
        Console::report(1, "applied", mode);
        Ok(())
    }

    async fn apply_down(
        &self,
        id: MigrationId,
        timeout: Option<u64>,
        source: ScriptSource,
        mode: RunMode,
        yes: bool,
        unlock: bool,
    ) -> Result<()> {
        let applied = self.repo.fetch_applied_ids().await?;
        if !applied.contains(&id) {
            anyhow::bail!("Migration {id} is not currently applied");
        }
        if let Some(latest) = applied.iter().next_back()
            && latest != &id
        {
            Console::warn_out_of_order(&[&id], latest);
            if !Console::confirm("Do you want to continue?", yes, None)? {
                return Err(blocked("Operation cancelled."));
            }
        }
        let down = self.down_script(&id, source).await?;
        let preview = || Console::preview_script(&id, &down, Direction::Down);
        self.confirm_one(yes, &format!("❓ Do you want to revert migration '{id}'?"), &preview)?;
        self.repo
            .revert_migration(&RevertRequest {
                id: id.clone(),
                down,
                timeout,
                mode,
                unlock,
            })
            .await?;
        Console::report(1, "reverted", mode);
        Ok(())
    }

    async fn list(&self, output: OutputFormat) -> Result<()> {
        let history = self.repo.fetch_history().await?;
        let tree = self.tree()?;
        let local = tree.list_ids()?;
        let rows = MigrationStatus::merge(&local, &history, |id| tree.read_meta(id))?;
        match output {
            | OutputFormat::Human => {
                if rows.is_empty() {
                    println!("No migrations found.");
                    return Ok(());
                }
                Console::render_table(&rows)
            },
            | OutputFormat::Json => {
                #[derive(serde::Serialize)]
                struct RowOut {
                    id: String,
                    remote: Option<chrono::DateTime<Utc>>,
                    local: bool,
                    comment: Option<String>,
                    locked: bool,
                }
                let rows = rows
                    .into_iter()
                    .map(|row| {
                        RowOut {
                            id: row.id.to_string(),
                            remote: row.applied_at.map(|naive| Utc.from_utc_datetime(&naive)),
                            local: row.local,
                            comment: row.comment,
                            locked: row.locked,
                        }
                    })
                    .collect::<Vec<_>>();
                println!("{}", serde_json::to_string_pretty(&rows)?);
                Ok(())
            },
        }
    }

    async fn diff(&self) -> Result<()> {
        let tree = self.tree()?;
        let local = tree.list_ids()?;
        let applied = self.repo.fetch_applied_ids().await?;
        let pending: Vec<MigrationId> = local.difference(&applied).cloned().collect();
        if pending.is_empty() {
            println!("All migrations are up to date.");
            return Ok(());
        }
        let kind = self.repo.script_files();
        for id in pending {
            let (scripts, _) = tree.read(&id, kind)?;
            Console::print_script(&id, &scripts.up, Direction::Up)?;
        }
        Ok(())
    }

    async fn history_fix(&self) -> Result<()> {
        let tree = self.tree()?;
        let local = tree.list_ids()?;
        let applied = self.repo.fetch_applied_ids().await?;
        let Some(latest) = applied.iter().next_back() else {
            println!("No out-of-order migrations to fix.");
            return Ok(());
        };
        let pending: Vec<MigrationId> = local.difference(&applied).filter(|id| *id < latest).cloned().collect();
        if pending.is_empty() {
            println!("No out-of-order migrations to fix.");
            return Ok(());
        }
        let mut next = applied
            .iter()
            .map(MigrationId::millis)
            .max()
            .unwrap_or(0)
            .max(chrono::Utc::now().timestamp_millis());
        for old in pending {
            next = next
                .checked_add(1)
                .context("migration id overflow while fixing history")?;
            let new_id = MigrationId::from_millis(next)?;
            tree.rename(&old, &new_id)?;
            println!("Shuffled migration {old} to {new_id}");
        }
        Ok(())
    }

    async fn history_sync(&self) -> Result<()> {
        let stored = self.repo.fetch_stored().await?;
        if stored.is_empty() {
            println!("No migrations to sync.");
            return Ok(());
        }
        let tree = self.tree()?;
        let kind = self.repo.script_files();
        for migration in stored {
            let id = migration.id.clone();
            tree.write_synced(&migration, kind)?;
            println!("Synced migration: {id}");
        }
        Ok(())
    }

    fn tree(&self) -> Result<MigrationTree> {
        MigrationTree::from_config_path(self.repo.config_path())
    }

    fn confirm_linear(&self, applied: &BTreeSet<MigrationId>, pending: &[MigrationId], yes: bool) -> Result<()> {
        let unordered = out_of_order(applied, pending);
        let Some(latest) = applied.iter().next_back() else {
            return Ok(());
        };
        if unordered.is_empty() {
            return Ok(());
        }
        Console::warn_out_of_order(&unordered, latest);
        if !Console::confirm("Do you want to continue?", yes, None)? {
            return Err(blocked("Operation cancelled."));
        }
        Ok(())
    }

    fn confirm_batch(
        &self,
        preview_first: bool,
        yes: bool,
        ids: &[MigrationId],
        verb: &str,
        preview: &dyn Fn() -> Result<()>,
    ) -> Result<()> {
        eprintln!("\n📋 About to {verb} {} migration(s):", ids.len());
        for id in ids {
            eprintln!("  - {id}");
        }
        if preview_first {
            preview()?;
        }
        if !Console::confirm(
            &format!("❓ Do you want to proceed with {verb}ing these migrations?"),
            yes,
            Some(preview),
        )? {
            return Err(blocked(format!("Migration {verb} cancelled.")));
        }
        Ok(())
    }

    fn confirm_one(&self, yes: bool, message: &str, preview: &dyn Fn() -> Result<()>) -> Result<()> {
        if !Console::confirm(message, yes, Some(preview))? {
            return Err(blocked("Operation cancelled."));
        }
        Ok(())
    }

    async fn down_scripts(&self, ids: &[MigrationId], source: ScriptSource) -> Result<Vec<(MigrationId, String)>> {
        let mut scripts = Vec::with_capacity(ids.len());
        for id in ids {
            scripts.push((id.clone(), self.down_script(id, source).await?));
        }
        Ok(scripts)
    }

    async fn down_script(&self, id: &MigrationId, source: ScriptSource) -> Result<String> {
        match source {
            | ScriptSource::Remote => self.repo.fetch_down_script(id).await,
            | ScriptSource::Local => {
                let (scripts, _) = self.tree()?.read(id, self.repo.script_files())?;
                Ok(scripts.down)
            },
        }
    }
}
