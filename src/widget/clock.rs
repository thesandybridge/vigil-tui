use anyhow::Result;
use chrono::Local;
use chrono::Timelike;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::digits::render_big_text;
use crate::theme::Theme;

pub struct ClockWidget {
    format: String,
}

impl ClockWidget {
    pub fn new(config: Option<&toml::Value>) -> Result<Self> {
        let format = config
            .and_then(|c| c.get("format"))
            .and_then(|v| v.as_str())
            .unwrap_or("24hr")
            .to_string();
        Ok(Self { format })
    }
}

impl super::Widget for ClockWidget {
    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let now = Local::now();

        let (time_str, suffix) = if self.format == "12hr" {
            let (is_pm, hour) = now.hour12();
            let suffix = if is_pm { "PM" } else { "AM" };
            (format!("{:>2}:{:02}", hour, now.minute()), suffix)
        } else {
            (format!("{:>2}:{:02}", now.hour(), now.minute()), "")
        };

        let big_lines = render_big_text(&time_str);

        // Content height: 5 big lines + 1 blank + 1 seconds/suffix line
        let content_height = 7u16;
        let inner_height = area.height.saturating_sub(2);
        let pad_top = inner_height.saturating_sub(content_height) / 2;

        let mut lines: Vec<Line> = Vec::new();

        for _ in 0..pad_top {
            lines.push(Line::from(""));
        }

        for big_line in &big_lines {
            lines.push(Line::from(Span::styled(
                big_line.clone(),
                Style::default().fg(theme.accent),
            )));
        }

        // Seconds + AM/PM on a separate line below the big digits
        let detail = if suffix.is_empty() {
            format!(":{:02}", now.second())
        } else {
            format!(":{:02} {suffix}", now.second())
        };

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            detail,
            Style::default().fg(theme.dim),
        )));

        let block = Block::default()
            .title(" Clock ")
            .borders(Borders::ALL)
            .border_type(theme.border_type())
            .border_style(Style::default().fg(theme.dim));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    fn min_size(&self) -> (u16, u16) {
        // 5 digits * 5w + 4 gaps + 2 border = ~33 wide
        // 5 digit lines + 2 detail lines + 2 border + 2 padding = ~11 tall
        (35, 11)
    }
}
