use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, ContentArrangement, Table, CellAlignment};
use std::collections::BTreeMap;
use chrono::{DateTime, TimeZone, Utc};
use {
    crate::core::migration as util,
    super::repo::MigrationRepository,
    anyhow::Result,
    std::path::Path,
};

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Human,
    Json,
}

pub struct MigrationService<R: MigrationRepository> {
    repo: R,
}

impl<R: MigrationRepository> MigrationService<R> {
    pub fn new(repo: R) -> Self { Self { repo } }

    pub async fn init(&self) -> Result<()> {
        self.repo.init_store().await
    }

    pub async fn new_migration(&self, path: &Path) -> Result<()> {
        let migration_id_path = util::create_migration_directory(path)?;
        println!("Created new migration: {}", migration_id_path.display());
        Ok(())
    }

    pub async fn apply_up(&self, path: &Path, id: &str, timeout: Option<u64>, yes: bool, dry_run: bool) -> Result<()> {
        let migration_dir = path.parent().ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
        let target_id = util::normalize_migration_id(id);
        let (up_sql, down_sql) = util::read_migration_files(migration_dir, &target_id)?;

        let diff_fn = || -> Result<()> { util::display_sql_migration(&target_id, &up_sql, "UP") };
        if !util::prompt_for_confirmation_with_diff(&format!("❓ Do you want to apply migration '{}'?",&target_id), yes, diff_fn)? {
            println!("❌ Migration cancelled.");
            return Ok(())
        }

        let pre = self.repo.fetch_last_id().await?;
        self.repo.apply_migration(&target_id, &up_sql, &down_sql, pre.as_deref(), timeout, dry_run).await?;
        util::print_migration_results(1, "applied");
        Ok(())
    }

    pub async fn apply_down(&self, path: &Path, id: &str, timeout: Option<u64>, remote: bool, yes: bool, dry_run: bool) -> Result<()> {
        let migration_dir = path.parent().ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
        let target_id = util::normalize_migration_id(id);
        let down_sql = if remote {
            self.repo.fetch_down_sql(&target_id).await?.unwrap_or_default()
        } else {
            let p = migration_dir.join(&target_id).join("down.sql");
            std::fs::read_to_string(&p)?
        };

        let diff_fn = || -> Result<()> { util::display_sql_migration(&target_id, &down_sql, "DOWN") };
        if !util::prompt_for_confirmation_with_diff(&format!("❓ Do you want to revert migration '{}'?",&target_id), yes, diff_fn)? {
            println!("❌ Revert cancelled.");
            return Ok(())
        }

        self.repo.revert_migration(&target_id, &down_sql, timeout, dry_run).await?;
        util::print_migration_results(1, "reverted");
        Ok(())
    }

    pub async fn list(&self, output: OutputFormat) -> Result<()> {
        let history = self.repo.fetch_history().await?;
        let local = util::get_local_migrations(self.repo.get_path())?;
        match output {
            OutputFormat::Human => {
                if history.is_empty() && local.is_empty() {
                    println!("No migrations found.");
                    return Ok(())
                }
                let mut all: BTreeMap<String, (Option<chrono::NaiveDateTime>, bool)> = BTreeMap::new();
                for id in &local {
                    let entry = all.entry(id.clone()).or_default();
                    entry.1 = true;
                }
                for (id, ts) in &history {
                    let entry = all.entry(id.clone()).or_default();
                    entry.0 = Some(*ts);
                }
                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL)
                    .apply_modifier(UTF8_ROUND_CORNERS)
                    .set_content_arrangement(ContentArrangement::Dynamic)
                    .set_header(vec![
                        Cell::new("Migration ID"),
                        Cell::new("Remote"),
                        Cell::new("Local"),
                    ]);
                for (id, (applied_at, is_local)) in all {
                    let remote_str = if let Some(ts) = applied_at {
                        let utc_dt = chrono::Local.from_utc_datetime(&ts);
                        utc_dt.format("%Y-%m-%d %H:%M:%S %Z").to_string()
                    } else { "❌".to_string() };
                    let local_str = if is_local { "✅" } else { "❌" };
                    table.add_row(vec![
                        Cell::new(id),
                        Cell::new(remote_str).set_alignment(CellAlignment::Center),
                        Cell::new(local_str).set_alignment(CellAlignment::Center),
                    ]);
                }
                println!("{table}");
                Ok(())
            }
            OutputFormat::Json => {
                #[derive(serde::Serialize)]
                struct RowOut {
                    id: String,
                    remote: Option<DateTime<Utc>>,
                    local: bool,
                }
                let mut all: BTreeMap<String, (Option<chrono::NaiveDateTime>, bool)> = BTreeMap::new();
                for id in &local {
                    let entry = all.entry(id.clone()).or_default();
                    entry.1 = true;
                }
                for (id, ts) in &history {
                    let entry = all.entry(id.clone()).or_default();
                    entry.0 = Some(*ts);
                }
                let mut rows: Vec<RowOut> = Vec::new();
                for (id, (applied_at, is_local)) in all {
                    rows.push(RowOut { 
                        id, 
                        remote: applied_at.map(|naive| Utc.from_utc_datetime(&naive)), 
                        local: is_local 
                    });
                }
                println!("{}", serde_json::to_string_pretty(&rows)?);
                Ok(())
            }
        }
    }

    pub async fn up(&self, path: &Path, timeout: Option<u64>, count: Option<usize>, yes: bool, dry_run: bool) -> Result<()> {
        let local = util::get_local_migrations(path)?;
        let applied = self.repo.fetch_applied_ids().await?;

        let mut to_apply: Vec<String> = local.difference(&applied).cloned().collect();
        to_apply.sort();
        if let Some(c) = count { to_apply.truncate(c); }

        if to_apply.is_empty() {
            println!("All migrations are up to date.");
            return Ok(())
        }

        // Non-linear warning
        let out_of_order = util::check_non_linear_history(&applied, &to_apply);
        if !out_of_order.is_empty() {
            let max_applied = applied.iter().max().cloned().unwrap_or_default();
            if !util::handle_non_linear_warning(&out_of_order, &max_applied)? { 
                println!("Operation cancelled.");
                return Ok(())
            }
        }

        // Confirm
        println!("\n📋 About to apply {} migration(s):", to_apply.len());
        for id in &to_apply { println!("  - {}", id); }
        let migration_dir = path.parent().ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
        let to_apply_for_diff = to_apply.clone();
        let diff_fn = move || -> Result<()> {
            for id in &to_apply_for_diff {
                let (up_sql, _down) = util::read_migration_files(migration_dir, id)?;
                util::display_sql_migration(id, &up_sql, "UP")?;
            }
            Ok(())
        };
        if !util::prompt_for_confirmation_with_diff("❓ Do you want to proceed with applying these migrations?", yes, diff_fn)? {
            println!("❌ Migration cancelled.");
            return Ok(())
        }

        let mut previous: Option<String> = self.repo.fetch_last_id().await?;
        let mut applied_count = 0usize;
        for id in to_apply {
            let (up_sql, down_sql) = util::read_migration_files(migration_dir, &id)?;
            self.repo.apply_migration(&id, &up_sql, &down_sql, previous.as_deref(), timeout, dry_run).await?;
            previous = Some(id.clone());
            applied_count += 1;
        }

        util::print_migration_results(applied_count, "applied");
        Ok(())
    }

    pub async fn down(&self, path: &Path, timeout: Option<u64>, count: Option<usize>, remote: bool, yes: bool, dry_run: bool) -> Result<()> {
        let applied = self.repo.fetch_applied_ids().await?;
        if applied.is_empty() {
            println!("No migrations applied.");
            return Ok(())
        }
        let mut applied_sorted: Vec<String> = applied.into_iter().collect();
        applied_sorted.sort();
        applied_sorted.reverse();
        let take_n = count.unwrap_or(1);
        let targets: Vec<String> = applied_sorted.into_iter().take(take_n).collect();

        if targets.is_empty() { println!("Nothing to revert."); return Ok(()) }

        let migration_dir = path.parent().ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
        let diff_fn = {
            let targets = targets.clone();
            move || -> Result<()> {
                for id in &targets {
                    let down_sql = if remote {
                        String::from("-- remote down sql omitted in preview")
                    } else {
                        let p = migration_dir.join(id).join("down.sql");
                        std::fs::read_to_string(&p)?
                    };
                    util::display_sql_migration(id, &down_sql, "DOWN")?;
                }
                Ok(())
            }
        };
        if !util::prompt_for_confirmation_with_diff("❓ Do you want to proceed with reverting these migrations?", yes, diff_fn)? {
            println!("❌ Revert cancelled.");
            return Ok(())
        }

        let mut reverted = 0usize;
        for id in targets {
            let down_sql = if remote {
                self.repo.fetch_down_sql(&id).await?.unwrap_or_default()
            } else {
                let p = migration_dir.join(&id).join("down.sql");
                std::fs::read_to_string(&p)?
            };
            self.repo.revert_migration(&id, &down_sql, timeout, dry_run).await?;
            reverted += 1;
        }

        util::print_migration_results(reverted, "reverted");
        Ok(())
    }
}


