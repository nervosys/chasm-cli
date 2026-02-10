// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! UI rendering for the TUI

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, TableState, Wrap,
    },
    Frame,
};

use super::app::{App, AppMode};

/// Color scheme for the TUI (Ayu Monokai)
#[allow(dead_code)]
pub struct Colors;

#[allow(dead_code)]
impl Colors {
    // Background colors
    pub const HEADER_BG: Color = Color::Rgb(26, 26, 36); // Dark background

    // Selection colors
    pub const SELECTED_BG: Color = Color::Rgb(53, 53, 71); // Muted selection background
    pub const SELECTED_FG: Color = Color::Rgb(166, 226, 46); // Monokai green

    // Border colors
    pub const BORDER: Color = Color::Rgb(58, 58, 78); // Muted border
    pub const BORDER_FOCUSED: Color = Color::Rgb(253, 151, 31); // Monokai orange

    // Text colors
    pub const TEXT: Color = Color::Rgb(232, 232, 232); // Light text
    pub const TEXT_DIM: Color = Color::Rgb(117, 113, 94); // Muted/comment color

    // Accent colors (Monokai palette)
    pub const ACCENT: Color = Color::Rgb(102, 217, 239); // Monokai cyan
    pub const SUCCESS: Color = Color::Rgb(166, 226, 46); // Monokai green
    pub const WARNING: Color = Color::Rgb(230, 219, 116); // Monokai yellow
    pub const INFO: Color = Color::Rgb(102, 217, 239); // Monokai cyan
    pub const PURPLE: Color = Color::Rgb(174, 129, 255); // Monokai purple
}

/// Render the entire UI
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Footer/status
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_main_content(frame, app, chunks[1]);
    render_footer(frame, app, chunks[2]);

    // Render help overlay if active
    if app.mode == AppMode::Help {
        render_help_overlay(frame);
    }
}

/// Render the header bar
fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = match app.mode {
        AppMode::Workspaces => " CCSM - Workspaces ",
        AppMode::Sessions => " CCSM - Sessions ",
        AppMode::SessionDetail => " CCSM - Session Details ",
        AppMode::Help => " CCSM - Help ",
    };

    let stats = format!(
        " {} workspaces | {} with chats | {} total sessions ",
        app.workspaces.len(),
        app.workspaces_with_chats(),
        app.total_sessions()
    );

    let header = Paragraph::new(Line::from(vec![
        Span::styled(title, Style::default().fg(Colors::ACCENT).bold()),
        Span::raw(" "),
        Span::styled(stats, Style::default().fg(Colors::TEXT_DIM)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Colors::BORDER_FOCUSED))
            .style(Style::default().bg(Colors::HEADER_BG)),
    );

    frame.render_widget(header, area);
}

/// Render the main content area
fn render_main_content(frame: &mut Frame, app: &App, area: Rect) {
    match app.mode {
        AppMode::Workspaces => render_workspaces_view(frame, app, area),
        AppMode::Sessions => render_sessions_view(frame, app, area),
        AppMode::SessionDetail => render_session_detail_view(frame, app, area),
        AppMode::Help => render_workspaces_view(frame, app, area), // Show workspaces behind help
    }
}

/// Render the workspaces table view
fn render_workspaces_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Left: Workspace table
    render_workspace_table(frame, app, chunks[0]);

    // Right: Session preview for selected workspace
    render_session_preview(frame, app, chunks[1]);
}

/// Render the workspace table
fn render_workspace_table(frame: &mut Frame, app: &App, area: Rect) {
    let header_cells = ["#", "Hash", "Project Path", "Sessions", "Status"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Colors::ACCENT).bold()));

    let header = Row::new(header_cells)
        .style(Style::default().bg(Colors::HEADER_BG))
        .height(1);

    let rows: Vec<Row> = app
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(display_idx, &actual_idx)| {
            let ws = &app.workspaces[actual_idx];
            let is_selected = display_idx == app.workspace_index;

            let hash = format!("{}...", &ws.hash[..8.min(ws.hash.len())]);
            let path = ws
                .project_path
                .clone()
                .unwrap_or_else(|| "(none)".to_string());
            let sessions = ws.chat_session_count.to_string();
            let status = if ws.has_chat_sessions { "[OK]" } else { "[-]" };

            let status_style = if ws.has_chat_sessions {
                Style::default().fg(Colors::SUCCESS)
            } else {
                Style::default().fg(Colors::TEXT_DIM)
            };

            let row_style = if is_selected {
                Style::default()
                    .bg(Colors::SELECTED_BG)
                    .fg(Colors::SELECTED_FG)
            } else {
                Style::default().fg(Colors::TEXT)
            };

            Row::new(vec![
                Cell::from(format!("{}", display_idx + 1)),
                Cell::from(hash).style(Style::default().fg(Colors::PURPLE)),
                Cell::from(truncate_path(&path, 40)),
                Cell::from(sessions).style(Style::default().fg(Colors::INFO)),
                Cell::from(status).style(status_style),
            ])
            .style(row_style)
            .height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Length(12),
            Constraint::Min(20),
            Constraint::Length(10),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Colors::BORDER_FOCUSED))
            .title(Span::styled(
                format!(
                    " Workspaces ({}/{}) ",
                    app.workspace_index + 1,
                    app.filtered_indices.len()
                ),
                Style::default().fg(Colors::ACCENT),
            )),
    )
    .row_highlight_style(Style::default().bg(Colors::SELECTED_BG))
    .highlight_symbol(">> ");

    let mut state = TableState::default();
    state.select(Some(app.workspace_index));

    frame.render_stateful_widget(table, area, &mut state);
}

/// Render session preview panel
fn render_session_preview(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Colors::BORDER))
        .title(Span::styled(
            " Sessions Preview ",
            Style::default().fg(Colors::ACCENT),
        ));

    if app.sessions.is_empty() {
        let text = Paragraph::new(Text::styled(
            "No sessions in this workspace",
            Style::default().fg(Colors::TEXT_DIM),
        ))
        .block(block);
        frame.render_widget(text, area);
        return;
    }

    let items: Vec<ListItem> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let title = s.session.title();
            let style = if i == app.session_index {
                Style::default().fg(Colors::SELECTED_FG).bold()
            } else {
                Style::default().fg(Colors::TEXT)
            };

            let content = Line::from(vec![
                Span::styled(
                    format!("{:2}. ", i + 1),
                    Style::default().fg(Colors::TEXT_DIM),
                ),
                Span::styled(truncate_string(&title, 30), style),
                Span::styled(
                    format!(" ({} msgs)", s.message_count),
                    Style::default().fg(Colors::INFO),
                ),
            ]);
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Colors::SELECTED_BG));

    frame.render_widget(list, area);
}

/// Render the sessions view
fn render_sessions_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: Session list with details
    render_session_table(frame, app, chunks[0]);

    // Right: Message preview
    render_message_preview(frame, app, chunks[1]);
}

/// Render session table
fn render_session_table(frame: &mut Frame, app: &App, area: Rect) {
    let ws_name = app
        .current_workspace()
        .and_then(|ws| ws.project_path.as_ref())
        .map(|p| truncate_path(p, 30))
        .unwrap_or_else(|| "(none)".to_string());

    let header_cells = ["#", "Title", "Messages", "Modified"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Colors::ACCENT).bold()));

    let header = Row::new(header_cells)
        .style(Style::default().bg(Colors::HEADER_BG))
        .height(1);

    let rows: Vec<Row> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let is_selected = i == app.session_index;
            let title = s.session.title();

            let row_style = if is_selected {
                Style::default()
                    .bg(Colors::SELECTED_BG)
                    .fg(Colors::SELECTED_FG)
            } else {
                Style::default().fg(Colors::TEXT)
            };

            Row::new(vec![
                Cell::from(format!("{}", i + 1)),
                Cell::from(truncate_string(&title, 35)),
                Cell::from(format!("{}", s.message_count)).style(Style::default().fg(Colors::INFO)),
                Cell::from(s.last_modified.clone()).style(Style::default().fg(Colors::TEXT_DIM)),
            ])
            .style(row_style)
            .height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Min(20),
            Constraint::Length(10),
            Constraint::Length(18),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Colors::BORDER_FOCUSED))
            .title(Span::styled(
                format!(" {} - Sessions ({}) ", ws_name, app.sessions.len()),
                Style::default().fg(Colors::ACCENT),
            )),
    )
    .row_highlight_style(Style::default().bg(Colors::SELECTED_BG))
    .highlight_symbol(">> ");

    let mut state = TableState::default();
    state.select(Some(app.session_index));

    frame.render_stateful_widget(table, area, &mut state);
}

/// Render message preview
fn render_message_preview(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Colors::BORDER))
        .title(Span::styled(
            " Message Preview ",
            Style::default().fg(Colors::ACCENT),
        ));

    let Some(session) = app.current_session() else {
        let text = Paragraph::new(Text::styled(
            "Select a session to preview messages",
            Style::default().fg(Colors::TEXT_DIM),
        ))
        .block(block);
        frame.render_widget(text, area);
        return;
    };

    let requests = &session.session.requests;
    if requests.is_empty() {
        let text = Paragraph::new(Text::styled(
            "No messages in this session",
            Style::default().fg(Colors::TEXT_DIM),
        ))
        .block(block);
        frame.render_widget(text, area);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    for (i, req) in requests.iter().take(10).enumerate() {
        // User message
        if let Some(msg) = &req.message {
            let text = msg.get_text();
            lines.push(Line::from(vec![Span::styled(
                format!("{}. User: ", i + 1),
                Style::default().fg(Colors::SUCCESS).bold(),
            )]));
            lines.push(Line::from(Span::styled(
                truncate_string(&text, 60),
                Style::default().fg(Colors::TEXT),
            )));
            lines.push(Line::raw(""));
        }
    }

    if requests.len() > 10 {
        lines.push(Line::from(Span::styled(
            format!("... and {} more messages", requests.len() - 10),
            Style::default().fg(Colors::TEXT_DIM).italic(),
        )));
    }

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Render session detail view
fn render_session_detail_view(frame: &mut Frame, app: &App, area: Rect) {
    let Some(session) = app.current_session() else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    // Session info header
    let info_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Colors::BORDER_FOCUSED))
        .title(Span::styled(
            " Session Info ",
            Style::default().fg(Colors::ACCENT),
        ));

    let title = session.session.title();
    let info_text = Text::from(vec![
        Line::from(vec![
            Span::styled("Title: ", Style::default().fg(Colors::TEXT_DIM)),
            Span::styled(&title, Style::default().fg(Colors::TEXT).bold()),
        ]),
        Line::from(vec![
            Span::styled("File: ", Style::default().fg(Colors::TEXT_DIM)),
            Span::styled(&session.filename, Style::default().fg(Colors::PURPLE)),
            Span::styled("  |  Modified: ", Style::default().fg(Colors::TEXT_DIM)),
            Span::styled(&session.last_modified, Style::default().fg(Colors::INFO)),
            Span::styled("  |  Messages: ", Style::default().fg(Colors::TEXT_DIM)),
            Span::styled(
                format!("{}", session.message_count),
                Style::default().fg(Colors::SUCCESS),
            ),
        ]),
    ]);

    let info_paragraph = Paragraph::new(info_text).block(info_block);
    frame.render_widget(info_paragraph, chunks[0]);

    // Message list
    let msg_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Colors::BORDER))
        .title(Span::styled(
            " Messages ",
            Style::default().fg(Colors::ACCENT),
        ));

    let mut lines: Vec<Line> = Vec::new();

    for (i, req) in session.session.requests.iter().enumerate() {
        // Timestamp
        if let Some(ts) = req.timestamp {
            let dt = chrono::DateTime::from_timestamp_millis(ts)
                .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            lines.push(Line::from(Span::styled(
                format!("--- {} ---", dt),
                Style::default().fg(Colors::TEXT_DIM),
            )));
        }

        // User message
        if let Some(msg) = &req.message {
            let text = msg.get_text();
            lines.push(Line::from(vec![Span::styled(
                format!("[{}] User: ", i + 1),
                Style::default().fg(Colors::SUCCESS).bold(),
            )]));
            for line in text.lines() {
                lines.push(Line::from(Span::styled(
                    format!("    {}", line),
                    Style::default().fg(Colors::TEXT),
                )));
            }
        }

        lines.push(Line::raw(""));
    }

    let paragraph = Paragraph::new(lines)
        .block(msg_block)
        .scroll((app.detail_scroll as u16, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, chunks[1]);

    // Scrollbar
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("^"))
        .end_symbol(Some("v"));

    let total_lines = session.session.requests.len() * 5; // Approximate
    let mut scrollbar_state = ScrollbarState::new(total_lines).position(app.detail_scroll);

    frame.render_stateful_widget(
        scrollbar,
        chunks[1].inner(ratatui::layout::Margin {
            horizontal: 0,
            vertical: 1,
        }),
        &mut scrollbar_state,
    );
}

/// Render the footer/status bar
fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let mode_hint = match app.mode {
        AppMode::Workspaces => {
            if app.filter_active {
                format!(
                    "Filter: {}_ | [Enter] confirm | [Esc] cancel",
                    app.filter_query
                )
            } else {
                "[j/k] navigate | [Enter] view sessions | [/] filter | [r] refresh | [?] help | [q] quit".to_string()
            }
        }
        AppMode::Sessions => {
            "[j/k] navigate | [Enter] view details | [Esc] back | [?] help | [q] quit".to_string()
        }
        AppMode::SessionDetail => "[j/k] scroll | [Esc] back | [?] help | [q] quit".to_string(),
        AppMode::Help => "Press any key to close help".to_string(),
    };

    let status = app.status_message.as_deref().unwrap_or("");

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(mode_hint, Style::default().fg(Colors::TEXT_DIM)),
        Span::styled("  ", Style::default()),
        Span::styled(status, Style::default().fg(Colors::WARNING)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Colors::BORDER))
            .style(Style::default().bg(Colors::HEADER_BG)),
    );

    frame.render_widget(footer, area);
}

/// Render help overlay
fn render_help_overlay(frame: &mut Frame) {
    let area = centered_rect(60, 70, frame.area());

    frame.render_widget(Clear, area);

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().fg(Colors::ACCENT).bold(),
        )),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default().fg(Colors::SUCCESS).bold(),
        )]),
        Line::from(vec![
            Span::styled("  j / Down    ", Style::default().fg(Colors::PURPLE)),
            Span::styled("Move down", Style::default().fg(Colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  k / Up      ", Style::default().fg(Colors::PURPLE)),
            Span::styled("Move up", Style::default().fg(Colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  g           ", Style::default().fg(Colors::PURPLE)),
            Span::styled("Go to top", Style::default().fg(Colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  G           ", Style::default().fg(Colors::PURPLE)),
            Span::styled("Go to bottom", Style::default().fg(Colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  PgUp/PgDn   ", Style::default().fg(Colors::PURPLE)),
            Span::styled("Page up/down", Style::default().fg(Colors::TEXT)),
        ]),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "Actions",
            Style::default().fg(Colors::SUCCESS).bold(),
        )]),
        Line::from(vec![
            Span::styled("  Enter       ", Style::default().fg(Colors::PURPLE)),
            Span::styled("Select / Enter view", Style::default().fg(Colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  Esc         ", Style::default().fg(Colors::PURPLE)),
            Span::styled("Go back / Cancel", Style::default().fg(Colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  /           ", Style::default().fg(Colors::PURPLE)),
            Span::styled(
                "Start filter (workspaces view)",
                Style::default().fg(Colors::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  r           ", Style::default().fg(Colors::PURPLE)),
            Span::styled("Refresh data", Style::default().fg(Colors::TEXT)),
        ]),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "General",
            Style::default().fg(Colors::SUCCESS).bold(),
        )]),
        Line::from(vec![
            Span::styled("  ?           ", Style::default().fg(Colors::PURPLE)),
            Span::styled("Toggle this help", Style::default().fg(Colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  q           ", Style::default().fg(Colors::PURPLE)),
            Span::styled("Quit application", Style::default().fg(Colors::TEXT)),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Colors::TEXT_DIM).italic(),
        )),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(Span::styled(
                    " Help ",
                    Style::default().fg(Colors::ACCENT).bold(),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Colors::BORDER_FOCUSED))
                .style(Style::default().bg(Color::Rgb(24, 24, 37))),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(help, area);
}

/// Helper to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Truncate a path for display
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }

    // Try to show the end of the path
    let parts: Vec<&str> = path.split(['/', '\\']).collect();
    let mut result = String::new();

    for part in parts.iter().rev() {
        if result.is_empty() {
            result = part.to_string();
        } else if result.len() + part.len() + 4 < max_len {
            result = format!("{}/{}", part, result);
        } else {
            result = format!(".../{}", result);
            break;
        }
    }

    result
}

/// Truncate a string for display
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
