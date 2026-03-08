use anyhow::Result;
use ratatui::layout::Rect;
use ratatui::style::Style;
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
        let block = Block::default()
            .title(" Date ")
            .borders(Borders::ALL)
            .border_type(theme.border_type())
            .border_style(Style::default().fg(theme.dim));
        let text = format!("Date [{}]", self.format);
        let paragraph = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(theme.fg));
        frame.render_widget(paragraph, area);
    }
}
