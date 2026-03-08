mod app;
mod config;
mod digits;
mod layout;
mod theme;
mod widget;

use std::io;
use std::time::Duration;

use anyhow::Result;
use app::App;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.toml".to_string());

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &config_path).await;

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config_path: &str,
) -> Result<()> {
    let mut app = App::from_config(config_path)?;

    loop {
        terminal.draw(|frame| app.draw(frame))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('r') => {
                            let _ = app.reload();
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    app.abort_update_tasks();
    Ok(())
}
