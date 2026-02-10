// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Application state for the TUI

use crate::models::{ChatSession, Workspace};
use crate::workspace::{discover_workspaces, get_chat_sessions_from_workspace};
use std::path::PathBuf;

/// Current view mode in the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Viewing list of workspaces
    Workspaces,
    /// Viewing sessions within a workspace
    Sessions,
    /// Viewing details of a session
    SessionDetail,
    /// Help overlay
    Help,
}

/// Session info for display
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub filename: String,
    #[allow(dead_code)]
    pub path: PathBuf,
    pub session: ChatSession,
    pub last_modified: String,
    pub message_count: usize,
}

/// Application state
pub struct App {
    /// Current mode/view
    pub mode: AppMode,
    /// Previous mode (for returning from help)
    pub previous_mode: AppMode,
    /// All discovered workspaces
    pub workspaces: Vec<Workspace>,
    /// Currently selected workspace index
    pub workspace_index: usize,
    /// Sessions for the currently selected workspace
    pub sessions: Vec<SessionInfo>,
    /// Currently selected session index
    pub session_index: usize,
    /// Scroll offset for session detail view
    pub detail_scroll: usize,
    /// Search/filter query
    pub filter_query: String,
    /// Is filter input active
    pub filter_active: bool,
    /// Filtered workspace indices
    pub filtered_indices: Vec<usize>,
    /// Status message to display
    pub status_message: Option<String>,
}

impl App {
    /// Create a new App instance and load workspaces
    pub fn new() -> anyhow::Result<Self> {
        let workspaces = discover_workspaces()?;
        let filtered_indices: Vec<usize> = (0..workspaces.len()).collect();

        let app = Self {
            mode: AppMode::Workspaces,
            previous_mode: AppMode::Workspaces,
            workspaces,
            workspace_index: 0,
            sessions: Vec::new(),
            session_index: 0,
            detail_scroll: 0,
            filter_query: String::new(),
            filter_active: false,
            filtered_indices,
            status_message: None,
        };

        Ok(app)
    }

    /// Get the currently selected workspace (if any)
    pub fn current_workspace(&self) -> Option<&Workspace> {
        if self.filtered_indices.is_empty() {
            return None;
        }
        let actual_index = self.filtered_indices.get(self.workspace_index)?;
        self.workspaces.get(*actual_index)
    }

    /// Get the currently selected session (if any)
    pub fn current_session(&self) -> Option<&SessionInfo> {
        self.sessions.get(self.session_index)
    }

    /// Load sessions for the currently selected workspace
    pub fn load_sessions_for_current_workspace(&mut self) {
        self.sessions.clear();
        self.session_index = 0;

        if let Some(ws) = self.current_workspace() {
            if let Ok(session_list) = get_chat_sessions_from_workspace(&ws.workspace_path) {
                for swp in session_list {
                    let modified = swp
                        .path
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            let datetime: chrono::DateTime<chrono::Utc> = t.into();
                            datetime.format("%Y-%m-%d %H:%M").to_string()
                        })
                        .unwrap_or_else(|| "unknown".to_string());

                    let msg_count = swp.session.request_count();
                    let filename = swp
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    self.sessions.push(SessionInfo {
                        filename,
                        path: swp.path,
                        session: swp.session,
                        last_modified: modified,
                        message_count: msg_count,
                    });
                }
            }
        }
    }

    /// Apply filter to workspaces
    pub fn apply_filter(&mut self) {
        if self.filter_query.is_empty() {
            self.filtered_indices = (0..self.workspaces.len()).collect();
        } else {
            let query = self.filter_query.to_lowercase();
            self.filtered_indices = self
                .workspaces
                .iter()
                .enumerate()
                .filter(|(_, ws)| {
                    ws.project_path
                        .as_ref()
                        .map(|p| p.to_lowercase().contains(&query))
                        .unwrap_or(false)
                        || ws.hash.to_lowercase().contains(&query)
                })
                .map(|(i, _)| i)
                .collect();
        }

        // Reset selection if out of bounds
        if self.workspace_index >= self.filtered_indices.len() {
            self.workspace_index = 0;
        }

        // Reload sessions for new selection
        self.load_sessions_for_current_workspace();
    }

    /// Navigate up in the current list
    pub fn navigate_up(&mut self) {
        match self.mode {
            AppMode::Workspaces => {
                if self.workspace_index > 0 {
                    self.workspace_index -= 1;
                }
            }
            AppMode::Sessions => {
                if self.session_index > 0 {
                    self.session_index -= 1;
                }
            }
            AppMode::SessionDetail => {
                if self.detail_scroll > 0 {
                    self.detail_scroll -= 1;
                }
            }
            AppMode::Help => {}
        }
    }

    /// Navigate down in the current list
    pub fn navigate_down(&mut self) {
        match self.mode {
            AppMode::Workspaces => {
                if self.workspace_index + 1 < self.filtered_indices.len() {
                    self.workspace_index += 1;
                }
            }
            AppMode::Sessions => {
                if self.session_index + 1 < self.sessions.len() {
                    self.session_index += 1;
                }
            }
            AppMode::SessionDetail => {
                self.detail_scroll += 1;
            }
            AppMode::Help => {}
        }
    }

    /// Page up (jump 10 items)
    pub fn page_up(&mut self) {
        match self.mode {
            AppMode::Workspaces => {
                self.workspace_index = self.workspace_index.saturating_sub(10);
            }
            AppMode::Sessions => {
                self.session_index = self.session_index.saturating_sub(10);
            }
            AppMode::SessionDetail => {
                self.detail_scroll = self.detail_scroll.saturating_sub(10);
            }
            AppMode::Help => {}
        }
    }

    /// Page down (jump 10 items)
    pub fn page_down(&mut self) {
        match self.mode {
            AppMode::Workspaces => {
                let max = self.filtered_indices.len().saturating_sub(1);
                self.workspace_index = (self.workspace_index + 10).min(max);
            }
            AppMode::Sessions => {
                let max = self.sessions.len().saturating_sub(1);
                self.session_index = (self.session_index + 10).min(max);
            }
            AppMode::SessionDetail => {
                self.detail_scroll += 10;
            }
            AppMode::Help => {}
        }
    }

    /// Go to top of list
    pub fn go_to_top(&mut self) {
        match self.mode {
            AppMode::Workspaces => {
                self.workspace_index = 0;
            }
            AppMode::Sessions => {
                self.session_index = 0;
            }
            AppMode::SessionDetail => {
                self.detail_scroll = 0;
            }
            AppMode::Help => {}
        }
    }

    /// Go to bottom of list
    pub fn go_to_bottom(&mut self) {
        match self.mode {
            AppMode::Workspaces => {
                self.workspace_index = self.filtered_indices.len().saturating_sub(1);
            }
            AppMode::Sessions => {
                self.session_index = self.sessions.len().saturating_sub(1);
            }
            AppMode::SessionDetail => {
                // Will be clamped by scroll logic
                self.detail_scroll = usize::MAX;
            }
            AppMode::Help => {}
        }
    }

    /// Enter/select current item
    pub fn enter(&mut self) {
        match self.mode {
            AppMode::Workspaces => {
                // Load sessions only when entering a workspace
                self.load_sessions_for_current_workspace();
                if !self.sessions.is_empty() {
                    self.mode = AppMode::Sessions;
                    self.session_index = 0;
                } else {
                    self.status_message = Some("No sessions in this workspace".to_string());
                }
            }
            AppMode::Sessions => {
                if self.current_session().is_some() {
                    self.mode = AppMode::SessionDetail;
                    self.detail_scroll = 0;
                }
            }
            AppMode::SessionDetail | AppMode::Help => {}
        }
    }

    /// Go back to previous view
    pub fn back(&mut self) {
        match self.mode {
            AppMode::Workspaces => {
                // Already at top level
            }
            AppMode::Sessions => {
                self.mode = AppMode::Workspaces;
            }
            AppMode::SessionDetail => {
                self.mode = AppMode::Sessions;
            }
            AppMode::Help => {
                self.mode = self.previous_mode;
            }
        }
    }

    /// Toggle help overlay
    pub fn toggle_help(&mut self) {
        if self.mode == AppMode::Help {
            self.mode = self.previous_mode;
        } else {
            self.previous_mode = self.mode;
            self.mode = AppMode::Help;
        }
    }

    /// Start filter input
    pub fn start_filter(&mut self) {
        self.filter_active = true;
        self.filter_query.clear();
    }

    /// Handle filter character input
    pub fn filter_input(&mut self, c: char) {
        if self.filter_active {
            self.filter_query.push(c);
            self.apply_filter();
        }
    }

    /// Handle backspace in filter
    pub fn filter_backspace(&mut self) {
        if self.filter_active {
            self.filter_query.pop();
            self.apply_filter();
        }
    }

    /// Confirm filter
    pub fn confirm_filter(&mut self) {
        self.filter_active = false;
    }

    /// Cancel filter
    pub fn cancel_filter(&mut self) {
        self.filter_active = false;
        self.filter_query.clear();
        self.apply_filter();
    }

    /// Refresh data
    pub fn refresh(&mut self) {
        if let Ok(workspaces) = discover_workspaces() {
            self.workspaces = workspaces;
            self.apply_filter();
            self.status_message = Some("Refreshed workspace data".to_string());
        }
    }

    /// Get count of workspaces with chats
    pub fn workspaces_with_chats(&self) -> usize {
        self.workspaces
            .iter()
            .filter(|w| w.has_chat_sessions)
            .count()
    }

    /// Get total session count
    pub fn total_sessions(&self) -> usize {
        self.workspaces.iter().map(|w| w.chat_session_count).sum()
    }
}
