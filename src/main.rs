mod cli;
mod config;
mod downloader;
mod imdb;
mod kitty;
mod playimdb;
mod player;
mod tui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("watchie=info".parse()?),
        )
        .with_target(false)
        .without_time()
        .init();

    let cli = Cli::parse();
    let config = Config::load()?;

    match cli.command {
        // No subcommand → launch TUI
        None => {
            tui::run(config).await?;
        }

        Some(Commands::Search { query, limit }) => {
            cli::commands::search(&query, limit, &config).await?;
        }

        Some(Commands::Info { id }) => {
            cli::commands::info(&id, &config).await?;
        }

        Some(Commands::Download { id, output, player }) => {
            cli::commands::download_or_play(&id, output.as_deref(), player, &config).await?;
        }

        Some(Commands::Play { id }) => {
            cli::commands::play(&id, &config).await?;
        }

        Some(Commands::Top { category, limit }) => {
            cli::commands::top(category.as_deref(), limit, &config).await?;
        }

        Some(Commands::Config { action }) => {
            cli::commands::config_cmd(action, &config).await?;
        }

    }

    Ok(())
}
