use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use sysinfo::{ProcessesToUpdate, System};
use tokio::task::JoinHandle;

use crate::theme::Theme;

struct ProcessInfo {
    name: String,
    cpu: f32,
    mem: u64,
}

struct SystemState {
    cpu_usage: f32,
    mem_used: u64,
    mem_total: u64,
    top_processes: Vec<ProcessInfo>,
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            mem_used: 0,
            mem_total: 0,
            top_processes: Vec::new(),
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
                sys.refresh_processes(ProcessesToUpdate::All, true);
                tokio::time::sleep(Duration::from_millis(200)).await;
                sys.refresh_memory();

                let mut procs: Vec<_> = sys.processes().values().collect();
                procs.sort_by(|a, b| {
                    b.cpu_usage()
                        .partial_cmp(&a.cpu_usage())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let top_processes = procs
                    .iter()
                    .take(3)
                    .map(|p| ProcessInfo {
                        name: p.name().to_string_lossy().to_string(),
                        cpu: p.cpu_usage(),
                        mem: p.memory(),
                    })
                    .collect();

                let data = SystemState {
                    cpu_usage: sys.global_cpu_usage(),
                    mem_used: sys.used_memory(),
                    mem_total: sys.total_memory(),
                    top_processes,
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
        // cpu label + bar + blank + mem label + bar + borders (processes clip gracefully)
        (20, 9)
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme, is_focused: bool) {
        let (cpu_usage, mem_used, mem_total, top_processes) = {
            let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let procs: Vec<(String, f32, u64)> = s
                .top_processes
                .iter()
                .map(|p| (p.name.clone(), p.cpu, p.mem))
                .collect();
            (s.cpu_usage, s.mem_used, s.mem_total, procs)
        };

        let mem_pct = if mem_total > 0 {
            (mem_used as f64 / mem_total as f64 * 100.0) as f32
        } else {
            0.0
        };

        let bar_width = area.width.saturating_sub(4); // borders + padding

        let cpu_color = bar_color(cpu_usage, theme.accent);
        let mem_color = bar_color(mem_pct, theme.accent);

        // cpu label + bar + blank + mem label + bar + blank + procs header + 3 procs = 10
        let content_height = 10u16;
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

        lines.push(Line::from(""));

        lines.push(Line::from(Span::styled(
            " Top Processes",
            Style::default().fg(theme.dim),
        )));

        let name_width = 12;
        for (name, cpu, mem) in &top_processes {
            let truncated: String = if name.len() > name_width {
                name[..name_width].to_string()
            } else {
                format!("{:<width$}", name, width = name_width)
            };
            lines.push(Line::from(vec![
                Span::styled(format!(" {truncated}"), Style::default().fg(theme.fg)),
                Span::styled(
                    format!(" {:>5.1}%", cpu),
                    Style::default().fg(bar_color(*cpu, theme.accent)),
                ),
                Span::styled(
                    format!("  {}", format_bytes(*mem)),
                    Style::default().fg(theme.dim),
                ),
            ]));
        }
        // pad if fewer than 3 processes
        for _ in top_processes.len()..3 {
            lines.push(Line::from(""));
        }

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
