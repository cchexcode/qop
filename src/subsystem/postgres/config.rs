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
pub struct SubsystemPostgres {
    pub connection: DataSource<String>,
    pub timeout: Option<u64>,
    pub schema: String,
    pub tables: Tables,
}

impl SubsystemPostgres {
    pub fn sample(connection: &str) -> crate::config::Config {
        crate::config::Config {
            version: format!(">={}", env!("CARGO_PKG_VERSION")),
            subsystem: crate::config::Subsystem::Postgres(Self {
                connection: DataSource::Static(connection.to_string()),
                timeout: Some(60),
                schema: "public".to_string(),
                tables: Tables::default(),
            }),
        }
    }
}

impl Default for SubsystemPostgres {
    fn default() -> Self {
        Self {
            connection: DataSource::Static(String::new()),
            timeout: None,
            schema: "public".to_string(),
            tables: Tables::default(),
        }
    }
}

impl fmt::Debug for SubsystemPostgres {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubsystemPostgres")
            .field("connection", &self.connection.redacted())
            .field("timeout", &self.timeout)
            .field("schema", &self.schema)
            .field("tables", &self.tables)
            .finish()
    }
}
