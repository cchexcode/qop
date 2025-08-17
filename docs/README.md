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

1.  **Initialize a new project:**
    ```bash
    qop init
    ```
    This will create a `qop.toml` file in your current directory.

2.  **Initialize the database and configuration:**
    ```bash
    # Create a sample config file
    qop subsystem postgres config init -p qop.toml
    qop subsystem sqlite   config init -p qop.toml

    # Initialize the migration table
    qop subsystem postgres init -p qop.toml
    qop subsystem sqlite   init -p qop.toml
    ```

3.  **Create your first migration:**
    ```bash
    qop subsystem postgres new     # For PostgreSQL
    qop subsystem sqlite new       # For SQLite
    ```
    This will create a new directory with `up.sql` and `down.sql` files.

4.  **Apply the migration:**
    ```bash
    qop subsystem postgres up      # For PostgreSQL
    qop subsystem sqlite up        # For SQLite
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
table = "migrations"
timeout = 30
```

Or with environment variables:

```toml
version = ">=0.1.0"

[subsystem.sqlite]
connection = { from_env = "DATABASE_URL" }
table = "migrations"
timeout = 30
```

The migration files are expected to be in a directory relative to the `qop.toml` file.

## Usage

`qop` provides several commands to manage your database migrations through subsystems.

### `init`

Initializes a new `qop` project by creating a `qop.toml` file in the current directory.

```bash
qop init --path path/to/your/qop.toml
```

**Arguments:**
*   `-p, --path <PATH>`: Path to the `qop.toml` configuration file. (default: `./qop.toml`)

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
*   `-d, --diff`: Show raw SQL that will be executed before applying
*   `-y, --yes`: Skip confirmation prompts and apply migrations automatically
*   `--dry`: Execute migrations in a transaction but rollback instead of committing (conflicts with `--yes`)

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
*   `-d, --diff`: Show raw SQL that will be executed before reverting
*   `-y, --yes`: Skip confirmation prompts and revert migrations automatically
*   `--dry`: Execute migrations in a transaction but rollback instead of committing (conflicts with `--yes`)

##### `qop subsystem postgres list`

Lists all migrations, showing their status (applied or not) and when they were applied.

```bash
qop subsystem postgres list --path path/to/your/qop.toml
```

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
qop subsystem postgres diff --path path/to/your/qop.toml
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
*   `--dry`: Execute migration in a transaction but rollback instead of committing (conflicts with `--yes`)

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
*   `--dry`: Execute migration in a transaction but rollback instead of committing (conflicts with `--yes`)

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
*   `-d, --diff`: Show raw SQL that will be executed before applying
*   `-y, --yes`: Skip confirmation prompts and apply migrations automatically
*   `--dry`: Execute migrations in a transaction but rollback instead of committing (conflicts with `--yes`)

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
*   `-d, --diff`: Show raw SQL that will be executed before reverting
*   `-y, --yes`: Skip confirmation prompts and revert migrations automatically
*   `--dry`: Execute migrations in a transaction but rollback instead of committing (conflicts with `--yes`)

##### `qop subsystem sqlite list`

Lists all migrations, showing their status and when they were applied.

```bash
qop subsystem sqlite list --path path/to/your/qop.toml
```

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
qop subsystem sqlite diff --path path/to/your/qop.toml
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
*   `--dry`: Execute migration in a transaction but rollback instead of committing (conflicts with `--yes`)

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
*   `--dry`: Execute migration in a transaction but rollback instead of committing (conflicts with `--yes`)

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

### Diff Functionality

The `diff` command and `--diff` flag allow you to preview the exact SQL that will be executed before applying migrations:

```bash
# Preview pending migrations
qop subsystem postgres diff

# Preview before applying
qop subsystem postgres up --diff

# Preview before reverting
qop subsystem postgres down --diff
```

The diff output shows the raw SQL content exactly as it will be executed, with no additional formatting.

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

### Automated and Dry-Run Modes

**Skip Confirmations with `--yes`:**
```bash
# Apply all pending migrations without prompts
qop subsystem postgres up --yes

# Revert last migration without prompts
qop subsystem postgres down --yes
```

**Test with Dry-Run Mode:**
```bash
# Execute migrations in a transaction but rollback (test only)
qop subsystem postgres up --dry

# Test specific migration revert
qop subsystem postgres down --dry
```

**Combined Usage:**
```bash
# Preview SQL, then apply with confirmation
qop subsystem postgres up --diff

# Test migration execution without committing
qop subsystem postgres up --dry

# Apply migrations automatically (useful for CI/CD)
qop subsystem postgres up --yes
```

**Note:** The `--dry` and `--yes` flags cannot be used together, as dry-run mode requires manual verification of the rollback.

### Practical Examples

**Development Workflow:**
```bash
# 1. Check what migrations are pending
qop subsystem postgres diff

# 2. Test the migrations safely
qop subsystem postgres up --dry

# 3. Apply with confirmation
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
qop subsystem postgres diff > pending_migrations.sql

# Test specific migration
qop subsystem postgres apply up id=123456789 --dry
```

**Database Rollback:**
```bash
# Preview what will be rolled back
qop subsystem postgres down --diff

# Rollback safely in test
qop subsystem postgres down --dry

# Rollback for real
qop subsystem postgres down --yes
```
