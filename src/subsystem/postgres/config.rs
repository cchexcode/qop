use serde::{Deserialize, Serialize};
use crate::config::DataSource;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SubsystemPostgres {
    pub connection: DataSource<String>,
    pub timeout: Option<u64>,
    pub schema: String,
    pub table: String,
}

impl Default for SubsystemPostgres {
    fn default() -> Self {
        Self {
            connection: DataSource::Static(String::new()),
            timeout: None,
            schema: "public".to_string(),
            table: "__qop".to_string(),
        }
    }
}


