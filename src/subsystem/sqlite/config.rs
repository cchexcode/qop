use {
    crate::config::{
        DataSource,
        Tables,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    std::fmt,
};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct SubsystemSqlite {
    pub connection: DataSource<String>,
    pub timeout: Option<u64>,
    pub tables: Tables,
}

impl SubsystemSqlite {
    pub fn sample(db_path: &std::path::Path) -> crate::config::Config {
        crate::config::Config {
            version: format!(">={}", env!("CARGO_PKG_VERSION")),
            subsystem: crate::config::Subsystem::Sqlite(Self {
                connection: DataSource::Static(db_path.to_string_lossy().into_owned()),
                timeout: Some(60),
                tables: Tables::default(),
            }),
        }
    }
}

impl Default for SubsystemSqlite {
    fn default() -> Self {
        Self {
            connection: DataSource::Static(String::new()),
            timeout: None,
            tables: Tables::default(),
        }
    }
}

impl fmt::Debug for SubsystemSqlite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubsystemSqlite")
            .field("connection", &self.connection.redacted())
            .field("timeout", &self.timeout)
            .field("tables", &self.tables)
            .finish()
    }
}

/// Bare relative paths are resolved against the config file, not the process
/// cwd.
pub fn sqlite_uri(config_path: &std::path::Path, raw: &str) -> String {
    let path = std::path::Path::new(raw);
    if raw.contains("://") || raw.starts_with("sqlite:") || path.is_absolute() {
        return raw.to_string();
    }
    let base = config_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    base.join(path).to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_database_path_follows_the_config_file() {
        let uri = sqlite_uri(std::path::Path::new("migrations/qop.toml"), "app.db");
        assert_eq!(uri, "migrations/app.db");
        assert_eq!(
            sqlite_uri(std::path::Path::new("qop.toml"), "sqlite://app.db"),
            "sqlite://app.db"
        );
    }
}
