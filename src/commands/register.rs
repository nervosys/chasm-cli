// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Register commands - Add sessions to VS Code's session index
//!
//! VS Code only displays sessions that are registered in the `chat.ChatSessionStore.index`
//! stored in `state.vscdb`. Sessions can exist on disk but be invisible to VS Code if
//! they're not in this index. These commands help register orphaned sessions.

use anyhow::Result;
use colored::*;
use std::collections::HashSet;
use std::path::PathBuf;

use crate::error::CsmError;
use crate::models::ChatSession;
use crate::storage::{
    add_session_to_index, get_workspace_storage_db, is_vscode_running, parse_session_json,
    read_chat_session_index, register_all_sessions_from_directory,
};
use crate::workspace::find_workspace_by_path;

/// Resolve a path option to an absolute PathBuf, handling "." and relative paths
fn resolve_path(path: Option<&str>) -> PathBuf {
    match path {
        Some(p) => {
            let path = PathBuf::from(p);
            path.canonicalize().unwrap_or(path)
        }
        None => std::env::current_dir().unwrap_or_default(),
    }
}


/// Register all sessions from a workspace into VS Code's index
pub fn register_all(project_path: Option<&str>, merge: bool, force: bool) -> Result<()> {
    let path = resolve_path(project_path);

    if merge {
        println!(
            "{} Merging and registering all sessions for: {}",
            "[CSM]".cyan().bold(),
            path.display()
        );

        // Use the existing merge functionality
        let path_str = path.to_string_lossy().to_string();
        return crate::commands::history_merge(
            Some(&path_str),
            None,  // title
            force, // force
            false, // no_backup
        );
    }

    println!(
        "{} Registering all sessions for: {}",
        "[CSM]".cyan().bold(),
        path.display()
    );

    // Find the workspace
    let path_str = path.to_string_lossy().to_string();
    let (ws_id, ws_path, _folder) = find_workspace_by_path(&path_str)?
        .ok_or_else(|| CsmError::WorkspaceNotFound(path.display().to_string()))?;

    let chat_sessions_dir = ws_path.join("chatSessions");

    if !chat_sessions_dir.exists() {
        println!(
            "{} No chatSessions directory found at: {}",
            "[!]".yellow(),
            chat_sessions_dir.display()
        );
        return Ok(());
    }

    // Check if VS Code is running
    if !force && is_vscode_running() {
        println!(
            "{} VS Code is running. Use {} to register anyway.",
            "[!]".yellow(),
            "--force".cyan()
        );
        println!("   Note: VS Code uses WAL mode so this is generally safe.");
        return Err(CsmError::VSCodeRunning.into());
    }

    // Count sessions on disk
    let sessions_on_disk = count_sessions_in_directory(&chat_sessions_dir)?;
    println!(
        "   Found {} session files on disk",
        sessions_on_disk.to_string().green()
    );

    // Register all sessions
    let registered = register_all_sessions_from_directory(&ws_id, &chat_sessions_dir, force)?;

    println!(
        "\n{} Registered {} sessions in VS Code's index",
        "[OK]".green().bold(),
        registered.to_string().cyan()
    );

    // Always show reload instructions since VS Code caches the index
    println!(
        "\n{} VS Code caches the session index in memory.",
        "[!]".yellow()
    );
    println!("   To see the new sessions, do one of the following:");
    println!(
        "   * Run: {} (if CSM extension is installed)",
        "code --command csm.reloadAndShowChats".cyan()
    );
    println!(
        "   * Or press {} in VS Code and run {}",
        "Ctrl+Shift+P".cyan(),
        "Developer: Reload Window".cyan()
    );
    println!("   * Or restart VS Code");

    Ok(())
}

/// Register specific sessions by ID or title
pub fn register_sessions(
    ids: &[String],
    titles: Option<&[String]>,
    project_path: Option<&str>,
    force: bool,
) -> Result<()> {
    let path = resolve_path(project_path);

    // Find the workspace
    let path_str = path.to_string_lossy().to_string();
    let (ws_id, ws_path, _folder) = find_workspace_by_path(&path_str)?
        .ok_or_else(|| CsmError::WorkspaceNotFound(path.display().to_string()))?;

    let chat_sessions_dir = ws_path.join("chatSessions");

    // Check if VS Code is running
    if !force && is_vscode_running() {
        println!(
            "{} VS Code is running. Use {} to register anyway.",
            "[!]".yellow(),
            "--force".cyan()
        );
        return Err(CsmError::VSCodeRunning.into());
    }

    // Get the database path
    let db_path = get_workspace_storage_db(&ws_id)?;

    let mut registered_count = 0;

    if let Some(titles) = titles {
        // Register by title
        println!(
            "{} Registering {} sessions by title:",
            "[CSM]".cyan().bold(),
            titles.len()
        );

        let sessions = find_sessions_by_titles(&chat_sessions_dir, titles)?;

        for (session, session_path) in sessions {
            let session_id = session.session_id.clone().unwrap_or_else(|| {
                session_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default()
            });
            let title = session.title();

            add_session_to_index(
                &db_path,
                &session_id,
                &title,
                session.last_message_date,
                session.is_imported,
                &session.initial_location,
                session.is_empty(),
            )?;

            let id_display = if session_id.len() > 12 {
                &session_id[..12]
            } else {
                &session_id
            };
            println!(
                "   {} {} (\"{}\")",
                "[OK]".green(),
                id_display.cyan(),
                title.yellow()
            );
            registered_count += 1;
        }
    } else {
        // Register by ID (default)
        println!(
            "{} Registering {} sessions by ID:",
            "[CSM]".cyan().bold(),
            ids.len()
        );

        for session_id in ids {
            match find_session_file(&chat_sessions_dir, session_id) {
                Ok(session_file) => {
                    let content = std::fs::read_to_string(&session_file)?;
                    let session: ChatSession = serde_json::from_str(&content)?;

                    let title = session.title();
                    let actual_session_id = session
                        .session_id
                        .clone()
                        .unwrap_or_else(|| session_id.to_string());

                    add_session_to_index(
                        &db_path,
                        &actual_session_id,
                        &title,
                        session.last_message_date,
                        session.is_imported,
                        &session.initial_location,
                        session.is_empty(),
                    )?;

                    let id_display = if actual_session_id.len() > 12 {
                        &actual_session_id[..12]
                    } else {
                        &actual_session_id
                    };
                    println!(
                        "   {} {} (\"{}\")",
                        "[OK]".green(),
                        id_display.cyan(),
                        title.yellow()
                    );
                    registered_count += 1;
                }
                Err(e) => {
                    println!(
                        "   {} {} - {}",
                        "[ERR]".red(),
                        session_id.cyan(),
                        e.to_string().red()
                    );
                }
            }
        }
    }

    println!(
        "\n{} Registered {} sessions in VS Code's index",
        "[OK]".green().bold(),
        registered_count.to_string().cyan()
    );

    if force && is_vscode_running() {
        println!(
            "   {} Sessions should appear in VS Code immediately",
            "->".cyan()
        );
    }

    Ok(())
}

/// List sessions that exist on disk but are not in VS Code's index
pub fn list_orphaned(project_path: Option<&str>) -> Result<()> {
    let path = resolve_path(project_path);

    println!(
        "{} Finding orphaned sessions for: {}",
        "[CSM]".cyan().bold(),
        path.display()
    );

    // Find the workspace
    let path_str = path.to_string_lossy().to_string();
    let (ws_id, ws_path, _folder) = find_workspace_by_path(&path_str)?
        .ok_or_else(|| CsmError::WorkspaceNotFound(path.display().to_string()))?;

    let chat_sessions_dir = ws_path.join("chatSessions");

    if !chat_sessions_dir.exists() {
        println!("{} No chatSessions directory found", "[!]".yellow());
        return Ok(());
    }

    // Get sessions currently in the index
    let db_path = get_workspace_storage_db(&ws_id)?;
    let index = read_chat_session_index(&db_path)?;
    let indexed_ids: HashSet<String> = index.entries.keys().cloned().collect();

    println!(
        "   {} sessions currently in VS Code's index",
        indexed_ids.len().to_string().cyan()
    );

    // Find sessions on disk
    let mut orphaned_sessions = Vec::new();

    for entry in std::fs::read_dir(&chat_sessions_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(session) = parse_session_json(&content) {
                    let session_id = session.session_id.clone().unwrap_or_else(|| {
                        path.file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default()
                    });

                    if !indexed_ids.contains(&session_id) {
                        let title = session.title();
                        let msg_count = session.requests.len();
                        orphaned_sessions.push((session_id, title, msg_count, path.clone()));
                    }
                }
            }
        }
    }

    if orphaned_sessions.is_empty() {
        println!(
            "\n{} No orphaned sessions found - all sessions are registered!",
            "[OK]".green().bold()
        );
        return Ok(());
    }

    println!(
        "\n{} Found {} orphaned sessions (on disk but not in index):\n",
        "[!]".yellow().bold(),
        orphaned_sessions.len().to_string().red()
    );

    for (session_id, title, msg_count, _path) in &orphaned_sessions {
        let id_display = if session_id.len() > 12 {
            &session_id[..12]
        } else {
            session_id
        };
        println!(
            "   {} {} ({} messages)",
            id_display.cyan(),
            format!("\"{}\"", title).yellow(),
            msg_count
        );
    }

    println!("\n{} To register all orphaned sessions:", "->".cyan());
    println!("   csm register all --force");
    println!("\n{} To register specific sessions:", "->".cyan());
    println!("   csm register session <ID1> <ID2> ... --force");

    Ok(())
}

/// Count session files in a directory
fn count_sessions_in_directory(dir: &PathBuf) -> Result<usize> {
    let mut count = 0;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry
            .path()
            .extension()
            .map(|e| e == "json")
            .unwrap_or(false)
        {
            count += 1;
        }
    }
    Ok(count)
}

/// Find a session file by ID (supports partial matches)
fn find_session_file(chat_sessions_dir: &PathBuf, session_id: &str) -> Result<PathBuf> {
    // First try exact match
    let exact_path = chat_sessions_dir.join(format!("{}.json", session_id));
    if exact_path.exists() {
        return Ok(exact_path);
    }

    // Try partial match (prefix)
    for entry in std::fs::read_dir(chat_sessions_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let filename = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            if filename.starts_with(session_id) {
                return Ok(path);
            }

            // Also check session_id inside the file
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(session) = parse_session_json(&content) {
                    if let Some(ref sid) = session.session_id {
                        if sid.starts_with(session_id) || sid == session_id {
                            return Ok(path);
                        }
                    }
                }
            }
        }
    }

    Err(CsmError::SessionNotFound(session_id.to_string()).into())
}

/// Find sessions by title (case-insensitive partial match)
fn find_sessions_by_titles(
    chat_sessions_dir: &PathBuf,
    titles: &[String],
) -> Result<Vec<(ChatSession, PathBuf)>> {
    let mut matches = Vec::new();
    let title_patterns: Vec<String> = titles.iter().map(|t| t.to_lowercase()).collect();

    for entry in std::fs::read_dir(chat_sessions_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(session) = parse_session_json(&content) {
                    let session_title = session.title().to_lowercase();

                    for pattern in &title_patterns {
                        if session_title.contains(pattern) {
                            matches.push((session, path.clone()));
                            break;
                        }
                    }
                }
            }
        }
    }

    if matches.is_empty() {
        println!(
            "{} No sessions found matching the specified titles",
            "[!]".yellow()
        );
    }

    Ok(matches)
}
