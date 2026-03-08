use std::path::Path;

use anyhow::Result;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use tokio::task::JoinHandle;

use crate::config::{AppConfig, LayoutMode, ZoneConfig};
use crate::layout::{self, check_terminal_size, ZoneLayout};
use crate::theme::{resolve_theme, Theme};
use crate::widget::{self, Widget};

struct ZoneEntry {
    layout: ZoneLayout,
    widget: Box<dyn Widget>,
}

pub struct App {
    config_path: String,
    zones: Vec<ZoneEntry>,
    zone_configs: Vec<ZoneConfig>,
    theme: Theme,
    update_tasks: Vec<JoinHandle<()>>,
    layout_mode: LayoutMode,
    config_error: Option<String>,
}

impl App {
    pub fn from_config(config_path: &str) -> Result<Self> {
        let config = AppConfig::load(Path::new(config_path))?;
        let theme = resolve_theme(&config.theme);
        let layout_mode = config.layout;

        let mut widgets: Vec<Box<dyn Widget>> = Vec::new();
        let mut update_tasks = Vec::new();

        for zone_cfg in &config.zones {
            let (widget, handle) =
                widget::create_widget(&zone_cfg.widget, zone_cfg.config.as_ref())?;
            if let Some(h) = handle {
                update_tasks.push(h);
            }
            widgets.push(widget);
        }

        let layouts = match layout_mode {
            LayoutMode::Absolute => layout::build_absolute(&config.zones, &widgets),
            LayoutMode::Rows => layout::build_rows(&config.zones, &widgets, 40),
        };

        let zones = layouts
            .into_iter()
            .zip(widgets)
            .map(|(layout, widget)| ZoneEntry { layout, widget })
            .collect();

        Ok(Self {
            config_path: config_path.to_string(),
            zone_configs: config.zones,
            zones,
            theme,
            update_tasks,
            layout_mode,
            config_error: None,
        })
    }

    pub fn draw(&self, frame: &mut Frame) {
        let size = frame.area();

        if self.layout_mode == LayoutMode::Rows {
            self.draw_rows(frame, size);
            return;
        }

        let layouts: Vec<_> = self.zones.iter().map(|z| &z.layout).collect();

        if let Some((req_w, req_h)) = check_terminal_size(&layouts, size.width, size.height) {
            self.draw_too_small(frame, size, req_w, req_h);
            return;
        }

        let bg_block = Block::default().style(Style::default().bg(self.theme.bg));
        frame.render_widget(bg_block, size);

        for entry in &self.zones {
            let area = entry.layout.to_rect(size.width, size.height);
            if let Some(err) = entry.widget.error() {
                self.draw_error(frame, area, &entry.layout.id, &err);
            } else {
                entry.widget.draw(frame, area, &self.theme);
            }
        }

        self.draw_config_error(frame, size);
    }

    fn draw_rows(&self, frame: &mut Frame, size: Rect) {
        // Build min-size proxies for layout computation
        let proxies: Vec<Box<dyn Widget>> = self
            .zones
            .iter()
            .map(|z| -> Box<dyn Widget> { Box::new(MinSizeProxy(z.widget.min_size())) })
            .collect();

        let layouts = layout::build_rows(&self.zone_configs, &proxies, size.height);

        let layout_refs: Vec<&ZoneLayout> = layouts.iter().collect();
        if let Some((req_w, req_h)) = check_terminal_size(&layout_refs, size.width, size.height) {
            self.draw_too_small(frame, size, req_w, req_h);
            return;
        }

        let bg_block = Block::default().style(Style::default().bg(self.theme.bg));
        frame.render_widget(bg_block, size);

        for (entry, layout) in self.zones.iter().zip(layouts.iter()) {
            let area = layout.to_rect(size.width, size.height);
            if let Some(err) = entry.widget.error() {
                self.draw_error(frame, area, &layout.id, &err);
            } else {
                entry.widget.draw(frame, area, &self.theme);
            }
        }

        self.draw_config_error(frame, size);
    }

    fn draw_too_small(&self, frame: &mut Frame, area: Rect, req_w: u16, req_h: u16) {
        let w_color = if area.width >= req_w {
            Color::Green
        } else {
            Color::Red
        };
        let h_color = if area.height >= req_h {
            Color::Green
        } else {
            Color::Red
        };

        let lines = vec![
            Line::from("Terminal too small"),
            Line::from(""),
            Line::from(vec![
                Span::raw("Current: "),
                Span::styled(
                    format!("{}", area.width),
                    Style::default().fg(w_color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" x "),
                Span::styled(
                    format!("{}", area.height),
                    Style::default().fg(h_color).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(format!("Required: {} x {}", req_w, req_h)),
        ];

        let paragraph = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red)),
            );
        frame.render_widget(paragraph, area);
    }

    fn draw_error(&self, frame: &mut Frame, area: Rect, id: &str, error: &str) {
        let block = Block::default()
            .title(format!(" {} [error] ", id))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));
        let paragraph = Paragraph::new(error.to_string())
            .block(block)
            .style(Style::default().fg(Color::Red));
        frame.render_widget(paragraph, area);
    }

    fn draw_config_error(&self, frame: &mut Frame, size: Rect) {
        if let Some(ref msg) = self.config_error {
            let banner_height = 3;
            let area = Rect::new(0, size.height.saturating_sub(banner_height), size.width, banner_height);
            let block = Block::default()
                .title(" config error ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red));
            frame.render_widget(Clear, area);
            let paragraph = Paragraph::new(msg.as_str())
                .block(block)
                .style(Style::default().fg(Color::Red).bg(self.theme.bg));
            frame.render_widget(paragraph, area);
        }
    }

    pub fn reload(&mut self) {
        match App::from_config(&self.config_path) {
            Ok(new_app) => {
                self.abort_update_tasks();
                self.zones = new_app.zones;
                self.zone_configs = new_app.zone_configs;
                self.theme = new_app.theme;
                self.update_tasks = new_app.update_tasks;
                self.layout_mode = new_app.layout_mode;
                self.config_error = None;
            }
            Err(e) => {
                self.config_error = Some(e.to_string());
            }
        }
    }

    pub fn set_config_error(&mut self, msg: String) {
        self.config_error = Some(msg);
    }

    pub fn clear_config_error(&mut self) {
        self.config_error = None;
    }

    pub fn abort_update_tasks(&mut self) {
        for handle in self.update_tasks.drain(..) {
            handle.abort();
        }
    }
}

/// Proxy that provides min_size for layout computation without needing real widgets.
struct MinSizeProxy((u16, u16));

impl Widget for MinSizeProxy {
    fn draw(&self, _frame: &mut Frame, _area: Rect, _theme: &Theme) {}
    fn min_size(&self) -> (u16, u16) {
        self.0
    }
}
