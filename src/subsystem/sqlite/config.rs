use serde::{Deserialize, Serialize};
use crate::config::DataSource;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SubsystemSqlite {
    pub connection: DataSource<String>,
    pub timeout: Option<u64>,
    pub tables: Tables,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Tables {
    pub migrations: String,
    pub log: String,
}

impl Default for SubsystemSqlite {
    fn default() -> Self {
        Self {
            connection: DataSource::Static(String::new()),
            timeout: None,
            tables: Tables {
                migrations: "__qop_migrations".to_string(),
                log: "__qop_log".to_string(),
            },
        }
    }
}
