use serde::{Deserialize, Serialize};
use crate::config::DataSource;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SubsystemSqlite {
    pub connection: DataSource<String>,
    pub timeout: Option<u64>,
    pub table: String,
}

impl Default for SubsystemSqlite {
    fn default() -> Self {
        Self {
            connection: DataSource::Static(String::new()),
            timeout: None,
            table: "__qop".to_string(),
        }
    }
}


