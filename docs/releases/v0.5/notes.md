# Release Notes: Version 0.5

## Upgrading from 0.4.x to 0.5.x

**Important**: Upgrades are unidirectional. Rollbacks from version 0.5.x to any lower version are _not supported_.

This release introduces several major changes to the qop migration system:

1. **Migration log table**: A new append-only log table (`__qop_log`) that stores all executed migration operations with their SQL commands
2. **Migration metadata**: New `comment` and `locked` columns in the migrations table
3. **Migration files structure**: New `meta.toml` files alongside `up.sql` and `down.sql` for storing migration metadata
4. **Table naming**: The main table is renamed from `__qop` to `__qop_migrations` to distinguish it from the new log table

These changes require manual database schema updates to upgrade from version 0.4.x.

## Config

The config version field now follows the `cargo` semver spec instead of the `pep` semver spec. Notable differences are `0.0.0-alpha.1` instead of `0.0.0-a1` as an example. Make sure to update accordingly.

## Database Schema Upgrade Instructions

### Subsystem: Postgres

```postgresql
-- Step 1: Rename the existing __qop table to __qop_migrations
ALTER TABLE "__qop" RENAME TO "__qop_migrations";

-- Step 2: Add missing columns to migrations table
ALTER TABLE "__qop_migrations" ADD COLUMN comment VARCHAR;
ALTER TABLE "__qop_migrations" ADD COLUMN locked BOOLEAN NOT NULL DEFAULT FALSE;

-- Step 3: Create the new __qop_log table
CREATE TABLE "__qop_log" (
    id VARCHAR PRIMARY KEY,
    migration_id VARCHAR NOT NULL,
    operation VARCHAR NOT NULL,
    sql_command TEXT NOT NULL,
    executed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### Subsystem: sqlite

```sql
-- Step 1: Rename the existing __qop table to __qop_migrations
ALTER TABLE "__qop" RENAME TO "__qop_migrations";

-- Step 2: Add missing columns to migrations table
ALTER TABLE "__qop_migrations" ADD COLUMN comment TEXT;
ALTER TABLE "__qop_migrations" ADD COLUMN locked BOOLEAN NOT NULL DEFAULT 0;

-- Step 3: Create the new __qop_log table
CREATE TABLE "__qop_log" (
    id TEXT PRIMARY KEY,
    migration_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    sql_command TEXT NOT NULL,
    executed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

## New Features in v0.5

### Enhanced Migration Files

This release introduces metadata support for migrations. New migrations will automatically include a `meta.toml` file containing metadata:

```toml
comment = "Created by username at 2024-01-15 10:30:00 UTC"
locked = false  # Optional, only present if migration is locked
```

**Existing migrations**: Your existing migration directories (without `meta.toml` files) will continue to work without modification. The system will treat them as having no comment and unlocked status.

**New migrations**: All new migrations created with `qop new` will automatically include a `meta.toml` file with a default comment including the username and timestamp.

### Configuration Compatibility

If you have custom table prefixes in your `qop.toml` configuration, no changes are required. The system will automatically use:
- `{prefix}_migrations` for the main migrations table (renamed from the old single table)  
- `{prefix}_log` for the new log table

For example, with `table_prefix = "__qop"`:
- Old: `__qop` table  
- New: `__qop_migrations` and `__qop_log` tables

### New CLI and System Features

#### Migration ID Handling
- **Command line usage**: The `id=` prefix is now optional when referencing migrations in CLI commands
- **Internal storage**: Migration IDs are stored without the `id=` prefix in the database
- **Directory names**: Migration directories still use the `id=` prefix format for consistency

#### Migration Locking
- New migrations can be created with a `--locked` flag to prevent accidental reversion
- Locked migrations require the `--unlock` flag to be reverted
- Existing migrations are treated as unlocked by default

#### Enhanced Logging System
- All migration operations (up/down) are now logged in the `__qop_log` table
- Each log entry includes the full SQL command that was executed
- Log entries are append-only and provide an audit trail of all migration activity
