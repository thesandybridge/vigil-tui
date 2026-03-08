use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use sysinfo::System;
use tokio::task::JoinHandle;

use crate::theme::Theme;

struct SystemState {
    cpu_usage: f32,
    mem_used: u64,
    mem_total: u64,
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            mem_used: 0,
            mem_total: 0,
        }
    }
}

pub struct SystemWidget {
    state: Arc<Mutex<SystemState>>,
}

impl SystemWidget {
    pub fn new(_config: Option<&toml::Value>) -> Result<Self> {
        Ok(Self {
            state: Arc::new(Mutex::new(SystemState::default())),
        })
    }

    pub fn start_updates(&self) -> JoinHandle<()> {
        let state = Arc::clone(&self.state);
        tokio::spawn(async move {
            let mut sys = System::new();
            loop {
                sys.refresh_cpu_usage();
                tokio::time::sleep(Duration::from_millis(200)).await;
                sys.refresh_memory();
                let data = SystemState {
                    cpu_usage: sys.global_cpu_usage(),
                    mem_used: sys.used_memory(),
                    mem_total: sys.total_memory(),
                };
                {
                    let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
                    *s = data;
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        })
    }
}

fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1_073_741_824;
    const MB: u64 = 1_048_576;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.0} MB", bytes as f64 / MB as f64)
    }
}

fn bar_color(pct: f32, default: Color) -> Color {
    if pct > 90.0 {
        Color::Red
    } else if pct > 70.0 {
        Color::Yellow
    } else {
        default
    }
}

fn render_bar<'a>(pct: f32, width: u16, filled_color: Color, empty_color: Color) -> Line<'a> {
    let filled = ((pct / 100.0) * width as f32).round() as u16;
    let empty = width.saturating_sub(filled);
    Line::from(vec![
        Span::styled(
            "█".repeat(filled as usize),
            Style::default().fg(filled_color),
        ),
        Span::styled(
            "░".repeat(empty as usize),
            Style::default().fg(empty_color),
        ),
    ])
}

impl super::Widget for SystemWidget {
    fn min_size(&self) -> (u16, u16) {
        // cpu label + bar + blank + mem label + bar + borders
        (20, 9)
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme, is_focused: bool) {
        let (cpu_usage, mem_used, mem_total) = {
            let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
            (s.cpu_usage, s.mem_used, s.mem_total)
        };

        let mem_pct = if mem_total > 0 {
            (mem_used as f64 / mem_total as f64 * 100.0) as f32
        } else {
            0.0
        };

        let bar_width = area.width.saturating_sub(4); // borders + padding

        let cpu_color = bar_color(cpu_usage, theme.accent);
        let mem_color = bar_color(mem_pct, theme.accent);

        // Content: cpu label, cpu bar, blank, mem label, mem bar = 5 lines
        let content_height = 5u16;
        let inner_height = area.height.saturating_sub(2);
        let pad_top = inner_height.saturating_sub(content_height) / 2;

        let mut lines: Vec<Line> = Vec::new();
        for _ in 0..pad_top {
            lines.push(Line::from(""));
        }

        lines.push(Line::from(Span::styled(
            format!(" CPU  {:.1}%", cpu_usage),
            Style::default().fg(theme.fg),
        )));
        lines.push(render_bar(cpu_usage, bar_width, cpu_color, theme.dim));

        lines.push(Line::from(""));

        lines.push(Line::from(Span::styled(
            format!(
                " Mem  {} / {}  ({:.1}%)",
                format_bytes(mem_used),
                format_bytes(mem_total),
                mem_pct,
            ),
            Style::default().fg(theme.fg),
        )));
        lines.push(render_bar(mem_pct, bar_width, mem_color, theme.dim));

        let block = Block::default()
            .title(" System ")
            .borders(Borders::ALL)
            .border_type(theme.border_type())
            .border_style(Style::default().fg(if is_focused { theme.accent } else { theme.dim }));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .style(Style::default().fg(theme.fg));

        frame.render_widget(paragraph, area);
    }
}
