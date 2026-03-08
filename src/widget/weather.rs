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
struct WeatherState {
    summary: String,
}

pub struct WeatherWidget {
    state: Arc<Mutex<WeatherState>>,
    interval: Duration,
}

impl WeatherWidget {
    pub fn new(config: Option<&toml::Value>) -> Result<Self> {
        let _latitude = config
            .and_then(|c| c.get("latitude"))
            .and_then(|v| v.as_float())
            .unwrap_or(0.0);
        let _longitude = config
            .and_then(|c| c.get("longitude"))
            .and_then(|v| v.as_float())
            .unwrap_or(0.0);

        Ok(Self {
            state: Arc::new(Mutex::new(WeatherState {
                summary: "Loading...".to_string(),
            })),
            interval: Duration::from_secs(600),
        })
    }

    pub fn start_updates(&self) -> JoinHandle<()> {
        let state = Arc::clone(&self.state);
        let interval = self.interval;
        tokio::spawn(async move {
            loop {
                // Placeholder: actual fetch will go here
                {
                    let mut s = state.lock().unwrap();
                    s.summary = "Weather placeholder".to_string();
                }
                tokio::time::sleep(interval).await;
            }
        })
    }
}

impl super::Widget for WeatherWidget {
    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let state = self.state.lock().unwrap();
        let block = Block::default()
            .title(" Weather ")
            .borders(Borders::ALL)
            .border_type(theme.border_type())
            .border_style(Style::default().fg(theme.dim));
        let paragraph = Paragraph::new(state.summary.as_str())
            .block(block)
            .style(Style::default().fg(theme.fg));
        frame.render_widget(paragraph, area);
    }
}
