use {
    super::config::SubsystemPostgres,
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

pub(crate) const NAME: &str = "postgres";

#[derive(Debug)]
pub(crate) enum ConfigCommand {
    Init { connection: String, force: bool },
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
    cli::migration_command(NAME, &["pg"], "Manages PostgreSQL migrations.", config_command())
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
                connection: cli::required(init, "conn")?,
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
        | Command::Config(ConfigCommand::Init { connection, force }) => {
            SubsystemPostgres::sample(&connection).write(&invocation.path, force)?;
            println!("Bootstrapped postgres config to {}", invocation.path.display());
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
                super::repo::PostgresRepo::from_config(&invocation.path, config, operation.checks_store_version())
                    .await?;
            MigrationService::new(repo).execute(operation).await
        },
    }
}

fn load(path: &std::path::Path) -> Result<SubsystemPostgres> {
    match Config::load(path)?.subsystem {
        | crate::config::Subsystem::Postgres(config) => Ok(config),
        #[cfg(any(feature = "sub+sqlite", feature = "sub+surrealdb"))]
        | _ => anyhow::bail!("Configuration is not a postgres subsystem"),
    }
}

fn config_command() -> clap::Command {
    clap::Command::new("config")
        .about("Configuration commands.")
        .subcommand_required(true)
        .subcommand(
            clap::Command::new("init")
                .about("Writes a sample configuration for Postgres.")
                .arg(
                    clap::Arg::new("conn")
                        .short('c')
                        .long("conn")
                        .help("Database connection string")
                        .required(true),
                )
                .arg(
                    clap::Arg::new("force")
                        .long("force")
                        .action(clap::ArgAction::SetTrue)
                        .help("Overwrite an existing configuration file"),
                ),
        )
}
