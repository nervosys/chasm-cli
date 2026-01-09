// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! VS Code storage (SQLite database) operations

use crate::error::{CsmError, Result};
use crate::models::{ChatSession, ChatSessionIndex, ChatSessionIndexEntry};
use crate::workspace::{get_empty_window_sessions_path, get_workspace_storage_path};
use regex::Regex;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use sysinfo::System;

/// Sanitize JSON content by replacing lone surrogates with replacement character.
/// VS Code sometimes writes invalid JSON with lone Unicode surrogates (e.g., \udde0).
fn sanitize_json_unicode(content: &str) -> String {
    // Match lone high surrogates (D800-DBFF) not followed by low surrogate (DC00-DFFF)
    // and lone low surrogates (DC00-DFFF) not preceded by high surrogate
    let re = Regex::new(r"\\u[dD][89aAbB][0-9a-fA-F]{2}(?!\\u[dD][cCdDeEfF][0-9a-fA-F]{2})|(?<!\\u[dD][89aAbB][0-9a-fA-F]{2})\\u[dD][cCdDeEfF][0-9a-fA-F]{2}")
        .unwrap();
    re.replace_all(content, "\\uFFFD").to_string()
}

/// Try to parse JSON, sanitizing invalid Unicode if needed
pub fn parse_session_json(content: &str) -> std::result::Result<ChatSession, serde_json::Error> {
    match serde_json::from_str::<ChatSession>(content) {
        Ok(session) => Ok(session),
        Err(e) => {
            // If parsing fails due to Unicode issue, try sanitizing
            if e.to_string().contains("surrogate") || e.to_string().contains("escape") {
                let sanitized = sanitize_json_unicode(content);
                serde_json::from_str::<ChatSession>(&sanitized)
            } else {
                Err(e)
            }
        }
    }
}

/// Get the path to the workspace storage database
pub fn get_workspace_storage_db(workspace_id: &str) -> Result<PathBuf> {
    let storage_path = get_workspace_storage_path()?;
    Ok(storage_path.join(workspace_id).join("state.vscdb"))
}

/// Read the chat session index from VS Code storage
pub fn read_chat_session_index(db_path: &Path) -> Result<ChatSessionIndex> {
    let conn = Connection::open(db_path)?;

    let result: std::result::Result<String, rusqlite::Error> = conn.query_row(
        "SELECT value FROM ItemTable WHERE key = ?",
        ["chat.ChatSessionStore.index"],
        |row| row.get(0),
    );

    match result {
        Ok(json_str) => serde_json::from_str(&json_str)
            .map_err(|e| CsmError::InvalidSessionFormat(e.to_string())),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(ChatSessionIndex::default()),
        Err(e) => Err(CsmError::SqliteError(e)),
    }
}

/// Write the chat session index to VS Code storage
pub fn write_chat_session_index(db_path: &Path, index: &ChatSessionIndex) -> Result<()> {
    let conn = Connection::open(db_path)?;
    let json_str = serde_json::to_string(index)?;

    // Check if the key exists
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM ItemTable WHERE key = ?",
        ["chat.ChatSessionStore.index"],
        |row| row.get(0),
    )?;

    if exists {
        conn.execute(
            "UPDATE ItemTable SET value = ? WHERE key = ?",
            [&json_str, "chat.ChatSessionStore.index"],
        )?;
    } else {
        conn.execute(
            "INSERT INTO ItemTable (key, value) VALUES (?, ?)",
            ["chat.ChatSessionStore.index", &json_str],
        )?;
    }

    Ok(())
}

/// Add a session to the VS Code index
pub fn add_session_to_index(
    db_path: &Path,
    session_id: &str,
    title: &str,
    last_message_date_ms: i64,
    is_imported: bool,
    initial_location: &str,
    is_empty: bool,
) -> Result<()> {
    let mut index = read_chat_session_index(db_path)?;

    index.entries.insert(
        session_id.to_string(),
        ChatSessionIndexEntry {
            session_id: session_id.to_string(),
            title: title.to_string(),
            last_message_date: last_message_date_ms,
            is_imported,
            initial_location: initial_location.to_string(),
            is_empty,
        },
    );

    write_chat_session_index(db_path, &index)
}

/// Remove a session from the VS Code index
pub fn remove_session_from_index(db_path: &Path, session_id: &str) -> Result<bool> {
    let mut index = read_chat_session_index(db_path)?;
    let removed = index.entries.remove(session_id).is_some();
    if removed {
        write_chat_session_index(db_path, &index)?;
    }
    Ok(removed)
}

/// Sync the VS Code index with sessions on disk (remove stale entries, add missing ones)
pub fn sync_session_index(
    workspace_id: &str,
    chat_sessions_dir: &Path,
    force: bool,
) -> Result<(usize, usize)> {
    let db_path = get_workspace_storage_db(workspace_id)?;

    if !db_path.exists() {
        return Err(CsmError::WorkspaceNotFound(format!(
            "Database not found: {}",
            db_path.display()
        )));
    }

    // Check if VS Code is running
    if !force && is_vscode_running() {
        return Err(CsmError::VSCodeRunning);
    }

    // Get current index
    let mut index = read_chat_session_index(&db_path)?;

    // Get session files on disk
    let mut files_on_disk: std::collections::HashSet<String> = std::collections::HashSet::new();
    if chat_sessions_dir.exists() {
        for entry in std::fs::read_dir(chat_sessions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Some(stem) = path.file_stem() {
                    files_on_disk.insert(stem.to_string_lossy().to_string());
                }
            }
        }
    }

    // Remove stale entries (in index but not on disk)
    let stale_ids: Vec<String> = index
        .entries
        .keys()
        .filter(|id| !files_on_disk.contains(*id))
        .cloned()
        .collect();

    let removed = stale_ids.len();
    for id in &stale_ids {
        index.entries.remove(id);
    }

    // Add/update sessions from disk
    let mut added = 0;
    for entry in std::fs::read_dir(chat_sessions_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(session) = parse_session_json(&content) {
                    let session_id = session.session_id.clone().unwrap_or_else(|| {
                        path.file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
                    });

                    let title = session.title();
                    let is_empty = session.is_empty();
                    let last_message_date = session.last_message_date;
                    let initial_location = session.initial_location.clone();

                    index.entries.insert(
                        session_id.clone(),
                        ChatSessionIndexEntry {
                            session_id,
                            title,
                            last_message_date,
                            is_imported: session.is_imported,
                            initial_location,
                            is_empty,
                        },
                    );
                    added += 1;
                }
            }
        }
    }

    // Write the synced index
    write_chat_session_index(&db_path, &index)?;

    Ok((added, removed))
}

/// Register all sessions from a directory into the VS Code index
pub fn register_all_sessions_from_directory(
    workspace_id: &str,
    chat_sessions_dir: &Path,
    force: bool,
) -> Result<usize> {
    let db_path = get_workspace_storage_db(workspace_id)?;

    if !db_path.exists() {
        return Err(CsmError::WorkspaceNotFound(format!(
            "Database not found: {}",
            db_path.display()
        )));
    }

    // Check if VS Code is running
    if !force && is_vscode_running() {
        return Err(CsmError::VSCodeRunning);
    }

    // Use sync to ensure index matches disk
    let (added, removed) = sync_session_index(workspace_id, chat_sessions_dir, force)?;

    // Print individual session info
    for entry in std::fs::read_dir(chat_sessions_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(session) = parse_session_json(&content) {
                    let session_id = session.session_id.clone().unwrap_or_else(|| {
                        path.file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
                    });

                    let title = session.title();

                    println!(
                        "[OK] Registered: {} ({}...)",
                        title,
                        &session_id[..12.min(session_id.len())]
                    );
                }
            }
        }
    }

    if removed > 0 {
        println!("[OK] Removed {} stale index entries", removed);
    }

    Ok(added)
}

/// Check if VS Code is currently running
pub fn is_vscode_running() -> bool {
    let mut sys = System::new();
    sys.refresh_processes();

    for process in sys.processes().values() {
        let name = process.name().to_lowercase();
        if name.contains("code") && !name.contains("codec") {
            return true;
        }
    }

    false
}

/// Backup workspace sessions to a timestamped directory
pub fn backup_workspace_sessions(workspace_dir: &Path) -> Result<Option<PathBuf>> {
    let chat_sessions_dir = workspace_dir.join("chatSessions");

    if !chat_sessions_dir.exists() {
        return Ok(None);
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let backup_dir = workspace_dir.join(format!("chatSessions-backup-{}", timestamp));

    // Copy directory recursively
    copy_dir_all(&chat_sessions_dir, &backup_dir)?;

    Ok(Some(backup_dir))
}

/// Recursively copy a directory
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

// =============================================================================
// Empty Window Sessions (ALL SESSIONS)
// =============================================================================

/// Read all empty window chat sessions (not tied to any workspace)
/// These appear in VS Code's "ALL SESSIONS" panel
pub fn read_empty_window_sessions() -> Result<Vec<ChatSession>> {
    let sessions_path = get_empty_window_sessions_path()?;

    if !sessions_path.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    for entry in std::fs::read_dir(&sessions_path)? {
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

    // Sort by last message date (most recent first)
    sessions.sort_by(|a, b| b.last_message_date.cmp(&a.last_message_date));

    Ok(sessions)
}

/// Get a specific empty window session by ID
#[allow(dead_code)]
pub fn get_empty_window_session(session_id: &str) -> Result<Option<ChatSession>> {
    let sessions_path = get_empty_window_sessions_path()?;
    let session_path = sessions_path.join(format!("{}.json", session_id));

    if !session_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&session_path)?;
    let session: ChatSession = serde_json::from_str(&content)
        .map_err(|e| CsmError::InvalidSessionFormat(e.to_string()))?;

    Ok(Some(session))
}

/// Write an empty window session
#[allow(dead_code)]
pub fn write_empty_window_session(session: &ChatSession) -> Result<PathBuf> {
    let sessions_path = get_empty_window_sessions_path()?;

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&sessions_path)?;

    let session_id = session.session_id.as_deref().unwrap_or("unknown");
    let session_path = sessions_path.join(format!("{}.json", session_id));
    let content = serde_json::to_string_pretty(session)?;
    std::fs::write(&session_path, content)?;

    Ok(session_path)
}

/// Delete an empty window session
#[allow(dead_code)]
pub fn delete_empty_window_session(session_id: &str) -> Result<bool> {
    let sessions_path = get_empty_window_sessions_path()?;
    let session_path = sessions_path.join(format!("{}.json", session_id));

    if session_path.exists() {
        std::fs::remove_file(&session_path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Count empty window sessions
pub fn count_empty_window_sessions() -> Result<usize> {
    let sessions_path = get_empty_window_sessions_path()?;

    if !sessions_path.exists() {
        return Ok(0);
    }

    let count = std::fs::read_dir(&sessions_path)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .count();

    Ok(count)
}
