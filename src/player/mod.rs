use anyhow::Result;
use std::process::Stdio;
use tokio::process::Command;

use crate::config::Config;

/// Launch the configured media player with the given URL or file path.
pub async fn play(url: &str, referer: Option<&str>, config: &Config) -> Result<()> {
    let player = &config.player.command;
    let mut args = config.player.extra_args.clone();

    // Dynamically inject referer and user-agent arguments if player is mpv or vlc
    if player == "mpv" {
        if let Some(ref_val) = referer {
            args.push(format!("--referrer={}", ref_val));
        }
        args.push(format!("--user-agent={}", config.network.user_agent));
    } else if player == "vlc" {
        if let Some(ref_val) = referer {
            args.push(format!("--http-referrer={}", ref_val));
        }
        args.push(format!("--http-user-agent={}", config.network.user_agent));
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
    if config.player.command == "mpv" {
        if let Some(ref_val) = referer {
            parts.push(format!("--referrer={}", ref_val));
        }
        parts.push(format!("--user-agent={}", config.network.user_agent));
    } else if config.player.command == "vlc" {
        if let Some(ref_val) = referer {
            parts.push(format!("--http-referrer={}", ref_val));
        }
        parts.push(format!("--http-user-agent={}", config.network.user_agent));
    }
    parts.push(url.to_string());
    parts.join(" ")
}
