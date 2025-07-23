use {
    anyhow::Result, clap::Arg, std::
        str::FromStr
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
            | _ => (),
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum Migration {
    Init {
        path: String,
    },
    New {
        path: String,
    },
    Up {
        path: String,
        timeout: Option<u64>,
        count: Option<usize>,
    },
    Down {
        path: String,
        timeout: Option<u64>,
        count: Option<usize>,
        remote: bool,
    },
    List {
        path: String,
    },
    Sync {
        path: String,
    },
    Fix {
        path: String,
    },
}

#[derive(Debug)]
pub(crate) enum Command {
    Manual {
        path: String,
        format: ManualFormat,
    },
    Autocomplete {
        path: String,
        shell: clap_complete::Shell,
    },
    Migration(Migration),
    Init {
        path: String,
    },
}

pub(crate) struct ClapArgumentLoader {}

impl ClapArgumentLoader {
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
                    .subcommand(
                        clap::Command::new("init")
                            .about("Initializes the database.")
                            .arg(clap::Arg::new("path").short('p').long("path").default_value("./qop.toml")),
                    )
                    .subcommand(
                        clap::Command::new("new")
                            .about("Creates a new migration.")
                            .arg(clap::Arg::new("path").short('p').long("path").default_value("./qop.toml")),
                    )
                    .subcommand(
                        clap::Command::new("up")
                            .about("Runs the migrations.")
                            .arg(clap::Arg::new("path").short('p').long("path").default_value("./qop.toml"))
                            .arg(clap::Arg::new("timeout").short('t').long("timeout").required(false))
                            .arg(clap::Arg::new("count").short('c').long("count").required(false)),
                    )
                    .subcommand(
                        clap::Command::new("down")
                            .about("Rolls back the migrations.")
                            .arg(clap::Arg::new("path").short('p').long("path").default_value("./qop.toml"))
                            .arg(clap::Arg::new("timeout").short('t').long("timeout").required(false))
                            .arg(clap::Arg::new("remote").short('r').long("remote").required(false).num_args(0))
                            .arg(clap::Arg::new("count").short('c').long("count").required(false)),
                    )
                    .subcommand(
                        clap::Command::new("list")
                            .about("Lists all applied migrations.")
                            .arg(clap::Arg::new("path").short('p').long("path").default_value("./qop.toml")),
                    )
                    .subcommand(
                        clap::Command::new("sync")
                            .about("Upserts all remote migrations locally.")
                            .arg(clap::Arg::new("path").short('p').long("path").default_value("./qop.toml")),
                    )
                    .subcommand(
                        clap::Command::new("fix")
                            .about("Shuffles all non-run local migrations to the end of the chain.")
                            .arg(clap::Arg::new("path").short('p').long("path").default_value("./qop.toml")),
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
                path: subc.get_one::<String>("out").unwrap().into(),
                format: match subc.get_one::<String>("format").unwrap().as_str() {
                    | "manpages" => ManualFormat::Manpages,
                    | "markdown" => ManualFormat::Markdown,
                    | _ => return Err(anyhow::anyhow!("argument \"format\": unknown format")),
                },
            }
        } else if let Some(subc) = command.subcommand_matches("autocomplete") {
            Command::Autocomplete {
                path: subc.get_one::<String>("out").unwrap().into(),
                shell: clap_complete::Shell::from_str(subc.get_one::<String>("shell").unwrap().as_str()).unwrap(),
            }
        } else if let Some(subc) = command.subcommand_matches("init") {
            Command::Init {
                path: subc.get_one::<String>("path").unwrap().into(),
            }
        } else if let Some(subc) = command.subcommand_matches("migration") {
            if let Some(subc) = subc.subcommand_matches("init") {
                Command::Migration(Migration::Init {
                    path: subc.get_one::<String>("path").unwrap().into(),
                })
            }
            else if let Some(subc) = subc.subcommand_matches("new") {
                Command::Migration(Migration::New {
                    path: subc.get_one::<String>("path").unwrap().into(),
                })
            } else if let Some(subc) = subc.subcommand_matches("up") {
                Command::Migration(Migration::Up {
                    path: subc.get_one::<String>("path").unwrap().into(),
                    timeout: subc.get_one::<String>("timeout").map(|s| s.parse::<u64>().unwrap()),
                    count: subc.get_one::<String>("count").map(|s| s.parse::<usize>().unwrap()),
                })
            } else if let Some(subc) = subc.subcommand_matches("down") {
                Command::Migration(Migration::Down {
                    path: subc.get_one::<String>("path").unwrap().into(),
                    timeout: subc.get_one::<String>("timeout").map(|s| s.parse::<u64>().unwrap()),
                    count: subc.get_one::<String>("count").map(|s| s.parse::<usize>().unwrap()),
                    remote: subc.get_flag("remote"),
                })
            } else if let Some(subc) = subc.subcommand_matches("list") {
                Command::Migration(Migration::List {
                    path: subc.get_one::<String>("path").unwrap().into(),
                })
            } else if let Some(subc) = subc.subcommand_matches("sync") {
                Command::Migration(Migration::Sync {
                    path: subc.get_one::<String>("path").unwrap().into(),
                })
            } else if let Some(subc) = subc.subcommand_matches("fix") {
                Command::Migration(Migration::Fix {
                    path: subc.get_one::<String>("path").unwrap().into(),
                })
            } else {
                return Err(anyhow::anyhow!("unknown command"));
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
