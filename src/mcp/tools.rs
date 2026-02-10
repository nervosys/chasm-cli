// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! MCP Tools - Expose csm functionality as MCP tools

#![allow(dead_code, unused_imports)]

use super::types::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

/// Workspace info for JSON serialization (local type to avoid conflict)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpWorkspaceInfo {
    hash: String,
    project_path: String,
    session_count: usize,
    has_chats: bool,
}

/// Get the list of available tools
pub fn list_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "csm_list_workspaces".to_string(),
            description: Some("List all VS Code workspaces with chat sessions".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "csm_find_workspace".to_string(),
            description: Some(
                "Find workspaces matching a pattern (project name or path)".to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Search pattern to match workspace names or paths"
                    }
                },
                "required": ["pattern"]
            }),
        },
        Tool {
            name: "csm_list_sessions".to_string(),
            description: Some(
                "List all chat sessions, optionally filtered by project path".to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": {
                        "type": "string",
                        "description": "Optional project path to filter sessions"
                    }
                },
                "required": []
            }),
        },
        Tool {
            name: "csm_list_orphaned".to_string(),
            description: Some(
                "List sessions on disk that are not in VS Code's index (invisible sessions)"
                    .to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Project path (defaults to current directory)"
                    }
                },
                "required": []
            }),
        },
        Tool {
            name: "csm_register_all".to_string(),
            description: Some(
                "Register all sessions from a workspace into VS Code's index".to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Project path (defaults to current directory)"
                    },
                    "merge": {
                        "type": "boolean",
                        "description": "Merge all sessions into one before registering"
                    },
                    "force": {
                        "type": "boolean",
                        "description": "Force registration even if VS Code is running"
                    }
                },
                "required": []
            }),
        },
        Tool {
            name: "csm_register_sessions".to_string(),
            description: Some("Register specific sessions by ID or title".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Session IDs to register"
                    },
                    "titles": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Session titles to match (partial match)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Project path (defaults to current directory)"
                    },
                    "force": {
                        "type": "boolean",
                        "description": "Force registration even if VS Code is running"
                    }
                },
                "required": []
            }),
        },
        Tool {
            name: "csm_show_session".to_string(),
            description: Some("Show details of a specific chat session".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID (supports partial prefix match)"
                    }
                },
                "required": ["session_id"]
            }),
        },
        Tool {
            name: "csm_show_history".to_string(),
            description: Some("Show chat history timeline for a project".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Project path (defaults to current directory)"
                    }
                },
                "required": []
            }),
        },
        Tool {
            name: "csm_merge_sessions".to_string(),
            description: Some("Merge multiple chat sessions into one unified history".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Project path to merge sessions from"
                    },
                    "title": {
                        "type": "string",
                        "description": "Title for the merged session"
                    },
                    "force": {
                        "type": "boolean",
                        "description": "Force merge even if VS Code is running"
                    }
                },
                "required": []
            }),
        },
        Tool {
            name: "csm_search".to_string(),
            description: Some("Full-text search across all harvested chat sessions".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results (default: 20)"
                    }
                },
                "required": ["query"]
            }),
        },
        Tool {
            name: "csm_detect".to_string(),
            description: Some(
                "Auto-detect workspace and available providers for a path".to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to detect workspace for"
                    }
                },
                "required": []
            }),
        },
        // CSM Database Tools (for csm-web integration)
        Tool {
            name: "csm_db_list_workspaces".to_string(),
            description: Some("List all workspaces from the CSM database (csm-web)".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "csm_db_list_sessions".to_string(),
            description: Some("List chat sessions from the CSM database (csm-web)".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "workspace_id": {
                        "type": "string",
                        "description": "Filter by workspace ID"
                    },
                    "provider": {
                        "type": "string",
                        "description": "Filter by provider (e.g., 'copilot', 'ollama', 'chatgpt')"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results (default: 100)"
                    }
                },
                "required": []
            }),
        },
        Tool {
            name: "csm_db_get_session".to_string(),
            description: Some(
                "Get a specific session with all its messages from CSM database".to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID to retrieve"
                    }
                },
                "required": ["session_id"]
            }),
        },
        Tool {
            name: "csm_db_search".to_string(),
            description: Some("Search sessions in CSM database by title".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query for session titles"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results (default: 20)"
                    }
                },
                "required": ["query"]
            }),
        },
        Tool {
            name: "csm_db_stats".to_string(),
            description: Some(
                "Get statistics about the CSM database (session counts by provider)".to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
    ]
}

/// Execute a tool call
pub fn call_tool(name: &str, arguments: &HashMap<String, serde_json::Value>) -> CallToolResult {
    let result = match name {
        "csm_list_workspaces" => execute_list_workspaces(),
        "csm_find_workspace" => {
            let pattern = arguments
                .get("pattern")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            execute_find_workspace(pattern)
        }
        "csm_list_sessions" => {
            let project_path = arguments.get("project_path").and_then(|v| v.as_str());
            execute_list_sessions(project_path)
        }
        "csm_list_orphaned" => {
            let path = arguments.get("path").and_then(|v| v.as_str());
            execute_list_orphaned(path)
        }
        "csm_register_all" => {
            let path = arguments.get("path").and_then(|v| v.as_str());
            let merge = arguments
                .get("merge")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let force = arguments
                .get("force")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            execute_register_all(path, merge, force)
        }
        "csm_register_sessions" => {
            let ids: Vec<String> = arguments
                .get("ids")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let titles: Option<Vec<String>> = arguments
                .get("titles")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                });
            let path = arguments.get("path").and_then(|v| v.as_str());
            let force = arguments
                .get("force")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            execute_register_sessions(&ids, titles.as_deref(), path, force)
        }
        "csm_show_session" => {
            let session_id = arguments
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            execute_show_session(session_id)
        }
        "csm_show_history" => {
            let path = arguments.get("path").and_then(|v| v.as_str());
            execute_show_history(path)
        }
        "csm_merge_sessions" => {
            let path = arguments.get("path").and_then(|v| v.as_str());
            let title = arguments.get("title").and_then(|v| v.as_str());
            let force = arguments
                .get("force")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            execute_merge_sessions(path, title, force)
        }
        "csm_search" => {
            let query = arguments
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let limit = arguments
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            execute_search(query, limit)
        }
        "csm_detect" => {
            let path = arguments.get("path").and_then(|v| v.as_str());
            execute_detect(path)
        }
        // CSM Database tools (csm-web integration)
        "csm_db_list_workspaces" => execute_db_list_workspaces(),
        "csm_db_list_sessions" => {
            let workspace_id = arguments.get("workspace_id").and_then(|v| v.as_str());
            let provider = arguments.get("provider").and_then(|v| v.as_str());
            let limit = arguments
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize)
                .unwrap_or(100);
            execute_db_list_sessions(workspace_id, provider, limit)
        }
        "csm_db_get_session" => {
            let session_id = arguments
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            execute_db_get_session(session_id)
        }
        "csm_db_search" => {
            let query = arguments
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let limit = arguments
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize)
                .unwrap_or(20);
            execute_db_search(query, limit)
        }
        "csm_db_stats" => execute_db_stats(),
        _ => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Unknown tool: {}", name),
            }],
            is_error: Some(true),
        },
    };

    result
}

// ============================================================================
// Tool Implementations
// ============================================================================

fn execute_list_workspaces() -> CallToolResult {
    use crate::workspace::discover_workspaces;

    match discover_workspaces() {
        Ok(workspaces) => {
            let infos: Vec<McpWorkspaceInfo> = workspaces
                .iter()
                .map(|ws| McpWorkspaceInfo {
                    hash: ws.hash.clone(),
                    project_path: ws.project_path.clone().unwrap_or_default(),
                    session_count: ws.chat_session_count,
                    has_chats: ws.has_chat_sessions,
                })
                .collect();

            CallToolResult {
                content: vec![ToolContent::Text {
                    text: serde_json::to_string_pretty(&json!({
                        "workspaces": infos,
                        "total": infos.len()
                    }))
                    .unwrap_or_default(),
                }],
                is_error: None,
            }
        }
        Err(e) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Error listing workspaces: {}", e),
            }],
            is_error: Some(true),
        },
    }
}

fn execute_find_workspace(pattern: &str) -> CallToolResult {
    use crate::workspace::{find_all_workspaces_for_project, get_chat_sessions_from_workspace};

    match find_all_workspaces_for_project(pattern) {
        Ok(workspaces) => {
            let infos: Vec<serde_json::Value> = workspaces
                .iter()
                .map(|(hash, workspace_path, project_path, _last_modified)| {
                    let sessions = get_chat_sessions_from_workspace(workspace_path)
                        .unwrap_or_default()
                        .iter()
                        .map(|s| {
                            json!({
                                "id": s.session.session_id,
                                "title": s.session.title(),
                                "path": s.path.display().to_string()
                            })
                        })
                        .collect::<Vec<_>>();
                    json!({
                        "hash": hash,
                        "project_path": project_path,
                        "workspace_path": workspace_path.display().to_string(),
                        "session_count": sessions.len(),
                        "sessions": sessions
                    })
                })
                .collect();

            CallToolResult {
                content: vec![ToolContent::Text {
                    text: serde_json::to_string_pretty(&json!({
                        "pattern": pattern,
                        "workspaces": infos,
                        "total": infos.len()
                    }))
                    .unwrap_or_default(),
                }],
                is_error: None,
            }
        }
        Err(e) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Error finding workspaces: {}", e),
            }],
            is_error: Some(true),
        },
    }
}

fn execute_list_sessions(project_path: Option<&str>) -> CallToolResult {
    use crate::workspace::{discover_workspaces, get_chat_sessions_from_workspace};

    match discover_workspaces() {
        Ok(workspaces) => {
            let mut all_sessions = Vec::new();

            for ws in &workspaces {
                // Filter by project path if specified
                if let Some(filter_path) = project_path {
                    if let Some(ref ws_path) = ws.project_path {
                        if !ws_path.to_lowercase().contains(&filter_path.to_lowercase()) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                if let Ok(sessions) = get_chat_sessions_from_workspace(&ws.workspace_path) {
                    for s in sessions {
                        all_sessions.push(json!({
                            "id": s.session.session_id,
                            "title": s.session.title(),
                            "message_count": s.session.requests.len(),
                            "workspace_hash": ws.hash,
                            "project_path": ws.project_path,
                            "file_path": s.path.display().to_string()
                        }));
                    }
                }
            }

            CallToolResult {
                content: vec![ToolContent::Text {
                    text: serde_json::to_string_pretty(&json!({
                        "sessions": all_sessions,
                        "total": all_sessions.len()
                    }))
                    .unwrap_or_default(),
                }],
                is_error: None,
            }
        }
        Err(e) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Error listing sessions: {}", e),
            }],
            is_error: Some(true),
        },
    }
}

fn execute_list_orphaned(path: Option<&str>) -> CallToolResult {
    use crate::commands::list_orphaned;

    // Capture stdout (list_orphaned prints to stdout)
    // For now, we'll call it and return a simplified response
    match list_orphaned(path) {
        Ok(_) => CallToolResult {
            content: vec![ToolContent::Text {
                text: json!({
                    "status": "success",
                    "message": "Orphaned session check completed. See console output for details."
                })
                .to_string(),
            }],
            is_error: None,
        },
        Err(e) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Error listing orphaned sessions: {}", e),
            }],
            is_error: Some(true),
        },
    }
}

fn execute_register_all(path: Option<&str>, merge: bool, force: bool) -> CallToolResult {
    use crate::commands::register_all;

    match register_all(path, merge, force) {
        Ok(_) => CallToolResult {
            content: vec![ToolContent::Text {
                text: json!({
                    "status": "success",
                    "message": "Sessions registered successfully",
                    "merge": merge,
                    "force": force
                })
                .to_string(),
            }],
            is_error: None,
        },
        Err(e) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Error registering sessions: {}", e),
            }],
            is_error: Some(true),
        },
    }
}

fn execute_register_sessions(
    ids: &[String],
    titles: Option<&[String]>,
    path: Option<&str>,
    force: bool,
) -> CallToolResult {
    use crate::commands::register_sessions;

    match register_sessions(ids, titles, path, force) {
        Ok(_) => CallToolResult {
            content: vec![ToolContent::Text {
                text: json!({
                    "status": "success",
                    "message": "Sessions registered successfully",
                    "ids": ids,
                    "titles": titles,
                    "force": force
                })
                .to_string(),
            }],
            is_error: None,
        },
        Err(e) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Error registering sessions: {}", e),
            }],
            is_error: Some(true),
        },
    }
}

fn execute_show_session(session_id: &str) -> CallToolResult {
    use crate::workspace::{discover_workspaces, get_chat_sessions_from_workspace};

    match discover_workspaces() {
        Ok(workspaces) => {
            for ws in &workspaces {
                if let Ok(sessions) = get_chat_sessions_from_workspace(&ws.workspace_path) {
                    for s in sessions {
                        let sid = s.session.session_id.clone().unwrap_or_default();
                        if sid.starts_with(session_id) || sid == session_id {
                            return CallToolResult {
                                content: vec![ToolContent::Text {
                                    text: serde_json::to_string_pretty(&json!({
                                        "id": sid,
                                        "title": s.session.title(),
                                        "message_count": s.session.requests.len(),
                                        "last_message_date": s.session.last_message_date,
                                        "is_imported": s.session.is_imported,
                                        "workspace_hash": ws.hash,
                                        "project_path": ws.project_path,
                                        "file_path": s.path.display().to_string(),
                                        "messages": s.session.requests.iter().take(10).map(|r| {
                                            let user_msg = r.message.as_ref()
                                                .map(|m| m.get_text())
                                                .unwrap_or_default();
                                            let response_text = r.response.as_ref()
                                                .and_then(|v| v.get("text"))
                                                .and_then(|t| t.as_str())
                                                .unwrap_or("");
                                            json!({
                                                "message": user_msg,
                                                "response": response_text
                                            })
                                        }).collect::<Vec<_>>()
                                    }))
                                    .unwrap_or_default(),
                                }],
                                is_error: None,
                            };
                        }
                    }
                }
            }

            CallToolResult {
                content: vec![ToolContent::Text {
                    text: format!("Session not found: {}", session_id),
                }],
                is_error: Some(true),
            }
        }
        Err(e) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Error finding session: {}", e),
            }],
            is_error: Some(true),
        },
    }
}

fn execute_show_history(path: Option<&str>) -> CallToolResult {
    use crate::commands::history_show;

    match history_show(path) {
        Ok(_) => CallToolResult {
            content: vec![ToolContent::Text {
                text: json!({
                    "status": "success",
                    "message": "History displayed. See console output for details."
                })
                .to_string(),
            }],
            is_error: None,
        },
        Err(e) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Error showing history: {}", e),
            }],
            is_error: Some(true),
        },
    }
}

fn execute_merge_sessions(path: Option<&str>, title: Option<&str>, force: bool) -> CallToolResult {
    use crate::commands::history_merge;

    match history_merge(path, title, force, false) {
        Ok(_) => CallToolResult {
            content: vec![ToolContent::Text {
                text: json!({
                    "status": "success",
                    "message": "Sessions merged successfully",
                    "title": title,
                    "force": force
                })
                .to_string(),
            }],
            is_error: None,
        },
        Err(e) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Error merging sessions: {}", e),
            }],
            is_error: Some(true),
        },
    }
}

fn execute_search(query: &str, limit: Option<usize>) -> CallToolResult {
    use crate::commands::harvest_search;

    let limit = limit.unwrap_or(20);

    match harvest_search(None, query, None, limit) {
        Ok(_) => CallToolResult {
            content: vec![ToolContent::Text {
                text: json!({
                    "status": "success",
                    "query": query,
                    "limit": limit,
                    "message": "Search completed. See console output for results."
                })
                .to_string(),
            }],
            is_error: None,
        },
        Err(e) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Error searching: {}", e),
            }],
            is_error: Some(true),
        },
    }
}

fn execute_detect(path: Option<&str>) -> CallToolResult {
    use crate::commands::detect_all;

    match detect_all(path, false) {
        Ok(_) => CallToolResult {
            content: vec![ToolContent::Text {
                text: json!({
                    "status": "success",
                    "message": "Detection completed. See console output for details."
                })
                .to_string(),
            }],
            is_error: None,
        },
        Err(e) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Error detecting: {}", e),
            }],
            is_error: Some(true),
        },
    }
}

// ============================================================================
// CSM Database Tool Implementations (csm-web integration)
// ============================================================================

fn execute_db_list_workspaces() -> CallToolResult {
    use super::db;

    if !db::csm_db_exists() {
        return CallToolResult {
            content: vec![ToolContent::Text {
                text: json!({
                    "error": "CSM database not found",
                    "message": "The csm-web database has not been initialized. Run 'csm api' to start the API server first.",
                    "db_path": db::get_csm_db_path().display().to_string()
                }).to_string(),
            }],
            is_error: Some(true),
        };
    }

    match db::list_db_workspaces() {
        Ok(workspaces) => {
            let infos: Vec<serde_json::Value> = workspaces
                .iter()
                .map(|ws| {
                    json!({
                        "id": ws.id,
                        "name": ws.name,
                        "path": ws.path,
                        "provider": ws.provider,
                        "created_at": ws.created_at,
                        "updated_at": ws.updated_at
                    })
                })
                .collect();

            CallToolResult {
                content: vec![ToolContent::Text {
                    text: serde_json::to_string_pretty(&json!({
                        "workspaces": infos,
                        "total": infos.len(),
                        "source": "csm-web database"
                    }))
                    .unwrap_or_default(),
                }],
                is_error: None,
            }
        }
        Err(e) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Error listing workspaces from CSM database: {}", e),
            }],
            is_error: Some(true),
        },
    }
}

fn execute_db_list_sessions(
    workspace_id: Option<&str>,
    provider: Option<&str>,
    limit: usize,
) -> CallToolResult {
    use super::db;

    if !db::csm_db_exists() {
        return CallToolResult {
            content: vec![ToolContent::Text {
                text: json!({
                    "error": "CSM database not found",
                    "message": "The csm-web database has not been initialized."
                })
                .to_string(),
            }],
            is_error: Some(true),
        };
    }

    match db::list_db_sessions(workspace_id, provider, limit) {
        Ok(sessions) => {
            let infos: Vec<serde_json::Value> = sessions
                .iter()
                .map(|s| {
                    json!({
                        "id": s.id,
                        "workspace_id": s.workspace_id,
                        "provider": s.provider,
                        "title": s.title,
                        "model": s.model,
                        "message_count": s.message_count,
                        "created_at": s.created_at,
                        "updated_at": s.updated_at,
                        "archived": s.archived
                    })
                })
                .collect();

            CallToolResult {
                content: vec![ToolContent::Text {
                    text: serde_json::to_string_pretty(&json!({
                        "sessions": infos,
                        "total": infos.len(),
                        "filters": {
                            "workspace_id": workspace_id,
                            "provider": provider,
                            "limit": limit
                        },
                        "source": "csm-web database"
                    }))
                    .unwrap_or_default(),
                }],
                is_error: None,
            }
        }
        Err(e) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Error listing sessions from CSM database: {}", e),
            }],
            is_error: Some(true),
        },
    }
}

fn execute_db_get_session(session_id: &str) -> CallToolResult {
    use super::db;

    if !db::csm_db_exists() {
        return CallToolResult {
            content: vec![ToolContent::Text {
                text: json!({
                    "error": "CSM database not found"
                })
                .to_string(),
            }],
            is_error: Some(true),
        };
    }

    match db::get_db_session(session_id) {
        Ok(Some(session)) => {
            // Also fetch messages
            let messages = db::get_db_messages(session_id).unwrap_or_default();

            let message_infos: Vec<serde_json::Value> = messages
                .iter()
                .map(|m| {
                    json!({
                        "id": m.id,
                        "role": m.role,
                        "content": m.content,
                        "model": m.model,
                        "created_at": m.created_at
                    })
                })
                .collect();

            CallToolResult {
                content: vec![ToolContent::Text {
                    text: serde_json::to_string_pretty(&json!({
                        "session": {
                            "id": session.id,
                            "workspace_id": session.workspace_id,
                            "provider": session.provider,
                            "title": session.title,
                            "model": session.model,
                            "message_count": session.message_count,
                            "created_at": session.created_at,
                            "updated_at": session.updated_at,
                            "archived": session.archived
                        },
                        "messages": message_infos,
                        "source": "csm-web database"
                    }))
                    .unwrap_or_default(),
                }],
                is_error: None,
            }
        }
        Ok(None) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Session not found: {}", session_id),
            }],
            is_error: Some(true),
        },
        Err(e) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Error getting session: {}", e),
            }],
            is_error: Some(true),
        },
    }
}

fn execute_db_search(query: &str, limit: usize) -> CallToolResult {
    use super::db;

    if !db::csm_db_exists() {
        return CallToolResult {
            content: vec![ToolContent::Text {
                text: json!({
                    "error": "CSM database not found"
                })
                .to_string(),
            }],
            is_error: Some(true),
        };
    }

    match db::search_db_sessions(query, limit) {
        Ok(sessions) => {
            let infos: Vec<serde_json::Value> = sessions
                .iter()
                .map(|s| {
                    json!({
                        "id": s.id,
                        "title": s.title,
                        "provider": s.provider,
                        "message_count": s.message_count,
                        "updated_at": s.updated_at
                    })
                })
                .collect();

            CallToolResult {
                content: vec![ToolContent::Text {
                    text: serde_json::to_string_pretty(&json!({
                        "query": query,
                        "results": infos,
                        "total": infos.len(),
                        "source": "csm-web database"
                    }))
                    .unwrap_or_default(),
                }],
                is_error: None,
            }
        }
        Err(e) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Error searching: {}", e),
            }],
            is_error: Some(true),
        },
    }
}

fn execute_db_stats() -> CallToolResult {
    use super::db;

    if !db::csm_db_exists() {
        return CallToolResult {
            content: vec![ToolContent::Text {
                text: json!({
                    "error": "CSM database not found",
                    "db_path": db::get_csm_db_path().display().to_string()
                })
                .to_string(),
            }],
            is_error: Some(true),
        };
    }

    match db::count_sessions_by_provider() {
        Ok(counts) => {
            let provider_counts: serde_json::Value = counts
                .iter()
                .map(|(provider, count)| (provider.clone(), *count))
                .collect();

            let total: i64 = counts.iter().map(|(_, c)| c).sum();

            CallToolResult {
                content: vec![ToolContent::Text {
                    text: serde_json::to_string_pretty(&json!({
                        "total_sessions": total,
                        "by_provider": provider_counts,
                        "db_path": db::get_csm_db_path().display().to_string(),
                        "source": "csm-web database"
                    }))
                    .unwrap_or_default(),
                }],
                is_error: None,
            }
        }
        Err(e) => CallToolResult {
            content: vec![ToolContent::Text {
                text: format!("Error getting stats: {}", e),
            }],
            is_error: Some(true),
        },
    }
}
