use anyhow::Result;
use std::process::Stdio;
use tokio::process::Command;

use crate::config::Config;

/// Launch the configured media player with the given URL or file path.
pub async fn play(url: &str, referer: Option<&str>, config: &Config) -> Result<()> {
    let player = &config.player.command;
    let mut args = config.player.extra_args.clone();

    // Dynamically inject referer arguments if player is mpv or vlc
    if let Some(ref_val) = referer {
        if player == "mpv" {
            args.push(format!("--referrer={}", ref_val));
        } else if player == "vlc" {
            args.push(format!("--http-referrer={}", ref_val));
        }
    }

    tracing::info!("Launching player: {} {} with referer: {:?}", player, url, referer);

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
pub fn build_command_string(config: &Config, url: &str, referer: Option<&str>) -> String {
    let mut parts = vec![config.player.command.clone()];
    parts.extend(config.player.extra_args.clone());
    if let Some(ref_val) = referer {
        if config.player.command == "mpv" {
            parts.push(format!("--referrer={}", ref_val));
        } else if config.player.command == "vlc" {
            parts.push(format!("--http-referrer={}", ref_val));
        }
    }
    parts.push(url.to_string());
    parts.join(" ")
}
