use {
    super::model::{
        Direction,
        MigrationId,
        MigrationMeta,
        MigrationScripts,
        ScriptFiles,
        StoredMigration,
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

/// On-disk migration tree. The root is the directory that contains `qop.toml`.
pub struct MigrationTree {
    root: PathBuf,
}

impl MigrationTree {
    pub fn from_config_path(config: &Path) -> Result<Self> {
        let root = config
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| anyhow::anyhow!("Invalid migration path: {}", config.display()))?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    pub fn list_ids(&self) -> Result<BTreeSet<MigrationId>> {
        let entries = std::fs::read_dir(&self.root)
            .with_context(|| format!("Failed to read migration directory: {}", self.root.display()))?;
        let mut ids = BTreeSet::new();
        for entry in entries {
            let entry = entry.with_context(|| format!("Failed to read entry in {}", self.root.display()))?;
            let file_type = entry
                .file_type()
                .with_context(|| format!("Failed to read file type for {}", entry.path().display()))?;
            if !file_type.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let Some(raw) = name.strip_prefix("id=") else {
                continue;
            };
            ids.insert(MigrationId::parse(raw).with_context(|| format!("Invalid migration directory '{name}'"))?);
        }
        Ok(ids)
    }

    pub fn read(&self, id: &MigrationId, files: ScriptFiles) -> Result<(MigrationScripts, MigrationMeta)> {
        let dir = self.dir(id);
        let up = read_file(&dir.join(files.file_name(Direction::Up)))?;
        let down = read_file(&dir.join(files.file_name(Direction::Down)))?;
        Ok((MigrationScripts { up, down }, self.read_meta(id)?))
    }

    pub fn read_meta(&self, id: &MigrationId) -> Result<MigrationMeta> {
        let path = self.dir(id).join("meta.toml");
        if !path.exists() {
            return Ok(MigrationMeta::default());
        }
        let content = read_file(&path)?;
        toml::from_str(&content).with_context(|| format!("Failed to parse meta.toml: {}", path.display()))
    }

    pub fn create(&self, comment: Option<&str>, locked: bool, files: ScriptFiles) -> Result<PathBuf> {
        let id = MigrationId::from_millis(chrono::Utc::now().timestamp_millis())?;
        let dir = self.dir(&id);
        if dir.exists() {
            anyhow::bail!("Migration directory already exists: {}", dir.display());
        }
        std::fs::create_dir_all(&dir).with_context(|| format!("Failed to create directory: {}", dir.display()))?;
        let placeholder = files.placeholder();
        write_file(&dir.join(files.file_name(Direction::Up)), placeholder)?;
        write_file(&dir.join(files.file_name(Direction::Down)), placeholder)?;
        let meta = match comment {
            | Some(comment) => {
                MigrationMeta {
                    comment: Some(comment.to_string()),
                    locked,
                }
            },
            | None => {
                let mut meta = MigrationMeta::with_default_comment();
                meta.locked = locked;
                meta
            },
        };
        self.write_meta(&id, &meta)?;
        Ok(dir)
    }

    pub fn write_synced(&self, stored: &StoredMigration, files: ScriptFiles) -> Result<()> {
        let dir = self.dir(&stored.id);
        std::fs::create_dir_all(&dir).with_context(|| format!("Failed to create directory: {}", dir.display()))?;
        write_file(&dir.join(files.file_name(Direction::Up)), &stored.up)?;
        write_file(&dir.join(files.file_name(Direction::Down)), &stored.down)?;
        self.write_meta(&stored.id, &MigrationMeta {
            comment: stored.comment.clone(),
            locked: stored.locked,
        })
    }

    pub fn rename(&self, from: &MigrationId, to: &MigrationId) -> Result<()> {
        let old = self.dir(from);
        let new = self.dir(to);
        if new.exists() {
            anyhow::bail!("Refusing to overwrite {}", new.display());
        }
        std::fs::rename(&old, &new)
            .with_context(|| format!("Failed to rename migration from {} to {}", old.display(), new.display()))
    }

    fn write_meta(&self, id: &MigrationId, meta: &MigrationMeta) -> Result<()> {
        let path = self.dir(id).join("meta.toml");
        let content =
            toml::to_string(meta).with_context(|| format!("Failed to serialize meta.toml for migration {}", id))?;
        write_file(&path, &content)
    }

    fn dir(&self, id: &MigrationId) -> PathBuf {
        self.root.join(id.dir_name())
    }
}

fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    std::fs::write(path, contents).with_context(|| format!("Failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_read_and_reject_invalid_directory() {
        let root = std::env::temp_dir().join(format!("qop-tree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let config = root.join("qop.toml");
        std::fs::write(&config, "").unwrap();
        let tree = MigrationTree::from_config_path(&config).unwrap();
        let files = ScriptFiles::new("sql", "-- SQL goes here");
        let created = tree.create(Some("note"), true, files).unwrap();
        assert!(created.join("up.sql").is_file());
        assert!(created.join("down.sql").is_file());
        let ids = tree.list_ids().unwrap();
        assert_eq!(ids.len(), 1);
        let (scripts, meta) = tree.read(ids.iter().next().unwrap(), files).unwrap();
        assert_eq!(scripts.up, "-- SQL goes here");
        assert!(meta.locked);
        assert_eq!(meta.comment.as_deref(), Some("note"));

        std::fs::create_dir(root.join("id=01")).unwrap();
        assert!(tree.list_ids().is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
