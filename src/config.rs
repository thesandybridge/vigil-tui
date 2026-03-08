use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

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
