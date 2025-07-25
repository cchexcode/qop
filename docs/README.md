# qop - A simple database migration tool

`qop` is a command-line tool for managing database migrations. It's designed to be simple, straightforward, and easy to use.

## Features

*   Backend-agnostic design (currently supports PostgreSQL)
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

2.  **Create your first migration:**
    ```bash
    qop migration new
    ```
    This will create a new directory with `up.sql` and `down.sql` files.

3.  **Apply the migration:**
    ```bash
    qop migration up
    ```

## Configuration

`qop` is configured using a `qop.toml` file. Here is an example for PostgreSQL:

```toml
[backend.postgres]
connection = { static = "postgresql://postgres:password@localhost:5432/postgres" }
schema = "public"
table = "migrations"

[backend.postgres.migrations]
timeout = 30
```

You can also use environment variables for the connection string:

```toml
[backend.postgres]
connection = { from_env = "DATABASE_URL" }
schema = "public"
table = "migrations"

[backend.postgres.migrations]
timeout = 30
```

The migration files are expected to be in a directory relative to the `qop.toml` file.

## Usage

`qop` provides several commands to manage your database migrations.

### `init`

Initializes a new `qop` project by creating a `qop.toml` file in the current directory.

```bash
qop init --path path/to/your/qop.toml
```

**Arguments:**
*   `-p, --path <PATH>`: Path to the `qop.toml` configuration file. (default: `qop.toml`)

### `migration`

The core set of commands for managing migrations.

#### `qop migration init`

Initializes the migration table in your database. This table is used to track which migrations have been applied.

```bash
qop migration init --path path/to/your/qop.toml
```

**Arguments:**
*   `-p, --path <PATH>`: Path to the `qop.toml` configuration file. (default: `qop.toml`)

#### `qop migration new`

Creates a new migration directory with `up.sql` and `down.sql` files. The directory name is a timestamp-based ID.

```bash
qop migration new --path path/to/your/qop.toml
```

This will create a directory structure like:
```
migrations/
└── id=1678886400000/
    ├── up.sql
    └── down.sql
```

**Arguments:**
*   `-p, --path <PATH>`: Path to the `qop.toml` configuration file. (default: `qop.toml`)

#### `qop migration up`

Applies pending migrations. By default, it applies all pending migrations.

```bash
qop migration up --path path/to/your/qop.toml
```

**Arguments:**
*   `-p, --path <PATH>`: Path to the `qop.toml` configuration file. (default: `qop.toml`)
*   `-c, --count <COUNT>`: The number of migrations to apply. If not specified, all pending migrations are applied.
*   `-t, --timeout <TIMEOUT>`: Statement timeout in seconds.

#### `qop migration down`

Reverts applied migrations. By default, it reverts the last applied migration.

```bash
qop migration down --path path/to/your/qop.toml
```

**Arguments:**
*   `-p, --path <PATH>`: Path to the `qop.toml` configuration file. (default: `qop.toml`)
*   `-c, --count <COUNT>`: The number of migrations to revert. (default: 1)
*   `-t, --timeout <TIMEOUT>`: Statement timeout in seconds.
*   `-r, --remote`: Use the `down.sql` from the database instead of the local file.

#### `qop migration list`

Lists all migrations, showing their status (applied or not) and when they were applied.

```bash
qop migration list --path path/to/your/qop.toml
```

**Arguments:**
*   `-p, --path <PATH>`: Path to the `qop.toml` configuration file. (default: `qop.toml`)

#### `qop migration history`

Manages migration history with commands for syncing and fixing migration order.

##### `qop migration history sync`

Upserts all remote migrations locally. This is useful for syncing migrations across multiple developers.

```bash
qop migration history sync --path path/to/your/qop.toml
```

**Arguments:**
*   `-p, --path <PATH>`: Path to the `qop.toml` configuration file. (default: `qop.toml`)

##### `qop migration history fix`

Shuffles all non-run local migrations to the end of the chain. This is useful when you have created migrations out of order.

```bash
qop migration history fix --path path/to/your/qop.toml
```

**Arguments:**
*   `-p, --path <PATH>`: Path to the `qop.toml` configuration file. (default: `qop.toml`)

### `manual`

Generates documentation for `qop`.

#### `qop manual`

```bash
qop manual --path docs/manual --format markdown
```

**Arguments:**
*   `-p, --path <PATH>`: Path to write documentation to. (default: `docs/manual`)
*   `--format <FORMAT>`: Format for the documentation. Can be `manpages` or `markdown`. (default: `manpages`)

### `autocomplete`

Generates shell completion scripts.

#### `qop autocomplete`

```bash
qop autocomplete --path completions --shell zsh
```

**Arguments:**
*   `-p, --path <PATH>`: Path to write completion script to. (default: `completions`)
*   `--shell <SHELL>`: The shell to generate completions for (e.g., `zsh`, `bash`, `fish`, `powershell`, `elvish`). (default: `zsh`)
