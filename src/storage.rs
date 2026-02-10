// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! VS Code storage (SQLite database) operations

use crate::error::{CsmError, Result};
use crate::models::{ChatRequest, ChatSession, ChatSessionIndex, ChatSessionIndexEntry};
use crate::workspace::{get_empty_window_sessions_path, get_workspace_storage_path};
use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use sysinfo::System;

/// Regex to match any Unicode escape sequence (valid or not)
static UNICODE_ESCAPE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\\u[0-9a-fA-F]{4}").unwrap());

/// VS Code session format version - helps identify which parsing strategy to use
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VsCodeSessionFormat {
    /// Legacy JSON format (VS Code < 1.109.0)
    /// Single JSON object with ChatSession structure
    LegacyJson,
    /// JSONL format (VS Code >= 1.109.0, January 2026+)
    /// JSON Lines with event sourcing: kind 0 (initial), kind 1 (delta), kind 2 (requests)
    JsonLines,
}

/// Session schema version - tracks the internal structure version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SessionSchemaVersion {
    /// Version 1 - Original format (basic fields)
    V1 = 1,
    /// Version 2 - Added more metadata fields
    V2 = 2,
    /// Version 3 - Current format with full request/response structure
    V3 = 3,
    /// Unknown version
    Unknown = 0,
}

impl SessionSchemaVersion {
    /// Create from version number
    pub fn from_version(v: u32) -> Self {
        match v {
            1 => Self::V1,
            2 => Self::V2,
            3 => Self::V3,
            _ => Self::Unknown,
        }
    }

    /// Get version number
    pub fn version_number(&self) -> u32 {
        match self {
            Self::V1 => 1,
            Self::V2 => 2,
            Self::V3 => 3,
            Self::Unknown => 0,
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            Self::V1 => "v1 (basic)",
            Self::V2 => "v2 (extended metadata)",
            Self::V3 => "v3 (full structure)",
            Self::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for SessionSchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Result of session format detection
#[derive(Debug, Clone)]
pub struct SessionFormatInfo {
    /// File format (JSON or JSONL)
    pub format: VsCodeSessionFormat,
    /// Schema version detected from content
    pub schema_version: SessionSchemaVersion,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
    /// Detection method used
    pub detection_method: &'static str,
}

impl VsCodeSessionFormat {
    /// Detect format from file path (by extension)
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|e| e.to_str()) {
            Some("jsonl") => Self::JsonLines,
            _ => Self::LegacyJson,
        }
    }

    /// Detect format from content by analyzing structure
    pub fn from_content(content: &str) -> Self {
        let trimmed = content.trim();
        
        // JSONL: Multiple lines starting with { or first line has {"kind":
        if trimmed.starts_with("{\"kind\":") || trimmed.starts_with("{ \"kind\":") {
            return Self::JsonLines;
        }
        
        // Count lines that look like JSON objects
        let mut json_object_lines = 0;
        let mut total_non_empty_lines = 0;
        
        for line in trimmed.lines().take(10) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            total_non_empty_lines += 1;
            
            // Check if line is a JSON object with "kind" field (JSONL marker)
            if line.starts_with('{') && line.contains("\"kind\"") {
                json_object_lines += 1;
            }
        }
        
        // If multiple lines look like JSONL entries, it's JSONL
        if json_object_lines >= 2 || (json_object_lines == 1 && total_non_empty_lines == 1 && trimmed.contains("\n{")) {
            return Self::JsonLines;
        }
        
        // Check if it's a single JSON object (legacy format)
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            // Look for ChatSession structure markers
            if trimmed.contains("\"sessionId\"") || trimmed.contains("\"creationDate\"") || trimmed.contains("\"requests\"") {
                return Self::LegacyJson;
            }
        }
        
        // Default to legacy JSON if unclear
        Self::LegacyJson
    }

    /// Get minimum VS Code version that uses this format
    pub fn min_vscode_version(&self) -> &'static str {
        match self {
            Self::LegacyJson => "1.0.0",
            Self::JsonLines => "1.109.0",
        }
    }

    /// Get human-readable format description
    pub fn description(&self) -> &'static str {
        match self {
            Self::LegacyJson => "Legacy JSON (single object)",
            Self::JsonLines => "JSON Lines (event-sourced, VS Code 1.109.0+)",
        }
    }
    
    /// Get short format name
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::LegacyJson => "json",
            Self::JsonLines => "jsonl",
        }
    }
}

impl std::fmt::Display for VsCodeSessionFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Sanitize JSON content by replacing lone surrogates with replacement character.
/// VS Code sometimes writes invalid JSON with lone Unicode surrogates (e.g., \udde0).
fn sanitize_json_unicode(content: &str) -> String {
    // Process all \uXXXX sequences and fix lone surrogates
    let mut result = String::with_capacity(content.len());
    let mut last_end = 0;

    // Collect all matches first to avoid borrowing issues
    let matches: Vec<_> = UNICODE_ESCAPE_RE.find_iter(content).collect();

    for (i, mat) in matches.iter().enumerate() {
        let start = mat.start();
        let end = mat.end();

        // Add content before this match
        result.push_str(&content[last_end..start]);

        // Parse the hex value from the match itself (always ASCII \uXXXX)
        let hex_str = &mat.as_str()[2..]; // Skip the \u prefix
        if let Ok(code_point) = u16::from_str_radix(hex_str, 16) {
            // Check if it's a high surrogate (D800-DBFF)
            if (0xD800..=0xDBFF).contains(&code_point) {
                // Check if next match is immediately following and is a low surrogate
                let is_valid_pair = if let Some(next_mat) = matches.get(i + 1) {
                    // Must be immediately adjacent (no gap)
                    if next_mat.start() == end {
                        let next_hex = &next_mat.as_str()[2..];
                        if let Ok(next_cp) = u16::from_str_radix(next_hex, 16) {
                            (0xDC00..=0xDFFF).contains(&next_cp)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                if is_valid_pair {
                    // Valid surrogate pair, keep the high surrogate
                    result.push_str(mat.as_str());
                } else {
                    // Lone high surrogate - replace with replacement char
                    result.push_str("\\uFFFD");
                }
            }
            // Check if it's a low surrogate (DC00-DFFF)
            else if (0xDC00..=0xDFFF).contains(&code_point) {
                // Check if previous match was immediately before and was a high surrogate
                let is_valid_pair = if i > 0 {
                    if let Some(prev_mat) = matches.get(i - 1) {
                        // Must be immediately adjacent (no gap)
                        if prev_mat.end() == start {
                            let prev_hex = &prev_mat.as_str()[2..];
                            if let Ok(prev_cp) = u16::from_str_radix(prev_hex, 16) {
                                (0xD800..=0xDBFF).contains(&prev_cp)
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                if is_valid_pair {
                    // Part of valid surrogate pair, keep it
                    result.push_str(mat.as_str());
                } else {
                    // Lone low surrogate - replace with replacement char
                    result.push_str("\\uFFFD");
                }
            }
            // Normal code point
            else {
                result.push_str(mat.as_str());
            }
        } else {
            // Invalid hex - keep as is
            result.push_str(mat.as_str());
        }
        last_end = end;
    }

    // Add remaining content
    result.push_str(&content[last_end..]);
    result
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

/// JSONL entry kinds for VS Code 1.109.0+ session format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JsonlKind {
    /// Initial session state (kind: 0)
    Initial = 0,
    /// Delta update to specific keys (kind: 1)  
    Delta = 1,
    /// Full requests array update (kind: 2)
    RequestsUpdate = 2,
}

/// Parse a JSONL (JSON Lines) session file (VS Code 1.109.0+ format)
/// Each line is a JSON object with 'kind' field indicating the type:
/// - kind 0: Initial session metadata with 'v' containing ChatSession-like structure
/// - kind 1: Delta update with 'k' (keys path) and 'v' (value)
/// - kind 2: Full requests array update with 'k' and 'v'
pub fn parse_session_jsonl(content: &str) -> std::result::Result<ChatSession, serde_json::Error> {
    let mut session = ChatSession {
        version: 3,
        session_id: None,
        creation_date: 0,
        last_message_date: 0,
        is_imported: false,
        initial_location: "panel".to_string(),
        custom_title: None,
        requester_username: None,
        requester_avatar_icon_uri: None,
        responder_username: None,
        responder_avatar_icon_uri: None,
        requests: Vec::new(),
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse each line as a JSON object
        let entry: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                // Try sanitizing Unicode
                let sanitized = sanitize_json_unicode(line);
                serde_json::from_str(&sanitized)?
            }
        };

        let kind = entry.get("kind").and_then(|k| k.as_u64()).unwrap_or(0);

        match kind {
            0 => {
                // Initial state - 'v' contains the session metadata
                if let Some(v) = entry.get("v") {
                    // Parse version
                    if let Some(version) = v.get("version").and_then(|x| x.as_u64()) {
                        session.version = version as u32;
                    }
                    // Parse session ID
                    if let Some(sid) = v.get("sessionId").and_then(|x| x.as_str()) {
                        session.session_id = Some(sid.to_string());
                    }
                    // Parse creation date
                    if let Some(cd) = v.get("creationDate").and_then(|x| x.as_i64()) {
                        session.creation_date = cd;
                    }
                    // Parse initial location
                    if let Some(loc) = v.get("initialLocation").and_then(|x| x.as_str()) {
                        session.initial_location = loc.to_string();
                    }
                    // Parse responder username
                    if let Some(ru) = v.get("responderUsername").and_then(|x| x.as_str()) {
                        session.responder_username = Some(ru.to_string());
                    }
                    // Parse requests array if present
                    if let Some(requests) = v.get("requests") {
                        if let Ok(reqs) =
                            serde_json::from_value::<Vec<ChatRequest>>(requests.clone())
                        {
                            session.requests = reqs;
                        }
                    }
                }
            }
            1 => {
                // Delta update - 'k' is array of key path, 'v' is the value
                if let (Some(keys), Some(value)) = (entry.get("k"), entry.get("v")) {
                    if let Some(keys_arr) = keys.as_array() {
                        // Handle known keys
                        if keys_arr.len() == 1 {
                            if let Some(key) = keys_arr[0].as_str() {
                                match key {
                                    "customTitle" => {
                                        if let Some(title) = value.as_str() {
                                            session.custom_title = Some(title.to_string());
                                        }
                                    }
                                    "lastMessageDate" => {
                                        if let Some(date) = value.as_i64() {
                                            session.last_message_date = date;
                                        }
                                    }
                                    _ => {} // Ignore unknown keys
                                }
                            }
                        }
                    }
                }
            }
            2 => {
                // Full requests array update - 'k' contains ["requests"], 'v' is the array
                if let Some(value) = entry.get("v") {
                    if let Ok(reqs) = serde_json::from_value::<Vec<ChatRequest>>(value.clone()) {
                        session.requests = reqs;
                        // Update last message date from last request
                        if let Some(last_req) = session.requests.last() {
                            if let Some(ts) = last_req.timestamp {
                                session.last_message_date = ts;
                            }
                        }
                    }
                }
            }
            _ => {} // Unknown kind, skip
        }
    }

    Ok(session)
}

/// Check if a file extension indicates a session file (.json or .jsonl)
pub fn is_session_file_extension(ext: &std::ffi::OsStr) -> bool {
    ext == "json" || ext == "jsonl"
}

/// Detect session format and version from content
pub fn detect_session_format(content: &str) -> SessionFormatInfo {
    let format = VsCodeSessionFormat::from_content(content);
    let trimmed = content.trim();
    
    // Detect schema version based on format
    let (schema_version, confidence, method) = match format {
        VsCodeSessionFormat::JsonLines => {
            // For JSONL, check the first line's "v" object for version
            if let Some(first_line) = trimmed.lines().next() {
                if let Ok(entry) = serde_json::from_str::<serde_json::Value>(first_line) {
                    if let Some(v) = entry.get("v") {
                        if let Some(ver) = v.get("version").and_then(|x| x.as_u64()) {
                            (SessionSchemaVersion::from_version(ver as u32), 0.95, "jsonl-version-field")
                        } else {
                            // No version field, likely v3 (current default)
                            (SessionSchemaVersion::V3, 0.7, "jsonl-default")
                        }
                    } else {
                        (SessionSchemaVersion::V3, 0.6, "jsonl-no-v-field")
                    }
                } else {
                    (SessionSchemaVersion::Unknown, 0.3, "jsonl-parse-error")
                }
            } else {
                (SessionSchemaVersion::Unknown, 0.2, "jsonl-empty")
            }
        }
        VsCodeSessionFormat::LegacyJson => {
            // For JSON, directly check the version field
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if let Some(ver) = json.get("version").and_then(|x| x.as_u64()) {
                    (SessionSchemaVersion::from_version(ver as u32), 0.95, "json-version-field")
                } else {
                    // Infer from structure
                    if json.get("requests").is_some() && json.get("sessionId").is_some() {
                        (SessionSchemaVersion::V3, 0.8, "json-structure-inference")
                    } else if json.get("messages").is_some() {
                        (SessionSchemaVersion::V1, 0.7, "json-legacy-structure")
                    } else {
                        (SessionSchemaVersion::Unknown, 0.4, "json-unknown-structure")
                    }
                }
            } else {
                // Try sanitizing and parsing again
                let sanitized = sanitize_json_unicode(trimmed);
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&sanitized) {
                    if let Some(ver) = json.get("version").and_then(|x| x.as_u64()) {
                        (SessionSchemaVersion::from_version(ver as u32), 0.9, "json-version-after-sanitize")
                    } else {
                        (SessionSchemaVersion::V3, 0.6, "json-default-after-sanitize")
                    }
                } else {
                    (SessionSchemaVersion::Unknown, 0.2, "json-parse-error")
                }
            }
        }
    };
    
    SessionFormatInfo {
        format,
        schema_version,
        confidence,
        detection_method: method,
    }
}

/// Parse session content with automatic format detection
pub fn parse_session_auto(content: &str) -> std::result::Result<(ChatSession, SessionFormatInfo), serde_json::Error> {
    let format_info = detect_session_format(content);
    
    let session = match format_info.format {
        VsCodeSessionFormat::JsonLines => parse_session_jsonl(content)?,
        VsCodeSessionFormat::LegacyJson => parse_session_json(content)?,
    };
    
    Ok((session, format_info))
}

/// Parse a session file, automatically detecting format from content (not just extension)
pub fn parse_session_file(path: &Path) -> std::result::Result<ChatSession, serde_json::Error> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string(),
        ))
    })?;

    // Use content-based auto-detection
    let (session, _format_info) = parse_session_auto(&content)?;
    Ok(session)
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
#[allow(dead_code)]
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
            if path
                .extension()
                .map(|e| is_session_file_extension(e))
                .unwrap_or(false)
            {
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

        if path
            .extension()
            .map(|e| is_session_file_extension(e))
            .unwrap_or(false)
        {
            if let Ok(session) = parse_session_file(&path) {
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

        if path
            .extension()
            .map(|e| is_session_file_extension(e))
            .unwrap_or(false)
        {
            if let Ok(session) = parse_session_file(&path) {
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

        if path
            .extension()
            .is_some_and(|e| is_session_file_extension(e))
        {
            if let Ok(session) = parse_session_file(&path) {
                sessions.push(session);
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
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| is_session_file_extension(ext))
        })
        .count();

    Ok(count)
}
