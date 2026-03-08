use std::io;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::Result;
use notify::{EventKind, RecursiveMode, Watcher};
use vigil_tui::app::App;
use vigil_tui::config::resolve_config_path;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = resolve_config_path(std::env::args().nth(1))?;

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

    let result = run(&mut terminal, config_path.to_str().unwrap_or("config.toml")).await;

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config_path: &str,
) -> Result<()> {
    let mut app = App::from_config(config_path)?;

    // Watch parent directory for config changes (editors like vim/neovim
    // do write-to-temp + rename, which replaces the inode and breaks
    // per-file watches after the first save)
    let config_abs = std::fs::canonicalize(config_path)?;
    let config_filename = config_abs.file_name().unwrap().to_owned();
    let parent_dir = config_abs.parent().unwrap().to_owned();

    let (tx, rx) = mpsc::channel();
    let mut _watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            let dominated = matches!(
                event.kind,
                EventKind::Modify(_) | EventKind::Create(_)
            );
            if dominated
                && event
                    .paths
                    .iter()
                    .any(|p| p.file_name() == Some(&config_filename))
            {
                let _ = tx.send(());
            }
        }
    })?;
    _watcher.watch(&parent_dir, RecursiveMode::NonRecursive)?;

    let mut last_reload = Instant::now();
    let mut pending_reload = false;

    loop {
        terminal.draw(|frame| app.draw(frame))?;

        // Drain all pending file change events
        while rx.try_recv().is_ok() {
            pending_reload = true;
        }
        if pending_reload && last_reload.elapsed() > Duration::from_millis(300) {
            pending_reload = false;
            last_reload = Instant::now();
            app.reload();
        }

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match (key.code, key.modifiers) {
                (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                (KeyCode::Char('r'), _) => {
                    app.reload();
                }
                _ => {}
            }
        }
    }

    app.abort_update_tasks();
    Ok(())
}
