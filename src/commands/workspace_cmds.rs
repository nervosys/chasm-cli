// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Workspace listing commands

use anyhow::Result;
use tabled::{settings::Style as TableStyle, Table, Tabled};

use crate::models::Workspace;
use crate::storage::{read_empty_window_sessions, VsCodeSessionFormat};
use crate::workspace::discover_workspaces;

#[derive(Tabled)]
struct WorkspaceRow {
    #[tabled(rename = "Hash")]
    hash: String,
    #[tabled(rename = "Project Path")]
    project_path: String,
    #[tabled(rename = "Sessions")]
    sessions: usize,
    #[tabled(rename = "Has Chats")]
    has_chats: String,
}

#[derive(Tabled)]
struct SessionRow {
    #[tabled(rename = "Project Path")]
    project_path: String,
    #[tabled(rename = "Session File")]
    session_file: String,
    #[tabled(rename = "Last Modified")]
    last_modified: String,
    #[tabled(rename = "Messages")]
    messages: usize,
}

#[derive(Tabled)]
struct SessionRowWithSize {
    #[tabled(rename = "Project Path")]
    project_path: String,
    #[tabled(rename = "Session File")]
    session_file: String,
    #[tabled(rename = "Last Modified")]
    last_modified: String,
    #[tabled(rename = "Messages")]
    messages: usize,
    #[tabled(rename = "Size")]
    size: String,
}

/// List all VS Code workspaces
pub fn list_workspaces() -> Result<()> {
    let workspaces = discover_workspaces()?;

    if workspaces.is_empty() {
        println!("No workspaces found.");
        return Ok(());
    }

    let rows: Vec<WorkspaceRow> = workspaces
        .iter()
        .map(|ws| WorkspaceRow {
            hash: format!("{}...", &ws.hash[..12.min(ws.hash.len())]),
            project_path: ws
                .project_path
                .clone()
                .unwrap_or_else(|| "(none)".to_string()),
            sessions: ws.chat_session_count,
            has_chats: if ws.has_chat_sessions {
                "Yes".to_string()
            } else {
                "No".to_string()
            },
        })
        .collect();

    let table = Table::new(rows)
        .with(TableStyle::ascii_rounded())
        .to_string();

    println!("{}", table);
    println!("\nTotal workspaces: {}", workspaces.len());

    // Show empty window sessions count (ALL SESSIONS)
    if let Ok(empty_count) = crate::storage::count_empty_window_sessions() {
        if empty_count > 0 {
            println!("Empty window sessions (ALL SESSIONS): {}", empty_count);
        }
    }

    Ok(())
}

/// Format file size in human-readable format
fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// List all chat sessions
pub fn list_sessions(project_path: Option<&str>, show_size: bool, provider: Option<&str>, all_providers: bool) -> Result<()> {
    // If provider filtering is requested, use the multi-provider approach
    if provider.is_some() || all_providers {
        return list_sessions_multi_provider(project_path, show_size, provider, all_providers);
    }

    // Default behavior: VS Code only (backward compatible)
    let workspaces = discover_workspaces()?;

    let filtered_workspaces: Vec<&Workspace> = if let Some(path) = project_path {
        let normalized = crate::workspace::normalize_path(path);
        workspaces
            .iter()
            .filter(|ws| {
                ws.project_path
                    .as_ref()
                    .map(|p| crate::workspace::normalize_path(p) == normalized)
                    .unwrap_or(false)
            })
            .collect()
    } else {
        workspaces.iter().collect()
    };

    if show_size {
        let mut rows: Vec<SessionRowWithSize> = Vec::new();
        let mut total_size: u64 = 0;

        // Add empty window sessions (ALL SESSIONS) if no specific project filter
        if project_path.is_none() {
            if let Ok(empty_sessions) = read_empty_window_sessions() {
                for session in empty_sessions {
                    let modified =
                        chrono::DateTime::from_timestamp_millis(session.last_message_date)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_else(|| "unknown".to_string());

                    let session_id = session.session_id.as_deref().unwrap_or("unknown");
                    rows.push(SessionRowWithSize {
                        project_path: "(ALL SESSIONS)".to_string(),
                        session_file: format!("{}.json", session_id),
                        last_modified: modified,
                        messages: session.request_count(),
                        size: "N/A".to_string(),
                    });
                }
            }
        }

        for ws in &filtered_workspaces {
            if !ws.has_chat_sessions {
                continue;
            }

            let sessions = crate::workspace::get_chat_sessions_from_workspace(&ws.workspace_path)?;

            for session_with_path in sessions {
                let metadata = session_with_path.path.metadata().ok();
                let file_size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                total_size += file_size;

                let modified = metadata
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        let datetime: chrono::DateTime<chrono::Utc> = t.into();
                        datetime.format("%Y-%m-%d %H:%M").to_string()
                    })
                    .unwrap_or_else(|| "unknown".to_string());

                rows.push(SessionRowWithSize {
                    project_path: ws
                        .project_path
                        .clone()
                        .unwrap_or_else(|| "(none)".to_string()),
                    session_file: session_with_path
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    last_modified: modified,
                    messages: session_with_path.session.request_count(),
                    size: format_file_size(file_size),
                });
            }
        }

        if rows.is_empty() {
            println!("No chat sessions found.");
            return Ok(());
        }

        let table = Table::new(&rows)
            .with(TableStyle::ascii_rounded())
            .to_string();
        println!("{}", table);
        println!(
            "\nTotal sessions: {} ({})",
            rows.len(),
            format_file_size(total_size)
        );
    } else {
        let mut rows: Vec<SessionRow> = Vec::new();

        // Add empty window sessions (ALL SESSIONS) if no specific project filter
        if project_path.is_none() {
            if let Ok(empty_sessions) = read_empty_window_sessions() {
                for session in empty_sessions {
                    let modified =
                        chrono::DateTime::from_timestamp_millis(session.last_message_date)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_else(|| "unknown".to_string());

                    let session_id = session.session_id.as_deref().unwrap_or("unknown");
                    rows.push(SessionRow {
                        project_path: "(ALL SESSIONS)".to_string(),
                        session_file: format!("{}.json", session_id),
                        last_modified: modified,
                        messages: session.request_count(),
                    });
                }
            }
        }

        for ws in &filtered_workspaces {
            if !ws.has_chat_sessions {
                continue;
            }

            let sessions = crate::workspace::get_chat_sessions_from_workspace(&ws.workspace_path)?;

            for session_with_path in sessions {
                let modified = session_with_path
                    .path
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        let datetime: chrono::DateTime<chrono::Utc> = t.into();
                        datetime.format("%Y-%m-%d %H:%M").to_string()
                    })
                    .unwrap_or_else(|| "unknown".to_string());

                rows.push(SessionRow {
                    project_path: ws
                        .project_path
                        .clone()
                        .unwrap_or_else(|| "(none)".to_string()),
                    session_file: session_with_path
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    last_modified: modified,
                    messages: session_with_path.session.request_count(),
                });
            }
        }

        if rows.is_empty() {
            println!("No chat sessions found.");
            return Ok(());
        }

        let table = Table::new(&rows)
            .with(TableStyle::ascii_rounded())
            .to_string();
        println!("{}", table);
        println!("\nTotal sessions: {}", rows.len());
    }

    Ok(())
}

/// List sessions from multiple providers
fn list_sessions_multi_provider(
    project_path: Option<&str>,
    show_size: bool,
    provider: Option<&str>,
    all_providers: bool,
) -> Result<()> {
    // Determine which storage paths to scan
    let storage_paths = if all_providers {
        get_agent_storage_paths(Some("all"))?
    } else if let Some(p) = provider {
        get_agent_storage_paths(Some(p))?
    } else {
        get_agent_storage_paths(None)?
    };

    if storage_paths.is_empty() {
        if let Some(p) = provider {
            println!("No storage found for provider: {}", p);
        } else {
            println!("No workspaces found");
        }
        return Ok(());
    }

    let target_path = project_path.map(|p| crate::workspace::normalize_path(p));

    #[derive(Tabled)]
    struct SessionRowMulti {
        #[tabled(rename = "Provider")]
        provider: String,
        #[tabled(rename = "Project Path")]
        project_path: String,
        #[tabled(rename = "Session File")]
        session_file: String,
        #[tabled(rename = "Modified")]
        last_modified: String,
        #[tabled(rename = "Msgs")]
        messages: usize,
    }

    #[derive(Tabled)]
    struct SessionRowMultiWithSize {
        #[tabled(rename = "Provider")]
        provider: String,
        #[tabled(rename = "Project Path")]
        project_path: String,
        #[tabled(rename = "Session File")]
        session_file: String,
        #[tabled(rename = "Modified")]
        last_modified: String,
        #[tabled(rename = "Msgs")]
        messages: usize,
        #[tabled(rename = "Size")]
        size: String,
    }

    let mut rows: Vec<SessionRowMulti> = Vec::new();
    let mut rows_with_size: Vec<SessionRowMultiWithSize> = Vec::new();
    let mut total_size: u64 = 0;

    for (provider_name, storage_path) in &storage_paths {
        if !storage_path.exists() {
            continue;
        }

        for entry in std::fs::read_dir(storage_path)?.filter_map(|e| e.ok()) {
            let workspace_dir = entry.path();
            if !workspace_dir.is_dir() {
                continue;
            }

            let chat_sessions_dir = workspace_dir.join("chatSessions");
            if !chat_sessions_dir.exists() {
                continue;
            }

            // Get project path from workspace.json
            let workspace_json = workspace_dir.join("workspace.json");
            let project = std::fs::read_to_string(&workspace_json)
                .ok()
                .and_then(|c| serde_json::from_str::<crate::models::WorkspaceJson>(&c).ok())
                .and_then(|ws| {
                    ws.folder
                        .map(|f| crate::workspace::decode_workspace_folder(&f))
                });

            // Filter by project path if specified
            if let Some(ref target) = target_path {
                if project
                    .as_ref()
                    .map(|p| crate::workspace::normalize_path(p) != *target)
                    .unwrap_or(true)
                {
                    continue;
                }
            }

            let project_display = project.clone().unwrap_or_else(|| "(none)".to_string());

            // List session files
            for session_entry in std::fs::read_dir(&chat_sessions_dir)?.filter_map(|e| e.ok()) {
                let session_path = session_entry.path();
                if !session_path.is_file() {
                    continue;
                }
                
                let ext = session_path.extension().and_then(|e| e.to_str());
                if ext != Some("json") && ext != Some("jsonl") {
                    continue;
                }

                let metadata = session_path.metadata().ok();
                let file_size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                total_size += file_size;

                let modified = metadata
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        let datetime: chrono::DateTime<chrono::Utc> = t.into();
                        datetime.format("%Y-%m-%d %H:%M").to_string()
                    })
                    .unwrap_or_else(|| "unknown".to_string());

                let session_file = session_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                // Try to get message count from the session file
                let messages = std::fs::read_to_string(&session_path)
                    .ok()
                    .map(|c| c.matches("\"message\":").count())
                    .unwrap_or(0);

                if show_size {
                    rows_with_size.push(SessionRowMultiWithSize {
                        provider: provider_name.clone(),
                        project_path: truncate_string(&project_display, 30),
                        session_file: truncate_string(&session_file, 20),
                        last_modified: modified,
                        messages,
                        size: format_file_size(file_size),
                    });
                } else {
                    rows.push(SessionRowMulti {
                        provider: provider_name.clone(),
                        project_path: truncate_string(&project_display, 30),
                        session_file: truncate_string(&session_file, 20),
                        last_modified: modified,
                        messages,
                    });
                }
            }
        }
    }

    if show_size {
        if rows_with_size.is_empty() {
            println!("No chat sessions found.");
            return Ok(());
        }
        let table = Table::new(&rows_with_size)
            .with(TableStyle::ascii_rounded())
            .to_string();
        println!("{}", table);
        println!(
            "\nTotal sessions: {} ({})",
            rows_with_size.len(),
            format_file_size(total_size)
        );
    } else {
        if rows.is_empty() {
            println!("No chat sessions found.");
            return Ok(());
        }
        let table = Table::new(&rows)
            .with(TableStyle::ascii_rounded())
            .to_string();
        println!("{}", table);
        println!("\nTotal sessions: {}", rows.len());
    }

    Ok(())
}

/// Find workspaces by search pattern
pub fn find_workspaces(pattern: &str) -> Result<()> {
    let workspaces = discover_workspaces()?;

    // Resolve "." to current directory name
    let pattern = if pattern == "." {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| pattern.to_string())
    } else {
        pattern.to_string()
    };
    let pattern_lower = pattern.to_lowercase();

    let matching: Vec<&Workspace> = workspaces
        .iter()
        .filter(|ws| {
            ws.project_path
                .as_ref()
                .map(|p| p.to_lowercase().contains(&pattern_lower))
                .unwrap_or(false)
                || ws.hash.to_lowercase().contains(&pattern_lower)
        })
        .collect();

    if matching.is_empty() {
        println!("No workspaces found matching '{}'", pattern);
        return Ok(());
    }

    let rows: Vec<WorkspaceRow> = matching
        .iter()
        .map(|ws| WorkspaceRow {
            hash: format!("{}...", &ws.hash[..12.min(ws.hash.len())]),
            project_path: ws
                .project_path
                .clone()
                .unwrap_or_else(|| "(none)".to_string()),
            sessions: ws.chat_session_count,
            has_chats: if ws.has_chat_sessions {
                "Yes".to_string()
            } else {
                "No".to_string()
            },
        })
        .collect();

    let table = Table::new(rows)
        .with(TableStyle::ascii_rounded())
        .to_string();

    println!("{}", table);
    println!("\nFound {} matching workspace(s)", matching.len());

    // Show session paths for each matching workspace
    for ws in &matching {
        if ws.has_chat_sessions {
            let project = ws.project_path.as_deref().unwrap_or("(none)");
            println!("\nSessions for {}:", project);

            if let Ok(sessions) =
                crate::workspace::get_chat_sessions_from_workspace(&ws.workspace_path)
            {
                for session_with_path in sessions {
                    println!("  {}", session_with_path.path.display());
                }
            }
        }
    }

    Ok(())
}

/// Find sessions by search pattern
#[allow(dead_code)]
pub fn find_sessions(pattern: &str, project_path: Option<&str>) -> Result<()> {
    let workspaces = discover_workspaces()?;
    let pattern_lower = pattern.to_lowercase();

    let filtered_workspaces: Vec<&Workspace> = if let Some(path) = project_path {
        let normalized = crate::workspace::normalize_path(path);
        workspaces
            .iter()
            .filter(|ws| {
                ws.project_path
                    .as_ref()
                    .map(|p| crate::workspace::normalize_path(p) == normalized)
                    .unwrap_or(false)
            })
            .collect()
    } else {
        workspaces.iter().collect()
    };

    let mut rows: Vec<SessionRow> = Vec::new();

    for ws in filtered_workspaces {
        if !ws.has_chat_sessions {
            continue;
        }

        let sessions = crate::workspace::get_chat_sessions_from_workspace(&ws.workspace_path)?;

        for session_with_path in sessions {
            // Check if session matches the pattern
            let session_id_matches = session_with_path
                .session
                .session_id
                .as_ref()
                .map(|id| id.to_lowercase().contains(&pattern_lower))
                .unwrap_or(false);
            let title_matches = session_with_path
                .session
                .title()
                .to_lowercase()
                .contains(&pattern_lower);
            let content_matches = session_with_path.session.requests.iter().any(|r| {
                r.message
                    .as_ref()
                    .map(|m| {
                        m.text
                            .as_ref()
                            .map(|t| t.to_lowercase().contains(&pattern_lower))
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
            });

            if !session_id_matches && !title_matches && !content_matches {
                continue;
            }

            let modified = session_with_path
                .path
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    let datetime: chrono::DateTime<chrono::Utc> = t.into();
                    datetime.format("%Y-%m-%d %H:%M").to_string()
                })
                .unwrap_or_else(|| "unknown".to_string());

            rows.push(SessionRow {
                project_path: ws
                    .project_path
                    .clone()
                    .unwrap_or_else(|| "(none)".to_string()),
                session_file: session_with_path
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                last_modified: modified,
                messages: session_with_path.session.request_count(),
            });
        }
    }

    if rows.is_empty() {
        println!("No sessions found matching '{}'", pattern);
        return Ok(());
    }

    let table = Table::new(&rows)
        .with(TableStyle::ascii_rounded())
        .to_string();

    println!("{}", table);
    println!("\nFound {} matching session(s)", rows.len());

    Ok(())
}

/// Optimized session search with filtering
///
/// This function is optimized for speed by:
/// 1. Filtering workspaces first (by name/path)
/// 2. Filtering by file modification date before reading content
/// 3. Only parsing JSON when needed
/// 4. Content search is opt-in (expensive)
/// 5. Parallel file scanning with rayon
pub fn find_sessions_filtered(
    pattern: &str,
    workspace_filter: Option<&str>,
    title_only: bool,
    search_content: bool,
    after: Option<&str>,
    before: Option<&str>,
    date: Option<&str>,
    all_workspaces: bool,
    provider: Option<&str>,
    all_providers: bool,
    limit: usize,
) -> Result<()> {
    use chrono::{NaiveDate, Utc};
    use rayon::prelude::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let pattern_lower = pattern.to_lowercase();

    // Parse date filters upfront
    let after_date = after.and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let before_date = before.and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let target_date = date.and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    // Determine which storage paths to scan based on provider filter
    let storage_paths = if all_providers {
        get_agent_storage_paths(Some("all"))?
    } else if let Some(p) = provider {
        get_agent_storage_paths(Some(p))?
    } else {
        // Default to VS Code only
        let vscode_path = crate::workspace::get_workspace_storage_path()?;
        if vscode_path.exists() {
            vec![("vscode".to_string(), vscode_path)]
        } else {
            vec![]
        }
    };

    if storage_paths.is_empty() {
        if let Some(p) = provider {
            println!("No storage found for provider: {}", p);
        } else {
            println!("No workspaces found");
        }
        return Ok(());
    }

    // Collect workspace directories with minimal I/O
    // If --all flag is set, don't filter by workspace
    let ws_filter_lower = if all_workspaces {
        None
    } else {
        workspace_filter.map(|s| s.to_lowercase())
    };

    let workspace_dirs: Vec<_> = storage_paths
        .iter()
        .flat_map(|(provider_name, storage_path)| {
            if !storage_path.exists() {
                return vec![];
            }
            std::fs::read_dir(storage_path)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|entry| {
                    let workspace_dir = entry.path();
                    let workspace_json_path = workspace_dir.join("workspace.json");

                    // Quick check: does chatSessions exist?
                    let chat_sessions_dir = workspace_dir.join("chatSessions");
                    if !chat_sessions_dir.exists() {
                        return None;
                    }

                    // Parse workspace.json for project path (needed for filtering)
                    let project_path =
                        std::fs::read_to_string(&workspace_json_path)
                            .ok()
                            .and_then(|content| {
                                serde_json::from_str::<crate::models::WorkspaceJson>(&content)
                                    .ok()
                                    .and_then(|ws| {
                                        ws.folder
                                            .map(|f| crate::workspace::decode_workspace_folder(&f))
                                    })
                            });

                    // Apply workspace filter early
                    if let Some(ref filter) = ws_filter_lower {
                        let hash = entry.file_name().to_string_lossy().to_lowercase();
                        let path_matches = project_path
                            .as_ref()
                            .map(|p| p.to_lowercase().contains(filter))
                            .unwrap_or(false);
                        if !hash.contains(filter) && !path_matches {
                            return None;
                        }
                    }

                    let ws_name = project_path
                        .as_ref()
                        .and_then(|p| std::path::Path::new(p).file_name())
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| {
                            entry.file_name().to_string_lossy()[..8.min(entry.file_name().len())]
                                .to_string()
                        });

                    Some((chat_sessions_dir, ws_name, provider_name.clone()))
                })
                .collect::<Vec<_>>()
        })
        .collect();

    if workspace_dirs.is_empty() {
        if let Some(ws) = workspace_filter {
            println!("No workspaces found matching '{}'", ws);
        } else {
            println!("No workspaces with chat sessions found");
        }
        return Ok(());
    }

    // Collect all session file paths
    let session_files: Vec<_> = workspace_dirs
        .iter()
        .flat_map(|(chat_dir, ws_name, provider_name)| {
            std::fs::read_dir(chat_dir)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "json" || ext == "jsonl")
                        .unwrap_or(false)
                })
                .map(|e| (e.path(), ws_name.clone(), provider_name.clone()))
                .collect::<Vec<_>>()
        })
        .collect();

    let total_files = session_files.len();
    let scanned = AtomicUsize::new(0);
    let skipped_by_date = AtomicUsize::new(0);

    // Process files in parallel
    let mut results: Vec<_> = session_files
        .par_iter()
        .filter_map(|(path, ws_name, provider_name)| {
            // Date filter using file metadata (very fast)
            if after_date.is_some() || before_date.is_some() {
                if let Ok(metadata) = path.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        let file_date: chrono::DateTime<Utc> = modified.into();
                        let file_naive = file_date.date_naive();

                        if let Some(after) = after_date {
                            if file_naive < after {
                                skipped_by_date.fetch_add(1, Ordering::Relaxed);
                                return None;
                            }
                        }
                        if let Some(before) = before_date {
                            if file_naive > before {
                                skipped_by_date.fetch_add(1, Ordering::Relaxed);
                                return None;
                            }
                        }
                    }
                }
            }

            scanned.fetch_add(1, Ordering::Relaxed);

            // Read file content once
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => return None,
            };

            // Check for internal message timestamps if --date filter is used
            if let Some(target) = target_date {
                // Look for timestamp fields in the JSON content
                // Timestamps are in milliseconds since epoch
                let has_matching_timestamp = content
                    .split("\"timestamp\":")
                    .skip(1) // Skip first split (before any timestamp)
                    .any(|part| {
                        // Extract the numeric value after "timestamp":
                        let num_str: String = part
                            .chars()
                            .skip_while(|c| c.is_whitespace())
                            .take_while(|c| c.is_ascii_digit())
                            .collect();
                        if let Ok(ts_ms) = num_str.parse::<i64>() {
                            if let Some(dt) = chrono::DateTime::from_timestamp_millis(ts_ms) {
                                return dt.date_naive() == target;
                            }
                        }
                        false
                    });

                if !has_matching_timestamp {
                    skipped_by_date.fetch_add(1, Ordering::Relaxed);
                    return None;
                }
            }

            // Extract title from content
            let title =
                extract_title_from_content(&content).unwrap_or_else(|| "Untitled".to_string());
            let title_lower = title.to_lowercase();

            // Check session ID from filename
            let session_id = path
                .file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let id_matches =
                !pattern_lower.is_empty() && session_id.to_lowercase().contains(&pattern_lower);

            // Check title match
            let title_matches = !pattern_lower.is_empty() && title_lower.contains(&pattern_lower);

            // Content search if requested
            let content_matches = if search_content
                && !title_only
                && !id_matches
                && !title_matches
                && !pattern_lower.is_empty()
            {
                content.to_lowercase().contains(&pattern_lower)
            } else {
                false
            };

            // Empty pattern matches everything (for listing)
            let matches =
                pattern_lower.is_empty() || id_matches || title_matches || content_matches;
            if !matches {
                return None;
            }

            let match_type = if pattern_lower.is_empty() {
                ""
            } else if id_matches {
                "ID"
            } else if title_matches {
                "title"
            } else {
                "content"
            };

            // Count messages from content (already loaded)
            let message_count = content.matches("\"message\":").count();

            // Get modification time
            let modified = path
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    let datetime: chrono::DateTime<chrono::Utc> = t.into();
                    datetime.format("%Y-%m-%d %H:%M").to_string()
                })
                .unwrap_or_else(|| "unknown".to_string());

            Some((
                title,
                ws_name.clone(),
                provider_name.clone(),
                modified,
                message_count,
                match_type.to_string(),
            ))
        })
        .collect();

    let scanned_count = scanned.load(Ordering::Relaxed);
    let skipped_count = skipped_by_date.load(Ordering::Relaxed);

    if results.is_empty() {
        println!("No sessions found matching '{}'", pattern);
        if skipped_count > 0 {
            println!("  ({} sessions skipped due to date filter)", skipped_count);
        }
        return Ok(());
    }

    // Sort by modification date (newest first)
    results.sort_by(|a, b| b.3.cmp(&a.3));

    // Apply limit
    results.truncate(limit);

    // Check if we have multiple providers to show provider column
    let show_provider_column = all_providers || storage_paths.len() > 1;

    #[derive(Tabled)]
    struct SearchResultRow {
        #[tabled(rename = "Title")]
        title: String,
        #[tabled(rename = "Workspace")]
        workspace: String,
        #[tabled(rename = "Modified")]
        modified: String,
        #[tabled(rename = "Msgs")]
        messages: usize,
        #[tabled(rename = "Match")]
        match_type: String,
    }

    #[derive(Tabled)]
    struct SearchResultRowWithProvider {
        #[tabled(rename = "Provider")]
        provider: String,
        #[tabled(rename = "Title")]
        title: String,
        #[tabled(rename = "Workspace")]
        workspace: String,
        #[tabled(rename = "Modified")]
        modified: String,
        #[tabled(rename = "Msgs")]
        messages: usize,
        #[tabled(rename = "Match")]
        match_type: String,
    }

    if show_provider_column {
        let rows: Vec<SearchResultRowWithProvider> = results
            .into_iter()
            .map(
                |(title, workspace, provider, modified, messages, match_type)| {
                    SearchResultRowWithProvider {
                        provider,
                        title: truncate_string(&title, 35),
                        workspace: truncate_string(&workspace, 15),
                        modified,
                        messages,
                        match_type,
                    }
                },
            )
            .collect();

        let table = Table::new(&rows)
            .with(TableStyle::ascii_rounded())
            .to_string();

        println!("{}", table);
        println!(
            "\nFound {} session(s) (scanned {} of {} files{})",
            rows.len(),
            scanned_count,
            total_files,
            if skipped_count > 0 {
                format!(", {} skipped by date", skipped_count)
            } else {
                String::new()
            }
        );
        if rows.len() >= limit {
            println!("  (results limited to {}; use --limit to show more)", limit);
        }
    } else {
        let rows: Vec<SearchResultRow> = results
            .into_iter()
            .map(
                |(title, workspace, _provider, modified, messages, match_type)| SearchResultRow {
                    title: truncate_string(&title, 40),
                    workspace: truncate_string(&workspace, 20),
                    modified,
                    messages,
                    match_type,
                },
            )
            .collect();

        let table = Table::new(&rows)
            .with(TableStyle::ascii_rounded())
            .to_string();

        println!("{}", table);
        println!(
            "\nFound {} session(s) (scanned {} of {} files{})",
            rows.len(),
            scanned_count,
            total_files,
            if skipped_count > 0 {
                format!(", {} skipped by date", skipped_count)
            } else {
                String::new()
            }
        );
        if rows.len() >= limit {
            println!("  (results limited to {}; use --limit to show more)", limit);
        }
    }

    Ok(())
}

/// Extract title from full JSON content (more reliable than header-only)
fn extract_title_from_content(content: &str) -> Option<String> {
    // Look for "customTitle" first (user-set title)
    if let Some(start) = content.find("\"customTitle\"") {
        if let Some(colon) = content[start..].find(':') {
            let after_colon = &content[start + colon + 1..];
            let trimmed = after_colon.trim_start();
            if let Some(stripped) = trimmed.strip_prefix('"') {
                if let Some(end) = stripped.find('"') {
                    let title = &stripped[..end];
                    if !title.is_empty() && title != "null" {
                        return Some(title.to_string());
                    }
                }
            }
        }
    }

    // Fall back to first request's message text
    if let Some(start) = content.find("\"text\"") {
        if let Some(colon) = content[start..].find(':') {
            let after_colon = &content[start + colon + 1..];
            let trimmed = after_colon.trim_start();
            if let Some(stripped) = trimmed.strip_prefix('"') {
                if let Some(end) = stripped.find('"') {
                    let title = &stripped[..end];
                    if !title.is_empty() && title.len() < 100 {
                        return Some(title.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Fast title extraction from JSON header
#[allow(dead_code)]
fn extract_title_fast(header: &str) -> Option<String> {
    extract_title_from_content(header)
}

/// Truncate string to max length with ellipsis
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Show workspace details
pub fn show_workspace(workspace: &str) -> Result<()> {
    use colored::Colorize;

    let workspaces = discover_workspaces()?;
    let workspace_lower = workspace.to_lowercase();

    // Find workspace by name or hash
    let matching: Vec<&Workspace> = workspaces
        .iter()
        .filter(|ws| {
            ws.hash.to_lowercase().contains(&workspace_lower)
                || ws
                    .project_path
                    .as_ref()
                    .map(|p| p.to_lowercase().contains(&workspace_lower))
                    .unwrap_or(false)
        })
        .collect();

    if matching.is_empty() {
        println!(
            "{} No workspace found matching '{}'",
            "!".yellow(),
            workspace
        );
        return Ok(());
    }

    for ws in matching {
        println!("\n{}", "=".repeat(60).bright_blue());
        println!("{}", "Workspace Details".bright_blue().bold());
        println!("{}", "=".repeat(60).bright_blue());

        println!("{}: {}", "Hash".bright_white().bold(), ws.hash);
        println!(
            "{}: {}",
            "Path".bright_white().bold(),
            ws.project_path.as_ref().unwrap_or(&"(none)".to_string())
        );
        println!(
            "{}: {}",
            "Has Sessions".bright_white().bold(),
            if ws.has_chat_sessions {
                "Yes".green()
            } else {
                "No".red()
            }
        );
        println!(
            "{}: {}",
            "Workspace Path".bright_white().bold(),
            ws.workspace_path.display()
        );

        if ws.has_chat_sessions {
            let sessions = crate::workspace::get_chat_sessions_from_workspace(&ws.workspace_path)?;
            println!(
                "{}: {}",
                "Session Count".bright_white().bold(),
                sessions.len()
            );

            if !sessions.is_empty() {
                println!("\n{}", "Sessions:".bright_yellow());
                for (i, s) in sessions.iter().enumerate() {
                    let title = s.session.title();
                    let msg_count = s.session.request_count();
                    println!(
                        "  {}. {} ({} messages)",
                        i + 1,
                        title.bright_cyan(),
                        msg_count
                    );
                }
            }
        }
    }

    Ok(())
}

/// Show session details
pub fn show_session(session_id: &str, project_path: Option<&str>) -> Result<()> {
    use colored::Colorize;

    let workspaces = discover_workspaces()?;
    let session_id_lower = session_id.to_lowercase();

    let filtered_workspaces: Vec<&Workspace> = if let Some(path) = project_path {
        let normalized = crate::workspace::normalize_path(path);
        workspaces
            .iter()
            .filter(|ws| {
                ws.project_path
                    .as_ref()
                    .map(|p| crate::workspace::normalize_path(p) == normalized)
                    .unwrap_or(false)
            })
            .collect()
    } else {
        workspaces.iter().collect()
    };

    for ws in filtered_workspaces {
        if !ws.has_chat_sessions {
            continue;
        }

        let sessions = crate::workspace::get_chat_sessions_from_workspace(&ws.workspace_path)?;

        for s in sessions {
            let filename = s
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let matches = s
                .session
                .session_id
                .as_ref()
                .map(|id| id.to_lowercase().contains(&session_id_lower))
                .unwrap_or(false)
                || filename.to_lowercase().contains(&session_id_lower);

            if matches {
                // Detect format from file extension
                let format = VsCodeSessionFormat::from_path(&s.path);

                println!("\n{}", "=".repeat(60).bright_blue());
                println!("{}", "Session Details".bright_blue().bold());
                println!("{}", "=".repeat(60).bright_blue());

                println!(
                    "{}: {}",
                    "Title".bright_white().bold(),
                    s.session.title().bright_cyan()
                );
                println!("{}: {}", "File".bright_white().bold(), filename);
                println!(
                    "{}: {}",
                    "Format".bright_white().bold(),
                    format.to_string().bright_magenta()
                );
                println!(
                    "{}: {}",
                    "Session ID".bright_white().bold(),
                    s.session
                        .session_id
                        .as_ref()
                        .unwrap_or(&"(none)".to_string())
                );
                println!(
                    "{}: {}",
                    "Messages".bright_white().bold(),
                    s.session.request_count()
                );
                println!(
                    "{}: {}",
                    "Workspace".bright_white().bold(),
                    ws.project_path.as_ref().unwrap_or(&"(none)".to_string())
                );

                // Show first few messages as preview
                println!("\n{}", "Preview:".bright_yellow());
                for (i, req) in s.session.requests.iter().take(3).enumerate() {
                    if let Some(msg) = &req.message {
                        if let Some(text) = &msg.text {
                            let preview: String = text.chars().take(100).collect();
                            let truncated = if text.len() > 100 { "..." } else { "" };
                            println!("  {}. {}{}", i + 1, preview.dimmed(), truncated);
                        }
                    }
                }

                return Ok(());
            }
        }
    }

    println!(
        "{} No session found matching '{}'",
        "!".yellow(),
        session_id
    );
    Ok(())
}

/// Get storage paths for agent mode sessions based on provider filter
/// Returns (provider_name, storage_path) tuples
fn get_agent_storage_paths(provider: Option<&str>) -> Result<Vec<(String, std::path::PathBuf)>> {
    let mut paths = Vec::new();

    // VS Code path
    let vscode_path = crate::workspace::get_workspace_storage_path()?;

    // Other provider paths
    let cursor_path = get_cursor_storage_path();
    let claudecode_path = get_claudecode_storage_path();
    let opencode_path = get_opencode_storage_path();
    let openclaw_path = get_openclaw_storage_path();
    let antigravity_path = get_antigravity_storage_path();

    match provider {
        None => {
            // Default: return VS Code path only for backward compatibility
            if vscode_path.exists() {
                paths.push(("vscode".to_string(), vscode_path));
            }
        }
        Some("all") => {
            // All providers that support agent mode
            if vscode_path.exists() {
                paths.push(("vscode".to_string(), vscode_path));
            }
            if let Some(cp) = cursor_path {
                if cp.exists() {
                    paths.push(("cursor".to_string(), cp));
                }
            }
            if let Some(cc) = claudecode_path {
                if cc.exists() {
                    paths.push(("claudecode".to_string(), cc));
                }
            }
            if let Some(oc) = opencode_path {
                if oc.exists() {
                    paths.push(("opencode".to_string(), oc));
                }
            }
            if let Some(ocl) = openclaw_path {
                if ocl.exists() {
                    paths.push(("openclaw".to_string(), ocl));
                }
            }
            if let Some(ag) = antigravity_path {
                if ag.exists() {
                    paths.push(("antigravity".to_string(), ag));
                }
            }
        }
        Some(p) => {
            let p_lower = p.to_lowercase();
            match p_lower.as_str() {
                "vscode" | "vs-code" | "copilot" => {
                    if vscode_path.exists() {
                        paths.push(("vscode".to_string(), vscode_path));
                    }
                }
                "cursor" => {
                    if let Some(cp) = cursor_path {
                        if cp.exists() {
                            paths.push(("cursor".to_string(), cp));
                        }
                    }
                }
                "claudecode" | "claude-code" | "claude" => {
                    if let Some(cc) = claudecode_path {
                        if cc.exists() {
                            paths.push(("claudecode".to_string(), cc));
                        }
                    }
                }
                "opencode" | "open-code" => {
                    if let Some(oc) = opencode_path {
                        if oc.exists() {
                            paths.push(("opencode".to_string(), oc));
                        }
                    }
                }
                "openclaw" | "open-claw" | "claw" => {
                    if let Some(ocl) = openclaw_path {
                        if ocl.exists() {
                            paths.push(("openclaw".to_string(), ocl));
                        }
                    }
                }
                "antigravity" | "anti-gravity" | "ag" => {
                    if let Some(ag) = antigravity_path {
                        if ag.exists() {
                            paths.push(("antigravity".to_string(), ag));
                        }
                    }
                }
                _ => {
                    // Unknown provider - return empty to trigger error message
                }
            }
        }
    }

    Ok(paths)
}

/// Get Cursor's workspace storage path
fn get_cursor_storage_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = dirs::data_dir() {
            let cursor_path = appdata.join("Cursor").join("User").join("workspaceStorage");
            if cursor_path.exists() {
                return Some(cursor_path);
            }
        }
        if let Ok(roaming) = std::env::var("APPDATA") {
            let roaming_path = std::path::PathBuf::from(roaming)
                .join("Cursor")
                .join("User")
                .join("workspaceStorage");
            if roaming_path.exists() {
                return Some(roaming_path);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
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
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(config) = dirs::config_dir() {
            let cursor_path = config.join("Cursor").join("User").join("workspaceStorage");
            if cursor_path.exists() {
                return Some(cursor_path);
            }
        }
    }

    None
}

/// Get ClaudeCode's storage path (Anthropic's Claude Code CLI)
fn get_claudecode_storage_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // ClaudeCode stores sessions in AppData
        if let Ok(appdata) = std::env::var("APPDATA") {
            let claude_path = std::path::PathBuf::from(&appdata)
                .join("claude-code")
                .join("sessions");
            if claude_path.exists() {
                return Some(claude_path);
            }
            // Alternative path with different naming
            let alt_path = std::path::PathBuf::from(&appdata)
                .join("ClaudeCode")
                .join("workspaceStorage");
            if alt_path.exists() {
                return Some(alt_path);
            }
        }
        if let Some(local) = dirs::data_local_dir() {
            let local_path = local.join("ClaudeCode").join("sessions");
            if local_path.exists() {
                return Some(local_path);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let claude_path = home
                .join("Library")
                .join("Application Support")
                .join("claude-code")
                .join("sessions");
            if claude_path.exists() {
                return Some(claude_path);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(config) = dirs::config_dir() {
            let claude_path = config.join("claude-code").join("sessions");
            if claude_path.exists() {
                return Some(claude_path);
            }
        }
    }

    None
}

/// Get OpenCode's storage path (open-source coding assistant)
fn get_opencode_storage_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let opencode_path = std::path::PathBuf::from(&appdata)
                .join("OpenCode")
                .join("workspaceStorage");
            if opencode_path.exists() {
                return Some(opencode_path);
            }
        }
        if let Some(local) = dirs::data_local_dir() {
            let local_path = local.join("OpenCode").join("sessions");
            if local_path.exists() {
                return Some(local_path);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let opencode_path = home
                .join("Library")
                .join("Application Support")
                .join("OpenCode")
                .join("workspaceStorage");
            if opencode_path.exists() {
                return Some(opencode_path);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(config) = dirs::config_dir() {
            let opencode_path = config.join("opencode").join("workspaceStorage");
            if opencode_path.exists() {
                return Some(opencode_path);
            }
        }
    }

    None
}

/// Get OpenClaw's storage path
fn get_openclaw_storage_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let openclaw_path = std::path::PathBuf::from(&appdata)
                .join("OpenClaw")
                .join("workspaceStorage");
            if openclaw_path.exists() {
                return Some(openclaw_path);
            }
        }
        if let Some(local) = dirs::data_local_dir() {
            let local_path = local.join("OpenClaw").join("sessions");
            if local_path.exists() {
                return Some(local_path);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let openclaw_path = home
                .join("Library")
                .join("Application Support")
                .join("OpenClaw")
                .join("workspaceStorage");
            if openclaw_path.exists() {
                return Some(openclaw_path);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(config) = dirs::config_dir() {
            let openclaw_path = config.join("openclaw").join("workspaceStorage");
            if openclaw_path.exists() {
                return Some(openclaw_path);
            }
        }
    }

    None
}

/// Get Antigravity's storage path
fn get_antigravity_storage_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let antigrav_path = std::path::PathBuf::from(&appdata)
                .join("Antigravity")
                .join("workspaceStorage");
            if antigrav_path.exists() {
                return Some(antigrav_path);
            }
        }
        if let Some(local) = dirs::data_local_dir() {
            let local_path = local.join("Antigravity").join("sessions");
            if local_path.exists() {
                return Some(local_path);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let antigrav_path = home
                .join("Library")
                .join("Application Support")
                .join("Antigravity")
                .join("workspaceStorage");
            if antigrav_path.exists() {
                return Some(antigrav_path);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(config) = dirs::config_dir() {
            let antigrav_path = config.join("antigravity").join("workspaceStorage");
            if antigrav_path.exists() {
                return Some(antigrav_path);
            }
        }
    }

    None
}

/// List agent mode sessions (chatEditingSessions / Copilot Edits)
pub fn list_agents_sessions(
    project_path: Option<&str>,
    show_size: bool,
    provider: Option<&str>,
) -> Result<()> {
    

    // Get storage paths based on provider filter
    let storage_paths = get_agent_storage_paths(provider)?;

    if storage_paths.is_empty() {
        if let Some(p) = provider {
            println!("No storage found for provider: {}", p);
            println!("\nSupported providers: vscode, cursor, claudecode, opencode, openclaw, antigravity");
        } else {
            println!("No workspaces found");
        }
        return Ok(());
    }

    #[derive(Tabled)]
    struct AgentSessionRow {
        #[tabled(rename = "Provider")]
        provider: String,
        #[tabled(rename = "Project")]
        project: String,
        #[tabled(rename = "Session ID")]
        session_id: String,
        #[tabled(rename = "Last Modified")]
        last_modified: String,
        #[tabled(rename = "Files")]
        file_count: usize,
    }

    #[derive(Tabled)]
    struct AgentSessionRowWithSize {
        #[tabled(rename = "Provider")]
        provider: String,
        #[tabled(rename = "Project")]
        project: String,
        #[tabled(rename = "Session ID")]
        session_id: String,
        #[tabled(rename = "Last Modified")]
        last_modified: String,
        #[tabled(rename = "Files")]
        file_count: usize,
        #[tabled(rename = "Size")]
        size: String,
    }

    let target_path = project_path.map(|p| crate::workspace::normalize_path(p));
    let mut total_size: u64 = 0;
    let mut rows_with_size: Vec<AgentSessionRowWithSize> = Vec::new();
    let mut rows: Vec<AgentSessionRow> = Vec::new();

    for (provider_name, storage_path) in &storage_paths {
        if !storage_path.exists() {
            continue;
        }

        for entry in std::fs::read_dir(storage_path)?.filter_map(|e| e.ok()) {
            let workspace_dir = entry.path();
            if !workspace_dir.is_dir() {
                continue;
            }

            let agent_sessions_dir = workspace_dir.join("chatEditingSessions");
            if !agent_sessions_dir.exists() {
                continue;
            }

            // Get project path from workspace.json
            let workspace_json = workspace_dir.join("workspace.json");
            let project = std::fs::read_to_string(&workspace_json)
                .ok()
                .and_then(|c| serde_json::from_str::<crate::models::WorkspaceJson>(&c).ok())
                .and_then(|ws| {
                    ws.folder
                        .map(|f| crate::workspace::decode_workspace_folder(&f))
                });

            // Filter by project path if specified
            if let Some(ref target) = target_path {
                if project
                    .as_ref()
                    .map(|p| crate::workspace::normalize_path(p) != *target)
                    .unwrap_or(true)
                {
                    continue;
                }
            }

            let project_name = project
                .as_ref()
                .and_then(|p| std::path::Path::new(p).file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| entry.file_name().to_string_lossy()[..8].to_string());

            // List agent session directories
            for session_entry in std::fs::read_dir(&agent_sessions_dir)?.filter_map(|e| e.ok()) {
                let session_dir = session_entry.path();
                if !session_dir.is_dir() {
                    continue;
                }

                let session_id = session_entry.file_name().to_string_lossy().to_string();
                let short_id = if session_id.len() > 8 {
                    format!("{}...", &session_id[..8])
                } else {
                    session_id.clone()
                };

                // Get last modified time and file count
                let mut last_mod = std::time::SystemTime::UNIX_EPOCH;
                let mut file_count = 0;
                let mut session_size: u64 = 0;

                if let Ok(files) = std::fs::read_dir(&session_dir) {
                    for file in files.filter_map(|f| f.ok()) {
                        file_count += 1;
                        if let Ok(meta) = file.metadata() {
                            session_size += meta.len();
                            if let Ok(mod_time) = meta.modified() {
                                if mod_time > last_mod {
                                    last_mod = mod_time;
                                }
                            }
                        }
                    }
                }

                total_size += session_size;

                let modified = if last_mod != std::time::SystemTime::UNIX_EPOCH {
                    let datetime: chrono::DateTime<chrono::Utc> = last_mod.into();
                    datetime.format("%Y-%m-%d %H:%M").to_string()
                } else {
                    "unknown".to_string()
                };

                if show_size {
                    rows_with_size.push(AgentSessionRowWithSize {
                        provider: provider_name.clone(),
                        project: project_name.clone(),
                        session_id: short_id,
                        last_modified: modified,
                        file_count,
                        size: format_file_size(session_size),
                    });
                } else {
                    rows.push(AgentSessionRow {
                        provider: provider_name.clone(),
                        project: project_name.clone(),
                        session_id: short_id,
                        last_modified: modified,
                        file_count,
                    });
                }
            }
        }
    }

    if show_size {
        if rows_with_size.is_empty() {
            println!("No agent mode sessions found.");
            return Ok(());
        }
        let table = Table::new(&rows_with_size)
            .with(TableStyle::ascii_rounded())
            .to_string();
        println!("{}", table);
        println!(
            "\nTotal agent sessions: {} ({})",
            rows_with_size.len(),
            format_file_size(total_size)
        );
    } else {
        if rows.is_empty() {
            println!("No agent mode sessions found.");
            return Ok(());
        }
        let table = Table::new(&rows)
            .with(TableStyle::ascii_rounded())
            .to_string();
        println!("{}", table);
        println!("\nTotal agent sessions: {}", rows.len());
    }

    Ok(())
}

/// Show agent mode session details
pub fn show_agent_session(session_id: &str, project_path: Option<&str>) -> Result<()> {
    use colored::*;

    let storage_path = crate::workspace::get_workspace_storage_path()?;
    let session_id_lower = session_id.to_lowercase();
    let target_path = project_path.map(|p| crate::workspace::normalize_path(p));

    for entry in std::fs::read_dir(&storage_path)?.filter_map(|e| e.ok()) {
        let workspace_dir = entry.path();
        if !workspace_dir.is_dir() {
            continue;
        }

        let agent_sessions_dir = workspace_dir.join("chatEditingSessions");
        if !agent_sessions_dir.exists() {
            continue;
        }

        // Get project path
        let workspace_json = workspace_dir.join("workspace.json");
        let project = std::fs::read_to_string(&workspace_json)
            .ok()
            .and_then(|c| serde_json::from_str::<crate::models::WorkspaceJson>(&c).ok())
            .and_then(|ws| {
                ws.folder
                    .map(|f| crate::workspace::decode_workspace_folder(&f))
            });

        // Filter by project path if specified
        if let Some(ref target) = target_path {
            if project
                .as_ref()
                .map(|p| crate::workspace::normalize_path(p) != *target)
                .unwrap_or(true)
            {
                continue;
            }
        }

        for session_entry in std::fs::read_dir(&agent_sessions_dir)?.filter_map(|e| e.ok()) {
            let full_id = session_entry.file_name().to_string_lossy().to_string();
            if !full_id.to_lowercase().contains(&session_id_lower) {
                continue;
            }

            let session_dir = session_entry.path();

            println!("\n{}", "=".repeat(60).bright_blue());
            println!("{}", "Agent Session Details".bright_blue().bold());
            println!("{}", "=".repeat(60).bright_blue());

            println!(
                "{}: {}",
                "Session ID".bright_white().bold(),
                full_id.bright_cyan()
            );
            println!(
                "{}: {}",
                "Project".bright_white().bold(),
                project.as_deref().unwrap_or("(none)")
            );
            println!(
                "{}: {}",
                "Path".bright_white().bold(),
                session_dir.display()
            );

            // List files in the session
            println!("\n{}", "Session Files:".bright_yellow());
            let mut total_size: u64 = 0;
            if let Ok(files) = std::fs::read_dir(&session_dir) {
                for file in files.filter_map(|f| f.ok()) {
                    let _path = file.path();
                    let name = file.file_name().to_string_lossy().to_string();
                    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
                    total_size += size;
                    println!("  {} ({})", name.dimmed(), format_file_size(size));
                }
            }
            println!(
                "\n{}: {}",
                "Total Size".bright_white().bold(),
                format_file_size(total_size)
            );

            return Ok(());
        }
    }

    println!(
        "{} No agent session found matching '{}'",
        "!".yellow(),
        session_id
    );
    Ok(())
}

/// Show timeline of session activity with gap visualization
pub fn show_timeline(
    project_path: Option<&str>,
    include_agents: bool,
    provider: Option<&str>,
    all_providers: bool,
) -> Result<()> {
    use colored::*;
    use std::collections::BTreeMap;

    // Determine which storage paths to scan
    let storage_paths = if all_providers {
        get_agent_storage_paths(Some("all"))?
    } else if let Some(p) = provider {
        get_agent_storage_paths(Some(p))?
    } else {
        // Default to VS Code only
        let vscode_path = crate::workspace::get_workspace_storage_path()?;
        if vscode_path.exists() {
            vec![("vscode".to_string(), vscode_path)]
        } else {
            vec![]
        }
    };

    if storage_paths.is_empty() {
        if let Some(p) = provider {
            println!("No storage found for provider: {}", p);
        } else {
            println!("No workspaces found");
        }
        return Ok(());
    }

    let target_path = project_path.map(|p| crate::workspace::normalize_path(p));

    // Collect all session dates (date -> (chat_count, agent_count, provider))
    let mut date_activity: BTreeMap<chrono::NaiveDate, (usize, usize)> = BTreeMap::new();
    let mut project_name = String::new();
    let mut providers_scanned: Vec<String> = Vec::new();

    for (provider_name, storage_path) in &storage_paths {
        if !storage_path.exists() {
            continue;
        }
        providers_scanned.push(provider_name.clone());

        for entry in std::fs::read_dir(storage_path)?.filter_map(|e| e.ok()) {
            let workspace_dir = entry.path();
            if !workspace_dir.is_dir() {
                continue;
            }

            // Get project path
            let workspace_json = workspace_dir.join("workspace.json");
            let project = std::fs::read_to_string(&workspace_json)
                .ok()
                .and_then(|c| serde_json::from_str::<crate::models::WorkspaceJson>(&c).ok())
                .and_then(|ws| {
                    ws.folder
                        .map(|f| crate::workspace::decode_workspace_folder(&f))
                });

            // Filter by project path if specified
            if let Some(ref target) = target_path {
                if project
                    .as_ref()
                    .map(|p| crate::workspace::normalize_path(p) != *target)
                    .unwrap_or(true)
                {
                    continue;
                }
                if project_name.is_empty() {
                    project_name = std::path::Path::new(target)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| target.clone());
                }
            }

            // Scan chatSessions
            let chat_sessions_dir = workspace_dir.join("chatSessions");
            if chat_sessions_dir.exists() {
                if let Ok(files) = std::fs::read_dir(&chat_sessions_dir) {
                    for file in files.filter_map(|f| f.ok()) {
                        if let Ok(meta) = file.metadata() {
                            if let Ok(modified) = meta.modified() {
                                let datetime: chrono::DateTime<chrono::Utc> = modified.into();
                                let date = datetime.date_naive();
                                let entry = date_activity.entry(date).or_insert((0, 0));
                                entry.0 += 1;
                            }
                        }
                    }
                }
            }

            // Scan chatEditingSessions (agent mode) if requested
            if include_agents {
                let agent_sessions_dir = workspace_dir.join("chatEditingSessions");
                if agent_sessions_dir.exists() {
                    if let Ok(dirs) = std::fs::read_dir(&agent_sessions_dir) {
                        for dir in dirs.filter_map(|d| d.ok()) {
                            if let Ok(meta) = dir.metadata() {
                                if let Ok(modified) = meta.modified() {
                                    let datetime: chrono::DateTime<chrono::Utc> = modified.into();
                                    let date = datetime.date_naive();
                                    let entry = date_activity.entry(date).or_insert((0, 0));
                                    entry.1 += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if date_activity.is_empty() {
        println!("No session activity found.");
        return Ok(());
    }

    let title = if project_name.is_empty() {
        "All Workspaces".to_string()
    } else {
        project_name
    };

    let provider_info = if providers_scanned.len() > 1 || all_providers {
        format!(" ({})", providers_scanned.join(", "))
    } else {
        String::new()
    };

    println!(
        "\n{} Session Timeline: {}{}",
        "[*]".blue(),
        title.cyan(),
        provider_info.dimmed()
    );
    println!("{}", "=".repeat(60));

    let dates: Vec<_> = date_activity.keys().collect();
    let first_date = **dates.first().unwrap();
    let last_date = **dates.last().unwrap();

    println!(
        "Range: {} to {}",
        first_date.format("%Y-%m-%d"),
        last_date.format("%Y-%m-%d")
    );
    println!();

    // Find gaps (more than 1 day between sessions)
    let mut gaps: Vec<(chrono::NaiveDate, chrono::NaiveDate, i64)> = Vec::new();
    let mut prev_date: Option<chrono::NaiveDate> = None;

    for date in dates.iter() {
        if let Some(prev) = prev_date {
            let diff = (**date - prev).num_days();
            if diff > 1 {
                gaps.push((prev, **date, diff));
            }
        }
        prev_date = Some(**date);
    }

    // Show recent activity (last 14 days worth)
    println!("{}", "Recent Activity:".bright_yellow());
    let recent_dates: Vec<_> = date_activity.iter().rev().take(14).collect();
    for (date, (chats, agents)) in recent_dates.iter().rev() {
        let chat_bar = "█".repeat((*chats).min(20));
        let agent_bar = if include_agents && *agents > 0 {
            format!(" {}", "▓".repeat((*agents).min(10)).bright_magenta())
        } else {
            String::new()
        };
        println!(
            "  {} │ {}{}",
            date.format("%Y-%m-%d"),
            chat_bar.bright_green(),
            agent_bar
        );
    }

    // Show gaps
    if !gaps.is_empty() {
        println!("\n{}", "Gaps (>1 day):".bright_red());
        for (start, end, days) in gaps.iter().take(10) {
            println!(
                "  {} → {} ({} days)",
                start.format("%Y-%m-%d"),
                end.format("%Y-%m-%d"),
                days
            );
        }
        if gaps.len() > 10 {
            println!("  ... and {} more gaps", gaps.len() - 10);
        }
    }

    // Summary
    let total_chats: usize = date_activity.values().map(|(c, _)| c).sum();
    let total_agents: usize = date_activity.values().map(|(_, a)| a).sum();
    let total_days = date_activity.len();
    let total_gap_days: i64 = gaps.iter().map(|(_, _, d)| d - 1).sum();

    println!("\n{}", "Summary:".bright_white().bold());
    println!("  Active days: {}", total_days);
    println!("  Chat sessions: {}", total_chats);
    if include_agents {
        println!("  Agent sessions: {}", total_agents);
    }
    println!("  Total gap days: {}", total_gap_days);

    if include_agents {
        println!(
            "\n{} {} = chat, {} = agent",
            "Legend:".dimmed(),
            "█".bright_green(),
            "▓".bright_magenta()
        );
    }

    Ok(())
}
