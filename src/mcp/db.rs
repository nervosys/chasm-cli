// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! MCP Database Access - Connect to the CSM database for csm-web integration
//!
//! This module provides read-only access to the CSM database, enabling the MCP server
//! to expose csm-web's chat sessions without modifying VS Code's workspace storage.

#![allow(dead_code, unused_imports)]

use crate::database::{ChatDatabase, Message, Session, Workspace};
use anyhow::Result;
use std::path::PathBuf;

/// Get the default CSM database path
pub fn get_csm_db_path() -> PathBuf {
    dirs::data_local_dir()
        .map(|p| p.join("csm").join("csm.db"))
        .unwrap_or_else(|| PathBuf::from("csm.db"))
}

/// Check if the CSM database exists
pub fn csm_db_exists() -> bool {
    get_csm_db_path().exists()
}

/// Open the CSM database (read-only mode)
pub fn open_csm_db() -> Result<ChatDatabase> {
    let path = get_csm_db_path();
    ChatDatabase::open(&path)
}

/// List all workspaces from the CSM database
pub fn list_db_workspaces() -> Result<Vec<Workspace>> {
    let db = open_csm_db()?;
    db.list_workspaces()
}

/// Get a specific workspace by ID
pub fn get_db_workspace(id: &str) -> Result<Option<Workspace>> {
    let db = open_csm_db()?;
    db.get_workspace(id)
}

/// List sessions from the CSM database
pub fn list_db_sessions(
    workspace_id: Option<&str>,
    provider: Option<&str>,
    limit: usize,
) -> Result<Vec<Session>> {
    let db = open_csm_db()?;
    db.list_sessions(workspace_id, provider, limit)
}

/// Get a specific session by ID
pub fn get_db_session(id: &str) -> Result<Option<Session>> {
    let db = open_csm_db()?;
    db.get_session(id)
}

/// Get messages for a session
pub fn get_db_messages(session_id: &str) -> Result<Vec<Message>> {
    let db = open_csm_db()?;
    db.get_messages(session_id)
}

/// Count sessions by provider
pub fn count_sessions_by_provider() -> Result<Vec<(String, i64)>> {
    let db = open_csm_db()?;
    db.count_sessions_by_provider()
}

/// Search sessions by title or content
pub fn search_db_sessions(query: &str, limit: usize) -> Result<Vec<Session>> {
    let db = open_csm_db()?;

    // Get all sessions and filter by title (simple search)
    // For full-text search, we'd use the harvest database
    let sessions = db.list_sessions(None, None, 1000)?;

    let query_lower = query.to_lowercase();
    let filtered: Vec<Session> = sessions
        .into_iter()
        .filter(|s| s.title.to_lowercase().contains(&query_lower))
        .take(limit)
        .collect();

    Ok(filtered)
}
