// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Event handling and main TUI loop

use std::io;

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use super::app::{App, AppMode};
use super::ui;

/// Run the TUI application
pub fn run_tui() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new()?;

    // Main loop
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {}", err);
    }

    Ok(())
}

/// Main application loop
fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> where B::Error: Send + Sync + 'static {
    // Initial draw
    terminal.draw(|f| ui::render(f, app))?;

    loop {
        // Block waiting for input - no polling delay, instant response
        if let Event::Key(key) = event::read()? {
            // Only handle key press events, ignore release/repeat to prevent double-triggering
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Clear status message on any keypress
            app.status_message = None;

            // Handle filter input mode specially
            if app.filter_active {
                match key.code {
                    KeyCode::Enter => app.confirm_filter(),
                    KeyCode::Esc => app.cancel_filter(),
                    KeyCode::Backspace => app.filter_backspace(),
                    KeyCode::Char(c) => app.filter_input(c),
                    _ => {}
                }
                terminal.draw(|f| ui::render(f, app))?;
                continue;
            }

            // Help mode - any key closes it
            if app.mode == AppMode::Help {
                app.back();
                terminal.draw(|f| ui::render(f, app))?;
                continue;
            }

            // Normal key handling
            match key.code {
                KeyCode::Char('q') => {
                    return Ok(());
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(());
                }
                KeyCode::Char('?') => {
                    app.toggle_help();
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    app.navigate_down();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    app.navigate_up();
                }
                KeyCode::Char('g') => {
                    app.go_to_top();
                }
                KeyCode::Char('G') => {
                    app.go_to_bottom();
                }
                KeyCode::PageUp => {
                    app.page_up();
                }
                KeyCode::PageDown => {
                    app.page_down();
                }
                KeyCode::Enter => {
                    app.enter();
                }
                KeyCode::Esc | KeyCode::Backspace => {
                    app.back();
                }
                KeyCode::Char('/') if app.mode == AppMode::Workspaces => {
                    app.start_filter();
                }
                KeyCode::Char('r') => {
                    app.refresh();
                }
                _ => continue, // No redraw needed for unhandled keys
            }

            // Redraw after handling input
            terminal.draw(|f| ui::render(f, app))?;
        }
    }
}


