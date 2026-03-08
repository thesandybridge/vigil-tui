use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use tokio::task::JoinHandle;

use crate::theme::Theme;

#[derive(Default)]
struct SystemState {
    summary: String,
}

pub struct SystemWidget {
    state: Arc<Mutex<SystemState>>,
    interval: Duration,
}

impl SystemWidget {
    pub fn new(_config: Option<&toml::Value>) -> Result<Self> {
        Ok(Self {
            state: Arc::new(Mutex::new(SystemState {
                summary: "Loading...".to_string(),
            })),
            interval: Duration::from_secs(2),
        })
    }

    pub fn start_updates(&self) -> JoinHandle<()> {
        let state = Arc::clone(&self.state);
        let interval = self.interval;
        tokio::spawn(async move {
            loop {
                // Placeholder: actual sysinfo polling will go here
                {
                    let mut s = state.lock().unwrap();
                    s.summary = "System placeholder".to_string();
                }
                tokio::time::sleep(interval).await;
            }
        })
    }
}

impl super::Widget for SystemWidget {
    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let state = self.state.lock().unwrap();
        let block = Block::default()
            .title(" System ")
            .borders(Borders::ALL)
            .border_type(theme.border_type())
            .border_style(Style::default().fg(theme.dim));
        let paragraph = Paragraph::new(state.summary.as_str())
            .block(block)
            .style(Style::default().fg(theme.fg));
        frame.render_widget(paragraph, area);
    }
}
