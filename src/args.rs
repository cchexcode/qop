use {
    anyhow::Result,
    clap::Arg,
    path_clean::PathClean,
    std::{path::PathBuf, str::FromStr},
};

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Privilege {
    Normal,
    Experimental,
}

#[derive(Debug)]
pub(crate) enum ManualFormat {
    Manpages,
    Markdown,
}

#[derive(Debug)]
pub(crate) struct CallArgs {
    pub privileges: Privilege,
    pub command: Command,
}

impl CallArgs {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.privileges == Privilege::Experimental {
            return Ok(());
        }

        match &self.command {
            | Command::Migration(Migration { command: MigrationCommand::Diff, .. }) => anyhow::bail!("diff is experimental"),
            | Command::Migration(Migration { command: MigrationCommand::Up { diff: true, .. }, .. }) => anyhow::bail!("diff is experimental"),
            | Command::Migration(Migration { command: MigrationCommand::Down { diff: true, .. }, .. }) => anyhow::bail!("diff is experimental"),
            | _ => (),
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum MigrationApply {
    Up {
        id: String,
        timeout: Option<u64>,
    },
    Down {
        id: String,
        timeout: Option<u64>,
        remote: bool,
    },
}

#[derive(Debug)]
pub(crate) enum MigrationCommand {
    Init,
    New,
    Up {
        timeout: Option<u64>,
        count: Option<usize>,
        diff: bool,
    },
    Down {
        timeout: Option<u64>,
        count: Option<usize>,
        remote: bool,
        diff: bool,
    },
    Apply(MigrationApply),
    List,
    Sync,
    Fix,
    Diff,
}

#[derive(Debug)]
pub(crate) struct Migration {
    pub path: PathBuf,
    pub command: MigrationCommand,
}

#[derive(Debug)]
pub(crate) enum Command {
    Manual {
        path: PathBuf,
        format: ManualFormat,
    },
    Autocomplete {
        path: PathBuf,
        shell: clap_complete::Shell,
    },
    Migration(Migration),
    Init {
        path: PathBuf,
    },
}

pub(crate) struct ClapArgumentLoader {}

impl ClapArgumentLoader {
    fn get_absolute_path(matches: &clap::ArgMatches, name: &str) -> Result<PathBuf> {
        let path_str: &String = matches.get_one(name).unwrap();
        let path = std::path::Path::new(path_str);
        if path.is_absolute() {
            Ok(path.to_path_buf().clean())
        } else {
            Ok(std::env::current_dir()?.join(path).clean())
        }
    }
    pub(crate) fn root_command() -> clap::Command {
        clap::Command::new("qop")
            .version(env!("CARGO_PKG_VERSION"))
            .about("Database migrations for savages.")
            .author("cchexcode <alexanderh.weber@outlook.com>")
            .propagate_version(true)
            .subcommand_required(true)
            .args([Arg::new("experimental")
                .short('e')
                .long("experimental")
                .help("Enables experimental features.")
                .num_args(0)])
            .subcommand(
                clap::Command::new("init")
                    .about("Initializes a new project.")
                    .arg(clap::Arg::new("path").short('p').long("path").default_value("./qop.toml")),
            )
            .subcommand(
                clap::Command::new("man")
                    .about("Renders the manual.")
                    .arg(clap::Arg::new("out").short('o').long("out").required(true))
                    .arg(
                        clap::Arg::new("format")
                            .short('f')
                            .long("format")
                            .value_parser(["manpages", "markdown"])
                            .required(true),
                    ),
            )
            .subcommand(
                clap::Command::new("autocomplete")
                    .about("Renders shell completion scripts.")
                    .arg(clap::Arg::new("out").short('o').long("out").required(true))
                    .arg(
                        clap::Arg::new("shell")
                            .short('s')
                            .long("shell")
                            .value_parser(["bash", "zsh", "fish", "elvish", "powershell"])
                            .required(true),
                    ),
            )
            .subcommand(
                clap::Command::new("migration")
                    .about("Manages migrations.")
                    .arg(clap::Arg::new("path").short('p').long("path").default_value("qop.toml"))
                    .subcommand_required(true)
                    .subcommand(
                        clap::Command::new("init")
                            .about("Initializes the database."),
                    )
                    .subcommand(
                        clap::Command::new("new")
                            .about("Creates a new migration."),
                    )
                    .subcommand(
                        clap::Command::new("up")
                            .about("Runs the migrations.")
                            .arg(clap::Arg::new("timeout").short('t').long("timeout").required(false))
                            .arg(clap::Arg::new("count").short('c').long("count").required(false))
                            .arg(clap::Arg::new("diff").short('d').long("diff").required(false).num_args(0).help("Show migration diff before applying")),
                    )
                    .subcommand(
                        clap::Command::new("down")
                            .about("Rolls back the migrations.")
                            .arg(clap::Arg::new("timeout").short('t').long("timeout").required(false))
                            .arg(clap::Arg::new("remote").short('r').long("remote").required(false).num_args(0))
                            .arg(clap::Arg::new("count").short('c').long("count").required(false))
                            .arg(clap::Arg::new("diff").short('d').long("diff").required(false).num_args(0).help("Show migration diff before applying")),
                    )
                    .subcommand(
                        clap::Command::new("list")
                            .about("Lists all applied migrations."),
                    )
                    .subcommand(
                        clap::Command::new("sync")
                            .about("Upserts all remote migrations locally."),
                    )
                    .subcommand(
                        clap::Command::new("fix")
                            .about("Shuffles all non-run local migrations to the end of the chain."),
                    )
                    .subcommand(
                        clap::Command::new("diff")
                            .about("Shows pending migration operations without applying them."),
                    )
                    .subcommand(
                        clap::Command::new("apply")
                            .about("Applies or reverts a specific migration by ID.")
                            .subcommand_required(true)
                            .subcommand(
                                clap::Command::new("up")
                                    .about("Applies a specific migration.")
                                    .arg(clap::Arg::new("id").help("Migration ID to apply").required(true))
                                    .arg(clap::Arg::new("timeout").short('t').long("timeout").required(false)),
                            )
                            .subcommand(
                                clap::Command::new("down")
                                    .about("Reverts a specific migration.")
                                    .arg(clap::Arg::new("id").help("Migration ID to revert").required(true))
                                    .arg(clap::Arg::new("timeout").short('t').long("timeout").required(false))
                                    .arg(clap::Arg::new("remote").short('r').long("remote").required(false).num_args(0)),
                            ),
                    ),
            )
    }

    pub(crate) fn load() -> Result<CallArgs> {
        let command = Self::root_command().get_matches();

        let privileges = if command.get_flag("experimental") {
            Privilege::Experimental
        } else {
            Privilege::Normal
        };

        let cmd = if let Some(subc) = command.subcommand_matches("man") {
            Command::Manual {
                path: Self::get_absolute_path(subc, "out")?,
                format: match subc.get_one::<String>("format").unwrap().as_str() {
                    | "manpages" => ManualFormat::Manpages,
                    | "markdown" => ManualFormat::Markdown,
                    | _ => return Err(anyhow::anyhow!("argument \"format\": unknown format")),
                },
            }
        } else if let Some(subc) = command.subcommand_matches("autocomplete") {
            Command::Autocomplete {
                path: Self::get_absolute_path(subc, "out")?,
                shell: clap_complete::Shell::from_str(subc.get_one::<String>("shell").unwrap().as_str()).unwrap(),
            }
        } else if let Some(subc) = command.subcommand_matches("init") {
            Command::Init {
                path: Self::get_absolute_path(subc, "path")?,
            }
        } else if let Some(subc) = command.subcommand_matches("migration") {
            let path = Self::get_absolute_path(subc, "path")?;
            let migration_cmd = if let Some(_) = subc.subcommand_matches("init") {
                MigrationCommand::Init
            } else if let Some(_) = subc.subcommand_matches("new") {
                MigrationCommand::New
            } else if let Some(up_subc) = subc.subcommand_matches("up") {
                MigrationCommand::Up {
                    timeout: up_subc.get_one::<String>("timeout").map(|s| s.parse::<u64>().unwrap()),
                    count: up_subc.get_one::<String>("count").map(|s| s.parse::<usize>().unwrap()),
                    diff: up_subc.get_flag("diff"),
                }
            } else if let Some(down_subc) = subc.subcommand_matches("down") {
                MigrationCommand::Down {
                    timeout: down_subc.get_one::<String>("timeout").map(|s| s.parse::<u64>().unwrap()),
                    count: down_subc.get_one::<String>("count").map(|s| s.parse::<usize>().unwrap()),
                    remote: down_subc.get_flag("remote"),
                    diff: down_subc.get_flag("diff"),
                }
            } else if let Some(_) = subc.subcommand_matches("list") {
                MigrationCommand::List
            } else if let Some(_) = subc.subcommand_matches("sync") {
                MigrationCommand::Sync
            } else if let Some(_) = subc.subcommand_matches("fix") {
                MigrationCommand::Fix
            } else if let Some(_) = subc.subcommand_matches("diff") {
                MigrationCommand::Diff
            } else if let Some(apply_subc) = subc.subcommand_matches("apply") {
                if let Some(up_subc) = apply_subc.subcommand_matches("up") {
                    MigrationCommand::Apply(MigrationApply::Up {
                        id: up_subc.get_one::<String>("id").unwrap().clone(),
                        timeout: up_subc.get_one::<String>("timeout").map(|s| s.parse::<u64>().unwrap()),
                    })
                } else if let Some(down_subc) = apply_subc.subcommand_matches("down") {
                    MigrationCommand::Apply(MigrationApply::Down {
                        id: down_subc.get_one::<String>("id").unwrap().clone(),
                        timeout: down_subc.get_one::<String>("timeout").map(|s| s.parse::<u64>().unwrap()),
                        remote: down_subc.get_flag("remote"),
                    })
                } else {
                    unreachable!();
                }
            } else {
                unreachable!();
            };
            Command::Migration(Migration {
                path,
                command: migration_cmd,
            })
        } else {
            return Err(anyhow::anyhow!("unknown command"));
        };

        let callargs = CallArgs {
            privileges,
            command: cmd,
        };

        callargs.validate()?;
        Ok(callargs)
    }
}
