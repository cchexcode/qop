use {
    crate::{
        core::outcome::usage,
        subsystem::cli,
    },
    anyhow::Result,
    clap::{
        Arg,
        ArgAction,
        value_parser,
    },
    std::path::PathBuf,
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
        // No command is experimental yet. The flag stays so a later command can be
        // gated without changing the CLI shape.
        match self.privileges {
            | Privilege::Normal | Privilege::Experimental => Ok(()),
        }
    }
}

/// One enabled subsystem. The payload is that subsystem's own command, not a
/// shared one.
#[derive(Debug)]
pub(crate) enum Subsystem {
    #[cfg(feature = "sub+postgres")]
    Postgres(crate::subsystem::postgres::commands::Invocation),
    #[cfg(feature = "sub+sqlite")]
    Sqlite(crate::subsystem::sqlite::commands::Invocation),
    #[cfg(feature = "sub+surrealdb")]
    Surrealdb(crate::subsystem::surrealdb::commands::Invocation),
}

#[derive(Debug)]
pub(crate) enum Command {
    Manual { path: PathBuf, format: ManualFormat },
    Autocomplete { path: PathBuf, shell: clap_complete::Shell },
    Subsystem(Box<Subsystem>),
}

pub(crate) struct ClapArgumentLoader;

impl ClapArgumentLoader {
    pub(crate) fn root_command() -> clap::Command {
        let enabled = enabled_subsystems();
        let enabled_str = if enabled.is_empty() {
            "none".to_string()
        } else {
            enabled.join(", ")
        };
        let mut root = clap::Command::new("qop")
            .version(env!("CARGO_PKG_VERSION"))
            .about(format!(
                "Database migrations for savages.\n\nEnabled subsystems: {enabled_str}"
            ))
            .author("cchexcode <alexanderh.weber@outlook.com>")
            .propagate_version(true)
            .subcommand_required(true)
            .arg(
                Arg::new("experimental")
                    .short('e')
                    .long("experimental")
                    .action(ArgAction::SetTrue)
                    .help("Enables experimental features."),
            )
            .subcommand(
                clap::Command::new("man")
                    .about("Renders the manual.")
                    .arg(Arg::new("out").short('o').long("out").required(true))
                    .arg(
                        Arg::new("format")
                            .short('f')
                            .long("format")
                            .value_parser(["manpages", "markdown"])
                            .required(true),
                    ),
            )
            .subcommand(
                clap::Command::new("autocomplete")
                    .about("Renders shell completion scripts.")
                    .arg(Arg::new("out").short('o').long("out").required(true))
                    .arg(
                        Arg::new("shell")
                            .short('s')
                            .long("shell")
                            .value_parser(value_parser!(clap_complete::Shell))
                            .required(true),
                    ),
            );

        #[cfg(any(feature = "sub+postgres", feature = "sub+sqlite", feature = "sub+surrealdb"))]
        {
            let mut subsystem = clap::Command::new("subsystem")
                .about(format!("Manages subsystems (enabled: {enabled_str})."))
                .subcommand_required(true)
                .aliases(["sub", "s"]);
            #[cfg(feature = "sub+postgres")]
            {
                subsystem = subsystem.subcommand(crate::subsystem::postgres::commands::clap_command());
            }
            #[cfg(feature = "sub+sqlite")]
            {
                subsystem = subsystem.subcommand(crate::subsystem::sqlite::commands::clap_command());
            }
            #[cfg(feature = "sub+surrealdb")]
            {
                subsystem = subsystem.subcommand(crate::subsystem::surrealdb::commands::clap_command());
            }
            root = root.subcommand(subsystem);
        }
        root
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
                path: cli::absolute_path(subc, "out")?,
                format: match subc.get_one::<String>("format").map(String::as_str) {
                    | Some("manpages") => ManualFormat::Manpages,
                    | Some("markdown") => ManualFormat::Markdown,
                    | _ => return Err(usage("argument \"format\": unknown format")),
                },
            }
        } else if let Some(subc) = command.subcommand_matches("autocomplete") {
            Command::Autocomplete {
                path: cli::absolute_path(subc, "out")?,
                shell: *subc
                    .get_one::<clap_complete::Shell>("shell")
                    .ok_or_else(|| usage("argument \"shell\" is required"))?,
            }
        } else if let Some(subsystem) = command.subcommand_matches("subsystem") {
            Command::Subsystem(Box::new(parse_subsystem(subsystem)?))
        } else {
            return Err(usage("unknown command"));
        };
        let callargs = CallArgs {
            privileges,
            command: cmd,
        };
        callargs.validate()?;
        Ok(callargs)
    }
}

fn enabled_subsystems() -> Vec<&'static str> {
    vec![
        #[cfg(feature = "sub+postgres")]
        crate::subsystem::postgres::commands::NAME,
        #[cfg(feature = "sub+surrealdb")]
        crate::subsystem::surrealdb::commands::NAME,
        #[cfg(feature = "sub+sqlite")]
        crate::subsystem::sqlite::commands::NAME,
    ]
}

fn parse_subsystem(matches: &clap::ArgMatches) -> Result<Subsystem> {
    #[cfg(feature = "sub+postgres")]
    if let Some(backend) = matches.subcommand_matches(crate::subsystem::postgres::commands::NAME) {
        return Ok(Subsystem::Postgres(crate::subsystem::postgres::commands::parse(
            backend,
        )?));
    }
    #[cfg(feature = "sub+sqlite")]
    if let Some(backend) = matches.subcommand_matches(crate::subsystem::sqlite::commands::NAME) {
        return Ok(Subsystem::Sqlite(crate::subsystem::sqlite::commands::parse(backend)?));
    }
    #[cfg(feature = "sub+surrealdb")]
    if let Some(backend) = matches.subcommand_matches(crate::subsystem::surrealdb::commands::NAME) {
        return Ok(Subsystem::Surrealdb(crate::subsystem::surrealdb::commands::parse(
            backend,
        )?));
    }
    Err(usage("subsystem required"))
}
