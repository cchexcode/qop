use {
    crate::core::{
        command::{
            ApplyOperation,
            DownOptions,
            HistoryOperation,
            Operation,
            UpOptions,
        },
        model::{
            MigrationId,
            OutputFormat,
            RunMode,
            ScriptSource,
        },
        outcome::usage,
    },
    anyhow::Result,
    clap::{
        Arg,
        ArgAction,
        ArgMatches,
        value_parser,
    },
    path_clean::PathClean,
    std::path::PathBuf,
};

/// Shared migration subcommands. Each subsystem adds its own config command and
/// owns execution.
pub(crate) fn migration_command(
    name: &'static str,
    aliases: &'static [&'static str],
    about: &'static str,
    config: clap::Command,
) -> clap::Command {
    clap::Command::new(name)
        .aliases(aliases)
        .about(about)
        .arg(
            Arg::new("path")
                .short('p')
                .long("path")
                .global(true)
                .default_value("qop.toml"),
        )
        .subcommand_required(true)
        .subcommand(config)
        .subcommand(clap::Command::new("init").about("Initializes the database."))
        .subcommand(
            clap::Command::new("new")
                .about("Creates a new migration.")
                .arg(
                    Arg::new("comment")
                        .short('c')
                        .long("comment")
                        .help("Comment for the migration"),
                )
                .arg(flag("lock", None, "Mark the migration as locked")),
        )
        .subcommand(batch_command("up", "Runs the migrations.", false))
        .subcommand(batch_command("down", "Rolls back the migrations.", true))
        .subcommand(
            clap::Command::new("list").about("Lists all migrations.").arg(
                Arg::new("output")
                    .short('o')
                    .long("output")
                    .value_parser(["human", "json"])
                    .help("Output format"),
            ),
        )
        .subcommand(
            clap::Command::new("history")
                .about("Manages migration history.")
                .subcommand_required(true)
                .subcommand(clap::Command::new("sync").about("Upserts all remote migrations locally."))
                .subcommand(
                    clap::Command::new("fix").about("Renames out-of-order local migrations to the end of the chain."),
                ),
        )
        .subcommand(clap::Command::new("diff").about("Shows pending migration scripts without applying them."))
        .subcommand(
            clap::Command::new("apply")
                .about("Applies or reverts a specific migration by ID.")
                .subcommand_required(true)
                .subcommand(
                    clap::Command::new("up")
                        .about("Applies a specific migration.")
                        .arg(Arg::new("id").help("Migration ID to apply").required(true))
                        .arg(timeout_arg())
                        .arg(flag(
                            "dry",
                            None,
                            "Execute in a transaction and roll back instead of committing",
                        ))
                        .arg(flag("yes", Some('y'), "Skip confirmation prompts"))
                        .arg(flag("lock", None, "Mark the applied migration as locked")),
                )
                .subcommand(
                    clap::Command::new("down")
                        .about("Reverts a specific migration.")
                        .arg(Arg::new("id").help("Migration ID to revert").required(true))
                        .arg(timeout_arg())
                        .arg(flag("remote", Some('r'), "Use the down script stored in the database"))
                        .arg(flag(
                            "dry",
                            None,
                            "Execute in a transaction and roll back instead of committing",
                        ))
                        .arg(flag("yes", Some('y'), "Skip confirmation prompts"))
                        .arg(flag("unlock", None, "Allow reverting a locked migration")),
                ),
        )
}

pub(crate) fn absolute_path(matches: &ArgMatches, name: &str) -> Result<PathBuf> {
    let path_str = required(matches, name)?;
    let path = std::path::Path::new(&path_str);
    if path.is_absolute() {
        Ok(path.to_path_buf().clean())
    } else {
        Ok(std::env::current_dir()?.join(path).clean())
    }
}

pub(crate) fn required(matches: &ArgMatches, name: &str) -> Result<String> {
    matches
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| usage(format!("argument \"{name}\" is required")))
}

pub(crate) fn parse_operation(matches: &ArgMatches) -> Result<Operation> {
    if matches.subcommand_matches("init").is_some() {
        return Ok(Operation::Init);
    }
    if let Some(new) = matches.subcommand_matches("new") {
        return Ok(Operation::New {
            comment: new.get_one::<String>("comment").cloned(),
            locked: new.get_flag("lock"),
        });
    }
    if let Some(up) = matches.subcommand_matches("up") {
        return Ok(Operation::Up(UpOptions {
            timeout: copied(up, "timeout"),
            count: optional_count(up)?,
            preview: up.get_flag("diff"),
            mode: run_mode(up),
            yes: up.get_flag("yes"),
        }));
    }
    if let Some(down) = matches.subcommand_matches("down") {
        return Ok(Operation::Down(DownOptions {
            timeout: copied(down, "timeout"),
            count: required_count(down)?,
            source: script_source(down),
            preview: down.get_flag("diff"),
            mode: run_mode(down),
            yes: down.get_flag("yes"),
            unlock: down.get_flag("unlock"),
        }));
    }
    if let Some(list) = matches.subcommand_matches("list") {
        let output = match list.get_one::<String>("output").map(String::as_str).unwrap_or("human") {
            | "json" => OutputFormat::Json,
            | "human" => OutputFormat::Human,
            | other => return Err(usage(format!("unknown output format {other}"))),
        };
        return Ok(Operation::List { output });
    }
    if let Some(history) = matches.subcommand_matches("history") {
        let command = if history.subcommand_matches("sync").is_some() {
            HistoryOperation::Sync
        } else if history.subcommand_matches("fix").is_some() {
            HistoryOperation::Fix
        } else {
            return Err(usage("history command required"));
        };
        return Ok(Operation::History(command));
    }
    if matches.subcommand_matches("diff").is_some() {
        return Ok(Operation::Diff);
    }
    if let Some(apply) = matches.subcommand_matches("apply") {
        if let Some(up) = apply.subcommand_matches("up") {
            return Ok(Operation::Apply(ApplyOperation::Up {
                id: migration_id(up)?,
                timeout: copied(up, "timeout"),
                mode: run_mode(up),
                yes: up.get_flag("yes"),
                locked: up.get_flag("lock"),
            }));
        }
        if let Some(down) = apply.subcommand_matches("down") {
            return Ok(Operation::Apply(ApplyOperation::Down {
                id: migration_id(down)?,
                timeout: copied(down, "timeout"),
                source: script_source(down),
                mode: run_mode(down),
                yes: down.get_flag("yes"),
                unlock: down.get_flag("unlock"),
            }));
        }
        return Err(usage("apply command required"));
    }
    Err(usage("subsystem command required"))
}

fn flag(id: &'static str, short: Option<char>, help: &'static str) -> Arg {
    let mut arg = Arg::new(id).long(id).action(ArgAction::SetTrue).help(help);
    if let Some(short) = short {
        arg = arg.short(short);
    }
    arg
}

fn timeout_arg() -> Arg {
    Arg::new("timeout")
        .short('t')
        .long("timeout")
        .value_parser(value_parser!(u64))
        .help("Statement timeout in seconds")
}

fn count_arg(default_one: bool) -> Arg {
    let mut arg = Arg::new("count")
        .short('c')
        .long("count")
        .value_parser(value_parser!(usize))
        .help("Number of migrations to run");
    if default_one {
        arg = arg.default_value("1");
    }
    arg
}

fn batch_command(name: &'static str, about: &'static str, revert: bool) -> clap::Command {
    let mut command = clap::Command::new(name)
        .about(about)
        .arg(timeout_arg())
        .arg(count_arg(revert))
        .arg(flag("diff", Some('d'), "Show the migration script before confirming"))
        .arg(flag(
            "dry",
            None,
            "Execute in a transaction and roll back instead of committing",
        ))
        .arg(flag("yes", Some('y'), "Skip confirmation prompts"));
    if revert {
        command = command
            .arg(flag("remote", Some('r'), "Use the down script stored in the database"))
            .arg(flag("unlock", None, "Allow reverting locked migrations"));
    }
    command
}

fn copied<T: Copy+Send+Sync+'static>(matches: &ArgMatches, name: &str) -> Option<T> {
    matches.get_one::<T>(name).copied()
}

fn optional_count(matches: &ArgMatches) -> Result<Option<usize>> {
    match matches.get_one::<usize>("count").copied() {
        | Some(0) => Err(usage("--count must be at least 1")),
        | count => Ok(count),
    }
}

fn required_count(matches: &ArgMatches) -> Result<usize> {
    let count = matches.get_one::<usize>("count").copied().unwrap_or(1);
    if count == 0 {
        return Err(usage("--count must be at least 1"));
    }
    Ok(count)
}

fn migration_id(matches: &ArgMatches) -> Result<MigrationId> {
    let raw = required(matches, "id")?;
    MigrationId::parse(&raw).map_err(|err| usage(err.to_string()))
}

fn run_mode(matches: &ArgMatches) -> RunMode {
    if matches.get_flag("dry") {
        RunMode::DryRun
    } else {
        RunMode::Apply
    }
}

fn script_source(matches: &ArgMatches) -> ScriptSource {
    if matches.get_flag("remote") {
        ScriptSource::Remote
    } else {
        ScriptSource::Local
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(args: &[&str]) -> Result<Operation> {
        let matches = clap::Command::new("qop")
            .subcommand(clap::Command::new("subsystem").subcommand(migration_command(
                "demo",
                &[],
                "demo",
                clap::Command::new("config"),
            )))
            .try_get_matches_from(args)
            .unwrap();
        parse_operation(
            matches
                .subcommand_matches("subsystem")
                .unwrap()
                .subcommand_matches("demo")
                .unwrap(),
        )
    }

    #[test]
    fn down_defaults_count_and_dry_does_not_conflict_with_yes() {
        match parsed(&["qop", "subsystem", "demo", "down"]).unwrap() {
            | Operation::Down(options) => assert_eq!(options.count, 1),
            | _ => panic!("expected down"),
        }
        let matches = clap::Command::new("qop")
            .subcommand(clap::Command::new("subsystem").subcommand(migration_command(
                "demo",
                &[],
                "demo",
                clap::Command::new("config"),
            )))
            .try_get_matches_from(["qop", "subsystem", "demo", "up", "--dry", "--yes"])
            .unwrap();
        let up = matches
            .subcommand_matches("subsystem")
            .unwrap()
            .subcommand_matches("demo")
            .unwrap()
            .subcommand_matches("up")
            .unwrap();
        assert!(up.get_flag("dry") && up.get_flag("yes"));
    }

    #[test]
    fn apply_captures_lock_and_rejects_traversal() {
        match parsed(&["qop", "subsystem", "demo", "apply", "up", "1756625168555", "--lock"]).unwrap() {
            | Operation::Apply(ApplyOperation::Up { locked, .. }) => assert!(locked),
            | _ => panic!("expected apply up"),
        }
        match parsed(&["qop", "subsystem", "demo", "apply", "down", "1756625168555", "--unlock"]).unwrap() {
            | Operation::Apply(ApplyOperation::Down { unlock, .. }) => assert!(unlock),
            | _ => panic!("expected apply down"),
        }
        assert!(parsed(&["qop", "subsystem", "demo", "down", "--count", "0"]).is_err());
        assert!(parsed(&["qop", "subsystem", "demo", "apply", "up", "../escape"]).is_err());
    }
}
