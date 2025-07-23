use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub backend: Backend,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Postgres {
        host: String,
        port: u16,
        username: Option<String>,
        password: Option<String>,
        database: String,
        schema: String,
        table: String,
    },
}
