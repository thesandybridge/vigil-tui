use anyhow::Result;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::theme::Theme;

pub struct TextWidget {
    content: String,
    title: Option<String>,
    align: Alignment,
}

impl TextWidget {
    pub fn new(config: Option<&toml::Value>) -> Result<Self> {
        let content = config
            .and_then(|c| c.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let title = config
            .and_then(|c| c.get("title"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let align = match config
            .and_then(|c| c.get("align"))
            .and_then(|v| v.as_str())
            .unwrap_or("center")
        {
            "left" => Alignment::Left,
            "right" => Alignment::Right,
            _ => Alignment::Center,
        };

        Ok(Self {
            content,
            title,
            align,
        })
    }
}

impl super::Widget for TextWidget {
    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let inner_height = area.height.saturating_sub(2);
        let content_lines: Vec<&str> = self.content.lines().collect();
        let content_height = content_lines.len() as u16;
        let pad_top = inner_height.saturating_sub(content_height) / 2;

        let mut lines: Vec<Line> = Vec::new();
        for _ in 0..pad_top {
            lines.push(Line::from(""));
        }
        for line in &content_lines {
            lines.push(Line::from(*line));
        }

        let block_title = self
            .title
            .as_ref()
            .map(|t| format!(" {} ", t))
            .unwrap_or_else(|| " Text ".to_string());

        let block = Block::default()
            .title(block_title)
            .borders(Borders::ALL)
            .border_type(theme.border_type())
            .border_style(Style::default().fg(theme.dim));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .alignment(self.align)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(theme.fg));

        frame.render_widget(paragraph, area);
    }
}
