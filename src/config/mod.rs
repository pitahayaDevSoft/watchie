use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─── Config model ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub download_dir: PathBuf,
    pub player: PlayerConfig,
    pub ui: UiConfig,
    pub network: NetworkConfig,
    #[serde(default)]
    pub api: ApiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiConfig {
    #[serde(default)]
    pub tmdb_key: String,
    #[serde(default)]
    pub playimdb_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerConfig {
    /// Preferred video player command (e.g. "mpv", "vlc", "mplayer")
    pub command: String,
    /// Extra arguments passed to the player
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Number of items per page in the TUI
    pub page_size: usize,
    /// Show poster images in Kitty terminal
    pub kitty_images: bool,
    /// Preferred color theme: "dark" | "light"
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub timeout_secs: u64,
    pub max_retries: u8,
    pub user_agent: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            download_dir: dirs::download_dir()
                .unwrap_or_else(|| PathBuf::from("~/Downloads"))
                .join("watchie"),
            player: PlayerConfig {
                command: crate::player::detect_best_player().unwrap_or_else(|| "mpv".to_string()),
                extra_args: vec![],
            },
            ui: UiConfig {
                page_size: 20,
                kitty_images: crate::kitty::is_kitty(),
                theme: "dark".to_string(),
            },
            network: NetworkConfig {
                timeout_secs: 15,
                max_retries: 3,
                user_agent:
                    "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0"
                        .to_string(),
            },
            api: ApiConfig::default(),
        }
    }
}

impl Config {
    /// Load config from the default location, creating it if it doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Reading config from {}", path.display()))?;
            let cfg: Config = toml::from_str(&content)
                .with_context(|| "Parsing config file")?;
            return Ok(cfg);
        }

        // Create default config
        let cfg = Config::default();
        cfg.save()?;
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .context("Cannot find config directory")?
            .join("watchie");
        Ok(dir.join("config.toml"))
    }

    pub fn ensure_download_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.download_dir)?;
        Ok(())
    }
}

// ─── Detection helpers ────────────────────────────────────────────────────────
