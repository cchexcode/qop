# qop - A simple database migration tool

`qop` is a command-line tool for managing database migrations for PostgreSQL and SQLite. It's designed to be simple, straightforward, and easy to use.

## Features

*   Backend-agnostic design (supports PostgreSQL and SQLite)
*   Simple migration file format (`up.sql`, `down.sql`)
*   Timestamp-based migration IDs
*   Command-line interface for managing migrations
*   No interactive UI; all confirmations happen via CLI prompts or can be bypassed with `--yes`

## Installation

```bash
cargo install --path .
```

## Getting Started

1.  **Create a migrations directory and config file:**
    - Create a directory to hold your migrations (for example, `migrations/`). Place your `qop.toml` inside this directory. The tool expects migration folders (like `id=.../`) to live alongside `qop.toml`.
    - Generate a sample config for your database:
      - PostgreSQL:
        ```bash
        qop subsystem postgres config init -p migrations/qop.toml -c "postgresql://postgres:password@localhost:5432/postgres"
        ```
      - SQLite:
        ```bash
        qop subsystem sqlite config init -p migrations/qop.toml -d ./app.db
        ```

2.  **Initialize the migration table:**
    ```bash
    qop subsystem postgres init -p migrations/qop.toml
    qop subsystem sqlite   init -p migrations/qop.toml
    ```

3.  **Create your first migration:**
    ```bash
    qop subsystem postgres new -p migrations/qop.toml    # For PostgreSQL
    qop subsystem sqlite   new -p migrations/qop.toml    # For SQLite
    ```
    This will create a new directory with `up.sql` and `down.sql` files.

4.  **Apply the migration:**
    ```bash
    qop subsystem postgres up -p migrations/qop.toml     # For PostgreSQL
    qop subsystem sqlite   up -p migrations/qop.toml     # For SQLite
    ```

## Configuration

`qop` is configured using a `qop.toml` file. Here are examples for both supported backends:

### PostgreSQL Configuration

```toml
version = ">=0.1.0"

[subsystem.postgres]
connection = { static = "postgresql://postgres:password@localhost:5432/postgres" }
schema = "public"
table = "__qop"
timeout = 30
```

You can also use environment variables for the connection string:

```toml
version = ">=0.1.0"

[subsystem.postgres]
connection = { from_env = "DATABASE_URL" }
schema = "public"
table = "__qop"
timeout = 30
```

### SQLite Configuration

```toml
version = ">=0.1.0"

[subsystem.sqlite]
connection = { static = "sqlite:///path/to/database.db" }
table = "__qop"
timeout = 30
```

Or with environment variables:

```toml
version = ">=0.1.0"

[subsystem.sqlite]
connection = { from_env = "DATABASE_URL" }
table = "__qop"
timeout = 30
```

The migration files live in the same directory as the `qop.toml` file (e.g., `migrations/`). Each migration is a folder named `id=<timestamp>/` containing `up.sql` and `down.sql`.

## Usage

`qop` provides several commands to manage your database migrations through subsystems.

### `subsystem`

The core command for managing database-specific operations. Available aliases: `sub`, `s`

```bash
qop subsystem <DATABASE> <COMMAND>
```

#### PostgreSQL Commands

All PostgreSQL operations are accessed through the `postgres` (alias: `pg`) subsystem:

##### `qop subsystem postgres init`

Initializes the migration table in your PostgreSQL database.

```bash
qop subsystem postgres init --path path/to/your/qop.toml
```

##### `qop subsystem postgres new`

Creates a new migration directory with `up.sql` and `down.sql` files.

```bash
qop subsystem postgres new --path path/to/your/qop.toml
```

This will create a directory structure like:
```
migrations/
└── id=1678886400000/
    ├── up.sql
    └── down.sql
```

##### `qop subsystem postgres up`

Applies pending migrations. By default, it applies all pending migrations.

```bash
qop subsystem postgres up --path path/to/your/qop.toml
```

**Arguments:**
*   `-p, --path <PATH>`: Path to the `qop.toml` configuration file. (default: `qop.toml`)
*   `-c, --count <COUNT>`: The number of migrations to apply. If not specified, all pending migrations are applied.
*   `-t, --timeout <TIMEOUT>`: Statement timeout in seconds.
*   `-y, --yes`: Skip confirmation prompts and apply migrations automatically

##### `qop subsystem postgres down`

Reverts applied migrations. By default, it reverts the last applied migration.

```bash
qop subsystem postgres down --path path/to/your/qop.toml
```

**Arguments:**
*   `-p, --path <PATH>`: Path to the `qop.toml` configuration file. (default: `qop.toml`)
*   `-c, --count <COUNT>`: The number of migrations to revert. (default: 1)
*   `-t, --timeout <TIMEOUT>`: Statement timeout in seconds.
*   `-r, --remote`: Use the `down.sql` from the database instead of the local file.
*   `-y, --yes`: Skip confirmation prompts and revert migrations automatically

##### `qop subsystem postgres list`

Lists all migrations, showing their status (applied or not) and when they were applied.

```bash
qop subsystem postgres list --path path/to/your/qop.toml
```

**Arguments:**
*   `-o, --output <FORMAT>`: Output format (`human` or `json`). (default: `human`)

##### `qop subsystem postgres history`

Manages migration history with commands for syncing and fixing migration order.

###### `qop subsystem postgres history sync`

Upserts all remote migrations locally. This is useful for syncing migrations across multiple developers.

```bash
qop subsystem postgres history sync --path path/to/your/qop.toml
```

###### `qop subsystem postgres history fix`

Shuffles all non-run local migrations to the end of the chain. This is useful when you have created migrations out of order.

```bash
qop subsystem postgres history fix --path path/to/your/qop.toml
```

##### `qop subsystem postgres diff`

Shows the raw SQL content of pending migrations without applying them.

```bash
qop --experimental subsystem postgres diff --path path/to/your/qop.toml
```

This command outputs the exact SQL content that would be executed for each pending migration, with no additional formatting or headers.

##### `qop subsystem postgres apply`

Applies or reverts a specific migration by ID.

###### `qop subsystem postgres apply up`

Applies a specific migration.

```bash
qop subsystem postgres apply up <ID> --path path/to/your/qop.toml
```

**Arguments:**
*   `<ID>`: Migration ID to apply (required)
*   `-t, --timeout <TIMEOUT>`: Statement timeout in seconds.
*   `-y, --yes`: Skip confirmation prompts and apply migration automatically

###### `qop subsystem postgres apply down`

Reverts a specific migration.

```bash
qop subsystem postgres apply down <ID> --path path/to/your/qop.toml
```

**Arguments:**
*   `<ID>`: Migration ID to revert (required)
*   `-t, --timeout <TIMEOUT>`: Statement timeout in seconds.
*   `-r, --remote`: Use the `down.sql` from the database instead of the local file.
*   `-y, --yes`: Skip confirmation prompts and revert migration automatically

#### SQLite Commands

All SQLite operations are accessed through the `sqlite` (alias: `sql`) subsystem and support the same commands as PostgreSQL:

##### `qop subsystem sqlite init`

Initializes the migration table in your SQLite database.

```bash
qop subsystem sqlite init --path path/to/your/qop.toml
```

##### `qop subsystem sqlite new`

Creates a new migration directory with `up.sql` and `down.sql` files.

```bash
qop subsystem sqlite new --path path/to/your/qop.toml
```

##### `qop subsystem sqlite up`

Applies pending migrations.

```bash
qop subsystem sqlite up --path path/to/your/qop.toml
```

**Arguments:**
*   `-p, --path <PATH>`: Path to the `qop.toml` configuration file. (default: `qop.toml`)
*   `-c, --count <COUNT>`: The number of migrations to apply.
*   `-t, --timeout <TIMEOUT>`: Statement timeout in seconds.
*   `-y, --yes`: Skip confirmation prompts and apply migrations automatically

##### `qop subsystem sqlite down`

Reverts applied migrations.

```bash
qop subsystem sqlite down --path path/to/your/qop.toml
```

**Arguments:**
*   `-p, --path <PATH>`: Path to the `qop.toml` configuration file. (default: `qop.toml`)
*   `-c, --count <COUNT>`: The number of migrations to revert.
*   `-t, --timeout <TIMEOUT>`: Statement timeout in seconds.
*   `-r, --remote`: Use the `down.sql` from the database instead of the local file.
*   `-y, --yes`: Skip confirmation prompts and revert migrations automatically

##### `qop subsystem sqlite list`

Lists all migrations, showing their status and when they were applied.

```bash
qop subsystem sqlite list --path path/to/your/qop.toml
```

**Arguments:**
*   `-o, --output <FORMAT>`: Output format (`human` or `json`). (default: `human`)

##### `qop subsystem sqlite history sync`

Upserts all remote migrations locally.

```bash
qop subsystem sqlite history sync --path path/to/your/qop.toml
```

##### `qop subsystem sqlite history fix`

Shuffles all non-run local migrations to the end of the chain.

```bash
qop subsystem sqlite history fix --path path/to/your/qop.toml
```

##### `qop subsystem sqlite diff`

Shows the raw SQL content of pending migrations without applying them.

```bash
qop --experimental subsystem sqlite diff --path path/to/your/qop.toml
```

This command outputs the exact SQL content that would be executed for each pending migration, with no additional formatting or headers.

##### `qop subsystem sqlite apply up`

Applies a specific migration by ID.

```bash
qop subsystem sqlite apply up <ID> --path path/to/your/qop.toml
```

**Arguments:**
*   `<ID>`: Migration ID to apply (required)
*   `-t, --timeout <TIMEOUT>`: Statement timeout in seconds.
*   `-y, --yes`: Skip confirmation prompts and apply migration automatically

##### `qop subsystem sqlite apply down`

Reverts a specific migration by ID.

```bash
qop subsystem sqlite apply down <ID> --path path/to/your/qop.toml
```

**Arguments:**
*   `<ID>`: Migration ID to revert (required)
*   `-t, --timeout <TIMEOUT>`: Statement timeout in seconds.
*   `-r, --remote`: Use the `down.sql` from the database instead of the local file.
*   `-y, --yes`: Skip confirmation prompts and revert migration automatically

### `man`

Renders the manual.

#### `qop man`

```bash
qop man --out docs/manual --format markdown
```

**Arguments:**
*   `-o, --out <PATH>`: Path to write documentation to (required)
*   `-f, --format <FORMAT>`: Format for the documentation. Can be `manpages` or `markdown` (required)

### `autocomplete`

Renders shell completion scripts.

#### `qop autocomplete`

```bash
qop autocomplete --out completions --shell zsh
```

**Arguments:**
*   `-o, --out <PATH>`: Path to write completion script to (required)
*   `-s, --shell <SHELL>`: The shell to generate completions for (`bash`, `zsh`, `fish`, `elvish`, `powershell`) (required)

## Migration Preview and Safety Features

### Preview SQL during confirmation

During confirmation prompts, type `d` or `diff` to preview the exact SQL for the operation:

```bash
# Apply pending migrations (press 'd' at the prompt to preview SQL)
qop subsystem postgres up -p migrations/qop.toml

# Revert last migration (press 'd' at the prompt to preview SQL)
qop subsystem postgres down -p migrations/qop.toml
```

The preview shows the raw SQL content exactly as it will be executed, with no additional formatting.

### Experimental: Diff command

You can also print pending SQL without prompts using the diff command (experimental; requires `--experimental`):

```bash
qop --experimental subsystem postgres diff -p migrations/qop.toml
qop --experimental subsystem sqlite   diff -p migrations/qop.toml
```

**Example Output:**
```sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_email ON users(email);
```

The output contains only the SQL statements from your migration files, making it easy to redirect to files or pipe to other tools.

### Automated mode

**Skip confirmations with `--yes`:**
```bash
# Apply all pending migrations without prompts
qop subsystem postgres up --yes

# Revert last migration without prompts
qop subsystem postgres down --yes
```

Dry-run flags are not currently supported in the stable path and may be introduced behind `--experimental` in future versions.

### Practical Examples

**Development Workflow:**
```bash
# 1. Check what migrations are pending
qop --experimental subsystem postgres diff

# 2. Apply with confirmation
qop subsystem postgres up
```

**CI/CD Pipeline:**
```bash
# Apply all pending migrations automatically
qop subsystem postgres up --yes
```

**Debugging:**
```bash
# Save pending SQL to a file for review
qop --experimental subsystem postgres diff > pending_migrations.sql

# Apply a specific migration
qop subsystem postgres apply up 123456789
```

**Database Rollback:**
```bash
# Preview what will be rolled back (press 'd' at the prompt)
qop subsystem postgres down

# Rollback for real
qop subsystem postgres down --yes
```
