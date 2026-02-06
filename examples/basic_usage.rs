//! Basic usage examples for CSM library
//!
//! Run with: cargo run --example basic_usage

use chasm::models::ChatSession;
use chasm::workspace::{discover_workspaces, find_workspace_by_path};

fn main() -> anyhow::Result<()> {
    println!("=== CSM Basic Usage Examples ===\n");

    // Example 1: Discover all workspaces
    println!("1. Discovering all VS Code workspaces...");
    let workspaces = discover_workspaces()?;
    println!("   Found {} workspaces", workspaces.len());

    // Show first 5 workspaces
    for ws in workspaces.iter().take(5) {
        println!(
            "   - {} | {} sessions | {}",
            &ws.hash[..12],
            ws.chat_session_count,
            ws.project_path.as_deref().unwrap_or("(none)")
        );
    }
    if workspaces.len() > 5 {
        println!("   ... and {} more", workspaces.len() - 5);
    }

    // Example 2: Find workspaces by pattern
    println!("\n2. Finding workspaces matching 'copilot'...");
    let matches: Vec<_> = workspaces
        .iter()
        .filter(|ws| {
            ws.project_path
                .as_deref()
                .map(|p| p.to_lowercase().contains("copilot"))
                .unwrap_or(false)
        })
        .collect();
    println!("   Found {} matching workspaces", matches.len());
    for ws in &matches {
        println!("   - {}", ws.project_path.as_deref().unwrap_or("(none)"));
    }

    // Example 3: Find workspace by exact path
    println!("\n3. Finding workspace by exact path...");
    if let Some(first_ws) = workspaces.first() {
        if let Some(path) = &first_ws.project_path {
            match find_workspace_by_path(path) {
                Ok(Some((hash, _, _))) => {
                    println!("   Found workspace: {}...", &hash[..16]);
                }
                Ok(None) => {
                    println!("   Workspace not found");
                }
                Err(e) => {
                    println!("   Error: {}", e);
                }
            }
        }
    }

    // Example 4: Parse a chat session file
    println!("\n4. Parsing chat session structure...");
    let sample_session = r#"{
        "version": 3,
        "sessionId": "abc-123-def",
        "customTitle": "Example Session",
        "creationDate": 1699999990000,
        "lastMessageDate": 1699999999000,
        "initialLocation": "panel",
        "requests": [
            {
                "timestamp": 1699999999000,
                "message": { "text": "How do I use CSM?" },
                "response": { "value": [{"value": "CSM helps manage chat sessions!"}] }
            }
        ]
    }"#;

    let session: ChatSession = serde_json::from_str(sample_session)?;
    println!("   Title: {}", session.title());
    println!("   Messages: {}", session.request_count());
    println!("   Empty: {}", session.is_empty());

    if let Some((first, last)) = session.timestamp_range() {
        println!("   Timestamp range: {} - {}", first, last);
    }

    println!("\n=== Examples Complete ===");
    Ok(())
}
