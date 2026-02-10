// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! History commands (show, fetch, merge)

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use colored::*;
use std::path::Path;
use uuid::Uuid;

use crate::models::{ChatRequest, ChatSession};
use crate::storage::{
    add_session_to_index, backup_workspace_sessions, get_workspace_storage_db, is_vscode_running,
    register_all_sessions_from_directory,
};
use crate::workspace::{
    discover_workspaces, find_all_workspaces_for_project, find_workspace_by_path,
    get_chat_sessions_from_workspace, normalize_path,
};

/// Show all chat sessions across workspaces for current project
pub fn history_show(project_path: Option<&str>) -> Result<()> {
    // Resolve the project path, handling "." specially
    let project_path = match project_path {
        Some(".") | None => std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string()),
        Some(p) => {
            // If it's a relative path, try to resolve it
            let path = Path::new(p);
            if path.is_relative() {
                std::env::current_dir()
                    .map(|cwd| cwd.join(path).to_string_lossy().to_string())
                    .unwrap_or_else(|_| p.to_string())
            } else {
                p.to_string()
            }
        }
    };

    let project_name = Path::new(&project_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| project_path.clone());

    println!(
        "\n{} Chat History for: {}",
        "[*]".blue(),
        project_name.cyan()
    );
    println!("{}", "=".repeat(70));

    // Find all workspaces for this project
    let all_workspaces = find_all_workspaces_for_project(&project_name)?;

    if all_workspaces.is_empty() {
        println!(
            "\n{} No workspaces found matching '{}'",
            "[!]".yellow(),
            project_name
        );
        return Ok(());
    }

    // Find current workspace
    let current_ws = find_workspace_by_path(&project_path)?;
    let current_ws_id = current_ws.as_ref().map(|(id, _, _)| id.clone());

    let mut total_sessions = 0;
    let mut total_requests = 0;

    for (ws_id, ws_dir, folder_path, last_mod) in &all_workspaces {
        let is_current = current_ws_id.as_ref() == Some(ws_id);
        let marker = if is_current { "-> " } else { "   " };
        let label = if is_current {
            " (current)".green().to_string()
        } else {
            "".to_string()
        };

        let mod_date: DateTime<Utc> = (*last_mod).into();
        let mod_str = mod_date.format("%Y-%m-%d %H:%M").to_string();

        let sessions = get_chat_sessions_from_workspace(ws_dir)?;

        println!(
            "\n{}Workspace: {}...{}",
            marker.cyan(),
            &ws_id[..16.min(ws_id.len())],
            label
        );
        println!("   Path: {}", folder_path.as_deref().unwrap_or("(none)"));
        println!("   Modified: {}", mod_str);
        println!("   Sessions: {}", sessions.len());

        if !sessions.is_empty() {
            for session_with_path in &sessions {
                let session = &session_with_path.session;
                let title = session.title();
                let request_count = session.request_count();

                // Get timestamp range
                let date_range = if let Some((first, last)) = session.timestamp_range() {
                    let first_date = timestamp_to_date(first);
                    let last_date = timestamp_to_date(last);
                    if first_date == last_date {
                        first_date
                    } else {
                        format!("{} -> {}", first_date, last_date)
                    }
                } else {
                    "empty".to_string()
                };

                println!(
                    "     {} {:<40} ({:3} msgs) [{}]",
                    "[-]".blue(),
                    truncate(&title, 40),
                    request_count,
                    date_range
                );

                total_requests += request_count;
                total_sessions += 1;
            }
        }
    }

    println!("\n{}", "=".repeat(70));
    println!(
        "Total: {} sessions, {} messages across {} workspace(s)",
        total_sessions,
        total_requests,
        all_workspaces.len()
    );

    Ok(())
}

/// Fetch chat sessions from other workspaces into current workspace
pub fn history_fetch(project_path: Option<&str>, force: bool, no_register: bool) -> Result<()> {
    // Resolve the path - canonicalize handles "." and relative paths
    let project_path = match project_path {
        Some(p) => {
            let path = Path::new(p);
            path.canonicalize()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| p.to_string())
        }
        None => std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string()),
    };

    let project_name = Path::new(&project_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| project_path.clone());

    println!(
        "\n{} Fetching Chat History for: {}",
        "[<]".blue(),
        project_name.cyan()
    );
    println!("{}", "=".repeat(70));

    // Find current workspace
    let current_ws = find_workspace_by_path(&project_path)?
        .context("Current workspace not found. Make sure the project is opened in VS Code")?;
    let (current_ws_id, current_ws_dir, _) = current_ws;

    // Find all workspaces for this project
    let all_workspaces = find_all_workspaces_for_project(&project_name)?;
    let historical_workspaces: Vec<_> = all_workspaces
        .into_iter()
        .filter(|(id, _, _, _)| *id != current_ws_id)
        .collect();

    if historical_workspaces.is_empty() {
        println!(
            "{} No historical workspaces found for '{}'",
            "[!]".yellow(),
            project_name
        );
        println!("   Only the current workspace exists.");
        return Ok(());
    }

    println!(
        "Found {} historical workspace(s)\n",
        historical_workspaces.len()
    );

    // Create chatSessions directory
    let chat_sessions_dir = current_ws_dir.join("chatSessions");
    std::fs::create_dir_all(&chat_sessions_dir)?;

    let mut fetched_count = 0;
    let mut skipped_count = 0;

    for (_, ws_dir, _, _) in &historical_workspaces {
        let sessions = get_chat_sessions_from_workspace(ws_dir)?;

        for session_with_path in sessions {
            // Get session ID from filename if not in data
            let session_id = session_with_path
                .session
                .session_id
                .clone()
                .unwrap_or_else(|| {
                    session_with_path
                        .path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
                });
            let dest_file = chat_sessions_dir.join(format!("{}.json", session_id));

            if dest_file.exists() && !force {
                println!(
                    "   {} Skipped (exists): {}...",
                    "[>]".yellow(),
                    &session_id[..16.min(session_id.len())]
                );
                skipped_count += 1;
            } else {
                std::fs::copy(&session_with_path.path, &dest_file)?;
                let title = session_with_path.session.title();
                println!(
                    "   {} Fetched: {} ({}...)",
                    "[OK]".green(),
                    truncate(&title, 40),
                    &session_id[..16.min(session_id.len())]
                );
                fetched_count += 1;
            }
        }
    }

    println!("\n{}", "=".repeat(70));
    println!("Fetched: {} sessions", fetched_count);
    if skipped_count > 0 {
        println!("Skipped: {} (use --force to overwrite)", skipped_count);
    }

    // Register sessions in VS Code index
    if fetched_count > 0 && !no_register {
        println!(
            "\n{} Registering sessions in VS Code index...",
            "[#]".blue()
        );

        if is_vscode_running() && !force {
            println!(
                "{} VS Code is running. Sessions may not appear until restart.",
                "[!]".yellow()
            );
            println!("   Run 'csm history fetch --force' after closing VS Code to register.");
        } else {
            let registered =
                register_all_sessions_from_directory(&current_ws_id, &chat_sessions_dir, true)?;
            println!(
                "{} Registered {} sessions in index",
                "[OK]".green(),
                registered
            );
        }
    }

    println!(
        "\n{} Reload VS Code (Ctrl+R) and check Chat history dropdown",
        "[i]".cyan()
    );

    Ok(())
}

/// Merge all chat sessions into a single unified chat ordered by timestamp
pub fn history_merge(
    project_path: Option<&str>,
    title: Option<&str>,
    force: bool,
    no_backup: bool,
) -> Result<()> {
    let project_path = project_path.map(|p| p.to_string()).unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    let project_name = Path::new(&project_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| project_path.clone());

    println!(
        "\n{} Merging Chat History for: {}",
        "[M]".blue(),
        project_name.cyan()
    );
    println!("{}", "=".repeat(70));

    // Find current workspace
    let current_ws = find_workspace_by_path(&project_path)?
        .context("Current workspace not found. Make sure the project is opened in VS Code")?;
    let (current_ws_id, current_ws_dir, _) = current_ws;

    // Find all workspaces for this project
    let all_workspaces = find_all_workspaces_for_project(&project_name)?;

    // Collect ALL sessions from ALL workspaces
    println!(
        "\n{} Collecting sessions from {} workspace(s)...",
        "[D]".blue(),
        all_workspaces.len()
    );

    let mut all_sessions = Vec::new();
    for (ws_id, ws_dir, _, _) in &all_workspaces {
        let sessions = get_chat_sessions_from_workspace(ws_dir)?;
        if !sessions.is_empty() {
            println!(
                "   {} {}... ({} sessions)",
                "[d]".blue(),
                &ws_id[..16.min(ws_id.len())],
                sessions.len()
            );
            all_sessions.extend(sessions);
        }
    }

    if all_sessions.is_empty() {
        println!("\n{} No chat sessions found in any workspace", "[X]".red());
        return Ok(());
    }

    println!("\n   Total: {} sessions collected", all_sessions.len());

    // Collect all requests with timestamps
    println!("\n{} Extracting and sorting messages...", "[*]".blue());

    let mut all_requests: Vec<ChatRequest> = Vec::new();
    for session_with_path in &all_sessions {
        let session = &session_with_path.session;
        let session_title = session.title();

        for req in &session.requests {
            let mut req = req.clone();
            // Add source session info
            req.source_session = Some(session_title.clone());
            if req.timestamp.is_some() {
                all_requests.push(req);
            }
        }
    }

    if all_requests.is_empty() {
        println!("\n{} No messages found in any session", "[X]".red());
        return Ok(());
    }

    // Sort by timestamp
    all_requests.sort_by_key(|r| r.timestamp.unwrap_or(0));

    // Get timeline info
    let first_time = all_requests.first().and_then(|r| r.timestamp).unwrap_or(0);
    let last_time = all_requests.last().and_then(|r| r.timestamp).unwrap_or(0);

    let first_date = timestamp_to_date(first_time);
    let last_date = timestamp_to_date(last_time);
    let days_span = if first_time > 0 && last_time > 0 {
        (last_time - first_time) / (1000 * 60 * 60 * 24)
    } else {
        0
    };

    println!("   Messages: {}", all_requests.len());
    println!(
        "   Timeline: {} -> {} ({} days)",
        first_date, last_date, days_span
    );

    // Create merged session
    println!("\n{} Creating merged session...", "[+]".blue());

    let merged_session_id = Uuid::new_v4().to_string();
    let merged_title = title.map(|t| t.to_string()).unwrap_or_else(|| {
        format!(
            "Merged History ({} sessions, {} days)",
            all_sessions.len(),
            days_span
        )
    });

    let merged_session = ChatSession {
        version: 3,
        session_id: Some(merged_session_id.clone()),
        creation_date: first_time,
        last_message_date: last_time,
        is_imported: false,
        initial_location: "panel".to_string(),
        custom_title: Some(merged_title.clone()),
        requester_username: Some("User".to_string()),
        requester_avatar_icon_uri: None, // Optional - VS Code will use default
        responder_username: Some("GitHub Copilot".to_string()),
        responder_avatar_icon_uri: Some(serde_json::json!({"id": "copilot"})),
        requests: all_requests.clone(),
    };

    // Create backup if requested
    let chat_sessions_dir = current_ws_dir.join("chatSessions");

    if !no_backup {
        if let Some(backup_dir) = backup_workspace_sessions(&current_ws_dir)? {
            println!(
                "   {} Backup: {}",
                "[B]".blue(),
                backup_dir.file_name().unwrap().to_string_lossy()
            );
        }
    }

    // Write merged session
    std::fs::create_dir_all(&chat_sessions_dir)?;
    let merged_file = chat_sessions_dir.join(format!("{}.json", merged_session_id));

    let json = serde_json::to_string_pretty(&merged_session)?;
    std::fs::write(&merged_file, json)?;

    println!(
        "   {} File: {}",
        "[F]".blue(),
        merged_file.file_name().unwrap().to_string_lossy()
    );

    // Register in VS Code index
    println!("\n{} Registering in VS Code index...", "[#]".blue());

    if is_vscode_running() && !force {
        println!(
            "{} VS Code is running. Close it and run again, or use --force",
            "[!]".yellow()
        );
    } else {
        let db_path = get_workspace_storage_db(&current_ws_id)?;
        add_session_to_index(
            &db_path,
            &merged_session_id,
            &merged_title,
            last_time,
            false,
            "panel",
            false,
        )?;
        println!("   {} Registered in index", "[OK]".green());
    }

    println!("\n{}", "=".repeat(70));
    println!("{} MERGE COMPLETE!", "[OK]".green().bold());
    println!("\n{} Summary:", "[=]".blue());
    println!("   - Sessions merged: {}", all_sessions.len());
    println!("   - Total messages: {}", all_requests.len());
    println!("   - Timeline: {} days", days_span);
    println!("   - Title: {}", merged_title);

    println!("\n{} Next Steps:", "[i]".cyan());
    println!("   1. Reload VS Code (Ctrl+R)");
    println!("   2. Open Chat history dropdown");
    println!("   3. Select: '{}'", merged_title);

    Ok(())
}

/// Merge chat sessions from workspaces matching a name pattern
pub fn merge_by_workspace_name(
    workspace_name: &str,
    title: Option<&str>,
    target_path: Option<&str>,
    force: bool,
    no_backup: bool,
) -> Result<()> {
    println!(
        "\n{} Merging Sessions by Workspace Name: {}",
        "[M]".blue(),
        workspace_name.cyan()
    );
    println!("{}", "=".repeat(70));

    // Find all workspaces matching the pattern
    let all_workspaces = find_all_workspaces_for_project(workspace_name)?;

    if all_workspaces.is_empty() {
        println!(
            "\n{} No workspaces found matching '{}'",
            "[X]".red(),
            workspace_name
        );
        return Ok(());
    }

    println!(
        "\n{} Found {} workspace(s) matching pattern:",
        "[D]".blue(),
        all_workspaces.len()
    );
    for (ws_id, _, folder_path, _) in &all_workspaces {
        println!(
            "   {} {}... -> {}",
            "[*]".blue(),
            &ws_id[..16.min(ws_id.len())],
            folder_path.as_deref().unwrap_or("(unknown)")
        );
    }

    // Determine target workspace
    let target_path = target_path.map(|p| p.to_string()).unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    let target_ws = find_workspace_by_path(&target_path)?
        .context("Target workspace not found. Make sure the project is opened in VS Code")?;
    let (target_ws_id, target_ws_dir, _) = target_ws;

    println!(
        "\n{} Target workspace: {}...",
        "[>]".blue(),
        &target_ws_id[..16.min(target_ws_id.len())]
    );

    // Collect sessions from all matching workspaces
    println!("\n{} Collecting sessions...", "[D]".blue());

    let mut all_sessions = Vec::new();
    for (ws_id, ws_dir, _, _) in &all_workspaces {
        let sessions = get_chat_sessions_from_workspace(ws_dir)?;
        if !sessions.is_empty() {
            println!(
                "   {} {}... ({} sessions)",
                "[d]".blue(),
                &ws_id[..16.min(ws_id.len())],
                sessions.len()
            );
            all_sessions.extend(sessions);
        }
    }

    if all_sessions.is_empty() {
        println!(
            "\n{} No chat sessions found in matching workspaces",
            "[X]".red()
        );
        return Ok(());
    }

    // Use the common merge logic
    merge_sessions_internal(
        all_sessions,
        title,
        &target_ws_id,
        &target_ws_dir,
        force,
        no_backup,
        &format!("Workspace: {}", workspace_name),
    )
}

/// Merge specific chat sessions by their IDs or filenames
pub fn merge_sessions_by_list(
    session_ids: &[String],
    title: Option<&str>,
    target_path: Option<&str>,
    force: bool,
    no_backup: bool,
) -> Result<()> {
    println!("\n{} Merging Specific Sessions", "[M]".blue());
    println!("{}", "=".repeat(70));

    println!(
        "\n{} Looking for {} session(s):",
        "[D]".blue(),
        session_ids.len()
    );
    for id in session_ids {
        println!("   {} {}", "[?]".blue(), id);
    }

    // Determine target workspace
    let target_path = target_path.map(|p| p.to_string()).unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    let target_ws = find_workspace_by_path(&target_path)?
        .context("Target workspace not found. Make sure the project is opened in VS Code")?;
    let (target_ws_id, target_ws_dir, _) = target_ws;

    // Find and collect requested sessions from all workspaces
    println!("\n{} Searching all workspaces...", "[D]".blue());

    let all_workspaces = crate::workspace::discover_workspaces()?;
    let mut found_sessions = Vec::new();
    let mut found_ids: Vec<String> = Vec::new();

    // Normalize session IDs for comparison (remove .json extension if present)
    let normalized_ids: Vec<String> = session_ids
        .iter()
        .map(|id| {
            let id = id.trim();
            if id.to_lowercase().ends_with(".json") {
                id[..id.len() - 5].to_string()
            } else {
                id.to_string()
            }
        })
        .collect();

    for ws in &all_workspaces {
        if !ws.has_chat_sessions {
            continue;
        }

        let sessions = get_chat_sessions_from_workspace(&ws.workspace_path)?;

        for session_with_path in sessions {
            let session_id = session_with_path
                .session
                .session_id
                .clone()
                .unwrap_or_else(|| {
                    session_with_path
                        .path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default()
                });

            // Check if this session matches any requested ID
            let matches = normalized_ids.iter().any(|req_id| {
                session_id.starts_with(req_id)
                    || req_id.starts_with(&session_id)
                    || session_id == *req_id
            });

            if matches && !found_ids.contains(&session_id) {
                println!(
                    "   {} Found: {} in workspace {}...",
                    "[OK]".green(),
                    truncate(&session_with_path.session.title(), 40),
                    &ws.hash[..16.min(ws.hash.len())]
                );
                found_ids.push(session_id);
                found_sessions.push(session_with_path);
            }
        }
    }

    if found_sessions.is_empty() {
        println!("\n{} No matching sessions found", "[X]".red());
        println!(
            "\n{} Tip: Use 'csm list sessions' or 'csm find session <pattern>' to find session IDs",
            "[i]".cyan()
        );
        return Ok(());
    }

    // Report any sessions that weren't found
    let not_found: Vec<_> = normalized_ids
        .iter()
        .filter(|id| {
            !found_ids
                .iter()
                .any(|found| found.starts_with(*id) || id.starts_with(found))
        })
        .collect();

    if !not_found.is_empty() {
        println!("\n{} Sessions not found:", "[!]".yellow());
        for id in not_found {
            println!("   {} {}", "[X]".red(), id);
        }
    }

    println!("\n   Total: {} sessions found", found_sessions.len());

    // Use the common merge logic
    merge_sessions_internal(
        found_sessions,
        title,
        &target_ws_id,
        &target_ws_dir,
        force,
        no_backup,
        &format!("{} selected sessions", session_ids.len()),
    )
}

/// Internal function to merge sessions and write to target workspace
fn merge_sessions_internal(
    sessions: Vec<crate::models::SessionWithPath>,
    title: Option<&str>,
    target_ws_id: &str,
    target_ws_dir: &Path,
    force: bool,
    no_backup: bool,
    source_description: &str,
) -> Result<()> {
    // Collect all requests with timestamps
    println!("\n{} Extracting and sorting messages...", "[*]".blue());

    let mut all_requests: Vec<ChatRequest> = Vec::new();
    for session_with_path in &sessions {
        let session = &session_with_path.session;
        let session_title = session.title();

        for req in &session.requests {
            let mut req = req.clone();
            req.source_session = Some(session_title.clone());
            if req.timestamp.is_some() {
                all_requests.push(req);
            }
        }
    }

    if all_requests.is_empty() {
        println!("\n{} No messages found in selected sessions", "[X]".red());
        return Ok(());
    }

    // Sort by timestamp
    all_requests.sort_by_key(|r| r.timestamp.unwrap_or(0));

    // Get timeline info
    let first_time = all_requests.first().and_then(|r| r.timestamp).unwrap_or(0);
    let last_time = all_requests.last().and_then(|r| r.timestamp).unwrap_or(0);

    let first_date = timestamp_to_date(first_time);
    let last_date = timestamp_to_date(last_time);
    let days_span = if first_time > 0 && last_time > 0 {
        (last_time - first_time) / (1000 * 60 * 60 * 24)
    } else {
        0
    };

    println!("   Messages: {}", all_requests.len());
    println!(
        "   Timeline: {} -> {} ({} days)",
        first_date, last_date, days_span
    );

    // Create merged session
    println!("\n{} Creating merged session...", "[+]".blue());

    let merged_session_id = Uuid::new_v4().to_string();
    let merged_title = title.map(|t| t.to_string()).unwrap_or_else(|| {
        format!(
            "Merged: {} ({} sessions, {} days)",
            source_description,
            sessions.len(),
            days_span
        )
    });

    let merged_session = ChatSession {
        version: 3,
        session_id: Some(merged_session_id.clone()),
        creation_date: first_time,
        last_message_date: last_time,
        is_imported: false,
        initial_location: "panel".to_string(),
        custom_title: Some(merged_title.clone()),
        requester_username: Some("User".to_string()),
        requester_avatar_icon_uri: None,
        responder_username: Some("GitHub Copilot".to_string()),
        responder_avatar_icon_uri: Some(serde_json::json!({"id": "copilot"})),
        requests: all_requests.clone(),
    };

    // Create backup if requested
    let chat_sessions_dir = target_ws_dir.join("chatSessions");

    if !no_backup {
        if let Some(backup_dir) = backup_workspace_sessions(target_ws_dir)? {
            println!(
                "   {} Backup: {}",
                "[B]".blue(),
                backup_dir.file_name().unwrap().to_string_lossy()
            );
        }
    }

    // Write merged session
    std::fs::create_dir_all(&chat_sessions_dir)?;
    let merged_file = chat_sessions_dir.join(format!("{}.json", merged_session_id));

    let json = serde_json::to_string_pretty(&merged_session)?;
    std::fs::write(&merged_file, json)?;

    println!(
        "   {} File: {}",
        "[F]".blue(),
        merged_file.file_name().unwrap().to_string_lossy()
    );

    // Register in VS Code index
    println!("\n{} Registering in VS Code index...", "[#]".blue());

    if is_vscode_running() && !force {
        println!(
            "{} VS Code is running. Close it and run again, or use --force",
            "[!]".yellow()
        );
    } else {
        let db_path = get_workspace_storage_db(target_ws_id)?;
        add_session_to_index(
            &db_path,
            &merged_session_id,
            &merged_title,
            last_time,
            false,
            "panel",
            false,
        )?;
        println!("   {} Registered in index", "[OK]".green());
    }

    println!("\n{}", "=".repeat(70));
    println!("{} MERGE COMPLETE!", "[OK]".green().bold());
    println!("\n{} Summary:", "[=]".blue());
    println!("   - Sessions merged: {}", sessions.len());
    println!("   - Total messages: {}", all_requests.len());
    println!("   - Timeline: {} days", days_span);
    println!("   - Title: {}", merged_title);

    println!("\n{} Next Steps:", "[i]".cyan());
    println!("   1. Reload VS Code (Ctrl+R)");
    println!("   2. Open Chat history dropdown");
    println!("   3. Select: '{}'", merged_title);

    Ok(())
}

/// Convert millisecond timestamp to date string
fn timestamp_to_date(timestamp: i64) -> String {
    if timestamp == 0 {
        return "unknown".to_string();
    }

    // Handle both milliseconds and seconds
    let secs = if timestamp > 1_000_000_000_000 {
        timestamp / 1000
    } else {
        timestamp
    };

    DateTime::from_timestamp(secs, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Truncate string to max length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Fetch sessions from workspaces matching a name pattern
pub fn fetch_by_workspace(
    workspace_name: &str,
    target_path: Option<&str>,
    force: bool,
    no_register: bool,
) -> Result<()> {
    use colored::Colorize;
    use std::fs;

    println!("\n{}", "=".repeat(70));
    println!("{} FETCH BY WORKSPACE", "[*]".cyan().bold());
    println!("{}", "=".repeat(70));

    // Find target workspace
    let target_dir = match target_path {
        Some(p) => {
            let path = std::path::PathBuf::from(p);
            path.canonicalize().unwrap_or(path)
        }
        None => std::env::current_dir().unwrap_or_default(),
    };
    let target_normalized = normalize_path(target_dir.to_str().unwrap_or(""));

    println!("\n{} Target: {}", "[>]".blue(), target_normalized);
    println!("{} Pattern: {}", "[>]".blue(), workspace_name);

    // Find all workspaces
    let all_workspaces = discover_workspaces()?;
    let pattern_lower = workspace_name.to_lowercase();

    // Find source workspaces matching pattern
    let source_workspaces: Vec<_> = all_workspaces
        .iter()
        .filter(|ws| {
            ws.project_path
                .as_ref()
                .map(|p| p.to_lowercase().contains(&pattern_lower))
                .unwrap_or(false)
        })
        .filter(|ws| ws.has_chat_sessions)
        .collect();

    if source_workspaces.is_empty() {
        println!(
            "\n{} No workspaces found matching '{}'",
            "[X]".red(),
            workspace_name
        );
        return Ok(());
    }

    println!(
        "\n{} Found {} matching workspace(s)",
        "[OK]".green(),
        source_workspaces.len()
    );

    // Find target workspace
    let target_ws = all_workspaces.iter().find(|ws| {
        ws.project_path
            .as_ref()
            .map(|p| normalize_path(p) == target_normalized)
            .unwrap_or(false)
    });

    let target_ws_dir = match target_ws {
        Some(ws) => ws.workspace_path.join("workspaceState"),
        None => {
            println!(
                "{} Target workspace not found, creating new...",
                "[!]".yellow()
            );
            // Would need to create workspace - for now return error
            anyhow::bail!("Target workspace not found. Please open the folder in VS Code first.");
        }
    };

    // Collect all sessions from matching workspaces
    let mut fetched_count = 0;

    for ws in source_workspaces {
        let sessions = get_chat_sessions_from_workspace(&ws.workspace_path)?;

        for session_with_path in sessions {
            let src_file = &session_with_path.path;
            let filename = src_file
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let dest_file = target_ws_dir.join(&filename);

            if dest_file.exists() && !force {
                println!("   {} Skipping (exists): {}", "[!]".yellow(), filename);
                continue;
            }

            fs::copy(src_file, &dest_file)?;
            fetched_count += 1;
            println!(
                "   {} Fetched: {}",
                "[OK]".green(),
                session_with_path.session.title()
            );
        }
    }

    println!(
        "\n{} Fetched {} session(s)",
        "[OK]".green().bold(),
        fetched_count
    );

    if !no_register {
        println!(
            "{} Sessions will appear in VS Code after reload",
            "[i]".cyan()
        );
    }

    Ok(())
}

/// Fetch specific sessions by their IDs
pub fn fetch_sessions(
    session_ids: &[String],
    target_path: Option<&str>,
    force: bool,
    no_register: bool,
) -> Result<()> {
    use colored::Colorize;
    use std::fs;

    println!("\n{}", "=".repeat(70));
    println!("{} FETCH SESSIONS BY ID", "[*]".cyan().bold());
    println!("{}", "=".repeat(70));

    if session_ids.is_empty() {
        println!("{} No session IDs provided", "[X]".red());
        return Ok(());
    }

    // Find target workspace
    let target_dir = match target_path {
        Some(p) => {
            let path = std::path::PathBuf::from(p);
            path.canonicalize().unwrap_or(path)
        }
        None => std::env::current_dir().unwrap_or_default(),
    };
    let target_normalized = normalize_path(target_dir.to_str().unwrap_or(""));

    println!("\n{} Target: {}", "[>]".blue(), target_normalized);
    println!("{} Sessions: {:?}", "[>]".blue(), session_ids);

    let all_workspaces = discover_workspaces()?;

    // Find target workspace
    let target_ws = all_workspaces.iter().find(|ws| {
        ws.project_path
            .as_ref()
            .map(|p| normalize_path(p) == target_normalized)
            .unwrap_or(false)
    });

    let target_ws_dir = match target_ws {
        Some(ws) => ws.workspace_path.join("workspaceState"),
        None => {
            anyhow::bail!("Target workspace not found. Please open the folder in VS Code first.");
        }
    };

    // Normalize session IDs
    let normalized_ids: Vec<String> = session_ids
        .iter()
        .flat_map(|s| s.split(',').map(|p| p.trim().to_lowercase()))
        .filter(|s| !s.is_empty())
        .collect();

    let mut fetched_count = 0;
    let mut found_ids = Vec::new();

    for ws in &all_workspaces {
        if !ws.has_chat_sessions {
            continue;
        }

        let sessions = get_chat_sessions_from_workspace(&ws.workspace_path)?;

        for session_with_path in sessions {
            let session_id = session_with_path
                .session
                .session_id
                .clone()
                .unwrap_or_else(|| {
                    session_with_path
                        .path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default()
                });

            let matches = normalized_ids.iter().any(|req_id| {
                session_id.to_lowercase().contains(req_id)
                    || req_id.contains(&session_id.to_lowercase())
            });

            if matches && !found_ids.contains(&session_id) {
                let src_file = &session_with_path.path;
                let filename = src_file
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                let dest_file = target_ws_dir.join(&filename);

                if dest_file.exists() && !force {
                    println!("   {} Skipping (exists): {}", "[!]".yellow(), filename);
                    found_ids.push(session_id);
                    continue;
                }

                fs::copy(src_file, &dest_file)?;
                fetched_count += 1;
                found_ids.push(session_id);
                println!(
                    "   {} Fetched: {}",
                    "[OK]".green(),
                    session_with_path.session.title()
                );
            }
        }
    }

    // Report not found
    let not_found: Vec<_> = normalized_ids
        .iter()
        .filter(|id| {
            !found_ids
                .iter()
                .any(|found| found.to_lowercase().contains(*id))
        })
        .collect();

    if !not_found.is_empty() {
        println!("\n{} Sessions not found:", "[!]".yellow());
        for id in not_found {
            println!("   {} {}", "[X]".red(), id);
        }
    }

    println!(
        "\n{} Fetched {} session(s)",
        "[OK]".green().bold(),
        fetched_count
    );

    if !no_register {
        println!(
            "{} Sessions will appear in VS Code after reload",
            "[i]".cyan()
        );
    }

    Ok(())
}

/// Merge chat sessions from multiple workspace name patterns
pub fn merge_by_workspace_names(
    workspace_names: &[String],
    title: Option<&str>,
    target_path: Option<&str>,
    force: bool,
    no_backup: bool,
) -> Result<()> {
    println!(
        "\n{} Merging Sessions from Multiple Workspaces",
        "[M]".blue().bold()
    );
    println!("{}", "=".repeat(70));

    println!("\n{} Workspace patterns:", "[D]".blue());
    for name in workspace_names {
        println!("   {} {}", "[*]".blue(), name.cyan());
    }

    // Collect all matching workspaces
    let mut all_matching_workspaces = Vec::new();
    let mut seen_ws_ids = std::collections::HashSet::new();

    for pattern in workspace_names {
        let workspaces = find_all_workspaces_for_project(pattern)?;
        for ws in workspaces {
            if !seen_ws_ids.contains(&ws.0) {
                seen_ws_ids.insert(ws.0.clone());
                all_matching_workspaces.push(ws);
            }
        }
    }

    if all_matching_workspaces.is_empty() {
        println!(
            "\n{} No workspaces found matching any of the patterns",
            "[X]".red()
        );
        return Ok(());
    }

    println!(
        "\n{} Found {} unique workspace(s):",
        "[D]".blue(),
        all_matching_workspaces.len()
    );
    for (ws_id, _, folder_path, _) in &all_matching_workspaces {
        println!(
            "   {} {}... -> {}",
            "[*]".blue(),
            &ws_id[..16.min(ws_id.len())],
            folder_path.as_deref().unwrap_or("(unknown)")
        );
    }

    // Determine target workspace
    let target_path = target_path.map(|p| p.to_string()).unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    let target_ws = find_workspace_by_path(&target_path)?
        .context("Target workspace not found. Make sure the project is opened in VS Code")?;
    let (target_ws_id, target_ws_dir, _) = target_ws;

    println!(
        "\n{} Target workspace: {}...",
        "[>]".blue(),
        &target_ws_id[..16.min(target_ws_id.len())]
    );

    // Collect sessions from all matching workspaces
    println!("\n{} Collecting sessions...", "[D]".blue());

    let mut all_sessions = Vec::new();
    for (ws_id, ws_dir, _, _) in &all_matching_workspaces {
        let sessions = get_chat_sessions_from_workspace(ws_dir)?;
        if !sessions.is_empty() {
            println!(
                "   {} {}... ({} sessions)",
                "[d]".blue(),
                &ws_id[..16.min(ws_id.len())],
                sessions.len()
            );
            all_sessions.extend(sessions);
        }
    }

    if all_sessions.is_empty() {
        println!(
            "\n{} No chat sessions found in matching workspaces",
            "[X]".red()
        );
        return Ok(());
    }

    // Generate title from workspace names if not provided
    let auto_title = format!("Merged: {}", workspace_names.join(" + "));
    let merge_title = title.unwrap_or(&auto_title);

    // Use the common merge logic
    merge_sessions_internal(
        all_sessions,
        Some(merge_title),
        &target_ws_id,
        &target_ws_dir,
        force,
        no_backup,
        &format!("{} workspaces", workspace_names.len()),
    )
}

/// Merge chat sessions from an LLM provider
pub fn merge_from_provider(
    provider_name: &str,
    title: Option<&str>,
    target_path: Option<&str>,
    session_ids: Option<&[String]>,
    force: bool,
    no_backup: bool,
) -> Result<()> {
    use crate::providers::{ProviderRegistry, ProviderType};

    println!(
        "\n{} Merging Sessions from Provider: {}",
        "[M]".blue().bold(),
        provider_name.cyan()
    );
    println!("{}", "=".repeat(70));

    // Parse provider name
    let provider_type = match provider_name.to_lowercase().as_str() {
        "copilot" | "github-copilot" | "vscode" => ProviderType::Copilot,
        "cursor" => ProviderType::Cursor,
        "ollama" => ProviderType::Ollama,
        "vllm" => ProviderType::Vllm,
        "foundry" | "azure" | "azure-foundry" => ProviderType::Foundry,
        "lm-studio" | "lmstudio" => ProviderType::LmStudio,
        "localai" | "local-ai" => ProviderType::LocalAI,
        "text-gen-webui" | "textgenwebui" | "oobabooga" => ProviderType::TextGenWebUI,
        "jan" | "jan-ai" => ProviderType::Jan,
        "gpt4all" => ProviderType::Gpt4All,
        "llamafile" => ProviderType::Llamafile,
        _ => {
            println!("{} Unknown provider: {}", "[X]".red(), provider_name);
            println!("\n{} Available providers:", "[i]".cyan());
            println!("   copilot, cursor, ollama, vllm, foundry, lm-studio,");
            println!("   localai, text-gen-webui, jan, gpt4all, llamafile");
            return Ok(());
        }
    };

    // Get provider sessions
    let registry = ProviderRegistry::new();
    let provider = registry
        .get_provider(provider_type)
        .context(format!("Provider '{}' not available", provider_name))?;

    if !provider.is_available() {
        println!(
            "{} Provider '{}' is not available or not configured",
            "[X]".red(),
            provider_name
        );
        return Ok(());
    }

    println!(
        "{} Provider: {} ({})",
        "[*]".blue(),
        provider.name(),
        provider_type.display_name()
    );

    // Get sessions from provider
    let provider_sessions = provider
        .list_sessions()
        .context("Failed to list sessions from provider")?;

    if provider_sessions.is_empty() {
        println!("{} No sessions found in provider", "[X]".red());
        return Ok(());
    }

    println!(
        "{} Found {} session(s) in provider",
        "[D]".blue(),
        provider_sessions.len()
    );

    // Filter sessions if specific IDs provided
    let sessions_to_merge: Vec<_> = if let Some(ids) = session_ids {
        let ids_lower: Vec<String> = ids.iter().map(|s| s.to_lowercase()).collect();
        provider_sessions
            .into_iter()
            .filter(|s| {
                let session_id = s
                    .session_id
                    .as_ref()
                    .unwrap_or(&String::new())
                    .to_lowercase();
                let title = s.title().to_lowercase();
                ids_lower
                    .iter()
                    .any(|id| session_id.contains(id) || title.contains(id))
            })
            .collect()
    } else {
        provider_sessions
    };

    if sessions_to_merge.is_empty() {
        println!("{} No matching sessions found", "[X]".red());
        return Ok(());
    }

    println!(
        "{} Merging {} session(s):",
        "[D]".blue(),
        sessions_to_merge.len()
    );
    for s in &sessions_to_merge {
        println!(
            "   {} {} ({} messages)",
            "[*]".blue(),
            truncate(&s.title(), 50),
            s.request_count()
        );
    }

    // Determine target workspace
    let target_path = target_path.map(|p| p.to_string()).unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    let target_ws = find_workspace_by_path(&target_path)?
        .context("Target workspace not found. Make sure the project is opened in VS Code")?;
    let (target_ws_id, target_ws_dir, _) = target_ws;

    println!(
        "\n{} Target workspace: {}...",
        "[>]".blue(),
        &target_ws_id[..16.min(target_ws_id.len())]
    );

    // Convert to SessionWithPath format (create temporary entries)
    let sessions_with_path: Vec<crate::models::SessionWithPath> = sessions_to_merge
        .into_iter()
        .map(|session| crate::models::SessionWithPath {
            session,
            path: std::path::PathBuf::new(), // Provider sessions don't have a file path
        })
        .collect();

    // Generate title
    let auto_title = format!("Imported from {}", provider.name());
    let merge_title = title.unwrap_or(&auto_title);

    // Use the common merge logic
    merge_sessions_internal(
        sessions_with_path,
        Some(merge_title),
        &target_ws_id,
        &target_ws_dir,
        force,
        no_backup,
        &format!("Provider: {}", provider.name()),
    )
}

/// Merge chat sessions from multiple providers (cross-provider merge)
pub fn merge_cross_provider(
    provider_names: &[String],
    title: Option<&str>,
    target_path: Option<&str>,
    workspace_filter: Option<&str>,
    force: bool,
    no_backup: bool,
) -> Result<()> {
    use crate::models::ChatSession;
    use crate::providers::{ProviderRegistry, ProviderType};

    println!("\n{} Cross-Provider Merge", "[M]".blue().bold());
    println!("{}", "=".repeat(70));
    println!(
        "{} Providers: {}",
        "[*]".blue(),
        provider_names.join(", ").cyan()
    );

    if let Some(ws) = workspace_filter {
        println!("{} Workspace filter: {}", "[*]".blue(), ws.cyan());
    }

    let registry = ProviderRegistry::new();
    let mut all_sessions: Vec<(String, ChatSession)> = Vec::new(); // (provider_name, session)

    // Parse and collect sessions from each provider
    for provider_name in provider_names {
        let provider_type = match provider_name.to_lowercase().as_str() {
            "copilot" | "github-copilot" | "vscode" => Some(ProviderType::Copilot),
            "cursor" => Some(ProviderType::Cursor),
            "ollama" => Some(ProviderType::Ollama),
            "vllm" => Some(ProviderType::Vllm),
            "foundry" | "azure" | "azure-foundry" => Some(ProviderType::Foundry),
            "lm-studio" | "lmstudio" => Some(ProviderType::LmStudio),
            "localai" | "local-ai" => Some(ProviderType::LocalAI),
            "text-gen-webui" | "textgenwebui" | "oobabooga" => Some(ProviderType::TextGenWebUI),
            "jan" | "jan-ai" => Some(ProviderType::Jan),
            "gpt4all" => Some(ProviderType::Gpt4All),
            "llamafile" => Some(ProviderType::Llamafile),
            _ => {
                println!(
                    "{} Unknown provider: {} (skipping)",
                    "[!]".yellow(),
                    provider_name
                );
                None
            }
        };

        if let Some(pt) = provider_type {
            if let Some(provider) = registry.get_provider(pt) {
                if provider.is_available() {
                    match provider.list_sessions() {
                        Ok(sessions) => {
                            let filtered: Vec<_> = if let Some(ws_filter) = workspace_filter {
                                let pattern = ws_filter.to_lowercase();
                                sessions
                                    .into_iter()
                                    .filter(|s| s.title().to_lowercase().contains(&pattern))
                                    .collect()
                            } else {
                                sessions
                            };

                            println!(
                                "{} {} ({}): {} session(s)",
                                "[D]".blue(),
                                provider.name(),
                                provider_type
                                    .as_ref()
                                    .map(|p| p.display_name())
                                    .unwrap_or("?"),
                                filtered.len()
                            );

                            for session in filtered {
                                all_sessions.push((provider.name().to_string(), session));
                            }
                        }
                        Err(e) => {
                            println!(
                                "{} Failed to get sessions from {}: {}",
                                "[!]".yellow(),
                                provider.name(),
                                e
                            );
                        }
                    }
                } else {
                    println!(
                        "{} Provider {} not available",
                        "[!]".yellow(),
                        provider.name()
                    );
                }
            }
        }
    }

    if all_sessions.is_empty() {
        println!("{} No sessions found across providers", "[X]".red());
        return Ok(());
    }

    println!(
        "\n{} Total: {} sessions from {} provider(s)",
        "[*]".green().bold(),
        all_sessions.len(),
        provider_names.len()
    );

    // Sort all sessions by timestamp
    all_sessions.sort_by(|(_, a), (_, b)| {
        let a_time = a
            .requests
            .first()
            .map(|r| r.timestamp.unwrap_or(0))
            .unwrap_or(0);
        let b_time = b
            .requests
            .first()
            .map(|r| r.timestamp.unwrap_or(0))
            .unwrap_or(0);
        a_time.cmp(&b_time)
    });

    // Print sessions being merged
    println!("\n{} Sessions to merge:", "[D]".blue());
    for (provider_name, session) in &all_sessions {
        println!(
            "   {} [{}] {} ({} messages)",
            "[*]".blue(),
            provider_name.cyan(),
            truncate(&session.title(), 40),
            session.request_count()
        );
    }

    // Determine target workspace
    let target_path = target_path.map(|p| p.to_string()).unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    let target_ws = find_workspace_by_path(&target_path)?
        .context("Target workspace not found. Make sure the project is opened in VS Code")?;
    let (target_ws_id, target_ws_dir, _) = target_ws;

    println!(
        "\n{} Target workspace: {}...",
        "[>]".blue(),
        &target_ws_id[..16.min(target_ws_id.len())]
    );

    // Convert to SessionWithPath format
    let sessions_with_path: Vec<crate::models::SessionWithPath> = all_sessions
        .into_iter()
        .map(|(_, session)| crate::models::SessionWithPath {
            session,
            path: std::path::PathBuf::new(),
        })
        .collect();

    // Generate title
    let auto_title = format!("Cross-provider merge: {}", provider_names.join(", "));
    let merge_title = title.unwrap_or(&auto_title);

    merge_sessions_internal(
        sessions_with_path,
        Some(merge_title),
        &target_ws_id,
        &target_ws_dir,
        force,
        no_backup,
        &format!("{} providers", provider_names.len()),
    )
}

/// Merge all sessions from all available providers
pub fn merge_all_providers(
    title: Option<&str>,
    target_path: Option<&str>,
    workspace_filter: Option<&str>,
    force: bool,
    no_backup: bool,
) -> Result<()> {
    use crate::models::ChatSession;
    use crate::providers::{ProviderRegistry, ProviderType};

    println!("\n{} Merge All Providers", "[M]".blue().bold());
    println!("{}", "=".repeat(70));

    if let Some(ws) = workspace_filter {
        println!("{} Workspace filter: {}", "[*]".blue(), ws.cyan());
    }

    let registry = ProviderRegistry::new();
    let mut all_sessions: Vec<(String, ChatSession)> = Vec::new();
    let mut providers_found = 0;

    // List of all provider types to check
    let all_provider_types = vec![
        ProviderType::Copilot,
        ProviderType::Cursor,
        ProviderType::Ollama,
        ProviderType::Vllm,
        ProviderType::Foundry,
        ProviderType::LmStudio,
        ProviderType::LocalAI,
        ProviderType::TextGenWebUI,
        ProviderType::Jan,
        ProviderType::Gpt4All,
        ProviderType::Llamafile,
    ];

    println!("{} Scanning providers...", "[*]".blue());

    for provider_type in all_provider_types {
        if let Some(provider) = registry.get_provider(provider_type) {
            if provider.is_available() {
                match provider.list_sessions() {
                    Ok(sessions) if !sessions.is_empty() => {
                        let filtered: Vec<_> = if let Some(ws_filter) = workspace_filter {
                            let pattern = ws_filter.to_lowercase();
                            sessions
                                .into_iter()
                                .filter(|s| s.title().to_lowercase().contains(&pattern))
                                .collect()
                        } else {
                            sessions
                        };

                        if !filtered.is_empty() {
                            println!(
                                "   {} {}: {} session(s)",
                                "[+]".green(),
                                provider.name(),
                                filtered.len()
                            );
                            providers_found += 1;

                            for session in filtered {
                                all_sessions.push((provider.name().to_string(), session));
                            }
                        }
                    }
                    Ok(_) => {
                        // Empty sessions, skip silently
                    }
                    Err(_) => {
                        // Failed to list, skip silently
                    }
                }
            }
        }
    }

    if all_sessions.is_empty() {
        println!("{} No sessions found across any providers", "[X]".red());
        return Ok(());
    }

    println!(
        "\n{} Found {} sessions across {} provider(s)",
        "[*]".green().bold(),
        all_sessions.len(),
        providers_found
    );

    // Sort all sessions by timestamp
    all_sessions.sort_by(|(_, a), (_, b)| {
        let a_time = a
            .requests
            .first()
            .map(|r| r.timestamp.unwrap_or(0))
            .unwrap_or(0);
        let b_time = b
            .requests
            .first()
            .map(|r| r.timestamp.unwrap_or(0))
            .unwrap_or(0);
        a_time.cmp(&b_time)
    });

    // Print sessions being merged (limit to first 20)
    println!("\n{} Sessions to merge:", "[D]".blue());
    for (i, (provider_name, session)) in all_sessions.iter().enumerate() {
        if i >= 20 {
            println!(
                "   {} ... and {} more",
                "[*]".blue(),
                all_sessions.len() - 20
            );
            break;
        }
        println!(
            "   {} [{}] {} ({} messages)",
            "[*]".blue(),
            provider_name.cyan(),
            truncate(&session.title(), 40),
            session.request_count()
        );
    }

    // Determine target workspace
    let target_path = target_path.map(|p| p.to_string()).unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    let target_ws = find_workspace_by_path(&target_path)?
        .context("Target workspace not found. Make sure the project is opened in VS Code")?;
    let (target_ws_id, target_ws_dir, _) = target_ws;

    println!(
        "\n{} Target workspace: {}...",
        "[>]".blue(),
        &target_ws_id[..16.min(target_ws_id.len())]
    );

    // Convert to SessionWithPath format
    let sessions_with_path: Vec<crate::models::SessionWithPath> = all_sessions
        .into_iter()
        .map(|(_, session)| crate::models::SessionWithPath {
            session,
            path: std::path::PathBuf::new(),
        })
        .collect();

    // Generate title
    let auto_title = format!("All providers merge ({})", providers_found);
    let merge_title = title.unwrap_or(&auto_title);

    merge_sessions_internal(
        sessions_with_path,
        Some(merge_title),
        &target_ws_id,
        &target_ws_dir,
        force,
        no_backup,
        &format!("{} providers (all)", providers_found),
    )
}
