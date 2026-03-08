use anyhow::Result;
use chrono::Local;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

pub struct DateWidget {
    format: String,
}

impl DateWidget {
    pub fn new(config: Option<&toml::Value>) -> Result<Self> {
        let format = config
            .and_then(|c| c.get("format"))
            .and_then(|v| v.as_str())
            .unwrap_or("%A, %B %d, %Y")
            .to_string();
        Ok(Self { format })
    }
}

impl super::Widget for DateWidget {
    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let now = Local::now();
        let date_str = now.format(&self.format).to_string();

        let inner_height = area.height.saturating_sub(2);
        let pad_top = inner_height.saturating_sub(1) / 2;

        let mut lines: Vec<Line> = Vec::new();
        for _ in 0..pad_top {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(date_str));

        let block = Block::default()
            .title(" Date ")
            .borders(Borders::ALL)
            .border_type(theme.border_type())
            .border_style(Style::default().fg(theme.dim));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.fg));

        frame.render_widget(paragraph, area);
    }

    fn min_size(&self) -> (u16, u16) {
        // "Wednesday, September 30, 2026" = ~30 chars + borders
        (22, 3)
    }
}
