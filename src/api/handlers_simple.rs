// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Simplified API request and response handlers
//!
//! This implementation works with the harvest database schema.

#![allow(dead_code, unused_variables)]

use actix_web::{web, HttpResponse, Responder};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::state::AppState;

/// Check if a string is an empty code block marker (just ``` with no content)
fn is_empty_code_block(s: &str) -> bool {
    // Match patterns like "```", "```\n", "```language", "```\n```", "```\n\n```"
    let s = s.trim();
    if s == "```" {
        return true;
    }
    // Check for code block with just a language identifier and no content
    if s.starts_with("```") && !s.contains('\n') {
        return true;
    }
    // Check for empty code block with opening and closing (possibly with whitespace-only lines)
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() >= 2 && lines[0].starts_with("```") && lines.last() == Some(&"```") {
        // Check if all lines between opening and closing are empty or whitespace
        let content_lines = &lines[1..lines.len() - 1];
        if content_lines.iter().all(|line| line.trim().is_empty()) {
            return true;
        }
    }
    false
}

// =============================================================================
// Response Types
// =============================================================================

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn success(data: T) -> HttpResponse {
        HttpResponse::Ok().json(Self {
            success: true,
            data: Some(data),
            error: None,
        })
    }

    fn error(message: &str) -> HttpResponse {
        HttpResponse::InternalServerError().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(message.to_string()),
        })
    }
}

// =============================================================================
// Query Parameters
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct SessionQuery {
    pub workspace_id: Option<String>,
    pub provider: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<usize>,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Derive a human-readable workspace name from a workspace_id (path)
fn derive_workspace_name(workspace_id: &str) -> String {
    // workspace_id is typically a path like "c:\Users\<username>\dev\project"
    // Extract the last path component as the name
    workspace_id
        .replace('\\', "/")
        .split('/')
        .rfind(|s| !s.is_empty())
        .unwrap_or(workspace_id)
        .to_string()
}

/// Look up workspace path from VS Code workspace storage
fn lookup_workspace_path(workspace_hash: &str) -> Option<String> {
    // VS Code stores workspace info in %APPDATA%/Code/User/workspaceStorage/<hash>/workspace.json
    let workspace_storage = dirs::config_dir()?
        .join("Code/User/workspaceStorage")
        .join(workspace_hash)
        .join("workspace.json");

    if workspace_storage.exists() {
        if let Ok(content) = std::fs::read_to_string(&workspace_storage) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                // Extract folder path from workspace.json
                if let Some(folder) = json.get("folder").and_then(|f| f.as_str()) {
                    // Decode file:// URL
                    let path = folder
                        .strip_prefix("file:///")
                        .unwrap_or(folder)
                        .replace("%3A", ":")
                        .replace("%20", " ");
                    return Some(path);
                }
            }
        }
    }
    None
}

/// Get workspace info (name and path) from hash
fn get_workspace_info(workspace_hash: &str) -> (String, String) {
    if let Some(path) = lookup_workspace_path(workspace_hash) {
        let name = derive_workspace_name(&path);
        (name, path)
    } else {
        // Fallback: use hash as both name and path
        (workspace_hash.to_string(), String::new())
    }
}

// =============================================================================
// Health Check
// =============================================================================

pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// =============================================================================
// Workspace Handlers (using harvest schema)
// =============================================================================

pub async fn list_workspaces(state: web::Data<AppState>) -> impl Responder {
    let db = state.db.lock().unwrap();

    // First try to get workspaces from the workspaces table
    let result: Result<Vec<serde_json::Value>, _> = (|| {
        let mut stmt = db.conn.prepare(
            "SELECT w.id, w.name, w.path, w.provider, COUNT(s.id) as session_count,
                    w.created_at, COALESCE(MAX(s.updated_at), w.updated_at) as updated_at
             FROM workspaces w
             LEFT JOIN sessions s ON w.id = s.workspace_id
             GROUP BY w.id
             ORDER BY updated_at DESC",
        )?;

        let workspaces: Vec<serde_json::Value> = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let path: Option<String> = row.get(2)?;
                let provider: String = row.get(3)?;
                let count: i64 = row.get(4)?;
                let created_at: Option<i64> = row.get(5).ok();
                let updated_at: Option<i64> = row.get(6).ok();
                Ok(serde_json::json!({
                    "id": id,
                    "name": name,
                    "path": path.unwrap_or_default(),
                    "provider": provider,
                    "sessionCount": count,
                    "createdAt": created_at.unwrap_or(0),
                    "updatedAt": updated_at.unwrap_or(0),
                }))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // If workspaces table is empty, derive workspaces from sessions
        if workspaces.is_empty() {
            let mut stmt = db.conn.prepare(
                "SELECT workspace_id, provider, COUNT(*) as session_count,
                        MIN(created_at) as created_at, MAX(updated_at) as updated_at
                 FROM sessions
                 WHERE workspace_id IS NOT NULL AND workspace_id != ''
                 GROUP BY workspace_id
                 ORDER BY MAX(updated_at) DESC",
            )?;

            let derived: Vec<serde_json::Value> = stmt
                .query_map([], |row| {
                    let id: String = row.get(0)?;
                    let provider: String = row.get(1)?;
                    let count: i64 = row.get(2)?;
                    let created_at: Option<i64> = row.get(3)?;
                    let updated_at: Option<i64> = row.get(4)?;
                    let (name, path) = get_workspace_info(&id);
                    Ok(serde_json::json!({
                        "id": id,
                        "name": name,
                        "path": path,
                        "provider": provider,
                        "sessionCount": count,
                        "createdAt": created_at.unwrap_or(0),
                        "updatedAt": updated_at.unwrap_or(0),
                    }))
                })?
                .collect::<Result<Vec<_>, _>>()?;

            return Ok(derived);
        }

        Ok::<_, rusqlite::Error>(workspaces)
    })();

    match result {
        Ok(workspaces) => {
            let total = workspaces.len();
            ApiResponse::success(serde_json::json!({
                "items": workspaces,
                "total": total,
                "limit": total,
                "offset": 0,
                "hasMore": false
            }))
        }
        Err(e) => ApiResponse::<()>::error(&e.to_string()),
    }
}

pub async fn get_workspace(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = state.db.lock().unwrap();
    let workspace_id = path.into_inner();

    let result: Result<Option<serde_json::Value>, _> = (|| {
        let mut stmt = db.conn.prepare(
            "SELECT w.id, w.name, w.path, w.provider, COUNT(s.id) as session_count
             FROM workspaces w
             LEFT JOIN sessions s ON w.id = s.workspace_id
             WHERE w.id = ?1
             GROUP BY w.id",
        )?;

        let workspace = stmt
            .query_row([&workspace_id], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let path: Option<String> = row.get(2)?;
                let provider: String = row.get(3)?;
                let count: i64 = row.get(4)?;
                Ok(serde_json::json!({
                    "id": id,
                    "name": name,
                    "path": path.unwrap_or_default(),
                    "provider": provider,
                    "session_count": count,
                }))
            })
            .optional()?;

        Ok::<_, rusqlite::Error>(workspace)
    })();

    match result {
        Ok(Some(workspace)) => ApiResponse::success(workspace),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Workspace not found"
        })),
        Err(e) => ApiResponse::<()>::error(&e.to_string()),
    }
}

// =============================================================================
// Session Handlers (using harvest schema)
// =============================================================================

pub async fn list_sessions(
    state: web::Data<AppState>,
    query: web::Query<SessionQuery>,
) -> impl Responder {
    let db = state.db.lock().unwrap();
    let limit = query.limit.unwrap_or(100) as i64;

    let result: Result<Vec<serde_json::Value>, _> = (|| {
        let mut sql = String::from(
            "SELECT id, provider, workspace_id, title, message_count, 
                    created_at, updated_at
             FROM sessions WHERE 1=1",
        );

        if query.workspace_id.is_some() {
            sql.push_str(" AND workspace_id = ?1");
        }
        if query.provider.is_some() {
            sql.push_str(" AND provider = ?2");
        }
        sql.push_str(" ORDER BY updated_at DESC LIMIT ?3");

        let mut stmt = db.conn.prepare(&sql)?;

        let sessions: Vec<serde_json::Value> = stmt
            .query_map(
                params![
                    query.workspace_id.as_deref().unwrap_or(""),
                    query.provider.as_deref().unwrap_or(""),
                    limit,
                ],
                |row| {
                    let workspace_id: Option<String> = row.get(2)?;
                    let workspace_name = workspace_id.as_ref().map(|id| {
                        let (name, _path) = get_workspace_info(id);
                        name
                    });
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "provider": row.get::<_, String>(1)?,
                        "workspaceId": workspace_id,
                        "workspaceName": workspace_name,
                        "title": row.get::<_, String>(3)?,
                        "messageCount": row.get::<_, i64>(4)?,
                        "createdAt": row.get::<_, i64>(5)?,
                        "updatedAt": row.get::<_, i64>(6)?,
                    }))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Ok::<_, rusqlite::Error>(sessions)
    })();

    match result {
        Ok(sessions) => {
            let total = sessions.len();
            ApiResponse::success(serde_json::json!({
                "items": sessions,
                "total": total,
                "limit": limit,
                "offset": 0,
                "hasMore": false
            }))
        }
        Err(e) => ApiResponse::<()>::error(&e.to_string()),
    }
}

pub async fn get_session(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = state.db.lock().unwrap();
    let session_id = path.into_inner();

    let result: Result<Option<serde_json::Value>, _> = (|| {
        // Get session info
        let mut stmt = db.conn.prepare(
            "SELECT id, provider, workspace_id, title, message_count,
                    created_at, updated_at, session_json
             FROM sessions WHERE id = ?1",
        )?;

        let session = stmt
            .query_row([&session_id], |row| {
                let session_json: String = row.get(7)?;
                let parsed: serde_json::Value =
                    serde_json::from_str(&session_json).unwrap_or(serde_json::json!({}));

                // Extract messages from session_json.requests
                let messages = extract_messages_from_session(&parsed);

                let workspace_id: Option<String> = row.get(2)?;
                let workspace_name = workspace_id.as_ref().map(|id| {
                    let (name, _path) = get_workspace_info(id);
                    name
                });

                Ok((
                    serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "provider": row.get::<_, String>(1)?,
                        "workspaceId": workspace_id,
                        "workspaceName": workspace_name,
                        "title": row.get::<_, String>(3)?,
                        "messageCount": row.get::<_, i64>(4)?,
                        "createdAt": row.get::<_, i64>(5)?,
                        "updatedAt": row.get::<_, i64>(6)?,
                    }),
                    messages,
                    session_id.clone(),
                ))
            })
            .optional()?;

        if let Some((session, messages, sid)) = session {
            // Try to get enhanced data from messages_v2 and tool_invocations
            let tool_invocations = get_tool_invocations(&db.conn, &sid)?;
            let file_changes = get_file_changes(&db.conn, &sid)?;

            Ok::<_, rusqlite::Error>(Some(serde_json::json!({
                "session": session,
                "messages": messages,
                "tool_invocations": tool_invocations,
                "file_changes": file_changes,
            })))
        } else {
            Ok::<_, rusqlite::Error>(None)
        }
    })();

    match result {
        Ok(Some(data)) => ApiResponse::success(data),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Session not found"
        })),
        Err(e) => ApiResponse::<()>::error(&e.to_string()),
    }
}

/// Extract messages from session_json.requests array with full markdown and tool invocations
fn extract_messages_from_session(session_json: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut messages = Vec::new();

    if let Some(requests) = session_json.get("requests").and_then(|r| r.as_array()) {
        for (idx, request) in requests.iter().enumerate() {
            let timestamp = request.get("timestamp").and_then(|t| t.as_i64());
            let request_id = request.get("requestId").and_then(|r| r.as_str());
            let response_id = request.get("responseId").and_then(|r| r.as_str());
            let model_id = request.get("modelId").and_then(|m| m.as_str());
            let is_canceled = request
                .get("isCanceled")
                .and_then(|c| c.as_bool())
                .unwrap_or(false);

            // Extract user message
            if let Some(message) = request.get("message") {
                let text = message
                    .get("text")
                    .or_else(|| message.get("content"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("");

                if !text.is_empty() {
                    messages.push(serde_json::json!({
                        "index": idx * 2,
                        "role": "user",
                        "content": text,
                        "content_raw": text,
                        "request_id": request_id,
                        "model_id": model_id,
                        "created_at": timestamp,
                        "variable_data": request.get("variableData"),
                    }));
                }
            }

            // Extract assistant response with full markdown and tool invocations
            if let Some(response) = request.get("response") {
                let (response_text, tool_invocations) = extract_response_with_tools(response);

                if !response_text.is_empty() || !tool_invocations.is_empty() {
                    messages.push(serde_json::json!({
                        "index": idx * 2 + 1,
                        "role": "assistant",
                        "content": response_text,
                        "content_raw": response_text,
                        "response_id": response_id,
                        "model_id": model_id,
                        "created_at": timestamp,
                        "is_canceled": is_canceled,
                        "tool_invocations": tool_invocations,
                        "content_references": request.get("contentReferences"),
                        "code_citations": request.get("codeCitations"),
                    }));
                }
            }
        }
    }

    messages
}

/// Extract text content and tool invocations from a response object
fn extract_response_with_tools(response: &serde_json::Value) -> (String, Vec<serde_json::Value>) {
    let mut text_parts = Vec::new();
    let mut tool_invocations = Vec::new();

    // Response can be an array of items
    if let Some(items) = response.as_array() {
        for item in items {
            // Check item kind
            let kind = item.get("kind").and_then(|k| k.as_str()).unwrap_or("");

            match kind {
                "toolInvocationSerialized" => {
                    // Extract tool invocation details
                    let tool_name = item.get("toolId").and_then(|t| t.as_str()).unwrap_or("");
                    let tool_call_id = item.get("toolCallId").and_then(|t| t.as_str());
                    let is_complete = item
                        .get("isComplete")
                        .and_then(|c| c.as_bool())
                        .unwrap_or(false);
                    let is_confirmed = item.get("isConfirmed");

                    // Extract tool-specific data (contains file edits, terminal commands, etc.)
                    let tool_data = item.get("toolSpecificData");

                    // Extract presentation data which may contain file paths for edits
                    let presentation = item.get("presentation");

                    // Extract source which may have the tool input
                    let source = item.get("source");

                    // Extract file changes from tool data, presentation, and source
                    let file_changes =
                        extract_file_changes_from_tool(tool_data, presentation, source, tool_name);

                    tool_invocations.push(serde_json::json!({
                        "tool_name": tool_name,
                        "tool_call_id": tool_call_id,
                        "is_complete": is_complete,
                        "is_confirmed": is_confirmed,
                        "invocation_message": item.get("invocationMessage"),
                        "tool_specific_data": tool_data,
                        "presentation": presentation,
                        "source": source,
                        "file_changes": file_changes,
                    }));
                }
                "prepareToolInvocation" => {
                    // Tool about to be invoked
                    let tool_name = item.get("toolName").and_then(|t| t.as_str()).unwrap_or("");
                    tool_invocations.push(serde_json::json!({
                        "tool_name": tool_name,
                        "status": "preparing",
                    }));
                }
                "thinking" => {
                    // Skip thinking blocks or include encrypted content reference
                    continue;
                }
                "inlineReference" => {
                    // Inline code reference (method names, file paths, symbols)
                    // These are stored as separate objects with a "name" field
                    if let Some(inline_ref) = item.get("inlineReference") {
                        if let Some(name) = inline_ref.get("name").and_then(|n| n.as_str()) {
                            // Wrap the name in backticks to represent inline code
                            text_parts.push(format!("`{}`", name));
                        }
                    }
                }
                _ => {
                    // Check if this item contains an inlineReference (VS Code stores them without a kind)
                    if let Some(inline_ref) = item.get("inlineReference") {
                        if let Some(name) = inline_ref.get("name").and_then(|n| n.as_str()) {
                            // Wrap the name in backticks to represent inline code
                            text_parts.push(format!("`{}`", name));
                        }
                    } else if let Some(value) = item.get("value").and_then(|v| v.as_str()) {
                        // Text/markdown content - filter out empty code block markers
                        let trimmed = value.trim();
                        if !trimmed.is_empty() && !is_empty_code_block(trimmed) {
                            text_parts.push(value.to_string());
                        }
                    }
                }
            }
        }
    }

    (text_parts.join(""), tool_invocations)
}

/// Extract file changes from tool-specific data, presentation, and source
fn extract_file_changes_from_tool(
    tool_data: Option<&serde_json::Value>,
    presentation: Option<&serde_json::Value>,
    source: Option<&serde_json::Value>,
    tool_name: &str,
) -> Vec<serde_json::Value> {
    let mut file_changes = Vec::new();

    // First, extract from toolSpecificData (terminal commands, etc.)
    if let Some(data) = tool_data {
        let kind = data.get("kind").and_then(|k| k.as_str()).unwrap_or("");

        match kind {
            "terminal" => {
                // Terminal command - extract command info
                if let Some(cmd) = data.get("commandLine") {
                    file_changes.push(serde_json::json!({
                        "type": "terminal_command",
                        "command": cmd.get("original"),
                        "edited": cmd.get("toolEdited"),
                        "output": data.get("terminalCommandOutput"),
                        "exit_code": data.get("terminalCommandState")
                            .and_then(|s| s.get("exitCode")),
                    }));
                }
            }
            "editFile" | "createFile" => {
                // File edit/create from toolSpecificData
                if let Some(uri) = data
                    .get("uri")
                    .and_then(|u| u.as_str())
                    .or_else(|| data.get("path").and_then(|p| p.as_str()))
                {
                    file_changes.push(serde_json::json!({
                        "type": kind,
                        "file_path": uri,
                        "old_string": data.get("oldString"),
                        "new_string": data.get("newString"),
                    }));
                }
            }
            _ => {
                // Other tool types - store raw data if relevant
                if !kind.is_empty() && kind != "thinking" {
                    file_changes.push(serde_json::json!({
                        "type": kind,
                        "data": data,
                    }));
                }
            }
        }
    }

    // If no file changes extracted from toolSpecificData, infer from tool name
    // Note: VS Code Copilot doesn't store file edit parameters in the session JSON
    if file_changes.is_empty() {
        match tool_name {
            n if n.contains("replaceString")
                || n.contains("replace_string")
                || n.contains("multiReplace")
                || n.contains("multi_replace") =>
            {
                file_changes.push(serde_json::json!({
                    "type": "file_edit",
                    "tool_name": tool_name,
                    "note": "File path not stored in session (VS Code limitation)",
                }));
            }
            n if n.contains("createFile") || n.contains("create_file") => {
                file_changes.push(serde_json::json!({
                    "type": "file_create",
                    "tool_name": tool_name,
                    "note": "File path not stored in session (VS Code limitation)",
                }));
            }
            n if n.contains("editNotebook") || n.contains("edit_notebook") => {
                file_changes.push(serde_json::json!({
                    "type": "notebook_edit",
                    "tool_name": tool_name,
                    "note": "File path not stored in session (VS Code limitation)",
                }));
            }
            n if n.contains("delete") || n.contains("remove") => {
                file_changes.push(serde_json::json!({
                    "type": "file_delete",
                    "tool_name": tool_name,
                    "note": "File path not stored in session (VS Code limitation)",
                }));
            }
            _ => {}
        }
    }

    file_changes
}

pub async fn search_sessions(
    state: web::Data<AppState>,
    query: web::Query<SearchQuery>,
) -> impl Responder {
    let db = state.db.lock().unwrap();
    let limit = query.limit.unwrap_or(20) as i64;
    let search_term = format!("%{}%", query.q);

    let result: Result<Vec<serde_json::Value>, _> = (|| {
        let mut stmt = db.conn.prepare(
            "SELECT DISTINCT s.id, s.title, s.provider, s.workspace_id, s.message_count, s.updated_at
             FROM sessions s
             LEFT JOIN messages m ON s.id = m.session_id
             WHERE s.title LIKE ?1 OR m.content LIKE ?1
             ORDER BY s.updated_at DESC
             LIMIT ?2"
        )?;

        let results: Vec<serde_json::Value> = stmt
            .query_map(params![search_term, limit], |row| {
                let workspace_id: Option<String> = row.get(3)?;
                let workspace_name = workspace_id.as_ref().map(|id| derive_workspace_name(id));
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "title": row.get::<_, String>(1)?,
                    "provider": row.get::<_, String>(2)?,
                    "workspace_id": workspace_id,
                    "workspace_name": workspace_name,
                    "message_count": row.get::<_, i64>(4)?,
                    "updated_at": row.get::<_, i64>(5)?,
                }))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok::<_, rusqlite::Error>(results)
    })();

    match result {
        Ok(results) => ApiResponse::success(serde_json::json!({
            "results": results,
            "query": query.q,
        })),
        Err(e) => ApiResponse::<()>::error(&e.to_string()),
    }
}

// =============================================================================
// Helper functions for enhanced message data
// =============================================================================

fn get_tool_invocations(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<Vec<serde_json::Value>, rusqlite::Error> {
    // Check if table exists first
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='tool_invocations'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !table_exists {
        return Ok(Vec::new());
    }

    let mut stmt = conn.prepare(
        "SELECT id, message_id, tool_name, tool_call_id, invocation_index, 
                input_json, output_json, status, is_confirmed, timestamp
         FROM tool_invocations 
         WHERE session_id = ?1 
         ORDER BY message_id, invocation_index",
    )?;

    let invocations: Vec<serde_json::Value> = stmt
        .query_map([session_id], |row| {
            let input: Option<String> = row.get(5)?;
            let output: Option<String> = row.get(6)?;

            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "message_id": row.get::<_, i64>(1)?,
                "tool_name": row.get::<_, String>(2)?,
                "tool_call_id": row.get::<_, Option<String>>(3)?,
                "invocation_index": row.get::<_, i64>(4)?,
                "input": input.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()),
                "output": output.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()),
                "status": row.get::<_, String>(7)?,
                "is_confirmed": row.get::<_, i64>(8)? > 0,
                "timestamp": row.get::<_, Option<i64>>(9)?,
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(invocations)
}

fn get_file_changes(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<Vec<serde_json::Value>, rusqlite::Error> {
    // Check if table exists first
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='file_changes'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !table_exists {
        return Ok(Vec::new());
    }

    let mut stmt = conn.prepare(
        "SELECT id, tool_invocation_id, file_path, change_type, 
                old_content, new_content, diff_unified, line_start, line_end, timestamp
         FROM file_changes 
         WHERE session_id = ?1 
         ORDER BY id",
    )?;

    let changes: Vec<serde_json::Value> = stmt
        .query_map([session_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "tool_invocation_id": row.get::<_, Option<i64>>(1)?,
                "file_path": row.get::<_, String>(2)?,
                "change_type": row.get::<_, String>(3)?,
                "old_content": row.get::<_, Option<String>>(4)?,
                "new_content": row.get::<_, Option<String>>(5)?,
                "diff_unified": row.get::<_, Option<String>>(6)?,
                "line_start": row.get::<_, Option<i64>>(7)?,
                "line_end": row.get::<_, Option<i64>>(8)?,
                "timestamp": row.get::<_, Option<i64>>(9)?,
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(changes)
}

// =============================================================================
// Stats Handler (using harvest schema)
// =============================================================================

pub async fn get_stats(state: web::Data<AppState>) -> impl Responder {
    let db = state.db.lock().unwrap();

    let result: Result<serde_json::Value, _> =
        (|| {
            let total_sessions: i64 =
                db.conn
                    .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;

            // Check if enhanced tables exist and query them safely
            let messages_v2_exists: bool = db.conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='messages_v2'",
            [],
            |row| row.get(0),
        ).unwrap_or(false);

            let tool_invocations_exists: bool = db.conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='tool_invocations'",
            [],
            |row| row.get(0),
        ).unwrap_or(false);

            let file_changes_exists: bool = db.conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='file_changes'",
            [],
            |row| row.get(0),
        ).unwrap_or(false);

            let total_messages: i64 = if messages_v2_exists {
                db.conn
                    .query_row("SELECT COUNT(*) FROM messages_v2", [], |row| row.get(0))
                    .unwrap_or(0)
            } else {
                // Fallback: estimate from session message_count
                db.conn
                    .query_row(
                        "SELECT COALESCE(SUM(message_count), 0) FROM sessions",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap_or(0)
            };

            let total_tool_invocations: i64 = if tool_invocations_exists {
                db.conn
                    .query_row("SELECT COUNT(*) FROM tool_invocations", [], |row| {
                        row.get(0)
                    })
                    .unwrap_or(0)
            } else {
                0
            };

            let total_file_changes: i64 = if file_changes_exists {
                db.conn
                    .query_row("SELECT COUNT(*) FROM file_changes", [], |row| row.get(0))
                    .unwrap_or(0)
            } else {
                0
            };

            let mut stmt = db.conn.prepare(
                "SELECT provider, COUNT(*) FROM sessions GROUP BY provider ORDER BY COUNT(*) DESC",
            )?;

            let by_provider: Vec<(String, i64)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<Result<_, _>>()?;

            // Count unique workspaces
            let total_workspaces: i64 = db.conn.query_row(
                "SELECT COUNT(DISTINCT workspace_id) FROM sessions",
                [],
                |row| row.get(0),
            )?;

            // Convert by_provider to expected format
            let sessions_by_provider: Vec<serde_json::Value> = by_provider
                .iter()
                .map(|(provider, count)| {
                    serde_json::json!({
                        "provider": provider,
                        "count": count
                    })
                })
                .collect();

            Ok::<_, rusqlite::Error>(serde_json::json!({
                "totalSessions": total_sessions,
                "totalMessages": total_messages,
                "totalWorkspaces": total_workspaces,
                "totalToolInvocations": total_tool_invocations,
                "totalFileChanges": total_file_changes,
                "tablesEnhanced": messages_v2_exists,
                "sessionsByProvider": sessions_by_provider,
            }))
        })();

    match result {
        Ok(stats) => ApiResponse::success(stats),
        Err(e) => ApiResponse::<()>::error(&e.to_string()),
    }
}

// =============================================================================
// Provider Handlers
// =============================================================================

/// Provider information for the API
#[derive(Debug, Serialize)]
struct ProviderInfo {
    id: String,
    name: String,
    #[serde(rename = "type")]
    provider_type: String,
    status: String,
    icon: String,
    color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    endpoint: Option<String>,
    models: Vec<String>,
}

impl ProviderInfo {
    fn cloud(
        id: &str,
        name: &str,
        icon: &str,
        color: &str,
        endpoint: Option<&str>,
        models: Vec<&str>,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            provider_type: "cloud".to_string(),
            status: if id == "copilot" {
                "connected".to_string()
            } else {
                "disconnected".to_string()
            },
            icon: icon.to_string(),
            color: color.to_string(),
            endpoint: endpoint.map(|s| s.to_string()),
            models: models.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    fn local(
        id: &str,
        name: &str,
        icon: &str,
        color: &str,
        endpoint: &str,
        models: Vec<&str>,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            provider_type: "local".to_string(),
            status: "disconnected".to_string(),
            icon: icon.to_string(),
            color: color.to_string(),
            endpoint: Some(endpoint.to_string()),
            models: models.into_iter().map(|s| s.to_string()).collect(),
        }
    }
}

pub async fn list_providers() -> impl Responder {
    let providers = vec![
        // ===========================================
        // Cloud Providers
        // ===========================================
        ProviderInfo::cloud(
            "copilot",
            "GitHub Copilot",
            "🤖",
            "#000000",
            None,
            vec![
                "gpt-4.1",
                "gpt-4.1-mini",
                "gpt-4o",
                "gpt-4o-mini",
                "o1",
                "o1-mini",
                "o1-preview",
                "o3",
                "o3-mini",
                "o4-mini",
                "claude-sonnet-4",
                "claude-3.5-sonnet",
                "gemini-2.0-flash",
                "gemini-2.5-pro",
            ],
        ),
        ProviderInfo::cloud(
            "openai",
            "OpenAI",
            "🧠",
            "#10a37f",
            Some("https://api.openai.com/v1"),
            vec![
                "gpt-4.1",
                "gpt-4.1-mini",
                "gpt-4.1-nano",
                "gpt-4o",
                "gpt-4o-mini",
                "gpt-4o-audio-preview",
                "gpt-4-turbo",
                "gpt-4",
                "gpt-3.5-turbo",
                "o1",
                "o1-mini",
                "o1-preview",
                "o3",
                "o3-mini",
                "o4-mini",
                "chatgpt-4o-latest",
            ],
        ),
        ProviderInfo::cloud(
            "anthropic",
            "Anthropic",
            "🎭",
            "#d4a574",
            Some("https://api.anthropic.com/v1"),
            vec![
                "claude-opus-4",
                "claude-sonnet-4",
                "claude-3.5-sonnet",
                "claude-3.5-haiku",
                "claude-3-opus",
                "claude-3-sonnet",
                "claude-3-haiku",
            ],
        ),
        ProviderInfo::cloud(
            "google",
            "Google AI",
            "✨",
            "#4285f4",
            Some("https://generativelanguage.googleapis.com/v1beta"),
            vec![
                "gemini-2.5-pro",
                "gemini-2.5-flash",
                "gemini-2.0-flash",
                "gemini-2.0-flash-thinking",
                "gemini-1.5-pro",
                "gemini-1.5-flash",
                "gemini-1.5-flash-8b",
                "gemini-pro",
                "gemini-pro-vision",
            ],
        ),
        ProviderInfo::cloud(
            "azure-openai",
            "Azure OpenAI",
            "☁️",
            "#0078d4",
            None,
            vec![
                "gpt-4o",
                "gpt-4o-mini",
                "gpt-4-turbo",
                "gpt-4",
                "gpt-35-turbo",
                "o1",
                "o1-mini",
            ],
        ),
        ProviderInfo::cloud(
            "ai-foundry",
            "Azure AI Foundry",
            "🏭",
            "#0078d4",
            None,
            vec![
                "gpt-4o",
                "gpt-4o-mini",
                "o1",
                "o1-mini",
                "Phi-4",
                "Phi-3.5-MoE-instruct",
                "Phi-3.5-mini-instruct",
                "Phi-3.5-vision-instruct",
                "Llama-3.3-70B-Instruct",
                "Llama-3.2-90B-Vision-Instruct",
                "Llama-3.1-405B-Instruct",
                "Mistral-large-2411",
                "Mistral-small",
                "Codestral-2501",
                "DeepSeek-R1",
                "DeepSeek-V3",
                "Cohere-command-r-plus",
                "JAIS-30b-chat",
            ],
        ),
        ProviderInfo::cloud(
            "github-models",
            "GitHub Models",
            "🐙",
            "#24292e",
            Some("https://models.inference.ai.azure.com"),
            vec![
                "gpt-4o",
                "gpt-4o-mini",
                "o1",
                "o1-mini",
                "o1-preview",
                "Phi-4",
                "Phi-3.5-MoE-instruct",
                "Llama-3.3-70B-Instruct",
                "Llama-3.2-90B-Vision-Instruct",
                "Meta-Llama-3.1-405B-Instruct",
                "Mistral-large-2411",
                "Mistral-small",
                "Codestral-2501",
                "DeepSeek-R1",
                "Cohere-command-r-plus",
            ],
        ),
        ProviderInfo::cloud(
            "deepseek",
            "DeepSeek",
            "🔍",
            "#4d6bfe",
            Some("https://api.deepseek.com/v1"),
            vec!["deepseek-chat", "deepseek-reasoner", "deepseek-coder"],
        ),
        ProviderInfo::cloud(
            "xai",
            "xAI",
            "🚀",
            "#000000",
            Some("https://api.x.ai/v1"),
            vec![
                "grok-3",
                "grok-3-fast",
                "grok-2",
                "grok-2-mini",
                "grok-2-vision",
                "grok-beta",
            ],
        ),
        ProviderInfo::cloud(
            "mistral",
            "Mistral AI",
            "🌬️",
            "#ff7000",
            Some("https://api.mistral.ai/v1"),
            vec![
                "mistral-large-latest",
                "mistral-large-2411",
                "mistral-medium-latest",
                "mistral-small-latest",
                "mistral-small-2501",
                "codestral-latest",
                "codestral-2501",
                "ministral-3b-latest",
                "ministral-8b-latest",
                "pixtral-large-latest",
                "pixtral-12b",
                "open-mistral-nemo",
                "open-codestral-mamba",
            ],
        ),
        ProviderInfo::cloud(
            "cohere",
            "Cohere",
            "🔗",
            "#39594d",
            Some("https://api.cohere.ai/v1"),
            vec![
                "command-r-plus",
                "command-r",
                "command",
                "command-light",
                "command-nightly",
                "aya-expanse-32b",
                "aya-expanse-8b",
            ],
        ),
        ProviderInfo::cloud(
            "perplexity",
            "Perplexity",
            "🔮",
            "#20808d",
            Some("https://api.perplexity.ai"),
            vec![
                "sonar-pro",
                "sonar",
                "sonar-deep-research",
                "sonar-reasoning-pro",
                "sonar-reasoning",
            ],
        ),
        ProviderInfo::cloud(
            "groq",
            "Groq",
            "⚡",
            "#f55036",
            Some("https://api.groq.com/openai/v1"),
            vec![
                "llama-3.3-70b-versatile",
                "llama-3.1-70b-versatile",
                "llama-3.1-8b-instant",
                "llama3-groq-70b-8192-tool-use-preview",
                "llama3-groq-8b-8192-tool-use-preview",
                "mixtral-8x7b-32768",
                "gemma2-9b-it",
                "deepseek-r1-distill-llama-70b",
            ],
        ),
        ProviderInfo::cloud(
            "together",
            "Together AI",
            "🤝",
            "#0f6fff",
            Some("https://api.together.xyz/v1"),
            vec![
                "meta-llama/Llama-3.3-70B-Instruct-Turbo",
                "meta-llama/Meta-Llama-3.1-405B-Instruct-Turbo",
                "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo",
                "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo",
                "Qwen/Qwen2.5-72B-Instruct-Turbo",
                "Qwen/QwQ-32B-Preview",
                "deepseek-ai/DeepSeek-R1",
                "deepseek-ai/DeepSeek-V3",
                "mistralai/Mixtral-8x22B-Instruct-v0.1",
                "databricks/dbrx-instruct",
            ],
        ),
        ProviderInfo::cloud(
            "fireworks",
            "Fireworks AI",
            "🎆",
            "#ff6b35",
            Some("https://api.fireworks.ai/inference/v1"),
            vec![
                "accounts/fireworks/models/llama-v3p3-70b-instruct",
                "accounts/fireworks/models/llama-v3p1-405b-instruct",
                "accounts/fireworks/models/qwen2p5-72b-instruct",
                "accounts/fireworks/models/mixtral-8x22b-instruct",
                "accounts/fireworks/models/deepseek-r1",
                "accounts/fireworks/models/deepseek-v3",
            ],
        ),
        ProviderInfo::cloud(
            "replicate",
            "Replicate",
            "🔄",
            "#000000",
            Some("https://api.replicate.com/v1"),
            vec![
                "meta/llama-3.3-70b-instruct",
                "meta/meta-llama-3.1-405b-instruct",
                "mistralai/mixtral-8x7b-instruct-v0.1",
                "anthropic/claude-3.5-sonnet",
            ],
        ),
        ProviderInfo::cloud(
            "openrouter",
            "OpenRouter",
            "🛤️",
            "#6467f2",
            Some("https://openrouter.ai/api/v1"),
            vec![
                "openai/gpt-4o",
                "openai/o1",
                "anthropic/claude-sonnet-4",
                "anthropic/claude-3.5-sonnet",
                "google/gemini-2.0-flash",
                "google/gemini-2.5-pro",
                "meta-llama/llama-3.3-70b-instruct",
                "deepseek/deepseek-r1",
                "deepseek/deepseek-chat",
                "mistralai/mistral-large-2411",
                "qwen/qwq-32b-preview",
            ],
        ),
        ProviderInfo::cloud(
            "aws-bedrock",
            "AWS Bedrock",
            "🪨",
            "#ff9900",
            None,
            vec![
                "anthropic.claude-3-5-sonnet-20241022-v2:0",
                "anthropic.claude-3-5-haiku-20241022-v1:0",
                "anthropic.claude-3-opus-20240229-v1:0",
                "anthropic.claude-3-sonnet-20240229-v1:0",
                "meta.llama3-3-70b-instruct-v1:0",
                "meta.llama3-1-405b-instruct-v1:0",
                "mistral.mistral-large-2411-v1:0",
                "amazon.nova-pro-v1:0",
                "amazon.nova-lite-v1:0",
                "amazon.nova-micro-v1:0",
                "amazon.titan-text-premier-v1:0",
                "cohere.command-r-plus-v1:0",
            ],
        ),
        ProviderInfo::cloud(
            "ai21",
            "AI21 Labs",
            "🧪",
            "#ec4899",
            Some("https://api.ai21.com/studio/v1"),
            vec!["jamba-1.5-large", "jamba-1.5-mini", "jamba-instruct"],
        ),
        ProviderInfo::cloud(
            "cursor",
            "Cursor",
            "📝",
            "#000000",
            None,
            vec![
                "cursor-small",
                "cursor-large",
                "gpt-4",
                "gpt-4o",
                "claude-3.5-sonnet",
            ],
        ),
        ProviderInfo::cloud(
            "m365-copilot",
            "Microsoft 365 Copilot",
            "📊",
            "#0078d4",
            None,
            vec![
                "copilot-chat",
                "copilot-word",
                "copilot-excel",
                "copilot-powerpoint",
                "copilot-outlook",
                "copilot-teams",
            ],
        ),
        // ===========================================
        // Local Providers
        // ===========================================
        ProviderInfo::local(
            "ollama",
            "Ollama",
            "🦙",
            "#ffffff",
            "http://localhost:11434",
            vec![
                "llama3.3:70b",
                "llama3.3:latest",
                "llama3.2:latest",
                "llama3.1:405b",
                "llama3.1:70b",
                "llama3.1:latest",
                "qwen2.5-coder:32b",
                "qwen2.5-coder:14b",
                "qwen2.5-coder:7b",
                "qwen2.5:72b",
                "qwen2.5:32b",
                "qwen2.5:14b",
                "qwen2.5:7b",
                "qwq:32b",
                "deepseek-r1:70b",
                "deepseek-r1:32b",
                "deepseek-r1:14b",
                "deepseek-r1:8b",
                "deepseek-r1:1.5b",
                "deepseek-coder-v2:latest",
                "codellama:70b",
                "codellama:34b",
                "codellama:13b",
                "codellama:7b",
                "mistral:latest",
                "mistral-nemo:latest",
                "mixtral:8x7b",
                "mixtral:8x22b",
                "phi4:latest",
                "phi3.5:latest",
                "phi3:latest",
                "gemma2:27b",
                "gemma2:9b",
                "gemma2:2b",
                "command-r:latest",
                "command-r-plus:latest",
                "yi:34b",
                "yi-coder:9b",
                "starcoder2:15b",
                "starcoder2:7b",
                "starcoder2:3b",
                "nomic-embed-text:latest",
                "mxbai-embed-large:latest",
            ],
        ),
        ProviderInfo::local(
            "lm-studio",
            "LM Studio",
            "🎬",
            "#1a1a2e",
            "http://localhost:1234/v1",
            vec!["loaded-model"],
        ),
        ProviderInfo::local(
            "localai",
            "LocalAI",
            "🏠",
            "#00d4aa",
            "http://localhost:8080/v1",
            vec![
                "gpt4all-j",
                "ggml-gpt4all-j",
                "wizardlm-13b-v1.2",
                "llama-2-7b-chat",
                "codellama-7b-instruct",
            ],
        ),
        ProviderInfo::local(
            "llamafile",
            "llamafile",
            "📁",
            "#fbbf24",
            "http://localhost:8080/v1",
            vec!["loaded-model"],
        ),
        ProviderInfo::local(
            "jan",
            "Jan",
            "💬",
            "#1d4ed8",
            "http://localhost:1337/v1",
            vec!["loaded-model"],
        ),
        ProviderInfo::local(
            "gpt4all",
            "GPT4All",
            "🌐",
            "#4ade80",
            "http://localhost:4891/v1",
            vec![
                "gpt4all-falcon-newbpe-q4_0",
                "gpt4all-mistral-7b-instruct-v0.2",
                "orca-2-7b",
                "nous-hermes-llama2-13b",
                "wizardlm-13b-v1.2",
            ],
        ),
        ProviderInfo::local(
            "text-gen-webui",
            "Text Generation WebUI",
            "🖥️",
            "#a855f7",
            "http://localhost:5000/v1",
            vec!["loaded-model"],
        ),
        ProviderInfo::local(
            "vllm",
            "vLLM",
            "⚙️",
            "#06b6d4",
            "http://localhost:8000/v1",
            vec![
                "meta-llama/Llama-3.3-70B-Instruct",
                "meta-llama/Llama-3.1-8B-Instruct",
                "mistralai/Mistral-7B-Instruct-v0.3",
                "Qwen/Qwen2.5-72B-Instruct",
                "deepseek-ai/DeepSeek-V3",
            ],
        ),
        ProviderInfo::local(
            "mlx",
            "MLX (Apple Silicon)",
            "🍎",
            "#a3a3a3",
            "http://localhost:8080/v1",
            vec![
                "mlx-community/Llama-3.3-70B-Instruct-4bit",
                "mlx-community/Qwen2.5-Coder-32B-Instruct-4bit",
                "mlx-community/Mistral-7B-Instruct-v0.3-4bit",
            ],
        ),
        ProviderInfo::local(
            "koboldcpp",
            "KoboldCpp",
            "🐉",
            "#dc2626",
            "http://localhost:5001/v1",
            vec!["loaded-model"],
        ),
        ProviderInfo::local(
            "tabby",
            "Tabby",
            "🐱",
            "#f59e0b",
            "http://localhost:8080",
            vec![
                "StarCoder-1B",
                "StarCoder-3B",
                "StarCoder-7B",
                "CodeLlama-7B",
                "CodeLlama-13B",
                "DeepSeek-Coder-1.3B",
                "DeepSeek-Coder-6.7B",
            ],
        ),
    ];

    ApiResponse::success(providers)
}

// =============================================================================
// MCP Tools Handlers (for introspective chat)
// =============================================================================

/// List available MCP tools
pub async fn list_mcp_tools() -> impl Responder {
    use crate::mcp::tools::list_tools;

    let tools = list_tools();

    // Convert to OpenAI-compatible function format for chat completions
    let openai_tools: Vec<serde_json::Value> = tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema,
                }
            })
        })
        .collect();

    ApiResponse::success(serde_json::json!({
        "tools": openai_tools,
        "mcp_tools": tools,
    }))
}

/// Execute an MCP tool call
#[derive(Debug, Deserialize)]
pub struct ToolCallRequest {
    pub name: String,
    pub arguments: std::collections::HashMap<String, serde_json::Value>,
}

pub async fn call_mcp_tool(body: web::Json<ToolCallRequest>) -> impl Responder {
    use crate::mcp::tools::call_tool;

    let request = body.into_inner();
    let result = call_tool(&request.name, &request.arguments);

    ApiResponse::success(serde_json::json!({
        "tool": request.name,
        "result": result,
    }))
}

/// Execute multiple MCP tool calls
#[derive(Debug, Deserialize)]
pub struct BatchToolCallRequest {
    pub calls: Vec<ToolCallRequest>,
}

pub async fn call_mcp_tools_batch(body: web::Json<BatchToolCallRequest>) -> impl Responder {
    use crate::mcp::tools::call_tool;

    let request = body.into_inner();
    let results: Vec<serde_json::Value> = request
        .calls
        .iter()
        .map(|call| {
            let result = call_tool(&call.name, &call.arguments);
            serde_json::json!({
                "tool": call.name,
                "result": result,
            })
        })
        .collect();

    ApiResponse::success(results)
}

/// Get system prompt with CSM tools context
pub async fn get_csm_system_prompt() -> impl Responder {
    use crate::mcp::tools::list_tools;

    let tools = list_tools();
    let tool_descriptions: Vec<String> = tools
        .iter()
        .filter(|t| t.name.starts_with("csm_db_")) // Only database tools for chat
        .map(|t| {
            format!(
                "- {}: {}",
                t.name,
                t.description.as_ref().unwrap_or(&String::new())
            )
        })
        .collect();

    let system_prompt = format!(
        r#"You are an AI assistant integrated with the Chat Session Manager (CSM) system. You have access to tools that let you introspect and query the user's chat history database.

Available CSM tools:
{}

When the user asks about their chat history, previous conversations, sessions, or workspaces, use these tools to provide accurate information.

Guidelines:
- Use csm_db_list_workspaces to see all projects/workspaces with chat sessions
- Use csm_db_list_sessions to see chat sessions, optionally filtered by workspace or provider
- Use csm_db_get_session to retrieve full conversation history from a specific session
- Use csm_db_search to search across session titles
- Use csm_db_stats to get overview statistics

Always be helpful and provide context when presenting results from these tools."#,
        tool_descriptions.join("\n")
    );

    ApiResponse::success(serde_json::json!({
        "system_prompt": system_prompt,
        "available_tools": tool_descriptions,
    }))
}

// =============================================================================
// Agent Endpoints
// =============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub instruction: String,
    pub role: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub temperature: f32,
    pub max_tokens: Option<i32>,
    pub tools: Vec<String>,
    pub sub_agents: Vec<String>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub metadata: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub description: Option<String>,
    pub instruction: String,
    pub role: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub tools: Option<Vec<String>>,
    pub sub_agents: Option<Vec<String>>,
    pub metadata: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAgentRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub instruction: Option<String>,
    pub role: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub tools: Option<Vec<String>>,
    pub sub_agents: Option<Vec<String>>,
    pub is_active: Option<bool>,
    pub metadata: Option<String>,
}

fn init_agents_table(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agents (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            instruction TEXT NOT NULL,
            role TEXT DEFAULT 'assistant',
            model TEXT,
            provider TEXT,
            temperature REAL DEFAULT 0.7,
            max_tokens INTEGER,
            tools TEXT DEFAULT '[]',
            sub_agents TEXT,
            is_active INTEGER DEFAULT 1,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
            metadata TEXT
        )",
        [],
    )?;
    Ok(())
}

/// List all agents
pub async fn list_agents(state: web::Data<AppState>) -> impl Responder {
    let db = state.db.lock().unwrap();

    // Ensure table exists
    if let Err(e) = init_agents_table(&db.conn) {
        return ApiResponse::<()>::error(&format!("Database error: {}", e));
    }

    let result: Result<Vec<serde_json::Value>, rusqlite::Error> = (|| {
        let mut stmt = db.conn.prepare(
            "SELECT id, name, description, instruction, role, model, provider, 
                    temperature, max_tokens, tools, sub_agents, is_active, 
                    created_at, updated_at, metadata 
             FROM agents ORDER BY updated_at DESC",
        )?;

        let agents: Vec<serde_json::Value> = stmt
            .query_map([], |row| {
                let tools_str: String = row.get::<_, Option<String>>(9)?.unwrap_or_default();
                let tools: Vec<String> = serde_json::from_str(&tools_str).unwrap_or_default();
                let sub_agents_str: String = row.get::<_, Option<String>>(10)?.unwrap_or_default();
                let sub_agents: Vec<String> =
                    serde_json::from_str(&sub_agents_str).unwrap_or_default();
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "description": row.get::<_, Option<String>>(2)?,
                    "instruction": row.get::<_, String>(3)?,
                    "role": row.get::<_, Option<String>>(4)?,
                    "model": row.get::<_, Option<String>>(5)?,
                    "provider": row.get::<_, Option<String>>(6)?,
                    "temperature": row.get::<_, f64>(7)?,
                    "maxTokens": row.get::<_, Option<i32>>(8)?,
                    "tools": tools,
                    "subAgents": sub_agents,
                    "isActive": row.get::<_, i32>(11)? == 1,
                    "createdAt": row.get::<_, i64>(12)?,
                    "updatedAt": row.get::<_, i64>(13)?,
                    "metadata": row.get::<_, Option<String>>(14)?,
                }))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(agents)
    })();

    match result {
        Ok(agents) => ApiResponse::success(agents),
        Err(e) => ApiResponse::<()>::error(&format!("Database error: {}", e)),
    }
}

/// Get a single agent
pub async fn get_agent(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let id = path.into_inner();
    let db = state.db.lock().unwrap();

    if let Err(e) = init_agents_table(&db.conn) {
        return ApiResponse::<()>::error(&format!("Database error: {}", e));
    }

    let result: Result<Option<serde_json::Value>, rusqlite::Error> = db
        .conn
        .query_row(
            "SELECT id, name, description, instruction, role, model, provider, 
                temperature, max_tokens, tools, sub_agents, is_active, 
                created_at, updated_at, metadata 
         FROM agents WHERE id = ?1",
            params![id],
            |row| {
                let tools_str: String = row.get::<_, Option<String>>(9)?.unwrap_or_default();
                let tools: Vec<String> = serde_json::from_str(&tools_str).unwrap_or_default();
                let sub_agents_str: String = row.get::<_, Option<String>>(10)?.unwrap_or_default();
                let sub_agents: Vec<String> =
                    serde_json::from_str(&sub_agents_str).unwrap_or_default();
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "description": row.get::<_, Option<String>>(2)?,
                    "instruction": row.get::<_, String>(3)?,
                    "role": row.get::<_, Option<String>>(4)?,
                    "model": row.get::<_, Option<String>>(5)?,
                    "provider": row.get::<_, Option<String>>(6)?,
                    "temperature": row.get::<_, f64>(7)?,
                    "maxTokens": row.get::<_, Option<i32>>(8)?,
                    "tools": tools,
                    "subAgents": sub_agents,
                    "isActive": row.get::<_, i32>(11)? == 1,
                    "createdAt": row.get::<_, i64>(12)?,
                    "updatedAt": row.get::<_, i64>(13)?,
                    "metadata": row.get::<_, Option<String>>(14)?,
                }))
            },
        )
        .optional();

    match result {
        Ok(Some(agent)) => ApiResponse::success(agent),
        Ok(None) => HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some("Agent not found".to_string()),
        }),
        Err(e) => ApiResponse::<()>::error(&format!("Database error: {}", e)),
    }
}

/// Create a new agent
pub async fn create_agent(
    state: web::Data<AppState>,
    body: web::Json<CreateAgentRequest>,
) -> impl Responder {
    let db = state.db.lock().unwrap();

    if let Err(e) = init_agents_table(&db.conn) {
        return ApiResponse::<()>::error(&format!("Database error: {}", e));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let tools_json = serde_json::to_string(&body.tools.clone().unwrap_or_default()).unwrap();
    let sub_agents_json =
        serde_json::to_string(&body.sub_agents.clone().unwrap_or_default()).unwrap();
    let role = body.role.clone().unwrap_or_else(|| "assistant".to_string());

    let result = db.conn.execute(
        "INSERT INTO agents (id, name, description, instruction, role, model, provider, 
                            temperature, max_tokens, tools, sub_agents, is_active, 
                            created_at, updated_at, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1, ?12, ?13, ?14)",
        params![
            id,
            body.name,
            body.description,
            body.instruction,
            role,
            body.model,
            body.provider,
            body.temperature.unwrap_or(0.7),
            body.max_tokens,
            tools_json,
            sub_agents_json,
            now,
            now,
            body.metadata
        ],
    );

    match result {
        Ok(_) => ApiResponse::success(serde_json::json!({
            "id": id,
            "name": body.name,
            "description": body.description,
            "instruction": body.instruction,
            "role": role,
            "model": body.model,
            "provider": body.provider,
            "temperature": body.temperature.unwrap_or(0.7),
            "maxTokens": body.max_tokens,
            "tools": body.tools.clone().unwrap_or_default(),
            "subAgents": body.sub_agents.clone().unwrap_or_default(),
            "isActive": true,
            "createdAt": now,
            "updatedAt": now,
            "metadata": body.metadata,
        })),
        Err(e) => ApiResponse::<()>::error(&format!("Failed to create agent: {}", e)),
    }
}

/// Update an agent
pub async fn update_agent(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<UpdateAgentRequest>,
) -> impl Responder {
    let id = path.into_inner();
    let db = state.db.lock().unwrap();

    if let Err(e) = init_agents_table(&db.conn) {
        return ApiResponse::<()>::error(&format!("Database error: {}", e));
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let result = db.conn.execute(
        "UPDATE agents SET updated_at = ?1, 
         name = COALESCE(?2, name),
         description = COALESCE(?3, description),
         instruction = COALESCE(?4, instruction),
         role = COALESCE(?5, role),
         model = COALESCE(?6, model),
         provider = COALESCE(?7, provider),
         temperature = COALESCE(?8, temperature),
         max_tokens = COALESCE(?9, max_tokens),
         tools = COALESCE(?10, tools),
         sub_agents = COALESCE(?11, sub_agents),
         is_active = COALESCE(?12, is_active),
         metadata = COALESCE(?13, metadata)
         WHERE id = ?14",
        params![
            now,
            body.name,
            body.description,
            body.instruction,
            body.role,
            body.model,
            body.provider,
            body.temperature,
            body.max_tokens,
            body.tools
                .as_ref()
                .map(|t| serde_json::to_string(t).unwrap()),
            body.sub_agents
                .as_ref()
                .map(|t| serde_json::to_string(t).unwrap()),
            body.is_active.map(|b| if b { 1 } else { 0 }),
            body.metadata.clone(),
            id
        ],
    );

    match result {
        Ok(0) => HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some("Agent not found".to_string()),
        }),
        Ok(_) => {
            // Return success with updated id
            ApiResponse::success(serde_json::json!({ "id": id, "updated": true }))
        }
        Err(e) => ApiResponse::<()>::error(&format!("Failed to update agent: {}", e)),
    }
}

/// Delete an agent
pub async fn delete_agent(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let id = path.into_inner();
    let db = state.db.lock().unwrap();

    let result = db
        .conn
        .execute("DELETE FROM agents WHERE id = ?1", params![id]);

    match result {
        Ok(0) => HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some("Agent not found".to_string()),
        }),
        Ok(_) => ApiResponse::success(serde_json::json!({ "deleted": true })),
        Err(e) => ApiResponse::<()>::error(&format!("Failed to delete agent: {}", e)),
    }
}

// =============================================================================
// Swarm Endpoints
// =============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwarmAgent {
    pub agent_id: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSwarmRequest {
    pub name: String,
    pub description: Option<String>,
    pub orchestration: String, // "sequential", "parallel", "hierarchical", "debate"
    pub agents: Vec<SwarmAgent>,
    pub max_iterations: Option<i32>,
}

fn init_swarms_table(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS swarms (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            orchestration TEXT NOT NULL DEFAULT 'sequential',
            agents TEXT NOT NULL DEFAULT '[]',
            max_iterations INTEGER DEFAULT 10,
            status TEXT DEFAULT 'idle',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(())
}

/// List all swarms
pub async fn list_swarms(state: web::Data<AppState>) -> impl Responder {
    let db = state.db.lock().unwrap();

    if let Err(e) = init_swarms_table(&db.conn) {
        return ApiResponse::<()>::error(&format!("Database error: {}", e));
    }

    let result: Result<Vec<serde_json::Value>, rusqlite::Error> = (|| {
        let mut stmt = db.conn.prepare(
            "SELECT id, name, description, orchestration, agents, max_iterations, 
                    status, created_at, updated_at 
             FROM swarms ORDER BY updated_at DESC",
        )?;

        let swarms: Vec<serde_json::Value> = stmt
            .query_map([], |row| {
                let agents_str: String = row.get(4)?;
                let agents: Vec<serde_json::Value> =
                    serde_json::from_str(&agents_str).unwrap_or_default();
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "description": row.get::<_, Option<String>>(2)?,
                    "orchestration": row.get::<_, String>(3)?,
                    "agents": agents,
                    "maxIterations": row.get::<_, Option<i32>>(5)?,
                    "status": row.get::<_, String>(6)?,
                    "createdAt": row.get::<_, i64>(7)?,
                    "updatedAt": row.get::<_, i64>(8)?,
                }))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(swarms)
    })();

    match result {
        Ok(swarms) => ApiResponse::success(swarms),
        Err(e) => ApiResponse::<()>::error(&format!("Database error: {}", e)),
    }
}

/// Get a single swarm
pub async fn get_swarm(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let id = path.into_inner();
    let db = state.db.lock().unwrap();

    if let Err(e) = init_swarms_table(&db.conn) {
        return ApiResponse::<()>::error(&format!("Database error: {}", e));
    }

    let result: Result<Option<serde_json::Value>, rusqlite::Error> = db
        .conn
        .query_row(
            "SELECT id, name, description, orchestration, agents, max_iterations, 
                status, created_at, updated_at 
         FROM swarms WHERE id = ?1",
            params![id],
            |row| {
                let agents_str: String = row.get(4)?;
                let agents: Vec<serde_json::Value> =
                    serde_json::from_str(&agents_str).unwrap_or_default();
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "description": row.get::<_, Option<String>>(2)?,
                    "orchestration": row.get::<_, String>(3)?,
                    "agents": agents,
                    "maxIterations": row.get::<_, Option<i32>>(5)?,
                    "status": row.get::<_, String>(6)?,
                    "createdAt": row.get::<_, i64>(7)?,
                    "updatedAt": row.get::<_, i64>(8)?,
                }))
            },
        )
        .optional();

    match result {
        Ok(Some(swarm)) => ApiResponse::success(swarm),
        Ok(None) => HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some("Swarm not found".to_string()),
        }),
        Err(e) => ApiResponse::<()>::error(&format!("Database error: {}", e)),
    }
}

/// Create a new swarm
pub async fn create_swarm(
    state: web::Data<AppState>,
    body: web::Json<CreateSwarmRequest>,
) -> impl Responder {
    let db = state.db.lock().unwrap();

    if let Err(e) = init_swarms_table(&db.conn) {
        return ApiResponse::<()>::error(&format!("Database error: {}", e));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let agents_json = serde_json::to_string(&body.agents).unwrap();

    let result = db.conn.execute(
        "INSERT INTO swarms (id, name, description, orchestration, agents, max_iterations, 
                            status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'idle', ?7, ?8)",
        params![
            id,
            body.name,
            body.description,
            body.orchestration,
            agents_json,
            body.max_iterations.unwrap_or(10),
            now,
            now
        ],
    );

    match result {
        Ok(_) => ApiResponse::success(serde_json::json!({
            "id": id,
            "name": body.name,
            "description": body.description,
            "orchestration": body.orchestration,
            "agents": body.agents,
            "maxIterations": body.max_iterations.unwrap_or(10),
            "status": "idle",
            "createdAt": now,
            "updatedAt": now,
        })),
        Err(e) => ApiResponse::<()>::error(&format!("Failed to create swarm: {}", e)),
    }
}

/// Delete a swarm
pub async fn delete_swarm(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let id = path.into_inner();
    let db = state.db.lock().unwrap();

    let result = db
        .conn
        .execute("DELETE FROM swarms WHERE id = ?1", params![id]);

    match result {
        Ok(0) => HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some("Swarm not found".to_string()),
        }),
        Ok(_) => ApiResponse::success(serde_json::json!({ "deleted": true })),
        Err(e) => ApiResponse::<()>::error(&format!("Failed to delete swarm: {}", e)),
    }
}

// =============================================================================
// Settings Endpoints
// =============================================================================

fn init_settings_table(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    // Insert default settings if not exist
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES 
         ('theme', '\"system\"', ?1),
         ('defaultProvider', '\"copilot\"', ?1),
         ('autoSync', 'true', ?1),
         ('syncInterval', '300000', ?1),
         ('maxHistoryDays', '365', ?1),
         ('enableNotifications', 'true', ?1),
         ('compactMode', 'false', ?1)",
        params![now],
    )?;
    Ok(())
}

/// Get all settings
pub async fn get_settings(state: web::Data<AppState>) -> HttpResponse {
    let db = state.db.lock().unwrap();

    if let Err(e) = init_settings_table(&db.conn) {
        return ApiResponse::<()>::error(&format!("Database error: {}", e));
    }

    let result: Result<serde_json::Value, rusqlite::Error> = (|| {
        let mut stmt = db.conn.prepare("SELECT key, value FROM settings")?;
        let mut settings = serde_json::Map::new();

        stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let value_str: String = row.get(1)?;
            let value: serde_json::Value =
                serde_json::from_str(&value_str).unwrap_or(serde_json::Value::String(value_str));
            Ok((key, value))
        })?
        .for_each(|r| {
            if let Ok((k, v)) = r {
                settings.insert(k, v);
            }
        });

        Ok(serde_json::Value::Object(settings))
    })();

    match result {
        Ok(settings) => ApiResponse::success(settings),
        Err(e) => ApiResponse::<()>::error(&format!("Database error: {}", e)),
    }
}

/// Update settings
pub async fn update_settings(
    state: web::Data<AppState>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let db = state.db.lock().unwrap();

    if let Err(e) = init_settings_table(&db.conn) {
        return ApiResponse::<()>::error(&format!("Database error: {}", e));
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    if let serde_json::Value::Object(map) = body.into_inner() {
        for (key, value) in map {
            let value_str = serde_json::to_string(&value).unwrap();
            let _ = db.conn.execute(
                "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
                params![key, value_str, now],
            );
        }
    }

    // Return success - re-fetch would require dropping lock
    ApiResponse::success(serde_json::json!({ "updated": true }))
}

// =============================================================================
// Provider Accounts Endpoints
// =============================================================================

fn init_accounts_table(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS provider_accounts (
            id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            name TEXT NOT NULL,
            credentials TEXT NOT NULL,
            is_default INTEGER DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    pub provider: String,
    pub credentials: serde_json::Value,
}

/// List provider accounts
pub async fn list_accounts(state: web::Data<AppState>) -> impl Responder {
    let db = state.db.lock().unwrap();

    if let Err(e) = init_accounts_table(&db.conn) {
        return ApiResponse::<()>::error(&format!("Database error: {}", e));
    }

    let result: Result<Vec<serde_json::Value>, rusqlite::Error> = (|| {
        let mut stmt = db.conn.prepare(
            "SELECT id, provider, name, is_default, created_at, updated_at 
             FROM provider_accounts ORDER BY created_at DESC",
        )?;

        let accounts: Vec<serde_json::Value> = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "provider": row.get::<_, String>(1)?,
                    "name": row.get::<_, String>(2)?,
                    "isDefault": row.get::<_, i32>(3)? == 1,
                    "createdAt": row.get::<_, i64>(4)?,
                    "updatedAt": row.get::<_, i64>(5)?,
                }))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(accounts)
    })();

    match result {
        Ok(accounts) => ApiResponse::success(accounts),
        Err(e) => ApiResponse::<()>::error(&format!("Database error: {}", e)),
    }
}

/// Create a provider account
pub async fn create_account(
    state: web::Data<AppState>,
    body: web::Json<CreateAccountRequest>,
) -> impl Responder {
    let db = state.db.lock().unwrap();

    if let Err(e) = init_accounts_table(&db.conn) {
        return ApiResponse::<()>::error(&format!("Database error: {}", e));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let name = format!("{} Account", body.provider);
    let credentials_json = serde_json::to_string(&body.credentials).unwrap();

    let result = db.conn.execute(
        "INSERT INTO provider_accounts (id, provider, name, credentials, is_default, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6)",
        params![id, body.provider, name, credentials_json, now, now],
    );

    match result {
        Ok(_) => ApiResponse::success(serde_json::json!({
            "id": id,
            "provider": body.provider,
            "name": name,
            "isDefault": false,
            "createdAt": now,
            "updatedAt": now,
        })),
        Err(e) => ApiResponse::<()>::error(&format!("Failed to create account: {}", e)),
    }
}

/// Delete a provider account
pub async fn delete_account(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let id = path.into_inner();
    let db = state.db.lock().unwrap();

    let result = db
        .conn
        .execute("DELETE FROM provider_accounts WHERE id = ?1", params![id]);

    match result {
        Ok(0) => HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some("Account not found".to_string()),
        }),
        Ok(_) => ApiResponse::success(serde_json::json!({ "deleted": true })),
        Err(e) => ApiResponse::<()>::error(&format!("Failed to delete account: {}", e)),
    }
}

// =============================================================================
// System Endpoints
// =============================================================================

static START_TIME: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Get system information
pub async fn get_system_info(state: web::Data<AppState>) -> impl Responder {
    let start = START_TIME.get_or_init(std::time::Instant::now);
    let uptime = start.elapsed().as_secs();

    let db = state.db.lock().unwrap();
    let db_size: i64 = db
        .conn
        .query_row(
            "SELECT page_count * page_size as size FROM pragma_page_count(), pragma_page_size()",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    ApiResponse::success(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "status": "healthy",
        "uptime": uptime,
        "database": {
            "path": state.db_path,
            "sizeBytes": db_size,
        },
        "features": {
            "agents": true,
            "swarms": true,
            "mcp": true,
            "chat": true,
        }
    }))
}

/// Get system health
pub async fn get_system_health(state: web::Data<AppState>) -> impl Responder {
    let start = START_TIME.get_or_init(std::time::Instant::now);
    let uptime = start.elapsed().as_secs();

    // Try a simple DB query to verify connection
    let db = state.db.lock().unwrap();
    let db_ok = db.conn.execute("SELECT 1", []).is_ok();

    ApiResponse::success(serde_json::json!({
        "status": if db_ok { "healthy" } else { "degraded" },
        "version": env!("CARGO_PKG_VERSION"),
        "uptime": uptime,
        "checks": {
            "database": if db_ok { "ok" } else { "error" },
            "api": "ok"
        }
    }))
}

/// Get provider health status  
pub async fn get_provider_health() -> impl Responder {
    // In a real implementation, this would check actual provider connectivity
    ApiResponse::success(serde_json::json!([
        {
            "provider": "copilot",
            "status": "connected",
            "latency": 45,
            "lastCheck": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64
        },
        {
            "provider": "ollama",
            "status": "disconnected",
            "latency": null,
            "lastCheck": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64
        }
    ]))
}
