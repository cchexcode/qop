use {
    crate::subsystem::surrealdb::config::SubsystemSurrealdb,
    anyhow::{
        Context,
        Result,
    },
    chrono::{
        DateTime,
        NaiveDateTime,
        Utc,
    },
    serde::{
        Deserialize,
        de::DeserializeOwned,
    },
    serde_json::Value,
    std::{
        collections::HashSet,
        path::Path,
        time::Duration,
    },
};

#[derive(Debug, Deserialize)]
struct SurrealdbStatement {
    status: String,
    result: Value,
}

#[derive(Clone)]
pub struct SurrealdbClient {
    client: reqwest::Client,
    endpoint: String,
    namespace: String,
    database: String,
    username: Option<String>,
    password: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MigrationIdRow {
    migration_id: String,
}

#[derive(Debug, Deserialize)]
struct VersionRow {
    version: String,
}

#[derive(Debug, Deserialize)]
struct LockedRow {
    #[serde(default)]
    locked: bool,
}

#[derive(Debug, Deserialize)]
struct DownRow {
    down: String,
}

#[derive(Debug, Deserialize)]
struct HistoryRow {
    migration_id: String,
    created_at: DateTime<Utc>,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    locked: bool,
}

#[derive(Debug, Deserialize)]
struct MigrationDataRow {
    migration_id: String,
    up: String,
    down: String,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    locked: bool,
}

impl SurrealdbClient {
    fn from_config(path: &Path, config: &SubsystemSurrealdb) -> Result<Self> {
        let connection = config
            .connection
            .resolve(&format!("[subsystem.surrealdb].connection in {}", path.display()))?;
        if connection.is_empty() {
            anyhow::bail!("SurrealDB connection cannot be empty");
        }
        if config.namespace.is_empty() {
            anyhow::bail!("SurrealDB namespace cannot be empty");
        }
        if config.database.is_empty() {
            anyhow::bail!("SurrealDB database cannot be empty");
        }

        let username = config
            .username
            .as_ref()
            .map(|source| source.resolve(&format!("[subsystem.surrealdb].username in {}", path.display())))
            .transpose()?;
        let password = config
            .password
            .as_ref()
            .map(|source| source.resolve(&format!("[subsystem.surrealdb].password in {}", path.display())))
            .transpose()?;
        if username.is_some() != password.is_some() {
            anyhow::bail!("SurrealDB username and password must be configured together");
        }

        Ok(Self {
            client: reqwest::Client::new(),
            endpoint: sql_endpoint(&connection),
            namespace: config.namespace.clone(),
            database: config.database.clone(),
            username,
            password,
        })
    }

    async fn execute(&self, sql: &str, timeout_seconds: Option<u64>) -> Result<Vec<SurrealdbStatement>> {
        let mut request = self
            .client
            .post(&self.endpoint)
            .header("Accept", "application/json")
            .header("Content-Type", "application/surrealql")
            .header("Surreal-NS", &self.namespace)
            .header("Surreal-DB", &self.database)
            .body(sql.to_string());

        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            request = request.basic_auth(username, Some(password));
        }

        if let Some(seconds) = timeout_seconds {
            request = request.timeout(Duration::from_secs(seconds));
        }

        let response = request
            .send()
            .await
            .with_context(|| format!("Failed to send SurrealDB request to {}", self.endpoint))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .context("Failed to read SurrealDB response body")?;

        if !status.is_success() {
            anyhow::bail!("SurrealDB request failed ({}): {}", status, body);
        }

        let statements: Vec<SurrealdbStatement> =
            serde_json::from_str(&body).with_context(|| format!("Failed to parse SurrealDB response: {}", body))?;
        for (index, statement) in statements.iter().enumerate() {
            if !statement.status.eq_ignore_ascii_case("OK") {
                anyhow::bail!(
                    "SurrealDB statement {} failed: {}",
                    index + 1,
                    display_result(&statement.result)
                );
            }
        }

        Ok(statements)
    }

    async fn query_rows<T>(&self, sql: &str, timeout_seconds: Option<u64>) -> Result<Vec<T>>
    where T: DeserializeOwned {
        let statements = self.execute(sql, timeout_seconds).await?;
        let statement = statements
            .last()
            .ok_or_else(|| anyhow::anyhow!("SurrealDB returned no statement results"))?;
        serde_json::from_value(statement.result.clone())
            .with_context(|| format!("Failed to decode SurrealDB result: {}", statement.result))
    }
}

pub(crate) async fn build_client_from_config(
    path: &Path,
    config: &SubsystemSurrealdb,
    check_cli_version: bool,
) -> Result<SurrealdbClient> {
    let client = SurrealdbClient::from_config(path, config)?;
    if check_cli_version
        && let Some(version) = get_table_version(&client, &config.tables.migrations, config.timeout).await?
    {
        crate::config::ensure_store_compatible(&version)?;
    }
    Ok(client)
}

pub(crate) async fn init_store(client: &SurrealdbClient, config: &SubsystemSurrealdb) -> Result<()> {
    let migrations_table = table_ident(&config.tables.migrations)?;
    let log_table = table_ident(&config.tables.log)?;
    let sql = format!(
        "DEFINE TABLE IF NOT EXISTS {} SCHEMALESS;\nDEFINE TABLE IF NOT EXISTS {} SCHEMALESS;",
        migrations_table, log_table
    );
    client.execute(&sql, config.timeout).await?;
    Ok(())
}

pub(crate) async fn get_applied_migrations(
    client: &SurrealdbClient,
    table: &str,
    timeout: Option<u64>,
) -> Result<HashSet<String>> {
    let table = table_ident(table)?;
    let rows: Vec<MigrationIdRow> = client
        .query_rows(
            &format!("SELECT migration_id FROM {} ORDER BY migration_id ASC;", table),
            timeout,
        )
        .await?;
    Ok(rows.into_iter().map(|row| row.migration_id).collect())
}

pub(crate) async fn get_last_migration_id(
    client: &SurrealdbClient,
    table: &str,
    timeout: Option<u64>,
) -> Result<Option<String>> {
    let table = table_ident(table)?;
    let rows: Vec<MigrationIdRow> = client
        .query_rows(
            &format!("SELECT migration_id FROM {} ORDER BY migration_id DESC LIMIT 1;", table),
            timeout,
        )
        .await?;
    Ok(rows.into_iter().next().map(|row| row.migration_id))
}

pub(crate) async fn apply_migration(
    client: &SurrealdbClient,
    config: &SubsystemSurrealdb,
    request: &crate::core::model::ApplyRequest,
) -> Result<()> {
    let timeout = get_effective_timeout(config, request.timeout);
    let mut sql = String::from("BEGIN TRANSACTION;\n");
    push_migration_sql(&mut sql, &request.scripts.up);
    sql.push_str(&migration_record_statement(
        &config.tables.migrations,
        request.id.as_str(),
        &request.scripts.up,
        &request.scripts.down,
        request.comment.as_deref(),
        request.previous.as_ref().map(crate::core::model::MigrationId::as_str),
        request.locked,
    )?);
    sql.push_str(&log_record_statement(
        &config.tables.log,
        request.id.as_str(),
        "up",
        &request.scripts.up,
    )?);
    if request.mode == crate::core::model::RunMode::DryRun {
        sql.push_str("CANCEL TRANSACTION;");
    } else {
        sql.push_str("COMMIT TRANSACTION;");
    }

    client
        .execute(&sql, timeout)
        .await
        .with_context(|| format!("Failed to apply SurrealDB migration {}", request.id))?;
    Ok(())
}

pub(crate) async fn revert_migration(
    client: &SurrealdbClient,
    config: &SubsystemSurrealdb,
    request: &crate::core::model::RevertRequest,
) -> Result<()> {
    let timeout = get_effective_timeout(config, request.timeout);
    let id = request.id.as_str();
    if is_migration_locked(client, &config.tables.migrations, id, timeout).await? && !request.unlock {
        return Err(crate::core::outcome::blocked(format!(
            "Migration {id} is locked and cannot be reverted without --unlock"
        )));
    }

    let mut sql = String::from("BEGIN TRANSACTION;\n");
    push_migration_sql(&mut sql, &request.down);
    sql.push_str(&format!("DELETE {};\n", thing_expr(&config.tables.migrations, id)?));
    sql.push_str(&log_record_statement(&config.tables.log, id, "down", &request.down)?);
    if request.mode == crate::core::model::RunMode::DryRun {
        sql.push_str("CANCEL TRANSACTION;");
    } else {
        sql.push_str("COMMIT TRANSACTION;");
    }

    client
        .execute(&sql, timeout)
        .await
        .with_context(|| format!("Failed to revert SurrealDB migration {}", request.id))?;
    Ok(())
}

pub(crate) async fn get_migration_history(
    client: &SurrealdbClient,
    table: &str,
    timeout: Option<u64>,
) -> Result<Vec<(String, NaiveDateTime, Option<String>, bool)>> {
    let table = table_ident(table)?;
    let rows: Vec<HistoryRow> = client
        .query_rows(
            &format!(
                "SELECT migration_id, created_at, comment, locked FROM {} ORDER BY migration_id ASC;",
                table
            ),
            timeout,
        )
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.migration_id, row.created_at.naive_utc(), row.comment, row.locked))
        .collect())
}

pub(crate) async fn get_migration_down_sql(
    client: &SurrealdbClient,
    table: &str,
    migration_id: &str,
    timeout: Option<u64>,
) -> Result<Option<String>> {
    let rows: Vec<DownRow> = client
        .query_rows(
            &format!("SELECT migration_id, down FROM {};", thing_expr(table, migration_id)?),
            timeout,
        )
        .await?;
    Ok(rows.into_iter().next().map(|row| row.down))
}

pub(crate) async fn get_all_migration_data(
    client: &SurrealdbClient,
    table: &str,
    timeout: Option<u64>,
) -> Result<Vec<(String, String, String, Option<String>, bool)>> {
    let table = table_ident(table)?;
    let rows: Vec<MigrationDataRow> = client
        .query_rows(
            &format!(
                "SELECT migration_id, up, down, comment, locked FROM {} ORDER BY migration_id ASC;",
                table
            ),
            timeout,
        )
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.migration_id, row.up, row.down, row.comment, row.locked))
        .collect())
}

fn get_effective_timeout(config: &SubsystemSurrealdb, provided_timeout: Option<u64>) -> Option<u64> {
    provided_timeout.or(config.timeout)
}

async fn get_table_version(client: &SurrealdbClient, table: &str, timeout: Option<u64>) -> Result<Option<String>> {
    let table = table_ident(table)?;
    let rows: Vec<VersionRow> = client
        .query_rows(
            &format!(
                "SELECT version, migration_id FROM {} ORDER BY migration_id DESC LIMIT 1;",
                table
            ),
            timeout,
        )
        .await?;
    Ok(rows.into_iter().next().map(|row| row.version))
}

async fn is_migration_locked(
    client: &SurrealdbClient,
    table: &str,
    migration_id: &str,
    timeout: Option<u64>,
) -> Result<bool> {
    let rows: Vec<LockedRow> = client
        .query_rows(
            &format!("SELECT locked FROM {};", thing_expr(table, migration_id)?),
            timeout,
        )
        .await?;
    Ok(rows.into_iter().next().map(|row| row.locked).unwrap_or(false))
}

fn migration_record_statement(
    table: &str,
    id: &str,
    up_sql: &str,
    down_sql: &str,
    comment: Option<&str>,
    pre_migration_id: Option<&str>,
    locked: bool,
) -> Result<String> {
    Ok(format!(
        "CREATE {} CONTENT {{ migration_id: {}, version: {}, up: {}, down: {}, created_at: time::now(), pre: {}, \
         comment: {}, locked: {} }};\n",
        thing_expr(table, id)?,
        string_literal(id)?,
        string_literal(env!("CARGO_PKG_VERSION"))?,
        string_literal(up_sql)?,
        string_literal(down_sql)?,
        optional_string_literal(pre_migration_id)?,
        optional_string_literal(comment)?,
        locked,
    ))
}

fn log_record_statement(table: &str, migration_id: &str, operation: &str, sql_command: &str) -> Result<String> {
    let log_id = uuid::Uuid::now_v7().to_string();
    Ok(format!(
        "CREATE {} CONTENT {{ log_id: {}, migration_id: {}, operation: {}, query: {}, executed_at: time::now() }};\n",
        thing_expr(table, &log_id)?,
        string_literal(&log_id)?,
        string_literal(migration_id)?,
        string_literal(operation)?,
        string_literal(sql_command)?,
    ))
}

fn push_migration_sql(output: &mut String, sql: &str) {
    output.push_str(sql);
    if !sql.ends_with('\n') {
        output.push('\n');
    }
}

fn sql_endpoint(connection: &str) -> String {
    let connection = connection.trim_end_matches('/');
    if connection.ends_with("/sql") {
        connection.to_string()
    } else {
        format!("{}/sql", connection)
    }
}

fn table_ident(table: &str) -> Result<&str> {
    let mut chars = table.chars();
    let Some(first) = chars.next() else {
        anyhow::bail!("SurrealDB table name cannot be empty");
    };
    if !(first.is_ascii_alphabetic() || first == '_') || !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        anyhow::bail!(
            "Invalid SurrealDB table name '{}'. Use ASCII letters, numbers, and underscores, starting with a letter \
             or underscore.",
            table
        );
    }
    Ok(table)
}

fn thing_expr(table: &str, id: &str) -> Result<String> {
    table_ident(table)?;
    Ok(format!(
        "type::record({}, {})",
        string_literal(table)?,
        string_literal(id)?
    ))
}

fn string_literal(value: &str) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}

fn optional_string_literal(value: Option<&str>) -> Result<String> {
    match value {
        | Some(value) => string_literal(value),
        | None => Ok("NULL".to_string()),
    }
}

fn display_result(value: &Value) -> String {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| value.to_string())
}
