# ADR-0014: Audit Logging and Table Naming Conventions

## Status

Accepted

## Date

2025-01-31T00:15:00Z

## Context

Operational transparency requires an immutable log of migration operations. The system maintains both a migrations table and a log table. Names are derived from a configurable `table_prefix` to avoid collisions and to enable multiple tool instances within the same database.

## Decision

- Every subsystem MUST maintain two tables:
  - Migrations table: `<prefix>_migrations`
  - Log table: `<prefix>_log`
- Names are computed from configuration via helper methods.
- All apply/revert operations SHOULD be logged with an operation type and SQL snapshot.

## Schema

PostgreSQL (schema-qualified):
```sql
CREATE TABLE IF NOT EXISTS "<schema>"."<prefix>_migrations" (
  id VARCHAR PRIMARY KEY,
  version VARCHAR NOT NULL,
  up VARCHAR NOT NULL,
  down VARCHAR NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  pre VARCHAR,
  comment VARCHAR,
  locked BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS "<schema>"."<prefix>_log" (
  id VARCHAR PRIMARY KEY,
  migration_id VARCHAR NOT NULL,
  operation VARCHAR NOT NULL,
  sql_command TEXT NOT NULL,
  executed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

SQLite:
```sql
CREATE TABLE IF NOT EXISTS "<prefix>_migrations" (
  id TEXT PRIMARY KEY,
  version TEXT NOT NULL,
  up TEXT NOT NULL,
  down TEXT NOT NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  pre TEXT,
  comment TEXT,
  locked BOOLEAN NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS "<prefix>_log" (
  id TEXT PRIMARY KEY,
  migration_id TEXT NOT NULL,
  operation TEXT NOT NULL,
  sql_command TEXT NOT NULL,
  executed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

## Consequences

### Positive
- Clear audit trail of migration operations
- Namespacing avoids table collisions
- Consistency across backends simplifies tooling

### Negative
- Additional storage for logs
- Log table can grow indefinitely (future retention policy MAY be needed)

## Implementation

- Naming helpers MUST derive table names from config:
  - Postgres: `SubsystemPostgres::{migrations_table, log_table}`
  - SQLite: `SubsystemSqlite::{migrations_table, log_table}`
- Logging helpers MUST insert a UUIDv7 ID, the migration ID, the operation type, and the full SQL.

## References

- `subsystem::<backend>::config::{migrations_table, log_table}`
- `subsystem::<backend>::migration::insert_log_entry`
- `subsystem::<backend>::migration::init_with_pool`
