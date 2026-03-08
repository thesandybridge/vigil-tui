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
            let suffix = if is_pm { " PM" } else { " AM" };
            (format!("{:2}:{:02}", hour, now.minute()), suffix)
        } else {
            (format!("{:2}:{:02}", now.hour(), now.minute()), "")
        };

        let big_lines = render_big_text(&time_str);
        let seconds_text = format!(":{:02}", now.second());

        // Total content: 5 lines for big digits + 1 blank + 1 seconds line = 7
        let content_height = 7u16;
        let inner_height = area.height.saturating_sub(2); // border
        let pad_top = inner_height.saturating_sub(content_height) / 2;

        let mut lines: Vec<Line> = Vec::new();

        for _ in 0..pad_top {
            lines.push(Line::from(""));
        }

        for big_line in &big_lines {
            let mut spans = vec![Span::styled(
                big_line.clone(),
                Style::default().fg(theme.accent),
            )];
            if !suffix.is_empty() {
                // Only append suffix on the last big-digit line
                if big_line == big_lines.last().unwrap() {
                    spans.push(Span::styled(
                        suffix.to_string(),
                        Style::default().fg(theme.dim),
                    ));
                }
            }
            lines.push(Line::from(spans));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            seconds_text,
            Style::default().fg(theme.dim),
        )));

        let block = Block::default()
            .title(" Clock ")
            .borders(Borders::ALL)
            .border_type(theme.border_type())
            .border_style(Style::default().fg(theme.dim));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.fg));

        frame.render_widget(paragraph, area);
    }
}
