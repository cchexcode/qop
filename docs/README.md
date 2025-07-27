# qop - A simple database migration tool

`qop` is a command-line tool for managing database migrations. It's designed to be simple, straightforward, and easy to use.

## Features

*   Backend-agnostic design (supports PostgreSQL and SQLite)
*   Simple migration file format (`up.sql`, `down.sql`)
*   Timestamp-based migration IDs
*   Command-line interface for managing migrations

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

2.  **Initialize the database:**
    ```bash
    qop subsystem postgres init    # For PostgreSQL
    qop subsystem sqlite init      # For SQLite
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
table = "migrations"
timeout = 30
```

You can also use environment variables for the connection string:

```toml
version = ">=0.1.0"

[subsystem.postgres]
connection = { from_env = "DATABASE_URL" }
schema = "public"
table = "migrations"
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
*   `-d, --diff`: Show migration diff before applying (experimental feature)

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
*   `-d, --diff`: Show migration diff before applying (experimental feature)

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

Shows pending migration operations without applying them (experimental feature).

```bash
qop subsystem postgres diff --path path/to/your/qop.toml
```

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

###### `qop subsystem postgres apply down`

Reverts a specific migration.

```bash
qop subsystem postgres apply down <ID> --path path/to/your/qop.toml
```

**Arguments:**
*   `<ID>`: Migration ID to revert (required)
*   `-t, --timeout <TIMEOUT>`: Statement timeout in seconds.
*   `-r, --remote`: Use the `down.sql` from the database instead of the local file.

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
*   `-d, --diff`: Show migration diff before applying (experimental feature)

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
*   `-d, --diff`: Show migration diff before applying (experimental feature)

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

Shows pending migration operations without applying them (experimental feature).

```bash
qop subsystem sqlite diff --path path/to/your/qop.toml
```

##### `qop subsystem sqlite apply up`

Applies a specific migration by ID.

```bash
qop subsystem sqlite apply up <ID> --path path/to/your/qop.toml
```

##### `qop subsystem sqlite apply down`

Reverts a specific migration by ID.

```bash
qop subsystem sqlite apply down <ID> --path path/to/your/qop.toml
```

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

### Experimental Features

Some features require the `--experimental` (or `-e`) flag to enable:

*   `diff` command and `--diff` flag for showing migration differences before applying
*   These features provide preview functionality but are still under development

```bash
qop --experimental subsystem postgres diff
qop --experimental subsystem postgres up --diff
```
