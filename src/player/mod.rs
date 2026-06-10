use anyhow::Result;
use std::process::Stdio;
use tokio::process::Command;

use crate::config::Config;

/// Launch the configured media player with the given URL or file path.
pub async fn play(url: &str, config: &Config) -> Result<()> {
    let player = &config.player.command;
    let args = &config.player.extra_args;

    tracing::info!("Launching player: {} {}", player, url);

    let mut cmd = Command::new(player);
    cmd.args(args);
    cmd.arg(url);
    cmd.stdin(Stdio::null());

    // Don't capture output — let it go to the terminal
    let mut child = cmd.spawn()?;
    child.wait().await?;

    Ok(())
}

/// Detect the best available player from common options.
pub fn detect_best_player() -> Option<String> {
    let candidates = ["mpv", "vlc", "mplayer", "celluloid", "totem", "smplayer"];
    for c in &candidates {
        if which::which(c).is_ok() {
            return Some(c.to_string());
        }
    }
    None
}

/// Build the player launch command as a string for display purposes.
pub fn build_command_string(config: &Config, url: &str) -> String {
    let mut parts = vec![config.player.command.clone()];
    parts.extend(config.player.extra_args.clone());
    parts.push(url.to_string());
    parts.join(" ")
}
