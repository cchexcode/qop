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
            | Command::Subsystem(Subsystem::Postgres { command: crate::subsystem::postgres::commands::Command::Diff, .. }) => anyhow::bail!("diff is experimental"),
            | Command::Subsystem(Subsystem::Postgres { command: crate::subsystem::postgres::commands::Command::Up { diff: true, .. }, .. }) => anyhow::bail!("diff is experimental"),
            | Command::Subsystem(Subsystem::Postgres { command: crate::subsystem::postgres::commands::Command::Down { diff: true, .. }, .. }) => anyhow::bail!("diff is experimental"),
            | Command::Subsystem(Subsystem::Sqlite { command: crate::subsystem::sqlite::commands::Command::Diff, .. }) => anyhow::bail!("diff is experimental"),
            | Command::Subsystem(Subsystem::Sqlite { command: crate::subsystem::sqlite::commands::Command::Up { diff: true, .. }, .. }) => anyhow::bail!("diff is experimental"),
            | Command::Subsystem(Subsystem::Sqlite { command: crate::subsystem::sqlite::commands::Command::Down { diff: true, .. }, .. }) => anyhow::bail!("diff is experimental"),
            | _ => (),
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum Subsystem {
    Postgres {
        path: PathBuf,
        command: crate::subsystem::postgres::commands::Command,
    },
    Sqlite {
        path: PathBuf,
        command: crate::subsystem::sqlite::commands::Command,
    },
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
    Subsystem(Subsystem),
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
                clap::Command::new("subsystem")
                    .about("Manages subsystems.")
                    .subcommand_required(true)
                    .aliases(["sub", "s"])
                    .subcommand(
                        clap::Command::new("postgres")
                            .aliases(["pg"])
                            .about("Manages PostgreSQL migrations.")
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
                                    .arg(clap::Arg::new("diff").short('d').long("diff").required(false).num_args(0).help("Show migration diff before applying"))
                                    .arg(clap::Arg::new("dry").long("dry").required(false).num_args(0).help("Execute migration in a transaction but rollback instead of committing")),
                            )
                            .subcommand(
                                clap::Command::new("down")
                                    .about("Rolls back the migrations.")
                                    .arg(clap::Arg::new("timeout").short('t').long("timeout").required(false))
                                    .arg(clap::Arg::new("remote").short('r').long("remote").required(false).num_args(0))
                                    .arg(clap::Arg::new("count").short('c').long("count").required(false))
                                    .arg(clap::Arg::new("diff").short('d').long("diff").required(false).num_args(0).help("Show migration diff before applying"))
                                    .arg(clap::Arg::new("dry").long("dry").required(false).num_args(0).help("Execute migration in a transaction but rollback instead of committing")),
                            )
                            .subcommand(
                                clap::Command::new("list")
                                    .about("Lists all applied migrations."),
                            )
                            .subcommand(
                                clap::Command::new("history")
                                    .about("Manages migration history.")
                                    .subcommand_required(true)
                                    .subcommand(
                                        clap::Command::new("sync")
                                            .about("Upserts all remote migrations locally."),
                                    )
                                    .subcommand(
                                        clap::Command::new("fix")
                                            .about("Shuffles all non-run local migrations to the end of the chain."),
                                    ),
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
                                            .arg(clap::Arg::new("timeout").short('t').long("timeout").required(false))
                                            .arg(clap::Arg::new("dry").long("dry").required(false).num_args(0).help("Execute migration in a transaction but rollback instead of committing")),
                                    )
                                    .subcommand(
                                        clap::Command::new("down")
                                            .about("Reverts a specific migration.")
                                            .arg(clap::Arg::new("id").help("Migration ID to revert").required(true))
                                            .arg(clap::Arg::new("timeout").short('t').long("timeout").required(false))
                                            .arg(clap::Arg::new("remote").short('r').long("remote").required(false).num_args(0))
                                            .arg(clap::Arg::new("dry").long("dry").required(false).num_args(0).help("Execute migration in a transaction but rollback instead of committing")),
                                    ),
                            ),
                    )
                    .subcommand(
                        clap::Command::new("sqlite")
                            .aliases(["sql"])
                            .about("Manages SQLite migrations.")
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
                                    .arg(clap::Arg::new("diff").short('d').long("diff").required(false).num_args(0).help("Show migration diff before applying"))
                                    .arg(clap::Arg::new("dry").long("dry").required(false).num_args(0).help("Execute migration in a transaction but rollback instead of committing")),
                            )
                            .subcommand(
                                clap::Command::new("down")
                                    .about("Rolls back the migrations.")
                                    .arg(clap::Arg::new("timeout").short('t').long("timeout").required(false))
                                    .arg(clap::Arg::new("remote").short('r').long("remote").required(false).num_args(0))
                                    .arg(clap::Arg::new("count").short('c').long("count").required(false))
                                    .arg(clap::Arg::new("diff").short('d').long("diff").required(false).num_args(0).help("Show migration diff before applying"))
                                    .arg(clap::Arg::new("dry").long("dry").required(false).num_args(0).help("Execute migration in a transaction but rollback instead of committing")),
                            )
                            .subcommand(
                                clap::Command::new("list")
                                    .about("Lists all applied migrations."),
                            )
                            .subcommand(
                                clap::Command::new("history")
                                    .about("Manages migration history.")
                                    .subcommand_required(true)
                                    .subcommand(
                                        clap::Command::new("sync")
                                            .about("Upserts all remote migrations locally."),
                                    )
                                    .subcommand(
                                        clap::Command::new("fix")
                                            .about("Shuffles all non-run local migrations to the end of the chain."),
                                    ),
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
                                            .arg(clap::Arg::new("timeout").short('t').long("timeout").required(false))
                                            .arg(clap::Arg::new("dry").long("dry").required(false).num_args(0).help("Execute migration in a transaction but rollback instead of committing")),
                                    )
                                    .subcommand(
                                        clap::Command::new("down")
                                            .about("Reverts a specific migration.")
                                            .arg(clap::Arg::new("id").help("Migration ID to revert").required(true))
                                            .arg(clap::Arg::new("timeout").short('t').long("timeout").required(false))
                                            .arg(clap::Arg::new("remote").short('r').long("remote").required(false).num_args(0))
                                            .arg(clap::Arg::new("dry").long("dry").required(false).num_args(0).help("Execute migration in a transaction but rollback instead of committing")),
                                    ),
                            ),
                    )
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
        } else if let Some(subsystem_subc) = command.subcommand_matches("subsystem") {
            if let Some(postgres_subc) = subsystem_subc.subcommand_matches("postgres") {
                let path = Self::get_absolute_path(postgres_subc, "path")?;
                let postgres_cmd = if let Some(_) = postgres_subc.subcommand_matches("init") {
                    crate::subsystem::postgres::commands::Command::Init
                } else if let Some(_) = postgres_subc.subcommand_matches("new") {
                    crate::subsystem::postgres::commands::Command::New
                } else if let Some(up_subc) = postgres_subc.subcommand_matches("up") {
                    crate::subsystem::postgres::commands::Command::Up {
                        timeout: up_subc.get_one::<String>("timeout").map(|s| s.parse::<u64>().unwrap()),
                        count: up_subc.get_one::<String>("count").map(|s| s.parse::<usize>().unwrap()),
                        diff: up_subc.get_flag("diff"),
                        dry: up_subc.get_flag("dry"),
                    }
                } else if let Some(down_subc) = postgres_subc.subcommand_matches("down") {
                    crate::subsystem::postgres::commands::Command::Down {
                        timeout: down_subc.get_one::<String>("timeout").map(|s| s.parse::<u64>().unwrap()),
                        count: down_subc.get_one::<String>("count").map(|s| s.parse::<usize>().unwrap()),
                        remote: down_subc.get_flag("remote"),
                        diff: down_subc.get_flag("diff"),
                        dry: down_subc.get_flag("dry"),
                    }
                } else if let Some(_) = postgres_subc.subcommand_matches("list") {
                    crate::subsystem::postgres::commands::Command::List
                } else if let Some(history_subc) = postgres_subc.subcommand_matches("history") {
                    let history_cmd = if let Some(_) = history_subc.subcommand_matches("sync") {
                        crate::subsystem::postgres::commands::HistoryCommand::Sync
                    } else if let Some(_) = history_subc.subcommand_matches("fix") {
                        crate::subsystem::postgres::commands::HistoryCommand::Fix
                    } else {
                        unreachable!();
                    };
                    crate::subsystem::postgres::commands::Command::History(history_cmd)
                } else if let Some(_) = postgres_subc.subcommand_matches("diff") {
                    crate::subsystem::postgres::commands::Command::Diff
                } else if let Some(apply_subc) = postgres_subc.subcommand_matches("apply") {
                    if let Some(up_subc) = apply_subc.subcommand_matches("up") {
                        crate::subsystem::postgres::commands::Command::Apply(crate::subsystem::postgres::commands::MigrationApply::Up {
                            id: up_subc.get_one::<String>("id").unwrap().clone(),
                            timeout: up_subc.get_one::<String>("timeout").map(|s| s.parse::<u64>().unwrap()),
                            dry: up_subc.get_flag("dry"),
                        })
                    } else if let Some(down_subc) = apply_subc.subcommand_matches("down") {
                        crate::subsystem::postgres::commands::Command::Apply(crate::subsystem::postgres::commands::MigrationApply::Down {
                            id: down_subc.get_one::<String>("id").unwrap().clone(),
                            timeout: down_subc.get_one::<String>("timeout").map(|s| s.parse::<u64>().unwrap()),
                            remote: down_subc.get_flag("remote"),
                            dry: down_subc.get_flag("dry"),
                        })
                    } else {
                        unreachable!();
                    }
                } else {
                    unreachable!();
                };
                Command::Subsystem(Subsystem::Postgres {
                    path,
                    command: postgres_cmd,
                })
            } else if let Some(sqlite_subc) = subsystem_subc.subcommand_matches("sqlite") {
                let path = Self::get_absolute_path(sqlite_subc, "path")?;
                let sqlite_cmd = if let Some(_) = sqlite_subc.subcommand_matches("init") {
                    crate::subsystem::sqlite::commands::Command::Init
                } else if let Some(_) = sqlite_subc.subcommand_matches("new") {
                    crate::subsystem::sqlite::commands::Command::New
                } else if let Some(up_subc) = sqlite_subc.subcommand_matches("up") {
                    crate::subsystem::sqlite::commands::Command::Up {
                        timeout: up_subc.get_one::<String>("timeout").map(|s| s.parse::<u64>().unwrap()),
                        count: up_subc.get_one::<String>("count").map(|s| s.parse::<usize>().unwrap()),
                        diff: up_subc.get_flag("diff"),
                        dry: up_subc.get_flag("dry"),
                    }
                } else if let Some(down_subc) = sqlite_subc.subcommand_matches("down") {
                    crate::subsystem::sqlite::commands::Command::Down {
                        timeout: down_subc.get_one::<String>("timeout").map(|s| s.parse::<u64>().unwrap()),
                        count: down_subc.get_one::<String>("count").map(|s| s.parse::<usize>().unwrap()),
                        remote: down_subc.get_flag("remote"),
                        diff: down_subc.get_flag("diff"),
                        dry: down_subc.get_flag("dry"),
                    }
                } else if let Some(_) = sqlite_subc.subcommand_matches("list") {
                    crate::subsystem::sqlite::commands::Command::List
                } else if let Some(history_subc) = sqlite_subc.subcommand_matches("history") {
                    let history_cmd = if let Some(_) = history_subc.subcommand_matches("sync") {
                        crate::subsystem::sqlite::commands::HistoryCommand::Sync
                    } else if let Some(_) = history_subc.subcommand_matches("fix") {
                        crate::subsystem::sqlite::commands::HistoryCommand::Fix
                    } else {
                        unreachable!();
                    };
                    crate::subsystem::sqlite::commands::Command::History(history_cmd)
                } else if let Some(_) = sqlite_subc.subcommand_matches("diff") {
                    crate::subsystem::sqlite::commands::Command::Diff
                } else if let Some(apply_subc) = sqlite_subc.subcommand_matches("apply") {
                    if let Some(up_subc) = apply_subc.subcommand_matches("up") {
                        crate::subsystem::sqlite::commands::Command::Apply(crate::subsystem::sqlite::commands::MigrationApply::Up {
                            id: up_subc.get_one::<String>("id").unwrap().clone(),
                            timeout: up_subc.get_one::<String>("timeout").map(|s| s.parse::<u64>().unwrap()),
                            dry: up_subc.get_flag("dry"),
                        })
                    } else if let Some(down_subc) = apply_subc.subcommand_matches("down") {
                        crate::subsystem::sqlite::commands::Command::Apply(crate::subsystem::sqlite::commands::MigrationApply::Down {
                            id: down_subc.get_one::<String>("id").unwrap().clone(),
                            timeout: down_subc.get_one::<String>("timeout").map(|s| s.parse::<u64>().unwrap()),
                            remote: down_subc.get_flag("remote"),
                            dry: down_subc.get_flag("dry"),
                        })
                    } else {
                        unreachable!();
                    }
                } else {
                    unreachable!();
                };
                Command::Subsystem(Subsystem::Sqlite {
                    path,
                    command: sqlite_cmd,
                })
            } else {
                return Err(anyhow::anyhow!("subsystem required"));
            }
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
