use {
    anyhow::{
        Context,
        Result,
    },
    semver::{
        Version,
        VersionReq,
    },
    serde::{
        Deserialize,
        Serialize,
        de::DeserializeOwned,
    },
    std::path::Path,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct WithVersion {
    pub version: String,
}

impl WithVersion {
    pub fn validate(&self, cli: &str) -> Result<(), anyhow::Error> {
        // if cli == "0.0.0" {
        //     return Ok(());
        // }

        // Parse CLI version (Cargo semver)
        let cli_version = Version::parse(cli).map_err(|e| anyhow::anyhow!("Invalid CLI version '{}': {}", cli, e))?;

        // Parse version requirement from config (Cargo semver expressions)
        // Examples: ">=0.5.0, <0.6.0", "^0.5", "~0.5.2", "=0.5.3"
        let version_req = VersionReq::parse(&self.version)
            .map_err(|e| anyhow::anyhow!("Invalid version requirement '{}': {}", self.version, e))?;

        // Check if CLI version matches the specification
        if !version_req.matches(&cli_version) {
            return Err(anyhow::anyhow!(
                "Version mismatch: Config indicates required CLI version '{}', but current CLI version is '{}'",
                self.version,
                cli
            ));
        }

        Ok(())
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let cfg: Self =
            toml::from_str(&content).with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        cfg.validate_cli(env!("CARGO_PKG_VERSION"))
            .with_context(|| format!("Failed to validate config file: {}", path.display()))?;
        Ok(cfg)
    }

    pub fn validate_cli(&self, cli: &str) -> Result<()> {
        WithVersion {
            version: self.version.clone(),
        }
        .validate(cli)
    }

    pub fn write(&self, path: &Path, force: bool) -> Result<()> {
        if path.exists() && !force {
            return Err(crate::core::outcome::blocked(format!(
                "Refusing to overwrite {}. Pass --force to replace it.",
                path.display()
            )));
        }
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
        let toml = toml::to_string(self).context("Failed to serialize configuration")?;
        std::fs::write(path, toml).with_context(|| format!("Failed to write config file to: {}", path.display()))?;
        Ok(())
    }
}

/// A migration store written by a newer qop must not be modified by an older
/// CLI. `0.0.0` is the in-development build and is allowed to read any store.
pub fn ensure_store_compatible(stored_version: &str) -> Result<()> {
    let cli = Version::parse(env!("CARGO_PKG_VERSION")).context("Invalid CLI version")?;
    if cli.major == 0 && cli.minor == 0 && cli.patch == 0 {
        return Ok(());
    }
    let stored = Version::parse(stored_version)
        .with_context(|| format!("Invalid version '{stored_version}' stored in the migration table"))?;
    if stored > cli {
        anyhow::bail!(
            "Latest applied migration was recorded by qop {stored}, which is newer than this CLI ({cli}). Upgrade qop \
             before continuing."
        );
    }
    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Config {
    pub version: String,
    pub subsystem: Subsystem,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Tables {
    pub migrations: String,
    pub log: String,
}

impl Default for Tables {
    fn default() -> Self {
        Self {
            migrations: "__qop_migrations".to_string(),
            log: "__qop_log".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: DeserializeOwned"))]
pub enum DataSource<T: Serialize+DeserializeOwned> {
    Static(T),
    FromEnv(String),
}

impl DataSource<String> {
    pub fn resolve(&self, context: &str) -> Result<String> {
        match self {
            | Self::Static(value) => Ok(value.clone()),
            | Self::FromEnv(var) => {
                std::env::var(var)
                    .with_context(|| format!("Missing environment variable '{var}' referenced by {context}"))
            },
        }
    }

    pub fn redacted(&self) -> String {
        match self {
            | Self::Static(value) => redact_credential(value),
            | Self::FromEnv(var) => format!("from_env({var})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_url_passwords_and_env_refs() {
        let source = DataSource::Static("postgresql://postgres:secret@localhost:5432/db".to_string());
        assert_eq!(source.redacted(), "postgresql://postgres:***@localhost:5432/db");
        assert_eq!(
            DataSource::FromEnv("DATABASE_URL".to_string()).redacted(),
            "from_env(DATABASE_URL)"
        );
    }
}

fn redact_credential(value: &str) -> String {
    let Some(scheme) = value.find("://") else {
        return value.to_string();
    };
    let rest = &value[scheme + 3..];
    let Some(at) = rest.rfind('@') else {
        return value.to_string();
    };
    let userinfo = &rest[..at];
    let Some(colon) = userinfo.find(':') else {
        return value.to_string();
    };
    let user = &userinfo[..colon];
    format!("{}://{user}:***@{}", &value[..scheme], &rest[at + 1..])
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Subsystem {
    #[cfg(feature = "sub+postgres")]
    Postgres(crate::subsystem::postgres::config::SubsystemPostgres),
    #[cfg(feature = "sub+surrealdb")]
    Surrealdb(crate::subsystem::surrealdb::config::SubsystemSurrealdb),
    #[cfg(feature = "sub+sqlite")]
    Sqlite(crate::subsystem::sqlite::config::SubsystemSqlite),
}
