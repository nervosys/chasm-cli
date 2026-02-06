// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Auto-detection commands for workspaces and providers

use anyhow::Result;
use colored::*;

use crate::models::Workspace;
use crate::providers::{ProviderRegistry, ProviderType};
use crate::workspace::{
    discover_workspaces, find_workspace_by_path, get_chat_sessions_from_workspace,
};

/// Detect workspace information for a given path
pub fn detect_workspace(path: Option<&str>) -> Result<()> {
    let project_path = path.map(|p| p.to_string()).unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    println!("\n{} Detecting Workspace", "[D]".blue().bold());
    println!("{}", "=".repeat(60));
    println!("{} Path: {}", "[*]".blue(), project_path.cyan());

    match find_workspace_by_path(&project_path)? {
        Some((ws_id, ws_dir, ws_name)) => {
            println!("\n{} Workspace Found!", "[+]".green().bold());
            println!("   {} ID: {}", "[*]".blue(), &ws_id[..16.min(ws_id.len())]);
            println!("   {} Directory: {}", "[*]".blue(), ws_dir.display());
            if let Some(name) = ws_name {
                println!("   {} Name: {}", "[*]".blue(), name.cyan());
            }

            // Get session count
            if let Ok(sessions) = get_chat_sessions_from_workspace(&ws_dir) {
                println!("   {} Sessions: {}", "[*]".blue(), sessions.len());

                if !sessions.is_empty() {
                    let total_messages: usize =
                        sessions.iter().map(|s| s.session.request_count()).sum();
                    println!("   {} Total Messages: {}", "[*]".blue(), total_messages);
                }
            }

            // Detect provider
            println!("\n{} Provider Detection:", "[*]".blue());
            println!(
                "   {} Provider: {}",
                "[*]".blue(),
                "GitHub Copilot (VS Code)".cyan()
            );
        }
        None => {
            println!("\n{} No workspace found for this path", "[X]".red());
            println!(
                "{} The project may not have been opened in VS Code yet",
                "[i]".yellow()
            );

            // Check if there are similar workspaces
            let all_workspaces = discover_workspaces()?;
            let path_lower = project_path.to_lowercase();
            let similar: Vec<&Workspace> = all_workspaces
                .iter()
                .filter(|ws| {
                    ws.project_path
                        .as_ref()
                        .map(|p| {
                            p.to_lowercase().contains(&path_lower)
                                || path_lower.contains(&p.to_lowercase())
                        })
                        .unwrap_or(false)
                })
                .take(5)
                .collect();

            if !similar.is_empty() {
                println!("\n{} Similar workspaces found:", "[i]".cyan());
                for ws in similar {
                    if let Some(p) = &ws.project_path {
                        println!("   {} {}...", p.cyan(), &ws.hash[..8.min(ws.hash.len())]);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Detect available providers and their status
pub fn detect_providers(with_sessions: bool) -> Result<()> {
    println!("\n{} Detecting Providers", "[D]".blue().bold());
    println!("{}", "=".repeat(60));

    let registry = ProviderRegistry::new();
    let mut found_count = 0;
    let mut with_sessions_count = 0;

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

    for provider_type in all_provider_types {
        if let Some(provider) = registry.get_provider(provider_type) {
            let available = provider.is_available();
            let session_count = if available {
                provider.list_sessions().map(|s| s.len()).unwrap_or(0)
            } else {
                0
            };

            if with_sessions && session_count == 0 {
                continue;
            }

            found_count += 1;
            if session_count > 0 {
                with_sessions_count += 1;
            }

            let status = if available {
                if session_count > 0 {
                    format!(
                        "{} ({} sessions)",
                        "+".green(),
                        session_count.to_string().cyan()
                    )
                } else {
                    format!("{} (no sessions)", "+".green())
                }
            } else {
                format!("{} not available", "x".red())
            };

            println!("   {} {}: {}", "[*]".blue(), provider.name().bold(), status);

            // Show endpoint for API-based providers
            if available {
                if let Some(endpoint) = provider_type.default_endpoint() {
                    println!("      {} Endpoint: {}", "`".dimmed(), endpoint.dimmed());
                }
                if let Some(path) = provider.sessions_path() {
                    println!(
                        "      {} Path: {}",
                        "`".dimmed(),
                        path.display().to_string().dimmed()
                    );
                }
            }
        }
    }

    println!("\n{} Summary:", "[*]".green().bold());
    println!("   {} providers available", found_count.to_string().cyan());
    println!(
        "   {} providers with sessions",
        with_sessions_count.to_string().cyan()
    );

    Ok(())
}

/// Detect which provider a session belongs to
pub fn detect_session(session_id: &str, path: Option<&str>) -> Result<()> {
    println!("\n{} Detecting Session Provider", "[D]".blue().bold());
    println!("{}", "=".repeat(60));
    println!("{} Session: {}", "[*]".blue(), session_id.cyan());

    let registry = ProviderRegistry::new();
    let session_lower = session_id.to_lowercase();
    let mut found = false;

    // First check VS Code workspaces
    let project_path = path.map(|p| p.to_string()).unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    // Check in VS Code/Copilot workspaces
    if let Ok(Some((_ws_id, ws_dir, ws_name))) = find_workspace_by_path(&project_path) {
        if let Ok(sessions) = get_chat_sessions_from_workspace(&ws_dir) {
            for swp in &sessions {
                let sid = swp
                    .session
                    .session_id
                    .as_ref()
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();
                let title = swp.session.title().to_lowercase();
                let filename = swp
                    .path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_lowercase())
                    .unwrap_or_default();

                if sid.contains(&session_lower)
                    || title.contains(&session_lower)
                    || filename.contains(&session_lower)
                {
                    found = true;
                    println!("\n{} Session Found!", "[+]".green().bold());
                    println!("   {} Provider: {}", "[*]".blue(), "GitHub Copilot".cyan());
                    println!("   {} Title: {}", "[*]".blue(), swp.session.title());
                    println!("   {} File: {}", "[*]".blue(), swp.path.display());
                    println!(
                        "   {} Messages: {}",
                        "[*]".blue(),
                        swp.session.request_count()
                    );
                    if let Some(name) = &ws_name {
                        println!("   {} Workspace: {}", "[*]".blue(), name);
                    }
                    break;
                }
            }
        }
    }

    // Check other providers
    if !found {
        let provider_types = vec![
            ProviderType::Cursor,
            ProviderType::Ollama,
            ProviderType::Jan,
            ProviderType::Gpt4All,
            ProviderType::LmStudio,
        ];

        for provider_type in provider_types {
            if let Some(provider) = registry.get_provider(provider_type) {
                if provider.is_available() {
                    if let Ok(sessions) = provider.list_sessions() {
                        for session in sessions {
                            let sid = session
                                .session_id
                                .as_ref()
                                .map(|s| s.to_lowercase())
                                .unwrap_or_default();
                            let title = session.title().to_lowercase();

                            if sid.contains(&session_lower) || title.contains(&session_lower) {
                                found = true;
                                println!("\n{} Session Found!", "[+]".green().bold());
                                println!(
                                    "   {} Provider: {}",
                                    "[*]".blue(),
                                    provider.name().cyan()
                                );
                                println!("   {} Title: {}", "[*]".blue(), session.title());
                                println!(
                                    "   {} Messages: {}",
                                    "[*]".blue(),
                                    session.request_count()
                                );
                                break;
                            }
                        }
                    }
                }
            }
            if found {
                break;
            }
        }
    }

    if !found {
        println!("\n{} Session not found", "[X]".red());
        println!(
            "{} Try providing a more specific session ID or check the path",
            "[i]".yellow()
        );
    }

    Ok(())
}

/// Detect everything (workspace, providers, sessions) for a path
pub fn detect_all(path: Option<&str>, verbose: bool) -> Result<()> {
    let project_path = path.map(|p| p.to_string()).unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    println!("\n{} Auto-Detection Report", "[D]".blue().bold());
    println!("{}", "=".repeat(70));
    println!("{} Path: {}", "[*]".blue(), project_path.cyan());
    println!();

    // 1. Workspace Detection
    println!("{} Workspace", "---".dimmed());
    let workspace_info = find_workspace_by_path(&project_path)?;

    match &workspace_info {
        Some((ws_id, ws_dir, ws_name)) => {
            println!("   {} Status: {}", "[+]".green(), "Found".green());
            println!(
                "   {} ID: {}...",
                "[*]".blue(),
                &ws_id[..16.min(ws_id.len())]
            );
            if let Some(name) = ws_name {
                println!("   {} Name: {}", "[*]".blue(), name.cyan());
            }

            // Get sessions from workspace
            if let Ok(sessions) = get_chat_sessions_from_workspace(ws_dir) {
                println!("   {} Sessions: {}", "[*]".blue(), sessions.len());

                if verbose && !sessions.is_empty() {
                    println!("\n   {} Recent Sessions:", "[*]".blue());
                    for (i, swp) in sessions.iter().take(5).enumerate() {
                        println!(
                            "      {}. {} ({} messages)",
                            i + 1,
                            truncate(&swp.session.title(), 40),
                            swp.session.request_count()
                        );
                    }
                    if sessions.len() > 5 {
                        println!("      ... and {} more", sessions.len() - 5);
                    }
                }
            }
        }
        None => {
            println!("   {} Status: {}", "[X]".red(), "Not found".red());
            println!(
                "   {} Open this project in VS Code to create a workspace",
                "[i]".yellow()
            );
        }
    }
    println!();

    // 2. Provider Detection
    println!("{} Available Providers", "---".dimmed());

    let registry = ProviderRegistry::new();
    let provider_types = vec![
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

    let mut total_sessions = 0;
    let mut provider_summary: Vec<(String, usize)> = Vec::new();

    for provider_type in provider_types {
        if let Some(provider) = registry.get_provider(provider_type) {
            if provider.is_available() {
                let session_count = provider.list_sessions().map(|s| s.len()).unwrap_or(0);

                if session_count > 0 || verbose {
                    let status = if session_count > 0 {
                        format!("{} sessions", session_count.to_string().cyan())
                    } else {
                        "ready".dimmed().to_string()
                    };
                    println!("   {} {}: {}", "[+]".green(), provider.name(), status);

                    total_sessions += session_count;
                    if session_count > 0 {
                        provider_summary.push((provider.name().to_string(), session_count));
                    }
                }
            }
        }
    }

    if provider_summary.is_empty() && !verbose {
        println!("   {} No providers with sessions found", "[i]".yellow());
        println!(
            "   {} Use --verbose to see all available providers",
            "[i]".dimmed()
        );
    }
    println!();

    // 3. Summary
    println!("{} Summary", "---".dimmed());

    let ws_status = if workspace_info.is_some() {
        "Yes".green()
    } else {
        "No".red()
    };
    println!("   {} Workspace detected: {}", "[*]".blue(), ws_status);
    println!(
        "   {} Total providers with sessions: {}",
        "[*]".blue(),
        provider_summary.len()
    );
    println!(
        "   {} Total sessions across providers: {}",
        "[*]".blue(),
        total_sessions
    );

    // 4. Recommendations
    if workspace_info.is_none() || total_sessions == 0 {
        println!();
        println!("{} Recommendations", "---".dimmed());

        if workspace_info.is_none() {
            println!(
                "   {} Open this project in VS Code to enable chat history tracking",
                "[->]".cyan()
            );
        }

        if total_sessions == 0 {
            println!(
                "   {} Start a chat session in your IDE to create history",
                "[->]".cyan()
            );
        }
    }

    Ok(())
}

/// Helper function to truncate strings
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Detect all workspace hashes for a project path (including orphaned workspaces)
/// This helps find sessions that exist on disk but are in old/orphaned workspace folders
pub fn detect_orphaned(path: Option<&str>, recover: bool) -> Result<()> {
    use crate::models::WorkspaceJson;
    use crate::workspace::{decode_workspace_folder, get_workspace_storage_path, normalize_path};

    let project_path = path.map(|p| p.to_string()).unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    println!("\n{} Scanning for Orphaned Sessions", "[D]".blue().bold());
    println!("{}", "=".repeat(60));
    println!("{} Path: {}", "[*]".blue(), project_path.cyan());

    let storage_path = get_workspace_storage_path()?;
    let target_path = normalize_path(&project_path);

    // Find ALL workspace hashes that match this path
    let mut all_workspaces: Vec<(String, std::path::PathBuf, usize, std::time::SystemTime)> =
        Vec::new();

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
                        // Count sessions in this workspace
                        let chat_sessions_dir = workspace_dir.join("chatSessions");
                        let session_count = if chat_sessions_dir.exists() {
                            std::fs::read_dir(&chat_sessions_dir)
                                .map(|entries| {
                                    entries
                                        .filter_map(|e| e.ok())
                                        .filter(|e| {
                                            e.path()
                                                .extension()
                                                .map(|ext| ext == "json")
                                                .unwrap_or(false)
                                        })
                                        .count()
                                })
                                .unwrap_or(0)
                        } else {
                            0
                        };

                        // Get last modified time
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
                                .unwrap_or(std::time::UNIX_EPOCH)
                        } else {
                            std::time::UNIX_EPOCH
                        };

                        all_workspaces.push((
                            entry.file_name().to_string_lossy().to_string(),
                            workspace_dir,
                            session_count,
                            last_modified,
                        ));
                    }
                }
            }
        }
    }

    if all_workspaces.is_empty() {
        println!("\n{} No workspaces found for this path", "[X]".red());
        return Ok(());
    }

    // Sort by last modified (newest first)
    all_workspaces.sort_by(|a, b| b.3.cmp(&a.3));

    // The first one (most recently modified) is the "active" workspace
    let active_dir = all_workspaces[0].1.clone();

    println!(
        "\n{} Found {} workspace(s) for this path:",
        "[+]".green().bold(),
        all_workspaces.len()
    );

    let mut total_orphaned_sessions = 0;
    let mut orphaned_workspaces: Vec<(String, std::path::PathBuf, usize)> = Vec::new();

    for (i, (hash, dir, session_count, _)) in all_workspaces.iter().enumerate() {
        let is_active = i == 0;
        let status = if is_active {
            format!("{}", "(active)".green())
        } else {
            format!("{}", "(orphaned)".yellow())
        };

        let session_str = if *session_count > 0 {
            format!("{} sessions", session_count.to_string().cyan())
        } else {
            "0 sessions".dimmed().to_string()
        };

        println!(
            "   {} {}... {} - {}",
            if is_active {
                "[*]".green()
            } else {
                "[!]".yellow()
            },
            &hash[..16.min(hash.len())],
            status,
            session_str
        );

        if !is_active && *session_count > 0 {
            total_orphaned_sessions += session_count;
            orphaned_workspaces.push((hash.clone(), dir.clone(), *session_count));

            // Show session details
            let chat_sessions_dir = dir.join("chatSessions");
            if let Ok(entries) = std::fs::read_dir(&chat_sessions_dir) {
                for entry in entries.filter_map(|e| e.ok()).take(3) {
                    let path = entry.path();
                    if path.extension().map(|e| e == "json").unwrap_or(false) {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(session) = crate::storage::parse_session_json(&content) {
                                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                                let size_str = if size > 1_000_000 {
                                    format!("{:.1}MB", size as f64 / 1_000_000.0)
                                } else if size > 1000 {
                                    format!("{:.1}KB", size as f64 / 1000.0)
                                } else {
                                    format!("{}B", size)
                                };
                                println!(
                                    "      {} {} ({}, {} msgs)",
                                    "`".dimmed(),
                                    truncate(&session.title(), 45),
                                    size_str.cyan(),
                                    session.request_count()
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // Summary
    println!();
    if total_orphaned_sessions > 0 {
        println!(
            "{} {} orphaned session(s) found in {} workspace(s)",
            "[!]".yellow().bold(),
            total_orphaned_sessions.to_string().yellow(),
            orphaned_workspaces.len()
        );

        if recover {
            // Recover orphaned sessions
            println!("\n{} Recovering orphaned sessions...", "[*]".blue());

            let active_chat_sessions = active_dir.join("chatSessions");
            if !active_chat_sessions.exists() {
                std::fs::create_dir_all(&active_chat_sessions)?;
            }

            let mut recovered = 0;
            for (hash, orphan_dir, _) in &orphaned_workspaces {
                let orphan_sessions = orphan_dir.join("chatSessions");
                if let Ok(entries) = std::fs::read_dir(&orphan_sessions) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let src = entry.path();
                        if src.extension().map(|e| e == "json").unwrap_or(false) {
                            let filename = src.file_name().unwrap();
                            let dest = active_chat_sessions.join(filename);
                            if !dest.exists() {
                                std::fs::copy(&src, &dest)?;
                                recovered += 1;
                                println!(
                                    "   {} Copied: {} (from {}...)",
                                    "[+]".green(),
                                    filename.to_string_lossy(),
                                    &hash[..8]
                                );
                            }
                        }
                    }
                }
            }

            println!(
                "\n{} Recovered {} session(s)",
                "[OK]".green().bold(),
                recovered
            );
            println!(
                "\n{} Run {} to make them visible in VS Code",
                "[i]".cyan(),
                "chasm register all --force".cyan()
            );
        } else {
            println!(
                "\n{} To recover, run: {}",
                "[->]".cyan(),
                format!(
                    "chasm detect orphaned --recover --path \"{}\"",
                    project_path
                )
                .cyan()
            );
        }
    } else {
        println!("{} No orphaned sessions found", "[OK]".green().bold());
    }

    Ok(())
}
