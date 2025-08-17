use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use pep440_rs::{Version, VersionSpecifiers};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WithVersion {
    pub version: String,
}

impl WithVersion {
    pub fn validate(&self, cli: &str) -> Result<(), anyhow::Error> {
        if cli == "0.0.0" {
            return Ok(());
        }

        // Parse CLI version
        let cli_version = Version::from_str(cli)
            .map_err(|e| anyhow::anyhow!("Invalid CLI version '{}': {}", cli, e))?;
        
        // Parse version specification from config
        let version_specifier = VersionSpecifiers::from_str(&self.version)
            .map_err(|e| anyhow::anyhow!("Invalid version specification '{}': {}", self.version, e))?;
        
        // Check if CLI version matches the specification
        if !version_specifier.contains(&cli_version) {
            return Err(anyhow::anyhow!(
                "Version mismatch: Config requires '{}', but CLI version is '{}'", 
                self.version, 
                cli
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    pub version: String,
    pub subsystem: Subsystem,
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
pub struct SubsystemPostgres {
    pub connection: DataSource<String>,
    pub timeout: Option<u64>,
    pub schema: String,
    pub table: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SubsystemSqlite {
    pub connection: DataSource<String>,
    pub timeout: Option<u64>,
    pub table: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Subsystem {
    Postgres(SubsystemPostgres),
    Sqlite(SubsystemSqlite),
}
