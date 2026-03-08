use anyhow::Result;
use ratatui::layout::Rect;
use ratatui::Frame;
use tokio::task::JoinHandle;

use crate::theme::Theme;

pub mod clock;
pub mod date;
pub mod system;
pub mod text;
pub mod weather;

/// All widgets implement this for rendering.
pub trait Widget: Send + Sync {
    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme);
    fn error(&self) -> Option<String> {
        None
    }
    /// Minimum (width, height) in characters this widget needs to render usefully.
    /// Includes border (2 chars each axis). Layout engine uses this to compute
    /// required terminal size.
    fn min_size(&self) -> (u16, u16) {
        (12, 5)
    }
}

pub type CreateResult = (Box<dyn Widget>, Option<JoinHandle<()>>);

pub fn create_widget(
    widget_type: &str,
    config: Option<&toml::Value>,
) -> Result<CreateResult> {
    match widget_type {
        "clock" => Ok((Box::new(clock::ClockWidget::new(config)?), None)),
        "weather" => {
            let w = weather::WeatherWidget::new(config)?;
            let handle = w.start_updates();
            Ok((Box::new(w), Some(handle)))
        }
        "date" => Ok((Box::new(date::DateWidget::new(config)?), None)),
        "system" => {
            let w = system::SystemWidget::new(config)?;
            let handle = w.start_updates();
            Ok((Box::new(w), Some(handle)))
        }
        "text" => Ok((Box::new(text::TextWidget::new(config)?), None)),
        _ => anyhow::bail!("Unknown widget type: {}", widget_type),
    }
}
