use {
    super::config::SubsystemSurrealdb,
    crate::{
        config::Config,
        core::{
            command::Operation,
            migration::MigrationTree,
            service::MigrationService,
        },
        subsystem::cli,
    },
    anyhow::Result,
    clap::ArgMatches,
    std::path::PathBuf,
};

pub(crate) const NAME: &str = "surrealdb";

#[derive(Debug)]
pub(crate) enum ConfigCommand {
    Init {
        connection: String,
        namespace: String,
        database: String,
        username: Option<String>,
        password: Option<String>,
        force: bool,
    },
}

#[derive(Debug)]
pub(crate) enum Command {
    Config(ConfigCommand),
    Run(Operation),
}

#[derive(Debug)]
pub(crate) struct Invocation {
    pub path: PathBuf,
    pub command: Command,
}

pub(crate) fn clap_command() -> clap::Command {
    cli::migration_command(NAME, &["surreal"], "Manages SurrealDB migrations.", config_command())
}

pub(crate) fn parse(matches: &ArgMatches) -> Result<Invocation> {
    let path = cli::absolute_path(matches, "path")?;
    if let Some(config) = matches.subcommand_matches("config") {
        let init = config
            .subcommand_matches("init")
            .ok_or_else(|| crate::core::outcome::usage("config command required"))?;
        return Ok(Invocation {
            path,
            command: Command::Config(ConfigCommand::Init {
                connection: cli::required(init, "endpoint")?,
                namespace: cli::required(init, "namespace")?,
                database: cli::required(init, "database")?,
                username: init.get_one::<String>("username").cloned(),
                password: init.get_one::<String>("password").cloned(),
                force: init.get_flag("force"),
            }),
        });
    }
    Ok(Invocation {
        path,
        command: Command::Run(cli::parse_operation(matches)?),
    })
}

pub(crate) async fn execute(invocation: Invocation) -> Result<()> {
    match invocation.command {
        | Command::Config(ConfigCommand::Init {
            connection,
            namespace,
            database,
            username,
            password,
            force,
        }) => {
            SubsystemSurrealdb::sample(
                &connection,
                &namespace,
                &database,
                username.as_deref(),
                password.as_deref(),
            )
            .write(&invocation.path, force)?;
            println!("Bootstrapped surrealdb config to {}", invocation.path.display());
            Ok(())
        },
        | Command::Run(Operation::New { comment, locked }) => {
            let created = MigrationTree::from_config_path(&invocation.path)?.create(
                comment.as_deref(),
                locked,
                super::SCRIPTS,
            )?;
            println!("Created new migration: {}", created.display());
            Ok(())
        },
        | Command::Run(operation) => {
            let config = load(&invocation.path)?;
            let repo =
                super::repo::SurrealdbRepo::from_config(&invocation.path, config, operation.checks_store_version())
                    .await?;
            MigrationService::new(repo).execute(operation).await
        },
    }
}

fn load(path: &std::path::Path) -> Result<SubsystemSurrealdb> {
    match Config::load(path)?.subsystem {
        | crate::config::Subsystem::Surrealdb(config) => Ok(config),
        #[cfg(any(feature = "sub+postgres", feature = "sub+sqlite"))]
        | _ => anyhow::bail!("Configuration is not a surrealdb subsystem"),
    }
}

fn config_command() -> clap::Command {
    clap::Command::new("config")
        .about("Configuration commands.")
        .subcommand_required(true)
        .subcommand(
            clap::Command::new("init")
                .about("Writes a sample configuration for SurrealDB.")
                .arg(
                    clap::Arg::new("endpoint")
                        .short('e')
                        .long("endpoint")
                        .help("SurrealDB HTTP endpoint")
                        .required(true),
                )
                .arg(
                    clap::Arg::new("namespace")
                        .short('n')
                        .long("namespace")
                        .help("SurrealDB namespace")
                        .required(true),
                )
                .arg(
                    clap::Arg::new("database")
                        .short('d')
                        .long("database")
                        .help("SurrealDB database")
                        .required(true),
                )
                .arg(
                    clap::Arg::new("username")
                        .long("user")
                        .help("SurrealDB username")
                        .requires("password"),
                )
                .arg(
                    clap::Arg::new("password")
                        .long("password")
                        .help("SurrealDB password")
                        .requires("username"),
                )
                .arg(
                    clap::Arg::new("force")
                        .long("force")
                        .action(clap::ArgAction::SetTrue)
                        .help("Overwrite an existing configuration file"),
                ),
        )
}
