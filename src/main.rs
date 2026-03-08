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
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
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

    // Install panic hook to restore terminal on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
        original_hook(panic_info);
    }));

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

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match (key.code, key.modifiers) {
                (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                (KeyCode::Char('r'), _) => {
                    let _ = app.reload();
                }
                _ => {}
            }
        }
    }

    app.abort_update_tasks();
    Ok(())
}
