// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Export and import commands

use anyhow::{Context, Result};
use colored::*;
use std::path::Path;

use crate::models::Workspace;
use crate::workspace::{get_workspace_by_hash, get_workspace_by_path};

/// Export chat sessions from a workspace
pub fn export_sessions(destination: &str, hash: Option<&str>, path: Option<&str>) -> Result<()> {
    let workspace = if let Some(h) = hash {
        get_workspace_by_hash(h)?.context(format!("Workspace not found with hash: {}", h))?
    } else if let Some(p) = path {
        get_workspace_by_path(p)?.context(format!("Workspace not found for path: {}", p))?
    } else {
        anyhow::bail!("Must specify either --hash or --path");
    };

    if !workspace.has_chat_sessions {
        println!("No chat sessions to export.");
        return Ok(());
    }

    // Create destination directory
    let dest_path = Path::new(destination);
    std::fs::create_dir_all(dest_path)?;

    // Copy all session files
    let mut exported_count = 0;
    for entry in std::fs::read_dir(&workspace.chat_sessions_path)? {
        let entry = entry?;
        let src_path = entry.path();

        if src_path.extension().map(|e| e == "json").unwrap_or(false) {
            let dest_file = dest_path.join(entry.file_name());
            std::fs::copy(&src_path, &dest_file)?;
            exported_count += 1;
        }
    }

    println!(
        "{} Exported {} chat session(s) to {}",
        "[OK]".green(),
        exported_count,
        destination
    );

    Ok(())
}

/// Import chat sessions into a workspace
pub fn import_sessions(
    source: &str,
    hash: Option<&str>,
    path: Option<&str>,
    force: bool,
) -> Result<()> {
    let src_path = Path::new(source);
    if !src_path.exists() {
        anyhow::bail!("Source path not found: {}", source);
    }

    let workspace = if let Some(h) = hash {
        get_workspace_by_hash(h)?.context(format!("Workspace not found with hash: {}", h))?
    } else if let Some(p) = path {
        get_workspace_by_path(p)?.context(format!("Workspace not found for path: {}", p))?
    } else {
        anyhow::bail!("Must specify either --hash or --path");
    };

    // Create chatSessions directory if it doesn't exist
    std::fs::create_dir_all(&workspace.chat_sessions_path)?;

    // Import all JSON files
    let mut imported_count = 0;
    let mut skipped_count = 0;

    for entry in std::fs::read_dir(src_path)? {
        let entry = entry?;
        let src_file = entry.path();

        if src_file.extension().map(|e| e == "json").unwrap_or(false) {
            let dest_file = workspace.chat_sessions_path.join(entry.file_name());

            if dest_file.exists() && !force {
                skipped_count += 1;
            } else {
                std::fs::copy(&src_file, &dest_file)?;
                imported_count += 1;
            }
        }
    }

    println!(
        "{} Imported {} chat session(s)",
        "[OK]".green(),
        imported_count
    );
    if skipped_count > 0 {
        println!(
            "{} Skipped {} existing session(s). Use --force to overwrite.",
            "[!]".yellow(),
            skipped_count
        );
    }

    Ok(())
}

/// Move chat sessions from one workspace to another (by path lookup)
#[allow(dead_code)]
pub fn move_sessions(source_hash: &str, target_path: &str) -> Result<()> {
    let source_ws = get_workspace_by_hash(source_hash)?
        .context(format!("Source workspace not found: {}", source_hash))?;

    let target_ws = get_workspace_by_path(target_path)?.context(format!(
        "Target workspace not found for path: {}",
        target_path
    ))?;

    // Prevent moving to self
    if source_ws.workspace_path == target_ws.workspace_path {
        println!(
            "{} Source and target are the same workspace",
            "[!]".yellow()
        );
        return Ok(());
    }

    move_sessions_internal(&source_ws, &target_ws, target_path)
}

/// Move chat sessions from one workspace to another (with explicit target workspace)
fn move_sessions_to_workspace(source_ws: &Workspace, target_ws: &Workspace) -> Result<()> {
    let target_path: &str = target_ws
        .project_path
        .as_deref()
        .unwrap_or("target workspace");
    move_sessions_internal(source_ws, target_ws, target_path)
}

/// Internal function to move sessions between workspaces
fn move_sessions_internal(
    source_ws: &Workspace,
    target_ws: &Workspace,
    display_path: &str,
) -> Result<()> {
    if !source_ws.has_chat_sessions {
        println!("No chat sessions to move.");
        return Ok(());
    }

    // Prevent moving to self
    if source_ws.workspace_path == target_ws.workspace_path {
        println!(
            "{} Source and target are the same workspace",
            "[!]".yellow()
        );
        return Ok(());
    }

    // Create chatSessions directory in target if needed
    std::fs::create_dir_all(&target_ws.chat_sessions_path)?;

    // Move all session files
    let mut moved_count = 0;
    let mut skipped_count = 0;
    for entry in std::fs::read_dir(&source_ws.chat_sessions_path)? {
        let entry = entry?;
        let src_file = entry.path();

        if src_file.extension().map(|e| e == "json").unwrap_or(false) {
            let dest_file = target_ws.chat_sessions_path.join(entry.file_name());

            // Skip if file already exists with same name (don't overwrite)
            if dest_file.exists() {
                skipped_count += 1;
                continue;
            }

            std::fs::rename(&src_file, &dest_file)?;
            moved_count += 1;
        }
    }

    println!(
        "{} Moved {} chat session(s) to {}",
        "[OK]".green(),
        moved_count,
        display_path
    );

    if skipped_count > 0 {
        println!(
            "{} Skipped {} session(s) that already exist in target",
            "[!]".yellow(),
            skipped_count
        );
    }

    Ok(())
}

/// Export specific sessions by ID
pub fn export_specific_sessions(
    destination: &str,
    session_ids: &[String],
    project_path: Option<&str>,
) -> Result<()> {
    use crate::workspace::{discover_workspaces, get_chat_sessions_from_workspace, normalize_path};

    let dest_path = Path::new(destination);
    std::fs::create_dir_all(dest_path)?;

    let workspaces = discover_workspaces()?;

    // Filter workspaces by project path if provided
    let filtered: Vec<_> = if let Some(path) = project_path {
        let normalized = normalize_path(path);
        workspaces
            .into_iter()
            .filter(|ws| {
                ws.project_path
                    .as_ref()
                    .map(|p| normalize_path(p) == normalized)
                    .unwrap_or(false)
            })
            .collect()
    } else {
        workspaces
    };

    let normalized_ids: Vec<String> = session_ids
        .iter()
        .flat_map(|s| s.split(',').map(|p| p.trim().to_lowercase()))
        .filter(|s| !s.is_empty())
        .collect();

    let mut exported_count = 0;
    let mut found_ids = Vec::new();

    for ws in filtered {
        if !ws.has_chat_sessions {
            continue;
        }

        let sessions = get_chat_sessions_from_workspace(&ws.workspace_path)?;

        for session in sessions {
            let session_id = session.session.session_id.clone().unwrap_or_else(|| {
                session
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
                let filename = session
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                let dest_file = dest_path.join(&filename);
                std::fs::copy(&session.path, &dest_file)?;
                exported_count += 1;
                found_ids.push(session_id);
                println!(
                    "   {} Exported: {}",
                    "[OK]".green(),
                    session.session.title()
                );
            }
        }
    }

    println!(
        "\n{} Exported {} session(s) to {}",
        "[OK]".green().bold(),
        exported_count,
        destination
    );

    Ok(())
}

/// Import specific session files
pub fn import_specific_sessions(
    session_files: &[String],
    target_path: Option<&str>,
    force: bool,
) -> Result<()> {
    let target_ws = if let Some(path) = target_path {
        get_workspace_by_path(path)?.context(format!("Workspace not found for path: {}", path))?
    } else {
        let cwd = std::env::current_dir()?;
        get_workspace_by_path(cwd.to_str().unwrap_or(""))?
            .context("Current directory is not a VS Code workspace")?
    };

    std::fs::create_dir_all(&target_ws.chat_sessions_path)?;

    let mut imported_count = 0;
    let mut skipped_count = 0;

    for file_path in session_files {
        let src_path = Path::new(file_path);

        if !src_path.exists() {
            println!("{} File not found: {}", "[!]".yellow(), file_path);
            continue;
        }

        let filename = src_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let dest_file = target_ws.chat_sessions_path.join(&filename);

        if dest_file.exists() && !force {
            println!("   {} Skipping (exists): {}", "[!]".yellow(), filename);
            skipped_count += 1;
        } else {
            std::fs::copy(src_path, &dest_file)?;
            imported_count += 1;
            println!("   {} Imported: {}", "[OK]".green(), filename);
        }
    }

    println!(
        "\n{} Imported {} session(s)",
        "[OK]".green().bold(),
        imported_count
    );
    if skipped_count > 0 {
        println!(
            "{} Skipped {} existing. Use --force to overwrite.",
            "[!]".yellow(),
            skipped_count
        );
    }

    Ok(())
}

/// Move all sessions from one workspace to another (by hash)
pub fn move_workspace(source_hash: &str, target: &str) -> Result<()> {
    // Get source workspace
    let source_ws = get_workspace_by_hash(source_hash)?
        .context(format!("Source workspace not found: {}", source_hash))?;

    // Try target as hash first, then as path
    // This prevents ambiguity when multiple workspaces share the same path
    let target_ws = get_workspace_by_hash(target)?
        .or_else(|| get_workspace_by_path(target).ok().flatten())
        .context(format!("Target workspace not found: {}", target))?;

    move_sessions_to_workspace(&source_ws, &target_ws)
}

/// Move specific sessions by ID
pub fn move_specific_sessions(session_ids: &[String], target_path: &str) -> Result<()> {
    use crate::workspace::{discover_workspaces, get_chat_sessions_from_workspace, normalize_path};

    let target_ws = get_workspace_by_path(target_path)?
        .context(format!("Target workspace not found: {}", target_path))?;

    std::fs::create_dir_all(&target_ws.chat_sessions_path)?;

    let workspaces = discover_workspaces()?;

    let normalized_ids: Vec<String> = session_ids
        .iter()
        .flat_map(|s| s.split(',').map(|p| p.trim().to_lowercase()))
        .filter(|s| !s.is_empty())
        .collect();

    let mut moved_count = 0;
    let mut found_ids = Vec::new();

    for ws in workspaces {
        if !ws.has_chat_sessions {
            continue;
        }

        // Skip target workspace
        if ws
            .project_path
            .as_ref()
            .map(|p| normalize_path(p) == normalize_path(target_path))
            .unwrap_or(false)
        {
            continue;
        }

        let sessions = get_chat_sessions_from_workspace(&ws.workspace_path)?;

        for session in sessions {
            let session_id = session.session.session_id.clone().unwrap_or_else(|| {
                session
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
                let filename = session
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                let dest_file = target_ws.chat_sessions_path.join(&filename);
                std::fs::rename(&session.path, &dest_file)?;
                moved_count += 1;
                found_ids.push(session_id);
                println!("   {} Moved: {}", "[OK]".green(), session.session.title());
            }
        }
    }

    println!(
        "\n{} Moved {} session(s) to {}",
        "[OK]".green().bold(),
        moved_count,
        target_path
    );

    Ok(())
}

/// Move sessions from one path to another
pub fn move_by_path(source_path: &str, target_path: &str) -> Result<()> {
    let source_ws = get_workspace_by_path(source_path)?
        .context(format!("Source workspace not found: {}", source_path))?;

    let target_ws = get_workspace_by_path(target_path)?
        .context(format!("Target workspace not found: {}", target_path))?;

    if !source_ws.has_chat_sessions {
        println!("No chat sessions to move.");
        return Ok(());
    }

    std::fs::create_dir_all(&target_ws.chat_sessions_path)?;

    let mut moved_count = 0;
    for entry in std::fs::read_dir(&source_ws.chat_sessions_path)? {
        let entry = entry?;
        let src_file = entry.path();

        if src_file.extension().map(|e| e == "json").unwrap_or(false) {
            let dest_file = target_ws.chat_sessions_path.join(entry.file_name());
            std::fs::rename(&src_file, &dest_file)?;
            moved_count += 1;
        }
    }

    println!(
        "{} Moved {} chat session(s) from {} to {}",
        "[OK]".green(),
        moved_count,
        source_path,
        target_path
    );

    Ok(())
}
