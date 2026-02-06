// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Workspace discovery and management

use crate::error::{CsmError, Result};
use crate::models::{SessionWithPath, Workspace, WorkspaceJson};
use crate::storage::{is_session_file_extension, parse_session_file};
use std::path::{Path, PathBuf};
use urlencoding::decode;

/// Type alias for workspace info tuple (hash, path, project_path, modified_time)
pub type WorkspaceInfo = (String, PathBuf, Option<String>, std::time::SystemTime);

/// Get the VS Code workspaceStorage path based on the operating system
pub fn get_workspace_storage_path() -> Result<PathBuf> {
    let path = if cfg!(target_os = "windows") {
        dirs::config_dir().map(|p| p.join("Code").join("User").join("workspaceStorage"))
    } else if cfg!(target_os = "macos") {
        dirs::home_dir().map(|p| p.join("Library/Application Support/Code/User/workspaceStorage"))
    } else {
        // Linux
        dirs::home_dir().map(|p| p.join(".config/Code/User/workspaceStorage"))
    };

    path.ok_or(CsmError::StorageNotFound)
}

/// Get the VS Code globalStorage path based on the operating system
pub fn get_global_storage_path() -> Result<PathBuf> {
    let path = if cfg!(target_os = "windows") {
        dirs::config_dir().map(|p| p.join("Code").join("User").join("globalStorage"))
    } else if cfg!(target_os = "macos") {
        dirs::home_dir().map(|p| p.join("Library/Application Support/Code/User/globalStorage"))
    } else {
        // Linux
        dirs::home_dir().map(|p| p.join(".config/Code/User/globalStorage"))
    };

    path.ok_or(CsmError::StorageNotFound)
}

/// Get the path to empty window chat sessions (ALL SESSIONS in VS Code)
/// These are chat sessions not tied to any specific workspace
pub fn get_empty_window_sessions_path() -> Result<PathBuf> {
    let global_storage = get_global_storage_path()?;
    Ok(global_storage.join("emptyWindowChatSessions"))
}

/// Decode a workspace folder URI to a path
pub fn decode_workspace_folder(folder_uri: &str) -> String {
    let mut folder = folder_uri.to_string();

    // Remove file:// prefix
    if folder.starts_with("file:///") {
        folder = folder[8..].to_string();
    } else if folder.starts_with("file://") {
        folder = folder[7..].to_string();
    }

    // URL decode
    if let Ok(decoded) = decode(&folder) {
        folder = decoded.into_owned();
    }

    // On Windows, convert forward slashes to backslashes
    if cfg!(target_os = "windows") {
        folder = folder.replace('/', "\\");
    }

    folder
}

/// Normalize a path for comparison
pub fn normalize_path(path: &str) -> String {
    let path = Path::new(path);
    if let Ok(canonical) = path.canonicalize() {
        canonical.to_string_lossy().to_lowercase()
    } else {
        // Fallback: lowercase and strip trailing slashes
        let normalized = path.to_string_lossy().to_lowercase();
        normalized.trim_end_matches(['/', '\\']).to_string()
    }
}

/// Discover all VS Code workspaces
pub fn discover_workspaces() -> Result<Vec<Workspace>> {
    let storage_path = get_workspace_storage_path()?;

    if !storage_path.exists() {
        return Ok(Vec::new());
    }

    let mut workspaces = Vec::new();

    for entry in std::fs::read_dir(&storage_path)? {
        let entry = entry?;
        let workspace_dir = entry.path();

        if !workspace_dir.is_dir() {
            continue;
        }

        let workspace_json_path = workspace_dir.join("workspace.json");
        if !workspace_json_path.exists() {
            continue;
        }

        // Parse workspace.json
        let project_path = match std::fs::read_to_string(&workspace_json_path) {
            Ok(content) => match serde_json::from_str::<WorkspaceJson>(&content) {
                Ok(ws_json) => ws_json.folder.map(|f| decode_workspace_folder(&f)),
                Err(_) => None,
            },
            Err(_) => None,
        };

        let chat_sessions_path = workspace_dir.join("chatSessions");
        let has_chat_sessions = chat_sessions_path.exists();

        let chat_session_count = if has_chat_sessions {
            std::fs::read_dir(&chat_sessions_path)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            e.path()
                                .extension()
                                .map(|ext| is_session_file_extension(ext))
                                .unwrap_or(false)
                        })
                        .count()
                })
                .unwrap_or(0)
        } else {
            0
        };

        // Get last modified time
        let last_modified = if has_chat_sessions {
            std::fs::read_dir(&chat_sessions_path)
                .ok()
                .and_then(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter_map(|e| e.metadata().ok())
                        .filter_map(|m| m.modified().ok())
                        .max()
                })
                .map(chrono::DateTime::<Utc>::from)
        } else {
            None
        };

        workspaces.push(Workspace {
            hash: entry.file_name().to_string_lossy().to_string(),
            project_path,
            workspace_path: workspace_dir.clone(),
            chat_sessions_path,
            chat_session_count,
            has_chat_sessions,
            last_modified,
        });
    }

    Ok(workspaces)
}

/// Find a workspace by its hash
pub fn get_workspace_by_hash(hash: &str) -> Result<Option<Workspace>> {
    let workspaces = discover_workspaces()?;
    Ok(workspaces
        .into_iter()
        .find(|w| w.hash == hash || w.hash.starts_with(hash)))
}

/// Find a workspace by project path
pub fn get_workspace_by_path(project_path: &str) -> Result<Option<Workspace>> {
    let workspaces = discover_workspaces()?;
    let target_path = normalize_path(project_path);

    Ok(workspaces.into_iter().find(|w| {
        w.project_path
            .as_ref()
            .map(|p| normalize_path(p) == target_path)
            .unwrap_or(false)
    }))
}

/// Find workspace by path, returning workspace ID, directory, and data.
/// When multiple workspaces match the same path, returns the most recently modified one.
pub fn find_workspace_by_path(
    project_path: &str,
) -> Result<Option<(String, PathBuf, Option<String>)>> {
    let storage_path = get_workspace_storage_path()?;

    if !storage_path.exists() {
        return Ok(None);
    }

    let target_path = normalize_path(project_path);
    let mut matches: Vec<(String, PathBuf, Option<String>, std::time::SystemTime)> = Vec::new();

    for entry in std::fs::read_dir(&storage_path)? {
        let entry = entry?;
        let workspace_dir = entry.path();

        if !workspace_dir.is_dir() {
            continue;
        }

        let workspace_json_path = workspace_dir.join("workspace.json");
        if !workspace_json_path.exists() {
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(&workspace_json_path) {
            if let Ok(ws_json) = serde_json::from_str::<WorkspaceJson>(&content) {
                if let Some(folder) = &ws_json.folder {
                    let folder_path = decode_workspace_folder(folder);
                    if normalize_path(&folder_path) == target_path {
                        // Get the most recent modification time from chatSessions or workspace dir
                        let chat_sessions_dir = workspace_dir.join("chatSessions");
                        let last_modified = if chat_sessions_dir.exists() {
                            std::fs::read_dir(&chat_sessions_dir)
                                .ok()
                                .and_then(|entries| {
                                    entries
                                        .filter_map(|e| e.ok())
                                        .filter_map(|e| e.metadata().ok())
                                        .filter_map(|m| m.modified().ok())
                                        .max()
                                })
                                .unwrap_or_else(|| {
                                    chat_sessions_dir
                                        .metadata()
                                        .and_then(|m| m.modified())
                                        .unwrap_or(std::time::UNIX_EPOCH)
                                })
                        } else {
                            workspace_dir
                                .metadata()
                                .and_then(|m| m.modified())
                                .unwrap_or(std::time::UNIX_EPOCH)
                        };

                        matches.push((
                            entry.file_name().to_string_lossy().to_string(),
                            workspace_dir,
                            Some(folder_path),
                            last_modified,
                        ));
                    }
                }
            }
        }
    }

    // Sort by last modified (newest first) and return the most recent
    matches.sort_by(|a, b| b.3.cmp(&a.3));

    Ok(matches
        .into_iter()
        .next()
        .map(|(id, path, folder, _)| (id, path, folder)))
}

/// Find all workspaces for a project (by name matching)
pub fn find_all_workspaces_for_project(project_name: &str) -> Result<Vec<WorkspaceInfo>> {
    let storage_path = get_workspace_storage_path()?;

    if !storage_path.exists() {
        return Ok(Vec::new());
    }

    let project_name_lower = project_name.to_lowercase();
    let mut workspaces = Vec::new();

    for entry in std::fs::read_dir(&storage_path)? {
        let entry = entry?;
        let workspace_dir = entry.path();

        if !workspace_dir.is_dir() {
            continue;
        }

        let workspace_json_path = workspace_dir.join("workspace.json");
        if !workspace_json_path.exists() {
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(&workspace_json_path) {
            if let Ok(ws_json) = serde_json::from_str::<WorkspaceJson>(&content) {
                if let Some(folder) = &ws_json.folder {
                    let folder_path = decode_workspace_folder(folder);

                    if folder_path.to_lowercase().contains(&project_name_lower) {
                        let chat_sessions_dir = workspace_dir.join("chatSessions");

                        let last_modified = if chat_sessions_dir.exists() {
                            std::fs::read_dir(&chat_sessions_dir)
                                .ok()
                                .and_then(|entries| {
                                    entries
                                        .filter_map(|e| e.ok())
                                        .filter_map(|e| e.metadata().ok())
                                        .filter_map(|m| m.modified().ok())
                                        .max()
                                })
                                .unwrap_or_else(|| {
                                    chat_sessions_dir
                                        .metadata()
                                        .and_then(|m| m.modified())
                                        .unwrap_or(std::time::UNIX_EPOCH)
                                })
                        } else {
                            workspace_dir
                                .metadata()
                                .and_then(|m| m.modified())
                                .unwrap_or(std::time::UNIX_EPOCH)
                        };

                        workspaces.push((
                            entry.file_name().to_string_lossy().to_string(),
                            workspace_dir,
                            Some(folder_path),
                            last_modified,
                        ));
                    }
                }
            }
        }
    }

    // Sort by last modified (newest first)
    workspaces.sort_by(|a, b| b.3.cmp(&a.3));

    Ok(workspaces)
}

/// Get all chat sessions from a workspace directory
pub fn get_chat_sessions_from_workspace(workspace_dir: &Path) -> Result<Vec<SessionWithPath>> {
    let chat_sessions_dir = workspace_dir.join("chatSessions");

    if !chat_sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    for entry in std::fs::read_dir(&chat_sessions_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path
            .extension()
            .map(|e| is_session_file_extension(e))
            .unwrap_or(false)
        {
            if let Ok(session) = parse_session_file(&path) {
                sessions.push(SessionWithPath { path, session });
            }
        }
    }

    Ok(sessions)
}

use chrono::Utc;
