use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub backend: Backend,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostgresMigrations {
    pub timeout: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostgresConnection {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Postgres {
        connection: PostgresConnection,
        migrations: PostgresMigrations,
        schema: String,
        table: String,
    },
}
