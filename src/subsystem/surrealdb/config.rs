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
pub struct SubsystemSurrealdb {
    pub connection: DataSource<String>,
    pub namespace: String,
    pub database: String,
    pub username: Option<DataSource<String>>,
    pub password: Option<DataSource<String>>,
    pub timeout: Option<u64>,
    pub tables: Tables,
}

impl SubsystemSurrealdb {
    pub fn sample(
        connection: &str,
        namespace: &str,
        database: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> crate::config::Config {
        crate::config::Config {
            version: format!(">={}", env!("CARGO_PKG_VERSION")),
            subsystem: crate::config::Subsystem::Surrealdb(Self {
                connection: DataSource::Static(connection.to_string()),
                namespace: namespace.to_string(),
                database: database.to_string(),
                username: username.map(|value| DataSource::Static(value.to_string())),
                password: password.map(|value| DataSource::Static(value.to_string())),
                timeout: Some(60),
                tables: Tables::default(),
            }),
        }
    }
}

impl Default for SubsystemSurrealdb {
    fn default() -> Self {
        Self {
            connection: DataSource::Static(String::new()),
            namespace: String::new(),
            database: String::new(),
            username: None,
            password: None,
            timeout: None,
            tables: Tables::default(),
        }
    }
}

impl fmt::Debug for SubsystemSurrealdb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubsystemSurrealdb")
            .field("connection", &self.connection.redacted())
            .field("namespace", &self.namespace)
            .field("database", &self.database)
            .field("username", &self.username.as_ref().map(DataSource::redacted))
            .field("password", &self.password.as_ref().map(|_| "***"))
            .field("timeout", &self.timeout)
            .field("tables", &self.tables)
            .finish()
    }
}
