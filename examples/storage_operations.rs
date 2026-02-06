//! Storage and database examples for CSM library
//!
//! Run with: cargo run --example storage_operations

use chasm::storage::{is_vscode_running, read_chat_session_index};
use chasm::workspace::discover_workspaces;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("=== CSM Storage Operations Examples ===\n");

    // Example 1: Check VS Code status
    println!("1. Checking VS Code status...");
    let vscode_running = is_vscode_running();
    println!("   VS Code running: {}", vscode_running);
    if vscode_running {
        println!("   ! Some operations may not take effect until VS Code restarts");
    }

    // Example 2: Get workspace storage path
    println!("\n2. Getting workspace storage path...");
    let storage_path = get_workspace_storage_path();
    println!("   Storage location: {}", storage_path.display());
    println!("   Exists: {}", storage_path.exists());

    // Example 3: Read session index from database
    println!("\n3. Reading session index from workspace database...");
    let workspaces = discover_workspaces()?;

    // Find a workspace with sessions
    if let Some(ws) = workspaces.iter().find(|w| w.chat_session_count > 0) {
        let db_path = storage_path.join(&ws.hash).join("state.vscdb");

        if db_path.exists() {
            println!("   Database: {}", db_path.display());

            match read_chat_session_index(&db_path) {
                Ok(index) => {
                    println!("   Sessions in index: {}", index.entries.len());
                    for (id, entry) in index.entries.iter().take(3) {
                        println!("     - {} ({})", &id[..16.min(id.len())], &entry.title);
                    }
                }
                Err(e) => {
                    println!("   Error reading index: {}", e);
                }
            }
        } else {
            println!("   Database not found: {}", db_path.display());
        }
    }

    // Example 4: Examine workspace structure
    println!("\n4. Examining workspace structure...");
    if let Some(ws) = workspaces.iter().find(|w| w.chat_session_count > 0) {
        let ws_path = storage_path.join(&ws.hash);

        println!("   Workspace hash: {}", ws.hash);
        println!(
            "   Project path: {}",
            ws.project_path.as_deref().unwrap_or("(none)")
        );

        // List files in workspace directory
        println!("   Contents:");
        for entry in std::fs::read_dir(&ws_path)? {
            let entry = entry?;
            let name = entry.file_name();
            let metadata = entry.metadata()?;
            let size = metadata.len();
            let is_dir = metadata.is_dir();

            println!(
                "     {} {} ({} bytes)",
                if is_dir { "[D]" } else { "[F]" },
                name.to_string_lossy(),
                size
            );
        }

        // Check chatSessions directory
        let sessions_dir = ws_path.join("chatSessions");
        if sessions_dir.exists() {
            let session_count = std::fs::read_dir(&sessions_dir)?.count();
            println!("   Chat sessions directory: {} files", session_count);
        }
    }

    // Example 5: Database integrity check
    println!("\n5. Checking database integrity...");
    let mut valid_dbs = 0;
    let mut missing_dbs = 0;
    let mut corrupted_dbs = 0;

    for ws in workspaces.iter().take(10) {
        let db_path = storage_path.join(&ws.hash).join("state.vscdb");
        if db_path.exists() {
            match rusqlite::Connection::open(&db_path) {
                Ok(conn) => {
                    match conn.query_row("SELECT COUNT(*) FROM ItemTable", [], |row| {
                        row.get::<_, i64>(0)
                    }) {
                        Ok(_) => valid_dbs += 1,
                        Err(_) => corrupted_dbs += 1,
                    }
                }
                Err(_) => corrupted_dbs += 1,
            }
        } else {
            missing_dbs += 1;
        }
    }

    println!(
        "   Checked: {} workspaces",
        valid_dbs + missing_dbs + corrupted_dbs
    );
    println!("   Valid databases: {}", valid_dbs);
    println!("   Missing databases: {}", missing_dbs);
    println!("   Corrupted databases: {}", corrupted_dbs);

    println!("\n=== Examples Complete ===");
    Ok(())
}

/// Get the OS-specific workspace storage path
fn get_workspace_storage_path() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");

    #[cfg(target_os = "windows")]
    {
        home.join("AppData/Roaming/Code/User/workspaceStorage")
    }

    #[cfg(target_os = "macos")]
    {
        home.join("Library/Application Support/Code/User/workspaceStorage")
    }

    #[cfg(target_os = "linux")]
    {
        home.join(".config/Code/User/workspaceStorage")
    }
}
