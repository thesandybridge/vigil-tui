use ratatui::style::Color;

#[derive(Debug, Clone)]
pub enum BorderStyle {
    Rounded,
    Plain,
    Double,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub fg: Color,
    pub bg: Color,
    pub accent: Color,
    pub dim: Color,
    pub border: BorderStyle,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            fg: Color::Reset,
            bg: Color::Reset,
            accent: Color::Reset,
            dim: Color::DarkGray,
            border: BorderStyle::Rounded,
        }
    }
}

impl Theme {
    pub fn border_type(&self) -> ratatui::widgets::BorderType {
        match self.border {
            BorderStyle::Rounded => ratatui::widgets::BorderType::Rounded,
            BorderStyle::Plain => ratatui::widgets::BorderType::Plain,
            BorderStyle::Double => ratatui::widgets::BorderType::Double,
        }
    }
}

fn hex_to_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

pub fn resolve_theme(config: &Option<crate::config::ThemeConfig>) -> Theme {
    match config {
        None => Theme::default(),
        Some(crate::config::ThemeConfig::Preset(name)) => preset(name),
        Some(crate::config::ThemeConfig::Custom(custom)) => {
            let base = Theme::default();
            Theme {
                fg: custom.fg.as_deref().and_then(hex_to_color).unwrap_or(base.fg),
                bg: custom.bg.as_deref().and_then(hex_to_color).unwrap_or(base.bg),
                accent: custom.accent.as_deref().and_then(hex_to_color).unwrap_or(base.accent),
                dim: custom.dim.as_deref().and_then(hex_to_color).unwrap_or(base.dim),
                border: custom.border.as_deref().map(|b| match b {
                    "plain" => BorderStyle::Plain,
                    "double" => BorderStyle::Double,
                    _ => BorderStyle::Rounded,
                }).unwrap_or(base.border),
            }
        }
    }
}

fn preset(name: &str) -> Theme {
    match name {
        "gruvbox" => Theme {
            fg: Color::Rgb(0xeb, 0xdb, 0xb2),
            bg: Color::Rgb(0x28, 0x28, 0x28),
            accent: Color::Rgb(0xfe, 0x80, 0x19),
            dim: Color::Rgb(0x66, 0x5c, 0x54),
            border: BorderStyle::Rounded,
        },
        "catppuccin-mocha" => Theme {
            fg: Color::Rgb(0xcd, 0xd6, 0xf4),
            bg: Color::Rgb(0x1e, 0x1e, 0x2e),
            accent: Color::Rgb(0x89, 0xb4, 0xfa),
            dim: Color::Rgb(0x58, 0x5b, 0x70),
            border: BorderStyle::Rounded,
        },
        "catppuccin-latte" => Theme {
            fg: Color::Rgb(0x4c, 0x4f, 0x69),
            bg: Color::Rgb(0xef, 0xf1, 0xf5),
            accent: Color::Rgb(0x1e, 0x66, 0xf5),
            dim: Color::Rgb(0x9c, 0xa0, 0xb0),
            border: BorderStyle::Rounded,
        },
        "nord" => Theme {
            fg: Color::Rgb(0xec, 0xef, 0xf4),
            bg: Color::Rgb(0x2e, 0x34, 0x40),
            accent: Color::Rgb(0x88, 0xc0, 0xd0),
            dim: Color::Rgb(0x4c, 0x56, 0x6a),
            border: BorderStyle::Rounded,
        },
        "tokyo-night" => Theme {
            fg: Color::Rgb(0xc0, 0xca, 0xf5),
            bg: Color::Rgb(0x1a, 0x1b, 0x26),
            accent: Color::Rgb(0x7a, 0xa2, 0xf7),
            dim: Color::Rgb(0x56, 0x5f, 0x89),
            border: BorderStyle::Rounded,
        },
        "dracula" => Theme {
            fg: Color::Rgb(0xf8, 0xf8, 0xf2),
            bg: Color::Rgb(0x28, 0x2a, 0x36),
            accent: Color::Rgb(0xbd, 0x93, 0xf9),
            dim: Color::Rgb(0x62, 0x72, 0xa4),
            border: BorderStyle::Rounded,
        },
        "solarized-dark" => Theme {
            fg: Color::Rgb(0x83, 0x94, 0x96),
            bg: Color::Rgb(0x00, 0x2b, 0x36),
            accent: Color::Rgb(0x26, 0x8b, 0xd2),
            dim: Color::Rgb(0x58, 0x6e, 0x75),
            border: BorderStyle::Rounded,
        },
        _ => {
            eprintln!("Unknown theme preset '{}', using terminal defaults", name);
            Theme::default()
        }
    }
}
