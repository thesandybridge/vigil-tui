use anyhow::Result;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

pub struct TextWidget {
    content: String,
}

impl TextWidget {
    pub fn new(config: Option<&toml::Value>) -> Result<Self> {
        let content = config
            .and_then(|c| c.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(Self { content })
    }
}

impl super::Widget for TextWidget {
    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(" Text ")
            .borders(Borders::ALL)
            .border_type(theme.border_type())
            .border_style(Style::default().fg(theme.dim));
        let paragraph = Paragraph::new(self.content.as_str())
            .block(block)
            .style(Style::default().fg(theme.fg));
        frame.render_widget(paragraph, area);
    }
}
