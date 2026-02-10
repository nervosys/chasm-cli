// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Migration commands

use anyhow::{Context, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::workspace::discover_workspaces;

/// Migration package manifest
#[derive(Debug, Serialize, Deserialize)]
struct MigrationManifest {
    version: String,
    created_at: String,
    source_os: String,
    workspaces: Vec<MigrationWorkspace>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MigrationWorkspace {
    hash: String,
    project_path: Option<String>,
    session_count: usize,
}

/// Create a migration package
pub fn create_migration(output: &str, projects: Option<&str>, include_all: bool) -> Result<()> {
    let output_path = Path::new(output);
    std::fs::create_dir_all(output_path)?;

    let workspaces = discover_workspaces()?;

    // Filter workspaces
    let filtered: Vec<_> = if include_all {
        workspaces.iter().filter(|w| w.has_chat_sessions).collect()
    } else if let Some(project_list) = projects {
        let paths: Vec<&str> = project_list.split(',').map(|p| p.trim()).collect();
        workspaces
            .iter()
            .filter(|w| {
                w.project_path
                    .as_ref()
                    .map(|p| paths.iter().any(|path| p.contains(path)))
                    .unwrap_or(false)
            })
            .collect()
    } else {
        workspaces.iter().filter(|w| w.has_chat_sessions).collect()
    };

    if filtered.is_empty() {
        println!("{} No workspaces with chat sessions found", "[!]".yellow());
        return Ok(());
    }

    let mut total_sessions = 0;
    let mut manifest_workspaces = Vec::new();

    for ws in &filtered {
        // Create workspace directory in package
        let ws_dir = output_path.join(&ws.hash);
        std::fs::create_dir_all(&ws_dir)?;

        // Copy chat sessions
        let sessions_dir = ws_dir.join("chatSessions");
        std::fs::create_dir_all(&sessions_dir)?;

        let mut session_count = 0;
        if ws.chat_sessions_path.exists() {
            for entry in std::fs::read_dir(&ws.chat_sessions_path)? {
                let entry = entry?;
                let src = entry.path();
                if src.extension().map(|e| e == "json").unwrap_or(false) {
                    let dst = sessions_dir.join(entry.file_name());
                    std::fs::copy(&src, &dst)?;
                    session_count += 1;
                }
            }
        }

        // Save workspace.json
        let ws_json = serde_json::json!({
            "folder": ws.project_path.as_ref().map(|p| format!("file:///{}", p.replace('\\', "/").replace(' ', "%20")))
        });
        std::fs::write(
            ws_dir.join("workspace.json"),
            serde_json::to_string_pretty(&ws_json)?,
        )?;

        manifest_workspaces.push(MigrationWorkspace {
            hash: ws.hash.clone(),
            project_path: ws.project_path.clone(),
            session_count,
        });

        total_sessions += session_count;
    }

    // Create manifest
    let manifest = MigrationManifest {
        version: "1.0".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        source_os: std::env::consts::OS.to_string(),
        workspaces: manifest_workspaces,
    };

    std::fs::write(
        output_path.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;

    println!("{} Migration package created", "[OK]".green());
    println!("   Package: {}", output_path.display());
    println!("   Workspaces: {}", filtered.len());
    println!("   Sessions: {}", total_sessions);
    println!("\nTo restore on new machine:");
    println!("   csm restore-migration \"{}\"", output_path.display());

    Ok(())
}

/// Restore a migration package
pub fn restore_migration(package: &str, mapping: Option<&str>, dry_run: bool) -> Result<()> {
    let package_path = Path::new(package);

    if !package_path.exists() {
        anyhow::bail!("Migration package not found: {}", package);
    }

    // Load manifest
    let manifest_path = package_path.join("manifest.json");
    let manifest: MigrationManifest = serde_json::from_str(
        &std::fs::read_to_string(&manifest_path).context("Failed to read manifest.json")?,
    )?;

    // Parse path mapping
    let path_map: std::collections::HashMap<String, String> = if let Some(m) = mapping {
        m.split(';')
            .filter_map(|pair| {
                let parts: Vec<&str> = pair.split(':').collect();
                if parts.len() == 2 {
                    Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
                } else {
                    None
                }
            })
            .collect()
    } else {
        std::collections::HashMap::new()
    };

    let storage_path = crate::workspace::get_workspace_storage_path()?;

    println!("{} Restoring migration package", "[P]".blue());
    println!("   Source: {}", package_path.display());
    println!("   Target: {}", storage_path.display());

    if dry_run {
        println!("\n{} DRY RUN - No changes will be made", "[!]".yellow());
    }

    let mut actions = Vec::new();
    let mut skipped = Vec::new();

    for ws in &manifest.workspaces {
        let src_dir = package_path.join(&ws.hash);

        // Apply path mapping
        let new_path = ws
            .project_path
            .as_ref()
            .map(|p| path_map.get(p).cloned().unwrap_or_else(|| p.clone()));

        // Check if target path exists
        if let Some(ref path) = new_path {
            if !Path::new(path).exists() {
                skipped.push(format!("{}: path does not exist", path));
                continue;
            }
        }

        let dst_dir = storage_path.join(&ws.hash);

        if !dry_run {
            // Create destination directory
            std::fs::create_dir_all(&dst_dir)?;

            // Copy workspace.json
            let ws_json_src = src_dir.join("workspace.json");
            if ws_json_src.exists() {
                std::fs::copy(&ws_json_src, dst_dir.join("workspace.json"))?;
            }

            // Copy chat sessions
            let sessions_src = src_dir.join("chatSessions");
            let sessions_dst = dst_dir.join("chatSessions");

            if sessions_src.exists() {
                std::fs::create_dir_all(&sessions_dst)?;
                for entry in std::fs::read_dir(&sessions_src)? {
                    let entry = entry?;
                    std::fs::copy(entry.path(), sessions_dst.join(entry.file_name()))?;
                }
            }
        }

        actions.push(format!(
            "Restored: {} ({} sessions)",
            new_path.as_deref().unwrap_or("(unknown)"),
            ws.session_count
        ));
    }

    println!("\n{} Actions:", "[*]".blue());
    for action in &actions {
        println!("   {}", action);
    }

    if !skipped.is_empty() {
        println!("\n{} Skipped:", "[!]".yellow());
        for skip in &skipped {
            println!("   {}", skip);
        }
    }

    if !dry_run {
        println!("\n{} Migration restored successfully!", "[OK]".green());
    }

    Ok(())
}
