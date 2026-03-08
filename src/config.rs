use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ThemeConfig {
    Preset(String),
    Custom(CustomTheme),
}

#[derive(Debug, Deserialize)]
pub struct CustomTheme {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub accent: Option<String>,
    pub dim: Option<String>,
    pub border: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ZoneConfig {
    pub id: String,
    pub widget: String,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub min_width: Option<u16>,
    pub min_height: Option<u16>,
    pub config: Option<toml::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub theme: Option<ThemeConfig>,
    pub icons: Option<bool>,
    pub zones: Vec<ZoneConfig>,
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;
        let config: AppConfig =
            toml::from_str(&contents).with_context(|| "Failed to parse config TOML")?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        for zone in &self.zones {
            anyhow::ensure!(
                zone.width > 0 && zone.height > 0,
                "Zone '{}' has zero width or height",
                zone.id
            );
            anyhow::ensure!(
                zone.x + zone.width <= 100 && zone.y + zone.height <= 100,
                "Zone '{}' exceeds 100% bounds (x+w={}, y+h={})",
                zone.id,
                zone.x + zone.width,
                zone.y + zone.height
            );
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn use_icons(&self) -> bool {
        self.icons.unwrap_or(true)
    }
}

const DEFAULT_CONFIG: &str = include_str!("../config.example.toml");

/// Returns the config path, resolving in order:
/// 1. Explicit CLI argument
/// 2. $XDG_CONFIG_HOME/vigil-tui/config.toml (or ~/.config/vigil-tui/config.toml)
/// 3. ./config.toml (legacy fallback)
///
/// If the XDG path doesn't exist, creates the directory and writes the default config.
pub fn resolve_config_path(cli_arg: Option<String>) -> Result<PathBuf> {
    // Explicit path from CLI
    if let Some(path) = cli_arg {
        return Ok(PathBuf::from(path));
    }

    // XDG config directory
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config")
        })
        .join("vigil-tui");

    let config_path = config_dir.join("config.toml");

    if config_path.exists() {
        return Ok(config_path);
    }

    // Legacy: check ./config.toml
    let local = PathBuf::from("config.toml");
    if local.exists() {
        return Ok(local);
    }

    // First run: create default config in XDG dir
    std::fs::create_dir_all(&config_dir)
        .with_context(|| format!("Failed to create config dir: {}", config_dir.display()))?;
    std::fs::write(&config_path, DEFAULT_CONFIG)
        .with_context(|| format!("Failed to write default config: {}", config_path.display()))?;

    eprintln!("Created default config at {}", config_path.display());

    Ok(config_path)
}
