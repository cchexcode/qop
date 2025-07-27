use {
    anyhow::{Context, Result},
    chrono::Utc,
    std::{
        collections::HashSet,
        path::Path,
    },
};

/// Normalize migration ID to include "id=" prefix if not present
pub fn normalize_migration_id(id: &str) -> String {
    if id.starts_with("id=") {
        id.to_string()
    } else {
        format!("id={}", id)
    }
}

/// Get local migrations from directory by scanning for "id=" prefixed directories
pub fn get_local_migrations(path: &Path) -> Result<HashSet<String>> {
    let migration_dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid migration path: {}", path.display()))?;
    Ok(std::fs::read_dir(migration_dir)
        .with_context(|| format!("Failed to read migration directory: {}", migration_dir.display()))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().ok()?.is_dir()
                && entry.file_name().to_string_lossy().starts_with("id=")
            {
                Some(entry.file_name().to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .collect())
}

/// Create a new migration directory with timestamp-based ID
pub fn create_migration_directory(path: &Path) -> Result<std::path::PathBuf> {
    let id = Utc::now().timestamp_millis().to_string();
    let migration_path = path.parent().unwrap();
    let migration_id_path = migration_path.join(format!("id={}", id));
    std::fs::create_dir_all(&migration_id_path).with_context(|| {
        format!("Failed to create directory: {}", migration_id_path.display())
    })?;
    
    let up_path = migration_id_path.join("up.sql");
    let down_path = migration_id_path.join("down.sql");
    std::fs::write(&up_path, "-- SQL goes here").with_context(|| {
        format!("Failed to write up migration: {}", up_path.display())
    })?;
    std::fs::write(&down_path, "-- SQL goes here").with_context(|| {
        format!("Failed to write down migration: {}", down_path.display())
    })?;
    
    Ok(migration_id_path)
}

/// Read migration SQL files for a given migration ID
pub fn read_migration_files(migration_dir: &Path, migration_id: &str) -> Result<(String, String)> {
    let migration_path = migration_dir.join(migration_id);
    let up_sql_path = migration_path.join("up.sql");
    let down_sql_path = migration_path.join("down.sql");

    let up_sql = std::fs::read_to_string(&up_sql_path).with_context(
        || format!("Failed to read up migration: {}", up_sql_path.display()),
    )?;
    let down_sql = std::fs::read_to_string(&down_sql_path).with_context(
        || {
            format!(
                "Failed to read down migration: {}",
                down_sql_path.display()
            )
        },
    )?;
    
    Ok((up_sql, down_sql))
}

/// Check if migration should be warned about for non-linear history
pub fn check_non_linear_history(
    applied_migrations: &HashSet<String>,
    migrations_to_apply: &[String],
) -> Vec<String> {
    if applied_migrations.is_empty() || migrations_to_apply.is_empty() {
        return Vec::new();
    }
    
    let max_applied_migration = applied_migrations.iter().max().cloned().unwrap_or_default();
    
    migrations_to_apply
        .iter()
        .filter(|id| id.as_str() < max_applied_migration.as_str())
        .cloned()
        .collect()
}

/// Display non-linear history warning and get user confirmation
pub fn handle_non_linear_warning(out_of_order_migrations: &[String], max_applied: &str) -> Result<bool> {
    use std::io::{self, Write};
    
    if out_of_order_migrations.is_empty() {
        return Ok(true);
    }
    
    println!("⚠️  Non-linear history detected!");
    println!("The following migrations would create a non-linear history:");
    for migration in out_of_order_migrations {
        println!("  - {}", migration);
    }
    println!("Latest applied migration: {}", max_applied);
    println!();
    println!("This could cause issues with database schema consistency.");
    println!("Alternatively, you can run history fix to rename out-of-order migrations.");
    
    print!("Do you want to continue? [y/N]: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();
    
    Ok(input == "y" || input == "yes")
}

/// Print migration application results
pub fn print_migration_results(applied_count: usize, action: &str) {
    if applied_count > 0 {
        println!("\n🎉 Successfully {} {} migration(s)!", action, applied_count);
    }
}