mod args;
mod config;
mod core;
mod reference;
mod subsystem;

use anyhow::{
    Context,
    Result,
};

#[tokio::main]
async fn main() {
    let code = match run().await {
        | Ok(()) => 0,
        | Err(err) => {
            let code = crate::core::outcome::exit_code(&err);
            eprintln!("Error: {err:#}");
            code
        },
    };
    std::process::exit(code);
}

async fn run() -> Result<()> {
    let cmd = args::ClapArgumentLoader::load()?;
    match cmd.command {
        | args::Command::Manual { path, format } => {
            std::fs::create_dir_all(&path)
                .with_context(|| format!("Failed to create directory: {}", path.display()))?;
            match format {
                | args::ManualFormat::Manpages => reference::build_manpages(&path)?,
                | args::ManualFormat::Markdown => reference::build_markdown(&path)?,
            }
            Ok(())
        },
        | args::Command::Autocomplete { path, shell } => {
            std::fs::create_dir_all(&path)
                .with_context(|| format!("Failed to create directory: {}", path.display()))?;
            reference::build_shell_completion(&path, &shell)?;
            Ok(())
        },
        | args::Command::Subsystem(subsystem) => subsystem::driver::dispatch(*subsystem).await,
    }
}
