pub mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "watchie",
    author,
    version,
    about = "Browse IMDB and stream/download movies via playimdb.com",
    long_about = None,
    propagate_version = true,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Search IMDB for movies/series
    Search {
        /// Search query
        query: String,

        /// Maximum results to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Show full details for a title
    Info {
        /// IMDB ID (e.g. tt1234567) or title name
        id: String,
    },

    /// Download a title (or stream directly with --play)
    #[command(alias = "dl")]
    Download {
        /// IMDB ID (e.g. tt1234567) or title name
        id: String,

        /// Output directory (overrides config)
        #[arg(short, long)]
        output: Option<String>,

        /// Open in player instead of downloading
        #[arg(short, long)]
        player: bool,
    },

    /// Stream a title directly in your media player
    Play {
        /// IMDB ID (e.g. tt1234567) or title name
        id: String,
    },

    /// Browse top charts or genres
    Top {
        /// Category: action, comedy, horror, top, toptv, boxoffice, etc.
        #[arg(short, long)]
        category: Option<String>,

        /// Number of results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Manage watchie configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set the download directory
    SetDownloadDir {
        /// Path to download directory
        path: String,
    },
    /// Set the media player command
    SetPlayer {
        /// Player command (e.g. mpv, vlc)
        command: String,
    },
    /// Set the TMDB API key
    SetTmdbKey {
        /// TMDB API key
        key: String,
    },
    /// Set a custom playimdb URL (or mirror)
    SetPlayimdbUrl {
        /// Custom URL (e.g. https://runimdb.com)
        url: String,
    },
    /// Show config file path
    Path,
}
