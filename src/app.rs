use std::path::Path;

use anyhow::Result;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use tokio::task::JoinHandle;

use crate::config::AppConfig;
use crate::layout::{check_terminal_size, ZoneLayout};
use crate::theme::{resolve_theme, Theme};
use crate::widget::{self, Widget};

struct ZoneEntry {
    layout: ZoneLayout,
    widget: Box<dyn Widget>,
}

pub struct App {
    config_path: String,
    zones: Vec<ZoneEntry>,
    theme: Theme,
    update_tasks: Vec<JoinHandle<()>>,
}

impl App {
    pub fn from_config(config_path: &str) -> Result<Self> {
        let config = AppConfig::load(Path::new(config_path))?;
        let theme = resolve_theme(&config.theme);

        let mut zones = Vec::new();
        let mut update_tasks = Vec::new();

        for zone_cfg in &config.zones {
            let (widget, handle) =
                widget::create_widget(&zone_cfg.widget, zone_cfg.config.as_ref())?;
            let layout = ZoneLayout::from_config(zone_cfg, widget.min_size());
            if let Some(h) = handle {
                update_tasks.push(h);
            }
            zones.push(ZoneEntry { layout, widget });
        }

        Ok(Self {
            config_path: config_path.to_string(),
            zones,
            theme,
            update_tasks,
        })
    }

    pub fn draw(&self, frame: &mut Frame) {
        let size = frame.area();
        let layouts: Vec<_> = self.zones.iter().map(|z| &z.layout).collect();

        if let Some((req_w, req_h)) = check_terminal_size(&layouts, size.width, size.height) {
            self.draw_too_small(frame, size, req_w, req_h);
            return;
        }

        // Fill background
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

    pub fn reload(&mut self) -> Result<()> {
        self.abort_update_tasks();
        let new_app = App::from_config(&self.config_path)?;
        self.zones = new_app.zones;
        self.theme = new_app.theme;
        self.update_tasks = new_app.update_tasks;
        Ok(())
    }

    pub fn abort_update_tasks(&mut self) {
        for handle in self.update_tasks.drain(..) {
            handle.abort();
        }
    }
}
