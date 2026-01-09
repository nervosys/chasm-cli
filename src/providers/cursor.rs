// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Cursor IDE chat provider

use super::{ChatProvider, ProviderType};
use crate::models::ChatSession;
use crate::storage::parse_session_json;
use anyhow::Result;
use std::path::PathBuf;

/// Cursor IDE chat provider
///
/// Cursor stores chat sessions in a similar format to VS Code Copilot,
/// located in the Cursor app data directory.
pub struct CursorProvider {
    /// Path to Cursor's workspace storage
    storage_path: PathBuf,
    /// Whether Cursor is installed and accessible
    available: bool,
}

impl CursorProvider {
    /// Discover Cursor installation and create provider
    pub fn discover() -> Option<Self> {
        let storage_path = Self::find_cursor_storage()?;

        Some(Self {
            available: storage_path.exists(),
            storage_path,
        })
    }

    /// Find Cursor's workspace storage directory
    fn find_cursor_storage() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            let appdata = dirs::data_dir()?;
            let cursor_path = appdata.join("Cursor").join("User").join("workspaceStorage");
            if cursor_path.exists() {
                return Some(cursor_path);
            }
            // Also check Roaming
            let roaming = std::env::var("APPDATA").ok()?;
            let roaming_path = PathBuf::from(roaming)
                .join("Cursor")
                .join("User")
                .join("workspaceStorage");
            if roaming_path.exists() {
                return Some(roaming_path);
            }
        }

        #[cfg(target_os = "macos")]
        {
            let home = dirs::home_dir()?;
            let cursor_path = home
                .join("Library")
                .join("Application Support")
                .join("Cursor")
                .join("User")
                .join("workspaceStorage");
            if cursor_path.exists() {
                return Some(cursor_path);
            }
        }

        #[cfg(target_os = "linux")]
        {
            let config = dirs::config_dir()?;
            let cursor_path = config.join("Cursor").join("User").join("workspaceStorage");
            if cursor_path.exists() {
                return Some(cursor_path);
            }
        }

        None
    }

    /// List all workspace directories with chat sessions
    fn list_workspaces(&self) -> Result<Vec<PathBuf>> {
        let mut workspaces = Vec::new();

        if self.storage_path.exists() {
            for entry in std::fs::read_dir(&self.storage_path)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    // Check for chat sessions directory
                    let chat_path = path.join("chatSessions");
                    if chat_path.exists() {
                        workspaces.push(path);
                    }
                }
            }
        }

        Ok(workspaces)
    }
}

impl ChatProvider for CursorProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Cursor
    }

    fn name(&self) -> &str {
        "Cursor"
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn sessions_path(&self) -> Option<PathBuf> {
        Some(self.storage_path.clone())
    }

    fn list_sessions(&self) -> Result<Vec<ChatSession>> {
        let mut sessions = Vec::new();

        for workspace in self.list_workspaces()? {
            let chat_path = workspace.join("chatSessions");

            if chat_path.exists() {
                for entry in std::fs::read_dir(&chat_path)? {
                    let entry = entry?;
                    let path = entry.path();

                    if path.extension().is_some_and(|e| e == "json") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(session) = parse_session_json(&content) {
                                sessions.push(session);
                            }
                        }
                    }
                }
            }
        }

        Ok(sessions)
    }

    fn import_session(&self, session_id: &str) -> Result<ChatSession> {
        // Search for the session file across all workspaces
        for workspace in self.list_workspaces()? {
            let session_path = workspace
                .join("chatSessions")
                .join(format!("{}.json", session_id));

            if session_path.exists() {
                let content = std::fs::read_to_string(&session_path)?;
                let session: ChatSession = serde_json::from_str(&content)?;
                return Ok(session);
            }
        }

        anyhow::bail!("Session not found: {}", session_id)
    }

    fn export_session(&self, _session: &ChatSession) -> Result<()> {
        // Cursor uses similar format to VS Code, so export is straightforward
        // However, we need a target workspace
        anyhow::bail!("Export to Cursor not yet implemented - use import instead")
    }
}
