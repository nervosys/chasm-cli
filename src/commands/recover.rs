// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Session Recovery Commands
//!
//! This module provides commands for recovering lost, corrupted, or orphaned
//! chat sessions from various sources including:
//! - Recording API server state
//! - SQLite database backups
//! - Corrupted JSONL files
//! - Orphaned files in workspaceStorage

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Get workspace storage path for a provider
fn get_provider_storage_path(provider: &str) -> Option<PathBuf> {
    let base = match std::env::consts::OS {
        "windows" => std::env::var("APPDATA").ok().map(PathBuf::from),
        "macos" => dirs::home_dir().map(|p| p.join("Library/Application Support")),
        _ => dirs::home_dir().map(|p| p.join(".config")),
    }?;

    let path = match provider {
        "vscode" => base.join("Code/User/workspaceStorage"),
        "cursor" => base.join("Cursor/User/workspaceStorage"),
        _ => return None,
    };

    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Get state database path for a provider
fn get_provider_state_db(provider: &str) -> Option<PathBuf> {
    let base = match std::env::consts::OS {
        "windows" => std::env::var("APPDATA").ok().map(PathBuf::from),
        "macos" => dirs::home_dir().map(|p| p.join("Library/Application Support")),
        _ => dirs::home_dir().map(|p| p.join(".config")),
    }?;

    let path = match provider {
        "vscode" => base.join("Code/User/globalStorage/state.vscdb"),
        "cursor" => base.join("Cursor/User/globalStorage/state.vscdb"),
        _ => return None,
    };

    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Get copilot chat history path
fn get_copilot_history_path(provider: &str) -> Option<PathBuf> {
    let base = match std::env::consts::OS {
        "windows" => std::env::var("APPDATA").ok().map(PathBuf::from),
        "macos" => dirs::home_dir().map(|p| p.join("Library/Application Support")),
        _ => dirs::home_dir().map(|p| p.join(".config")),
    }?;

    let path = match provider {
        "vscode" => base.join("Code/User/History/copilot-chat"),
        "cursor" => base.join("Cursor/User/History"),
        _ => return None,
    };

    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Scan for recoverable sessions from various sources
pub fn recover_scan(provider: &str, verbose: bool, _include_old: bool) -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║               Session Recovery Scanner v1.3.2                     ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝\n");

    let providers_to_scan = if provider == "all" {
        vec!["vscode", "cursor"]
    } else {
        vec![provider]
    };

    let mut total_recoverable = 0;
    let mut total_corrupted = 0;

    for prov in &providers_to_scan {
        println!("[*] Scanning {} workspaces...", prov);

        // Scan workspace storage
        if let Some(storage_path) = get_provider_storage_path(prov) {
            let mut count = 0;
            if let Ok(entries) = fs::read_dir(&storage_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        // Look for session files
                        let sessions_dir = path.join("state.vscdb");
                        let history_dir = path.join("history");
                        
                        if sessions_dir.exists() || history_dir.exists() {
                            count += 1;
                            if verbose {
                                println!("    [+] Found workspace: {}", path.display());
                            }
                        }
                    }
                }
            }
            println!("    Found {} workspace directories", count);
            total_recoverable += count;
        }

        // Scan for JSONL files with parse errors
        if let Some(copilot_path) = get_copilot_history_path(prov) {
            let mut corrupted_count = 0;
            if let Ok(entries) = fs::read_dir(&copilot_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "jsonl") {
                        // Try to parse the file
                        if let Ok(content) = fs::read_to_string(&path) {
                            let lines: Vec<&str> = content.lines().collect();
                            let mut errors = 0;
                            for line in &lines {
                                if !line.is_empty() {
                                    if serde_json::from_str::<serde_json::Value>(line).is_err() {
                                        errors += 1;
                                    }
                                }
                            }
                            if errors > 0 {
                                corrupted_count += 1;
                                if verbose {
                                    println!("    [!] Corrupted JSONL: {} ({} bad lines)", path.display(), errors);
                                }
                            }
                        }
                    }
                }
            }
            if corrupted_count > 0 {
                println!("    Found {} potentially corrupted JSONL files", corrupted_count);
                total_corrupted += corrupted_count;
            }
        }
    }

    println!();
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║                       Recovery Summary                            ║");
    println!("╠═══════════════════════════════════════════════════════════════════╣");
    println!("║  Workspace directories found: {:>5}                              ║", total_recoverable);
    println!("║  Corrupted files:             {:>5}                              ║", total_corrupted);
    println!("╚═══════════════════════════════════════════════════════════════════╝");

    if total_corrupted > 0 {
        println!();
        println!("[i] Use 'chasm recover jsonl <file>' to attempt repair of corrupted files");
    }

    Ok(())
}

/// Recover sessions from the recording API server
pub fn recover_from_recording(server: &str, session_id: Option<&str>, output: Option<&str>) -> Result<()> {
    println!("[*] Connecting to recording server: {}", server);

    // Build the recovery URL
    let url = if let Some(sid) = session_id {
        format!("{}/recording/session/{}/recovery", server, sid)
    } else {
        format!("{}/recording/sessions", server)
    };

    // Make HTTP request using blocking reqwest
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    
    let response = client.get(&url)
        .send()
        .context("Failed to connect to recording server")?;

    if !response.status().is_success() {
        anyhow::bail!("Server returned error: {}", response.status());
    }

    let body = response.text()?;
    
    if let Some(sid) = session_id {
        // Single session recovery
        let output_path = output
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("{}_recovered.json", sid)));

        fs::write(&output_path, &body)?;
        println!("[+] Recovered session saved to: {}", output_path.display());
    } else {
        // List all sessions
        let sessions: serde_json::Value = serde_json::from_str(&body)?;
        
        if let Some(arr) = sessions.get("active_sessions").and_then(|v| v.as_array()) {
            println!();
            println!("╔═══════════════════════════════════════════════════════════════════╗");
            println!("║                    Active Recording Sessions                      ║");
            println!("╠═══════════════════════════════════════════════════════════════════╣");
            
            for session in arr {
                let id = session.get("session_id").and_then(|v| v.as_str()).unwrap_or("?");
                let provider = session.get("provider").and_then(|v| v.as_str()).unwrap_or("?");
                let msgs = session.get("message_count").and_then(|v| v.as_i64()).unwrap_or(0);
                let title = session.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
                
                println!("║ {:36} {:10} {:>4} msgs  ║", 
                    &id[..id.len().min(36)],
                    provider,
                    msgs
                );
                if title != "Untitled" {
                    println!("║   └─ {}{}║", 
                        &title[..title.len().min(55)],
                        " ".repeat(55 - title.len().min(55))
                    );
                }
            }
            
            println!("╚═══════════════════════════════════════════════════════════════════╝");
            println!();
            println!("[i] Use 'chasm recover recording --session <ID>' to recover a specific session");
        } else {
            println!("[!] No active sessions found on recording server");
        }
    }

    Ok(())
}

/// Recover sessions from a SQLite database backup
pub fn recover_from_database(backup_path: &str, session_id: Option<&str>, output: Option<&str>, format: &str) -> Result<()> {
    println!("[*] Opening database backup: {}", backup_path);

    let conn = rusqlite::Connection::open(backup_path)?;

    // Check for sessions table
    let table_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='sessions')",
        [],
        |row| row.get(0),
    )?;

    if !table_exists {
        // Try VS Code state.vscdb format
        let state_format: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='ItemTable')",
            [],
            |row| row.get(0),
        )?;

        if state_format {
            return recover_from_vscdb(&conn, session_id, output, format);
        }
        
        anyhow::bail!("Database does not contain recognized session tables");
    }

    // Query sessions
    let query = if let Some(sid) = session_id {
        format!("SELECT id, title, provider, created_at, data FROM sessions WHERE id = '{}'", sid)
    } else {
        "SELECT id, title, provider, created_at, data FROM sessions ORDER BY created_at DESC LIMIT 50".to_string()
    };

    let mut stmt = conn.prepare(&query)?;
    let sessions: Vec<(String, String, String, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                row.get::<_, Option<String>>(4)?.unwrap_or_default(),
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if sessions.is_empty() {
        println!("[!] No sessions found in database");
        return Ok(());
    }

    if let Some(sid) = session_id {
        // Export single session
        let session = &sessions[0];
        let output_path = output
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("{}_recovered.{}", sid, format)));

        let content = match format {
            "json" => session.4.clone(),
            "jsonl" => session.4.lines().collect::<Vec<_>>().join("\n"),
            _ => session.4.clone(),
        };

        fs::write(&output_path, content)?;
        println!("[+] Session recovered to: {}", output_path.display());
    } else {
        // List sessions
        println!();
        println!("╔═══════════════════════════════════════════════════════════════════╗");
        println!("║                    Sessions in Database Backup                    ║");
        println!("╠═══════════════════════════════════════════════════════════════════╣");
        
        for (id, title, provider, created, _) in &sessions {
            let title_display = if title.is_empty() { "Untitled" } else { title };
            println!("║ {:36} {:10} {:16}  ║",
                &id[..id.len().min(36)],
                &provider[..provider.len().min(10)],
                &created[..created.len().min(16)]
            );
            if !title.is_empty() {
                println!("║   └─ {}{}║",
                    &title_display[..title_display.len().min(55)],
                    " ".repeat(55 - title_display.len().min(55))
                );
            }
        }
        
        println!("╚═══════════════════════════════════════════════════════════════════╝");
        println!();
        println!("[i] Use 'chasm recover database {} --session <ID>' to export a session", backup_path);
    }

    Ok(())
}

/// Recover from VS Code state.vscdb format
fn recover_from_vscdb(conn: &rusqlite::Connection, _session_id: Option<&str>, output: Option<&str>, _format: &str) -> Result<()> {
    println!("[*] Detected VS Code state.vscdb format");

    // Query for chat history keys
    let mut stmt = conn.prepare(
        "SELECT key, value FROM ItemTable WHERE key LIKE '%chat%' OR key LIKE '%copilot%'"
    )?;

    let items: Vec<(String, Vec<u8>)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    if items.is_empty() {
        println!("[!] No chat-related data found in state database");
        return Ok(());
    }

    println!("[+] Found {} chat-related entries", items.len());

    let output_dir = output.map(PathBuf::from).unwrap_or_else(|| PathBuf::from("recovered_vscdb"));
    fs::create_dir_all(&output_dir)?;

    for (key, value) in &items {
        // Try to parse as UTF-8
        if let Ok(text) = String::from_utf8(value.clone()) {
            // Check if it's JSON
            if text.starts_with('{') || text.starts_with('[') {
                let safe_key = key.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
                let output_path = output_dir.join(format!("{}.json", safe_key));
                fs::write(&output_path, &text)?;
                println!("  [+] Extracted: {}", output_path.display());
            }
        }
    }

    println!();
    println!("[+] Recovery output written to: {}", output_dir.display());

    Ok(())
}

/// Recover sessions from corrupted JSONL files
pub fn recover_jsonl(file_path: &str, output: Option<&str>, aggressive: bool) -> Result<()> {
    println!("[*] Attempting to recover JSONL file: {}", file_path);

    let content = fs::read_to_string(file_path)?;
    let lines: Vec<&str> = content.lines().collect();

    let mut recovered_objects: Vec<serde_json::Value> = Vec::new();
    let mut errors = 0;
    let mut recovered = 0;

    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }

        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(obj) => {
                recovered_objects.push(obj);
                recovered += 1;
            }
            Err(e) => {
                errors += 1;
                if aggressive {
                    // Try to fix common issues
                    let fixed = attempt_json_repair(line);
                    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&fixed) {
                        recovered_objects.push(obj);
                        recovered += 1;
                        println!("  [+] Repaired line {}", i + 1);
                    } else {
                        println!("  [!] Could not repair line {}: {}", i + 1, e);
                    }
                } else {
                    println!("  [!] Error on line {}: {}", i + 1, e);
                }
            }
        }
    }

    println!();
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║                    JSONL Recovery Summary                         ║");
    println!("╠═══════════════════════════════════════════════════════════════════╣");
    println!("║  Total lines:     {:>5}                                          ║", lines.len());
    println!("║  Recovered:       {:>5}                                          ║", recovered);
    println!("║  Errors:          {:>5}                                          ║", errors);
    println!("╚═══════════════════════════════════════════════════════════════════╝");

    if recovered > 0 {
        let output_path = output.map(|s| PathBuf::from(s)).unwrap_or_else(|| {
            let p = Path::new(file_path);
            p.with_extension("recovered.jsonl")
        });

        let mut output_content = String::new();
        for obj in &recovered_objects {
            output_content.push_str(&serde_json::to_string(obj)?);
            output_content.push('\n');
        }

        fs::write(&output_path, output_content)?;
        println!();
        println!("[+] Recovered data written to: {}", output_path.display());
    }

    Ok(())
}

/// Attempt to repair malformed JSON
fn attempt_json_repair(line: &str) -> String {
    let mut fixed = line.to_string();

    // Fix unescaped quotes
    // This is a simple heuristic - real repair would need more sophisticated parsing
    
    // Fix trailing commas
    fixed = fixed.replace(",}", "}").replace(",]", "]");

    // Fix missing closing braces/brackets
    let open_braces = fixed.matches('{').count();
    let close_braces = fixed.matches('}').count();
    if open_braces > close_braces {
        fixed.push_str(&"}".repeat(open_braces - close_braces));
    }

    let open_brackets = fixed.matches('[').count();
    let close_brackets = fixed.matches(']').count();
    if open_brackets > close_brackets {
        fixed.push_str(&"]".repeat(open_brackets - close_brackets));
    }

    fixed
}

/// List orphaned sessions in workspaceStorage
pub fn recover_orphans(provider: &str, unindexed: bool, _verify: bool) -> Result<()> {
    println!("[*] Scanning for orphaned sessions...");

    let providers_to_scan = if provider == "all" {
        vec!["vscode", "cursor"]
    } else {
        vec![provider]
    };

    let mut total_orphans = 0;

    for prov in &providers_to_scan {
        println!("\n[*] Checking {}...", prov);

        if let Some(storage_path) = get_provider_storage_path(prov) {
            // Get list of workspaces from database if checking for unindexed
            let indexed_workspaces: std::collections::HashSet<String> = if unindexed {
                if let Some(db_path) = get_provider_state_db(prov) {
                    if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                        if let Ok(mut stmt) = conn.prepare("SELECT key FROM ItemTable WHERE key LIKE 'workspaceStorage/%'") {
                            stmt.query_map([], |row| row.get::<_, String>(0))
                                .ok()
                                .map(|iter| iter.flatten().collect())
                                .unwrap_or_default()
                        } else {
                            std::collections::HashSet::new()
                        }
                    } else {
                        std::collections::HashSet::new()
                    }
                } else {
                    std::collections::HashSet::new()
                }
            } else {
                std::collections::HashSet::new()
            };

            if let Ok(entries) = fs::read_dir(&storage_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let dir_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        
                        // Check if this workspace has session data
                        let has_sessions = path.join("state.vscdb").exists() 
                            || path.join("history").exists();

                        if !has_sessions {
                            continue;
                        }

                        let is_indexed = !unindexed || indexed_workspaces.contains(&dir_name);
                        
                        if !is_indexed {
                            total_orphans += 1;
                            println!("  [?] Unindexed: {}", dir_name);
                        }
                    }
                }
            }
        }
    }

    println!();
    if total_orphans > 0 {
        println!("[i] Found {} potentially orphaned workspace(s)", total_orphans);
        println!("[i] Use 'chasm register all' to re-index these workspaces");
    } else {
        println!("[+] No orphaned sessions found");
    }

    Ok(())
}

/// Repair corrupted session files in place
pub fn recover_repair(path: &str, create_backup: bool, dry_run: bool) -> Result<()> {
    let path = Path::new(path);

    if dry_run {
        println!("[*] DRY RUN - no changes will be made");
    }

    if path.is_dir() {
        println!("[*] Scanning directory for repairable files: {}", path.display());
        
        let mut repaired = 0;
        for entry in walkdir::WalkDir::new(path).into_iter().flatten() {
            let file_path = entry.path();
            if file_path.extension().map_or(false, |e| e == "jsonl" || e == "json") {
                if let Ok(content) = fs::read_to_string(file_path) {
                    let needs_repair = content.lines().any(|line| {
                        !line.is_empty() && serde_json::from_str::<serde_json::Value>(line).is_err()
                    });

                    if needs_repair {
                        println!("  [!] Needs repair: {}", file_path.display());
                        if !dry_run {
                            repair_file(file_path, create_backup)?;
                            repaired += 1;
                        }
                    }
                }
            }
        }

        println!();
        if dry_run {
            println!("[i] {} file(s) would be repaired", repaired);
        } else {
            println!("[+] Repaired {} file(s)", repaired);
        }
    } else {
        // Single file
        if !dry_run {
            repair_file(path, create_backup)?;
            println!("[+] File repaired: {}", path.display());
        } else {
            println!("[i] Would repair: {}", path.display());
        }
    }

    Ok(())
}

fn repair_file(path: &Path, create_backup: bool) -> Result<()> {
    if create_backup {
        let backup_path = path.with_extension("backup");
        fs::copy(path, &backup_path)?;
    }

    let content = fs::read_to_string(path)?;
    let mut output = String::new();

    for line in content.lines() {
        if line.is_empty() {
            output.push('\n');
            continue;
        }

        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(_) => {
                output.push_str(line);
                output.push('\n');
            }
            Err(_) => {
                let fixed = attempt_json_repair(line);
                if serde_json::from_str::<serde_json::Value>(&fixed).is_ok() {
                    output.push_str(&fixed);
                    output.push('\n');
                }
                // Skip unrecoverable lines
            }
        }
    }

    fs::write(path, output)?;
    Ok(())
}

/// Show recovery status and recommendations
pub fn recover_status(provider: &str, check_system: bool) -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║                    Recovery Status Report                         ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝\n");

    let providers_to_check = if provider == "all" {
        vec!["vscode", "cursor"]
    } else {
        vec![provider]
    };

    for name in &providers_to_check {
        println!("[*] {} Status:", name.to_uppercase());

        // Check state database
        if let Some(db_path) = get_provider_state_db(name) {
            let size = fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
            println!("    Database: {} ({:.1} MB)", db_path.display(), size as f64 / 1024.0 / 1024.0);
            
            // Check if database is accessible
            match rusqlite::Connection::open(&db_path) {
                Ok(conn) => {
                    if let Ok(count) = conn.query_row::<i64, _, _>(
                        "SELECT COUNT(*) FROM ItemTable", [], |r| r.get(0)
                    ) {
                        println!("    Items in database: {}", count);
                    }
                }
                Err(e) => {
                    println!("    [!] Database error: {}", e);
                }
            }
        } else {
            println!("    Database: Not found");
        }

        // Check workspace storage
        if let Some(storage_path) = get_provider_storage_path(name) {
            let count = fs::read_dir(&storage_path)
                .map(|r| r.count())
                .unwrap_or(0);
            println!("    Workspace folders: {}", count);
        }

        // Check copilot history
        if let Some(history_path) = get_copilot_history_path(name) {
            let count = fs::read_dir(&history_path)
                .map(|r| r.filter(|e| e.as_ref().map(|e| e.path().extension().map_or(false, |ext| ext == "jsonl")).unwrap_or(false)).count())
                .unwrap_or(0);
            println!("    JSONL session files: {}", count);
        }

        println!();
    }

    if check_system {
        println!("[*] System Status:");
        
        // Get available disk space
        #[cfg(windows)]
        {
            // Windows: check C: drive
            if let Ok(output) = std::process::Command::new("wmic")
                .args(["logicaldisk", "get", "freespace,size"])
                .output()
            {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    println!("    Disk space: {}", text.lines().nth(1).unwrap_or("Unknown"));
                }
            }
        }

        #[cfg(not(windows))]
        {
            if let Ok(output) = std::process::Command::new("df")
                .args(["-h", "/"])
                .output()
            {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    if let Some(line) = text.lines().nth(1) {
                        println!("    Disk space: {}", line);
                    }
                }
            }
        }
    }

    println!("[*] Recommendations:");
    println!("    1. Run 'chasm recover scan' to find recoverable sessions");
    println!("    2. Use 'chasm harvest run' to consolidate all sessions");
    println!("    3. Consider setting up the recording API for crash protection");

    Ok(())
}

// ============================================================================
// Convert Command - Convert between JSON and JSONL formats
// ============================================================================

/// Convert session files between JSON and JSONL formats
pub fn recover_convert(
    input: &str,
    output: Option<&str>,
    format: Option<&str>,
    compat: &str,
) -> Result<()> {
    use crate::storage::{parse_session_auto, detect_session_format, VsCodeSessionFormat};
    

    let input_path = Path::new(input);
    if !input_path.exists() {
        anyhow::bail!("Input file does not exist: {}", input);
    }

    // Read content first for auto-detection
    let content = fs::read_to_string(input_path)
        .with_context(|| format!("Failed to read input file: {}", input))?;

    // Auto-detect format from content (not just extension)
    let format_info = detect_session_format(&content);
    
    // Determine output format
    let output_format = if let Some(fmt) = format {
        fmt.to_lowercase()
    } else if let Some(out) = output {
        // Infer from output extension
        Path::new(out)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or(match format_info.format {
                VsCodeSessionFormat::JsonLines => "json",
                VsCodeSessionFormat::LegacyJson => "jsonl",
            })
            .to_lowercase()
    } else {
        // Default to opposite of detected input format
        match format_info.format {
            VsCodeSessionFormat::JsonLines => "json".to_string(),
            VsCodeSessionFormat::LegacyJson => "jsonl".to_string(),
        }
    };

    // Determine output path
    let output_path = if let Some(out) = output {
        PathBuf::from(out)
    } else {
        let stem = input_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("converted");
        input_path.with_file_name(format!("{}.{}", stem, output_format))
    };

    println!("[*] Session Format Converter");
    println!("    Input:  {}", input);
    println!("    Output: {} ({})", output_path.display(), output_format.to_uppercase());
    println!("    Compat: {}", compat);
    println!();
    println!("[*] Auto-detected source format:");
    println!("    Format:     {} ({})", format_info.format.short_name(), format_info.format);
    println!("    Schema:     {}", format_info.schema_version);
    println!("    Confidence: {:.0}%", format_info.confidence * 100.0);
    println!("    Method:     {}", format_info.detection_method);
    println!();

    // Parse using auto-detection
    let (session, _) = parse_session_auto(&content)
        .with_context(|| "Failed to parse session")?;

    println!("[+] Parsed session:");
    println!("    Session ID: {}", session.session_id.as_deref().unwrap_or("none"));
    println!("    Version:    {}", session.version);
    println!("    Requests:   {}", session.requests.len());
    println!("    Created:    {}", format_timestamp(session.creation_date));
    println!();

    // Convert to output format
    let output_content = match output_format.as_str() {
        "json" => {
            // Convert to legacy JSON format (VS Code < 1.109.0)
            serde_json::to_string_pretty(&session)
                .with_context(|| "Failed to serialize to JSON")?
        }
        "jsonl" => {
            // Convert to JSONL format (VS Code >= 1.109.0)
            convert_to_jsonl(&session)
                .with_context(|| "Failed to serialize to JSONL")?
        }
        "md" | "markdown" => {
            // Convert to readable markdown
            convert_to_markdown(&session)
        }
        _ => anyhow::bail!("Unknown output format: {}. Use json, jsonl, or md", output_format),
    };

    // Write output
    fs::write(&output_path, &output_content)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    println!("[+] Converted successfully!");
    println!("    Output size: {} bytes", output_content.len());

    Ok(())
}

/// Convert ChatSession to JSONL format (VS Code 1.109.0+)
fn convert_to_jsonl(session: &crate::models::ChatSession) -> Result<String> {
    let mut lines = Vec::new();

    // Line 1: kind 0 - Initial session state
    let initial = serde_json::json!({
        "kind": 0,
        "v": {
            "version": session.version,
            "sessionId": session.session_id,
            "creationDate": session.creation_date,
            "initialLocation": session.initial_location,
            "responderUsername": session.responder_username,
            "requests": session.requests
        }
    });
    lines.push(serde_json::to_string(&initial)?);

    // Line 2: kind 1 - lastMessageDate delta
    if session.last_message_date > 0 {
        let delta = serde_json::json!({
            "kind": 1,
            "k": ["lastMessageDate"],
            "v": session.last_message_date
        });
        lines.push(serde_json::to_string(&delta)?);
    }

    // Line 3: kind 1 - customTitle delta (if set)
    if let Some(ref title) = session.custom_title {
        let delta = serde_json::json!({
            "kind": 1,
            "k": ["customTitle"],
            "v": title
        });
        lines.push(serde_json::to_string(&delta)?);
    }

    Ok(lines.join("\n"))
}

/// Convert ChatSession to readable markdown
fn convert_to_markdown(session: &crate::models::ChatSession) -> String {
    let mut md = String::new();

    md.push_str("# Chat Session\n\n");
    
    if let Some(ref title) = session.custom_title {
        md.push_str(&format!("**Title:** {}\n\n", title));
    }
    
    if let Some(ref session_id) = session.session_id {
        md.push_str(&format!("**Session ID:** `{}`\n\n", session_id));
    }
    
    md.push_str(&format!("**Created:** {}\n\n", format_timestamp(session.creation_date)));
    md.push_str(&format!("**Messages:** {}\n\n", session.requests.len()));
    md.push_str("---\n\n");

    for (i, request) in session.requests.iter().enumerate() {
        md.push_str(&format!("## Turn {}\n\n", i + 1));
        
        // User message
        md.push_str("### User\n\n");
        if let Some(ref msg) = request.message {
            md.push_str(&format!("{}\n\n", msg.text.as_deref().unwrap_or("")));
        }

        // Assistant response  
        if let Some(ref response) = request.response {
            md.push_str("### Assistant\n\n");
            // Response is a serde_json::Value - extract text from 'value' or 'text' field
            let response_text = response.get("value")
                .or_else(|| response.get("text"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            md.push_str(&format!("{}\n\n", response_text));
        }

        md.push_str("---\n\n");
    }

    md
}

// ============================================================================
// Extract Command - Extract sessions from a project path
// ============================================================================

/// Extract sessions from a VS Code workspace by project path
pub fn recover_extract(
    project_path: &str,
    output: Option<&str>,
    all_formats: bool,
    include_edits: bool,
) -> Result<()> {
    let project_path = Path::new(project_path);
    
    // Normalize the path
    let canonical_path = if project_path.exists() {
        let p = project_path.canonicalize()
            .with_context(|| format!("Failed to canonicalize path: {}", project_path.display()))?;
        // Strip Windows extended path prefix (\\?\) if present
        let path_str = p.to_string_lossy();
        if path_str.starts_with("\\\\?\\") {
            PathBuf::from(&path_str[4..])
        } else {
            p
        }
    } else {
        PathBuf::from(project_path)
    };

    println!("[*] Session Extractor");
    println!("    Project: {}", canonical_path.display());
    println!();

    // Normalize the path for comparison
    let normalized_path = canonical_path.display().to_string()
        .replace('\\', "/")
        .to_lowercase();

    // Search for matching workspace directories by reading workspace.json files
    println!("[*] Searching for workspace matching: {}", normalized_path);
    println!();

    // Search in all providers
    let providers = ["vscode", "cursor"];
    let mut found_sessions = Vec::new();
    let mut matched_workspaces = Vec::new();

    for provider in &providers {
        if let Some(storage_path) = get_provider_storage_path(provider) {
            // Iterate through all workspace directories and check workspace.json
            if let Ok(entries) = fs::read_dir(&storage_path) {
                for entry in entries.flatten() {
                    let workspace_dir = entry.path();
                    if !workspace_dir.is_dir() {
                        continue;
                    }
                    
                    let workspace_json = workspace_dir.join("workspace.json");
                    if let Ok(content) = fs::read_to_string(&workspace_json) {
                        // Parse workspace.json to get folder URI
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(folder) = json.get("folder").and_then(|f| f.as_str()) {
                                // Normalize the folder URI for comparison
                                // file:///c%3A/path -> c:/path
                                let folder_path = folder
                                    .trim_start_matches("file:///")
                                    .trim_start_matches("file://")
                                    .replace("%3A", ":")
                                    .replace("%3a", ":")
                                    .to_lowercase();
                                
                                if folder_path == normalized_path ||
                                   folder_path.trim_end_matches('/') == normalized_path.trim_end_matches('/') {
                                    matched_workspaces.push((provider.to_string(), workspace_dir.clone()));
                                    println!("[+] Found {} workspace: {}", provider, workspace_dir.display());
                                    println!("    Folder: {}", folder);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Now collect sessions from all matched workspaces
    for (provider, workspace_dir) in &matched_workspaces {
        // Check for JSONL sessions (modern format)
        if let Some(history_path) = get_copilot_history_path(provider) {
            if let Ok(entries) = fs::read_dir(&history_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if ext == "jsonl" {
                        found_sessions.push((provider.to_string(), path, "jsonl".to_string()));
                    } else if all_formats && ext == "json" {
                        found_sessions.push((provider.to_string(), path, "json".to_string()));
                    }
                }
            }
        }

        // Check workspace-specific state
        let state_db = workspace_dir.join("state.vscdb");
        if state_db.exists() {
            found_sessions.push((provider.to_string(), state_db.clone(), "sqlite".to_string()));
        }

        // Check for editing sessions if requested
        if include_edits {
            let edits_dir = workspace_dir.join("workspaceEditingSessions");
            if edits_dir.exists() {
                if let Ok(entries) = fs::read_dir(&edits_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        found_sessions.push((provider.to_string(), path, "edit".to_string()));
                    }
                }
            }
        }
    }

    if found_sessions.is_empty() {
        println!("[-] No sessions found for this project");
        println!();
        println!("[*] Tips:");
        println!("    - Make sure the path matches exactly what VS Code opened");
        println!("    - Try 'chasm recover scan' to see all available sessions");
        return Ok(());
    }

    // Determine output directory
    let output_dir = if let Some(out) = output {
        PathBuf::from(out)
    } else {
        canonical_path.join(".chasm_recovery")
    };

    fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir.display()))?;

    println!();
    println!("[*] Extracting {} items to: {}", found_sessions.len(), output_dir.display());
    println!();

    let mut total_size = 0u64;
    let mut file_count = 0;
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (provider, source_path, format_type) in &found_sessions {
        // Generate unique filename including workspace hash if needed
        let mut dest_name = format!("{}_{}_{}",
            provider,
            format_type,
            source_path.file_name().unwrap_or_default().to_string_lossy()
        );
        
        // If we've seen this name, add the parent directory name (workspace hash) to make it unique
        if seen_names.contains(&dest_name) {
            if let Some(parent) = source_path.parent() {
                if let Some(parent_name) = parent.file_name() {
                    dest_name = format!("{}_{}_{}_{}",
                        provider,
                        format_type,
                        parent_name.to_string_lossy(),
                        source_path.file_name().unwrap_or_default().to_string_lossy()
                    );
                }
            }
        }
        seen_names.insert(dest_name.clone());
        let dest_path = output_dir.join(&dest_name);

        if source_path.is_file() {
            if let Ok(metadata) = fs::metadata(&source_path) {
                total_size += metadata.len();
            }
            
            fs::copy(&source_path, &dest_path)
                .with_context(|| format!("Failed to copy: {}", source_path.display()))?;
            
            file_count += 1;
            println!("    [+] {} -> {}", source_path.display(), dest_name);
        } else if source_path.is_dir() {
            // Copy directory recursively
            copy_dir_recursive(&source_path, &dest_path)?;
            file_count += 1;
            println!("    [+] {} (directory)", dest_name);
        }
    }

    println!();
    println!("[+] Extraction complete!");
    println!("    Files:      {}", file_count);
    println!("    Total size: {} bytes", total_size);
    println!("    Output:     {}", output_dir.display());

    Ok(())
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}

/// Format a Unix timestamp for display
fn format_timestamp(ts: i64) -> String {
    use std::time::{Duration, UNIX_EPOCH};
    
    if ts <= 0 {
        return "Unknown".to_string();
    }
    
    // Handle both seconds and milliseconds
    let ts_secs = if ts > 10_000_000_000 { ts / 1000 } else { ts };
    
    match UNIX_EPOCH.checked_add(Duration::from_secs(ts_secs as u64)) {
        Some(time) => {
            let datetime: chrono::DateTime<chrono::Utc> = time.into();
            datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
        }
        None => format!("{}", ts),
    }
}

// ============================================================================
// Detect Command - Detect session format and version
// ============================================================================

/// Detect and display session format and version information
pub fn recover_detect(file: &str, verbose: bool, output_json: bool) -> Result<()> {
    use crate::storage::{detect_session_format, parse_session_auto, VsCodeSessionFormat};

    let file_path = Path::new(file);
    if !file_path.exists() {
        anyhow::bail!("File does not exist: {}", file);
    }

    // Read file content
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file))?;

    // Detect format
    let format_info = detect_session_format(&content);

    // Try to parse the session
    let parse_result = parse_session_auto(&content);

    if output_json {
        // JSON output
        let mut result = serde_json::json!({
            "file": file,
            "file_size": content.len(),
            "format": {
                "type": format_info.format.short_name(),
                "description": format_info.format.description(),
                "min_vscode_version": format_info.format.min_vscode_version(),
            },
            "schema": {
                "version": format_info.schema_version.version_number(),
                "description": format_info.schema_version.description(),
            },
            "detection": {
                "confidence": format_info.confidence,
                "method": format_info.detection_method,
            },
        });

        if let Ok((session, _)) = &parse_result {
            result["session"] = serde_json::json!({
                "id": session.session_id,
                "version": session.version,
                "requests": session.requests.len(),
                "creation_date": session.creation_date,
                "last_message_date": session.last_message_date,
                "title": session.custom_title,
                "responder": session.responder_username,
            });
            result["parse_success"] = serde_json::json!(true);
        } else {
            result["parse_success"] = serde_json::json!(false);
            if let Err(e) = &parse_result {
                result["parse_error"] = serde_json::json!(e.to_string());
            }
        }

        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        // Human-readable output
        println!("[*] Session Format Detection");
        println!("    File: {}", file);
        println!("    Size: {} bytes", content.len());
        println!();
        
        println!("[*] Detected Format:");
        println!("    Type:        {} ({})", format_info.format.short_name().to_uppercase(), format_info.format);
        println!("    Min VS Code: {}", format_info.format.min_vscode_version());
        println!();
        
        println!("[*] Schema Version:");
        println!("    Version:     {}", format_info.schema_version);
        println!("    Confidence:  {:.0}%", format_info.confidence * 100.0);
        if verbose {
            println!("    Method:      {}", format_info.detection_method);
        }
        println!();

        match &parse_result {
            Ok((session, _)) => {
                println!("[+] Session Parsed Successfully:");
                println!("    Session ID:  {}", session.session_id.as_deref().unwrap_or("none"));
                println!("    Version:     {}", session.version);
                println!("    Requests:    {}", session.requests.len());
                println!("    Created:     {}", format_timestamp(session.creation_date));
                if session.last_message_date > 0 {
                    println!("    Last Msg:    {}", format_timestamp(session.last_message_date));
                }
                if let Some(ref title) = session.custom_title {
                    println!("    Title:       {}", title);
                }
                if let Some(ref responder) = session.responder_username {
                    println!("    Responder:   {}", responder);
                }
                
                if verbose && !session.requests.is_empty() {
                    println!();
                    println!("[*] Request Summary:");
                    for (i, req) in session.requests.iter().take(5).enumerate() {
                        let msg_preview = req.message
                            .as_ref()
                            .and_then(|m| m.text.as_ref())
                            .map(|t| {
                                let preview: String = t.chars().take(50).collect();
                                if t.len() > 50 { format!("{}...", preview) } else { preview }
                            })
                            .unwrap_or_else(|| "[no message]".to_string());
                        println!("    {}. {}", i + 1, msg_preview);
                    }
                    if session.requests.len() > 5 {
                        println!("    ... and {} more requests", session.requests.len() - 5);
                    }
                }
            }
            Err(e) => {
                println!("[-] Parse Error:");
                println!("    {}", e);
                if verbose {
                    // Show first few lines of content for debugging
                    println!();
                    println!("[*] File Preview:");
                    for (i, line) in content.lines().take(5).enumerate() {
                        let preview: String = line.chars().take(100).collect();
                        println!("    {}: {}{}", i + 1, preview, if line.len() > 100 { "..." } else { "" });
                    }
                }
            }
        }
        
        // Show conversion recommendations
        println!();
        println!("[*] Recommendations:");
        match format_info.format {
            VsCodeSessionFormat::LegacyJson => {
                println!("    - This is legacy JSON format (VS Code < 1.109.0)");
                println!("    - Convert to JSONL: chasm recover convert \"{}\" --format jsonl", file);
            }
            VsCodeSessionFormat::JsonLines => {
                println!("    - This is modern JSONL format (VS Code >= 1.109.0)");
                println!("    - Convert to JSON: chasm recover convert \"{}\" --format json", file);
            }
        }
        println!("    - Export to Markdown: chasm recover convert \"{}\" --format md", file);
    }

    Ok(())
}
