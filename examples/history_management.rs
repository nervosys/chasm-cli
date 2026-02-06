//! History management examples for CSM library
//!
//! Run with: cargo run --example history_management

use chasm::workspace::{
    discover_workspaces, find_all_workspaces_for_project, get_chat_sessions_from_workspace,
};
use std::collections::HashMap;

fn main() -> anyhow::Result<()> {
    println!("=== CSM History Management Examples ===\n");

    // Example 1: Find all workspaces for a project (across renames/moves)
    println!("1. Finding all workspaces for 'copilot_chat_relink'...");
    let project_workspaces = find_all_workspaces_for_project("copilot_chat_relink")?;

    println!(
        "   Found {} workspace instance(s):",
        project_workspaces.len()
    );
    for (hash, ws_dir, _folder_path, last_mod) in &project_workspaces {
        let sessions = get_chat_sessions_from_workspace(ws_dir)?;
        let mod_time: chrono::DateTime<chrono::Utc> = (*last_mod).into();
        println!(
            "   - {}... | {} sessions | {}",
            &hash[..12],
            sessions.len(),
            mod_time.format("%Y-%m-%d")
        );
    }

    // Example 2: Build a project history summary
    println!("\n2. Building project history summary...");
    let mut total_messages = 0;
    let mut total_sessions = 0;
    let mut session_titles: Vec<String> = Vec::new();

    for (_, ws_dir, _, _) in &project_workspaces {
        let sessions = get_chat_sessions_from_workspace(ws_dir)?;
        for session_with_path in &sessions {
            total_sessions += 1;
            total_messages += session_with_path.session.request_count();
            session_titles.push(session_with_path.session.title());
        }
    }

    println!("   Total sessions: {}", total_sessions);
    println!("   Total messages: {}", total_messages);
    println!("   Session titles:");
    for title in session_titles.iter().take(5) {
        println!("     - {}", title);
    }

    // Example 3: Analyze workspace distribution
    println!("\n3. Analyzing workspace distribution...");
    let all_workspaces = discover_workspaces()?;

    let mut by_session_count: HashMap<usize, usize> = HashMap::new();
    for ws in &all_workspaces {
        *by_session_count.entry(ws.chat_session_count).or_insert(0) += 1;
    }

    println!("   Workspaces by session count:");
    let mut counts: Vec<_> = by_session_count.iter().collect();
    counts.sort_by_key(|(k, _)| *k);
    for (session_count, workspace_count) in counts {
        println!(
            "     {} session(s): {} workspace(s)",
            session_count, workspace_count
        );
    }

    // Example 4: Find workspaces with most sessions
    println!("\n4. Top workspaces by session count...");
    let mut sorted_workspaces = all_workspaces.clone();
    sorted_workspaces.sort_by(|a, b| b.chat_session_count.cmp(&a.chat_session_count));

    for ws in sorted_workspaces.iter().take(5) {
        if ws.chat_session_count > 0 {
            println!(
                "   - {} sessions: {}",
                ws.chat_session_count,
                ws.project_path.as_deref().unwrap_or("(none)")
            );
        }
    }

    // Example 5: Calculate total chat history size
    println!("\n5. Calculating total history metrics...");
    let total_workspaces = all_workspaces.len();
    let workspaces_with_chats = all_workspaces
        .iter()
        .filter(|w| w.has_chat_sessions)
        .count();
    let total_session_files: usize = all_workspaces.iter().map(|w| w.chat_session_count).sum();

    println!("   Total workspaces: {}", total_workspaces);
    println!(
        "   Workspaces with chats: {} ({:.1}%)",
        workspaces_with_chats,
        (workspaces_with_chats as f64 / total_workspaces as f64) * 100.0
    );
    println!("   Total session files: {}", total_session_files);

    println!("\n=== Examples Complete ===");
    Ok(())
}
