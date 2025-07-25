use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    pub backend: Backend,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PostgresMigrations {
    pub timeout: Option<u64>,
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: DeserializeOwned"))]
pub enum DataSource<T: Serialize + DeserializeOwned> {
    Static(T),
    FromEnv(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Postgres {
        connection: DataSource<String>,
        migrations: PostgresMigrations,
        schema: String,
        table: String,
    },
}
