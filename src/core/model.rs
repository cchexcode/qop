use {
    anyhow::{
        Context,
        Result,
    },
    chrono::NaiveDateTime,
    serde::{
        Deserialize,
        Serialize,
    },
    std::{
        cmp::Ordering,
        collections::BTreeSet,
        fmt,
    },
};

/// Millisecond timestamp used as a migration id.
///
/// Only canonical decimal digits are accepted. That keeps directory joins
/// inside the migration tree: a value such as `../` cannot be an id.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MigrationId(String);

impl MigrationId {
    pub fn parse(raw: &str) -> Result<Self> {
        let id = raw.strip_prefix("id=").unwrap_or(raw);
        if !is_canonical_timestamp(id) {
            anyhow::bail!(
                "Invalid migration id '{raw}'. Expected a millisecond timestamp, optionally prefixed with 'id='."
            );
        }
        id.parse::<i64>()
            .with_context(|| format!("Migration id '{raw}' is out of range"))?;
        Ok(Self(id.to_string()))
    }

    pub fn from_millis(millis: i64) -> Result<Self> {
        if millis < 0 {
            anyhow::bail!("Migration id cannot be negative");
        }
        Self::parse(&millis.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn dir_name(&self) -> String {
        format!("id={}", self.0)
    }

    pub fn millis(&self) -> i64 {
        // parse() only constructs ids that fit in i64.
        self.0.parse().expect("migration id is a validated i64")
    }
}

impl PartialOrd for MigrationId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MigrationId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.millis().cmp(&other.millis())
    }
}

impl fmt::Display for MigrationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn is_canonical_timestamp(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()) && !(id.len() > 1 && id.starts_with('0'))
}

/// Backend-owned migration file naming. Core does not know which subsystem
/// produced it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScriptFiles {
    extension: &'static str,
    placeholder: &'static str,
}

impl ScriptFiles {
    pub const fn new(extension: &'static str, placeholder: &'static str) -> Self {
        assert!(!extension.is_empty(), "migration file extension cannot be empty");
        Self { extension, placeholder }
    }

    pub fn placeholder(self) -> &'static str {
        self.placeholder
    }

    pub fn file_name(self, direction: Direction) -> String {
        format!("{}.{}", direction.as_str(), self.extension)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Up,
    Down,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            | Self::Up => "up",
            | Self::Down => "down",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            | Self::Up => "UP",
            | Self::Down => "DOWN",
        }
    }
}

/// Whether a migration transaction is committed or rolled back.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunMode {
    Apply,
    DryRun,
}

/// Where a down script is read from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScriptSource {
    Local,
    Remote,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    Human,
    Json,
}

/// On-disk `meta.toml`. Absent `locked` means unlocked.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct MigrationMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub locked: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl MigrationMeta {
    pub fn with_default_comment() -> Self {
        let username = whoami::username();
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        Self {
            comment: Some(format!("Created by {username} at {timestamp}")),
            locked: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationScripts {
    pub up: String,
    pub down: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppliedMigration {
    pub id: MigrationId,
    pub applied_at: NaiveDateTime,
    pub comment: Option<String>,
    pub locked: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredMigration {
    pub id: MigrationId,
    pub up: String,
    pub down: String,
    pub comment: Option<String>,
    pub locked: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationStatus {
    pub id: MigrationId,
    pub applied_at: Option<NaiveDateTime>,
    pub local: bool,
    pub comment: Option<String>,
    pub locked: bool,
}

impl MigrationStatus {
    pub fn merge(
        local: &BTreeSet<MigrationId>,
        remote: &[AppliedMigration],
        meta: impl Fn(&MigrationId) -> Result<MigrationMeta>,
    ) -> Result<Vec<Self>> {
        let mut rows = std::collections::BTreeMap::new();
        for id in local {
            let meta = meta(id)?;
            rows.insert(id.clone(), Self {
                id: id.clone(),
                applied_at: None,
                local: true,
                comment: meta.comment,
                locked: meta.locked,
            });
        }
        for applied in remote {
            let row = rows.entry(applied.id.clone()).or_insert_with(|| {
                Self {
                    id: applied.id.clone(),
                    applied_at: None,
                    local: false,
                    comment: None,
                    locked: false,
                }
            });
            row.applied_at = Some(applied.applied_at);
            row.comment = applied.comment.clone();
            row.locked = applied.locked;
        }
        Ok(rows.into_values().collect())
    }
}

#[derive(Clone, Debug)]
pub struct ApplyRequest {
    pub id: MigrationId,
    pub scripts: MigrationScripts,
    pub comment: Option<String>,
    pub previous: Option<MigrationId>,
    pub timeout: Option<u64>,
    pub mode: RunMode,
    pub locked: bool,
}

#[derive(Clone, Debug)]
pub struct RevertRequest {
    pub id: MigrationId,
    pub down: String,
    pub timeout: Option<u64>,
    pub mode: RunMode,
    pub unlock: bool,
}

pub fn out_of_order<'a>(applied: &BTreeSet<MigrationId>, pending: &'a [MigrationId]) -> Vec<&'a MigrationId> {
    let Some(latest) = applied.iter().next_back() else {
        return Vec::new();
    };
    pending.iter().filter(|id| *id < latest).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_optional_prefix_and_rejects_traversal() {
        assert_eq!(
            MigrationId::parse("id=1756625168555").unwrap().as_str(),
            "1756625168555"
        );
        assert!(MigrationId::parse("../etc").is_err());
        assert!(MigrationId::parse("id=../etc").is_err());
        assert!(MigrationId::parse("id=").is_err());
        assert!(MigrationId::parse("01").is_err());
        assert!(MigrationId::parse("").is_err());
    }

    #[test]
    fn orders_by_numeric_value_not_string_length() {
        let nine = MigrationId::parse("9").unwrap();
        let ten = MigrationId::parse("10").unwrap();
        assert!(nine < ten);
    }

    #[test]
    fn absent_locked_field_means_unlocked() {
        let meta: MigrationMeta = toml::from_str("comment = \"note\"\n").unwrap();
        assert!(!meta.locked);
        assert_eq!(meta.comment.as_deref(), Some("note"));
    }

    #[test]
    fn out_of_order_uses_numeric_order() {
        let applied = BTreeSet::from([MigrationId::parse("10").unwrap()]);
        let pending = vec![MigrationId::parse("9").unwrap(), MigrationId::parse("11").unwrap()];
        let found = out_of_order(&applied, &pending);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].as_str(), "9");
    }
}
