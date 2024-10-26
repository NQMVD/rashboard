use std::ffi::OsStr;
use std::io;
use std::process::Command;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use sysinfo::System;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(1000);
    let mut last_tick = Instant::now();

    let mut sys = System::new_all();

    loop {
        terminal.draw(|f| ui(f, &mut sys))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    // Exit on 'q'
                    break;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui<B: Backend>(f: &mut tui::Frame<B>, sys: &mut System) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ]
            .as_ref(),
        )
        .split(f.size());

    // Memory and Disk Usage
    draw_memory_disk(f, chunks[0], sys);

    // Uptime
    draw_uptime(f, chunks[1]);

    // Available Updates via apt
    draw_apt_updates(f, chunks[2]);

    // Status of Certain Programs
    draw_program_status(f, chunks[3], sys, &["nginx", "mysql"]);

    // Pueue Tasks Status
    draw_pueue_status(f, chunks[4]);
}

fn draw_memory_disk<B: Backend>(f: &mut tui::Frame<B>, area: Rect, sys: &mut System) {
    // sys.refresh_memory();
    sys.refresh_all();

    let total_memory = sys.total_memory() / 1024; // in MB
    let used_memory = sys.used_memory() / 1024; // in MB
    let memory_usage = format!("Memory Usage: {}/{} MB", used_memory, total_memory);

    let text = format!("{}", memory_usage);

    let paragraph = Paragraph::new(text)
        .block(Block::default().title("Memory Usage").borders(Borders::ALL))
        .style(Style::default().fg(Color::Cyan));

    f.render_widget(paragraph, area);
}

fn draw_uptime<B: Backend>(f: &mut tui::Frame<B>, area: Rect) {
    // No need to refresh the system for uptime
    let uptime_seconds = sysinfo::System::uptime();

    let uptime = format!(
        "Uptime: {}d {}h {}m {}s",
        uptime_seconds / 86400,
        (uptime_seconds % 86400) / 3600,
        (uptime_seconds % 3600) / 60,
        uptime_seconds % 60
    );

    let paragraph = Paragraph::new(uptime)
        .block(
            Block::default()
                .title("System Uptime")
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::Green));

    f.render_widget(paragraph, area);
}

fn draw_apt_updates<B: Backend>(f: &mut tui::Frame<B>, area: Rect) {
    let output = Command::new("bash")
        .arg("-c")
        .arg("apt list --upgradable 2>/dev/null | wc -l")
        .output()
        .expect("Failed to execute command");

    let count_str = String::from_utf8_lossy(&output.stdout);
    let count: i32 = count_str.trim().parse().unwrap_or(1) - 1; // Exclude header line

    let updates = format!("Available Updates: {}", count);

    let paragraph = Paragraph::new(updates)
        .block(Block::default().title("Apt Updates").borders(Borders::ALL))
        .style(Style::default().fg(Color::Yellow));

    f.render_widget(paragraph, area);
}

fn draw_program_status<B: Backend>(
    f: &mut tui::Frame<B>,
    area: Rect,
    sys: &mut System,
    programs: &[&str],
) {
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, false);
    let mut statuses = String::new();

    for &program in programs {
        let is_running = sys
            .processes_by_exact_name(OsStr::new(program))
            .next()
            .is_some();
        let status = if is_running {
            format!("{}: Running\n", program)
        } else {
            format!("{}: Not Running\n", program)
        };
        statuses.push_str(&status);
    }

    let paragraph = Paragraph::new(statuses)
        .block(
            Block::default()
                .title("Program Status")
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::Magenta));

    f.render_widget(paragraph, area);
}

fn draw_pueue_status<B: Backend>(f: &mut tui::Frame<B>, area: Rect) {
    let output = Command::new("pueue")
        .arg("status")
        .arg("-g")
        .arg("SERVICES")
        .output()
        .expect("Failed to execute pueue");

    let status_str = String::from_utf8_lossy(&output.stdout);

    let paragraph = Paragraph::new(status_str)
        .block(
            Block::default()
                .title("Pueue SERVICES Group")
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::Blue));

    f.render_widget(paragraph, area);
}
