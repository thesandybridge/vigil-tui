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
    // Absolute mode fields (layout = "absolute" or omitted)
    #[serde(default)]
    pub x: u16,
    #[serde(default)]
    pub y: u16,
    #[serde(default)]
    pub width: u16,
    #[serde(default)]
    pub height: u16,
    // Rows mode fields
    pub row: Option<u16>,
    pub min_width: Option<u16>,
    pub min_height: Option<u16>,
    pub config: Option<toml::Value>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    Absolute,
    Rows,
}

impl Default for LayoutMode {
    fn default() -> Self {
        Self::Absolute
    }
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub layout: LayoutMode,
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
        if self.layout == LayoutMode::Absolute {
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
        } else {
            // Rows mode: validate column widths within each row group
            let mut row_groups: std::collections::HashMap<u16, Vec<&ZoneConfig>> =
                std::collections::HashMap::new();
            let mut auto_row = 0u16;
            for zone in &self.zones {
                let row = zone.row.unwrap_or_else(|| {
                    auto_row += 1;
                    auto_row
                });
                row_groups.entry(row).or_default().push(zone);
            }
            for (row, zones) in &row_groups {
                if zones.len() > 1 {
                    let total_width: u16 = zones.iter().map(|z| z.width).sum();
                    anyhow::ensure!(
                        total_width <= 100,
                        "Row {} column widths sum to {}%, must be <= 100%",
                        row,
                        total_width
                    );
                }
            }
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
/// 2. Platform config dir (via dirs crate)
/// 3. ./config.toml (legacy fallback)
///
/// On first run, creates config dir and writes the default config.
pub fn resolve_config_path(cli_arg: Option<String>) -> Result<PathBuf> {
    if let Some(path) = cli_arg {
        return Ok(PathBuf::from(path));
    }

    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
        .join("vigil-tui");

    let config_path = config_dir.join("config.toml");

    if config_path.exists() {
        return Ok(config_path);
    }

    let local = PathBuf::from("config.toml");
    if local.exists() {
        return Ok(local);
    }

    std::fs::create_dir_all(&config_dir)
        .with_context(|| format!("Failed to create config dir: {}", config_dir.display()))?;
    std::fs::write(&config_path, DEFAULT_CONFIG)
        .with_context(|| format!("Failed to write default config: {}", config_path.display()))?;

    eprintln!("Created default config at {}", config_path.display());

    Ok(config_path)
}
