use serde::{Deserialize, Serialize};
use crate::config::DataSource;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SubsystemPostgres {
    pub connection: DataSource<String>,
    pub timeout: Option<u64>,
    pub schema: String,
    pub table_prefix: String,
}

impl SubsystemPostgres {
    /// Get the migrations table name from the prefix
    pub fn migrations_table(&self) -> String {
        format!("{}_migrations", self.table_prefix)
    }
    
    /// Get the log table name from the prefix  
    pub fn log_table(&self) -> String {
        format!("{}_log", self.table_prefix)
    }
}

impl Default for SubsystemPostgres {
    fn default() -> Self {
        Self {
            connection: DataSource::Static(String::new()),
            timeout: None,
            schema: "public".to_string(),
            table_prefix: "__qop".to_string(),
        }
    }
}
