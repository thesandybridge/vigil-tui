use anyhow::Result;

fn is_browser_target(target: &str, mode: Option<&str>) -> bool {
    mode == Some("browser")
        || target.starts_with("http://")
        || target.starts_with("https://")
}

/// Launch a target. Browser targets open via the `open` crate (fire-and-forget).
/// Terminal targets suspend the TUI, run `sh -c target`, then restore.
pub fn launch(target: &str, mode: Option<&str>) -> Result<()> {
    if is_browser_target(target, mode) {
        let url = if target.starts_with("http://") || target.starts_with("https://") {
            target.to_string()
        } else {
            format!("https://{target}")
        };
        open::that(&url).map_err(|e| anyhow::anyhow!("failed to open browser: {e}"))?;
        return Ok(());
    }

    launch_terminal(target)
}

fn launch_terminal(target: &str) -> Result<()> {
    use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
    use crossterm::terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    };
    use crossterm::ExecutableCommand;
    use std::io;

    // Suspend TUI
    disable_raw_mode()?;
    io::stdout().execute(DisableMouseCapture)?;
    io::stdout().execute(LeaveAlternateScreen)?;

    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(target)
        .status();

    // Restore TUI regardless of command result
    io::stdout().execute(EnterAlternateScreen)?;
    io::stdout().execute(EnableMouseCapture)?;
    enable_raw_mode()?;

    match status {
        Ok(s) if !s.success() => {
            anyhow::bail!("command exited with status {s}");
        }
        Err(e) => anyhow::bail!("failed to run command: {e}"),
        Ok(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_is_browser() {
        assert!(is_browser_target("http://example.com", None));
    }

    #[test]
    fn https_is_browser() {
        assert!(is_browser_target("https://example.com", None));
    }

    #[test]
    fn mode_browser_forces_browser() {
        assert!(is_browser_target("wttr.in", Some("browser")));
    }

    #[test]
    fn plain_command_is_terminal() {
        assert!(!is_browser_target("htop", None));
    }
}
