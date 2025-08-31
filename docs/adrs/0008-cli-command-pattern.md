# ADR-0008: CLI Command Pattern Implementation

## Status

Accepted

## Date

2025-01-27T23:00:00Z

## Context

The `qop` migration tool has a complex command-line interface with:
1. Top-level commands (manual generation, autocomplete)
2. Subsystem-specific commands with different database backends
3. Nested command hierarchies (e.g., `subsystem postgres history sync`)
4. Feature-conditional availability of subsystems
5. Validation requirements across different command combinations

The CLI needs to be:
- Extensible for new commands and subsystems
- Type-safe to prevent invalid command combinations
- Maintainable with clear separation between parsing and execution
- User-friendly with comprehensive help and error messages
- Compatible with the existing subsystem architecture

The Command pattern provides a clean way to encapsulate command logic while maintaining type safety and extensibility.

## Decision

The codebase MUST implement a structured Command pattern that separates argument parsing, command validation, and command execution, with type-safe representations of all command variants.

### Command Architecture

1. **Command Types**: Strongly-typed enums representing all possible commands
2. **Argument Parsing**: Centralized parsing logic using Clap with builders
3. **Validation Layer**: Command validation separate from parsing
4. **Dispatch Layer**: Route commands to appropriate subsystem handlers
5. **Execution Layer**: Business logic execution via services and repositories

### Implementation Requirements

1. **Type Safety**: All commands MUST be represented as strongly-typed enums
2. **Feature Integration**: Command availability MUST respect feature flags
3. **Validation**: Command validation MUST be separate from parsing
4. **Extensibility**: New commands MUST follow established patterns
5. **Error Handling**: Consistent error handling across all command paths

## Consequences

### Positive

- **Type Safety**: Invalid command combinations are caught at compile time
- **Extensibility**: New commands and subsystems can be added systematically
- **Maintainability**: Clear separation between parsing, validation, and execution
- **Consistency**: All commands follow the same architectural patterns
- **Testing**: Command logic can be tested independently of CLI parsing
- **User Experience**: Comprehensive help and error messages

### Negative

- **Boilerplate**: Each new command requires enum variants and parsing code
- **Complexity**: Multi-level command hierarchies create complex enum structures
- **Compilation Time**: Large command structures may increase compilation time
- **Code Duplication**: Similar commands across subsystems require similar patterns

## Implementation

### Command Type Hierarchy

```rust
#[derive(Debug)]
pub struct CallArgs {
    pub privileges: Privilege,
    pub command: Command,
}

#[derive(Debug)]
pub enum Command {
    Manual {
        path: PathBuf,
        format: ManualFormat,
    },
    Autocomplete {
        path: PathBuf,
        shell: clap_complete::Shell,
    },
    Subsystem(Subsystem),
}

#[derive(Debug)]
pub enum Subsystem {
    #[cfg(feature = "sub+postgres")]
    Postgres {
        path: PathBuf,
        config: SubsystemPostgres,
        command: crate::subsystem::postgres::commands::Command,
    },
    #[cfg(feature = "sub+sqlite")]
    Sqlite {
        path: PathBuf,
        config: SubsystemSqlite,
        command: crate::subsystem::sqlite::commands::Command,
    },
}
```

### Subsystem Command Structure

```rust
// In subsystem::postgres::commands
#[derive(Debug)]
pub enum Command {
    Init,
    New { comment: Option<String>, locked: bool },
    Up { timeout: Option<u64>, count: Option<usize>, diff: bool, dry: bool, yes: bool },
    Down { timeout: Option<u64>, count: usize, remote: bool, diff: bool, dry: bool, yes: bool, unlock: bool },
    List { output: Output },
    Config(ConfigCommand),
    History(HistoryCommand),
    Diff,
    Apply(MigrationApply),
}

#[derive(Debug)]
pub enum ConfigCommand {
    Init { connection: String },
}

#[derive(Debug)]  
pub enum HistoryCommand {
    Sync,
    Fix,
}

#[derive(Debug)]
pub enum MigrationApply {
    Up { id: String, timeout: Option<u64>, dry: bool, yes: bool },
    Down { id: String, timeout: Option<u64>, remote: bool, dry: bool, yes: bool, unlock: bool },
}
```

### Clap Builder Pattern

```rust
impl ClapArgumentLoader {
    pub fn root_command() -> clap::Command {
        let mut root = clap::Command::new("qop")
            .version(env!("CARGO_PKG_VERSION"))
            .about("Database migrations for savages.")
            .subcommand_required(false)
            .args([Arg::new("experimental").short('e').long("experimental")]);

        // Feature-conditional subsystem registration
        #[cfg(any(feature = "sub+postgres", feature = "sub+sqlite"))]
        {
            let mut subsystem = clap::Command::new("subsystem")
                .aliases(["sub", "s"])
                .subcommand_required(true);

            #[cfg(feature = "sub+postgres")]
            {
                let pg = clap::Command::new("postgres")
                    .aliases(["pg"])
                    .arg(clap::Arg::new("path").short('p').long("path").default_value("qop.toml"))
                    .subcommand_required(true)
                    .subcommand(clap::Command::new("init"))
                    .subcommand(
                        clap::Command::new("up")
                            .arg(clap::Arg::new("timeout").short('t').long("timeout"))
                            .arg(clap::Arg::new("count").short('c').long("count"))
                            .arg(clap::Arg::new("dry").long("dry").num_args(0))
                            .arg(clap::Arg::new("yes").short('y').long("yes").num_args(0))
                    );
                subsystem = subsystem.subcommand(pg);
            }
            
            root = root.subcommand(subsystem);
        }

        root
    }
}
```

### Command Parsing Pattern

```rust
impl ClapArgumentLoader {
    pub fn load() -> Result<CallArgs> {
        let matches = Self::root_command().get_matches();

        let privileges = if matches.get_flag("experimental") {
            Privilege::Experimental
        } else {
            Privilege::Normal
        };

        let cmd = if let Some(subsystem_matches) = matches.subcommand_matches("subsystem") {
            #[cfg(feature = "sub+postgres")]
            {
                if let Some(pg_matches) = subsystem_matches.subcommand_matches("postgres") {
                    let path = Self::get_absolute_path(pg_matches, "path")?;
                    
                    // Load and validate configuration
                    let config: Config = toml::from_str(&std::fs::read_to_string(&path)?)?;
                    let pg_cfg = match config.subsystem { 
                        Subsystem::Postgres(c) => c, 
                        _ => anyhow::bail!("config is not postgres") 
                    };
                    
                    // Parse postgres-specific commands
                    let postgres_cmd = if pg_matches.subcommand_matches("init").is_some() {
                        postgres::commands::Command::Init
                    } else if let Some(up_matches) = pg_matches.subcommand_matches("up") {
                        postgres::commands::Command::Up {
                            timeout: up_matches.get_one::<String>("timeout").map(|s| s.parse().unwrap()),
                            count: up_matches.get_one::<String>("count").map(|s| s.parse().unwrap()),
                            diff: up_matches.get_flag("diff"),
                            dry: up_matches.get_flag("dry"),
                            yes: up_matches.get_flag("yes"),
                        }
                    } else {
                        unreachable!();
                    };
                    
                    return Ok(CallArgs { 
                        privileges, 
                        command: Command::Subsystem(Subsystem::Postgres { 
                            path, 
                            config: pg_cfg, 
                            command: postgres_cmd 
                        })
                    });
                }
            }
            // Similar pattern for other subsystems...
        } else {
            // Handle other top-level commands...
        };

        let callargs = CallArgs { privileges, command: cmd };
        callargs.validate()?;
        Ok(callargs)
    }
}
```

### Validation Rules

The validation layer MUST ensure commands are structurally valid, but the `diff` capability is considered stable and MUST NOT require experimental privileges.

### Command Dispatch Pattern

```rust
// In main.rs
#[tokio::main]
async fn main() -> Result<()> {
    let cmd = crate::args::ClapArgumentLoader::load()?;

    match cmd.command {
        Command::Manual { path, format } => {
            std::fs::create_dir_all(&path)?;
            match format {
                ManualFormat::Manpages => reference::build_manpages(&path)?,
                ManualFormat::Markdown => reference::build_markdown(&path)?,
            }
            Ok(())
        },
        Command::Autocomplete { path, shell } => {
            std::fs::create_dir_all(&path)?;
            reference::build_shell_completion(&path, &shell)?;
            Ok(())
        },
        Command::Subsystem(subsystem) => {
            crate::subsystem::driver::dispatch(subsystem).await
        },
    }
}
```

### Subsystem Dispatch Pattern

```rust
pub async fn dispatch(subsystem: Subsystem) -> Result<()> {
    match subsystem {
        #[cfg(feature = "sub+postgres")]
        Subsystem::Postgres { path, config, command } => {
            match command {
                postgres::commands::Command::Init => {
                    let repo = postgres::repo::PostgresRepo::from_config(&path, config, false).await?;
                    let svc = MigrationService::new(repo);
                    svc.init().await
                }
                postgres::commands::Command::Up { timeout, count, dry, yes, .. } => {
                    let repo = postgres::repo::PostgresRepo::from_config(&path, config, true).await?;
                    let svc = MigrationService::new(repo);
                    svc.up(&path, timeout, count, yes, dry).await
                }
                // ... other command handlers
            }
        }
        // ... other subsystem handlers
    }
}
```

## Design Guidelines

### Command Structure Principles

1. **Hierarchical Organization**: Commands MUST follow logical hierarchies (`subsystem > backend > operation`)
2. **Type Safety**: All command variants MUST be strongly typed with appropriate parameters
3. **Feature Integration**: Commands MUST be conditionally compiled based on feature flags
4. **Validation Separation**: Command parsing and validation MUST be separate concerns
5. **Consistent Patterns**: All subsystems MUST follow the same command structure patterns

### Argument Handling

```rust
// Good: Strongly typed with validation
Command::Up { 
    timeout: Option<u64>,     // Parsed and validated
    count: Option<usize>,     // Optional with type safety  
    dry: bool,                // Boolean flags
    yes: bool                 // Clear intent
}

// Bad: Stringly typed
Command::Up {
    args: HashMap<String, String>  // Loses type safety
}
```

### Error Messages and Help

1. **Contextual Help**: Each command level MUST provide relevant help information
2. **Error Context**: Invalid commands MUST provide suggestions for correction
3. **Feature Awareness**: Help messages MUST reflect enabled features
4. **Consistent Formatting**: All command output MUST follow consistent formatting

### Extensibility Patterns

```rust
// Adding a new command to existing subsystem
#[derive(Debug)]
pub enum Command {
    // ... existing commands
    NewCommand { param1: String, param2: Option<u64> },
}

// Adding parsing support
let postgres_cmd = if let Some(new_matches) = pg_matches.subcommand_matches("new-command") {
    Command::NewCommand {
        param1: new_matches.get_one::<String>("param1").unwrap().clone(),
        param2: new_matches.get_one::<String>("param2").map(|s| s.parse().unwrap()),
    }
} else {
    // ... existing command parsing
};

// Adding dispatch handling  
match command {
    Command::NewCommand { param1, param2 } => {
        // Implementation
    },
    // ... existing command handling
}
```

## Anti-Patterns

```rust
// Don't: String-based command handling
match command_str {
    "up" => handle_up(),
    "down" => handle_down(),
    _ => Err("Unknown command"),
}

// Don't: Mixed parsing and execution
fn handle_command(matches: &ArgMatches) -> Result<()> {
    let timeout = matches.get_one::<String>("timeout");
    // BAD: Business logic mixed with parsing
    let repo = PostgresRepo::new()?;
    repo.apply_migrations()?;
}

// Don't: Inconsistent command structures across subsystems
enum PostgresCommand { Up { dry: bool } }
enum SqliteCommand { Up { dry_run: bool } }  // Inconsistent naming
```

## References

- [Command Pattern](https://refactoring.guru/design-patterns/command)
- [Clap Documentation](https://docs.rs/clap/)
- [Rust CLI Working Group Guide](https://rust-cli.github.io/book/)
