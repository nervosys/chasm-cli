//! Session manipulation examples for CSM library
//!
//! Run with: cargo run --example session_operations

use chasm::models::{ChatMessage, ChatRequest, ChatSession};
use chasm::workspace::{find_workspace_by_path, get_chat_sessions_from_workspace};
use uuid::Uuid;

fn main() -> anyhow::Result<()> {
    println!("=== CSM Session Operations Examples ===\n");

    // Example 1: Create a new chat session programmatically
    println!("1. Creating a new chat session...");
    let new_session = create_sample_session();
    println!(
        "   Session ID: {}",
        new_session.session_id.as_deref().unwrap_or("none")
    );
    println!("   Title: {}", new_session.title());
    println!("   Messages: {}", new_session.request_count());

    // Example 2: Serialize session to JSON
    println!("\n2. Serializing to JSON...");
    let json = serde_json::to_string_pretty(&new_session)?;
    println!("   JSON length: {} bytes", json.len());
    println!("   Preview: {}...", &json[..100.min(json.len())]);

    // Example 3: Read sessions from a workspace
    println!("\n3. Reading sessions from workspace...");
    // Try to find a workspace with sessions
    let test_path = std::env::current_dir()?;
    match find_workspace_by_path(&test_path.to_string_lossy()) {
        Ok(Some((hash, ws_dir, _))) => {
            println!("   Found workspace: {}...", &hash[..16]);
            let sessions = get_chat_sessions_from_workspace(&ws_dir)?;
            println!("   Sessions in workspace: {}", sessions.len());

            for (i, session_with_path) in sessions.iter().take(3).enumerate() {
                println!(
                    "   [{}] {} ({} msgs)",
                    i + 1,
                    session_with_path.session.title(),
                    session_with_path.session.request_count()
                );
            }
        }
        Ok(None) => {
            println!("   No workspace found for current directory");
        }
        Err(e) => {
            println!("   Error finding workspace: {}", e);
        }
    }

    // Example 4: Merge multiple sessions
    println!("\n4. Merging sessions chronologically...");
    let session1 = create_session_with_requests(
        "Session 1",
        vec![("First message", 1000), ("Third message", 3000)],
    );
    let session2 = create_session_with_requests(
        "Session 2",
        vec![("Second message", 2000), ("Fourth message", 4000)],
    );

    let merged = merge_sessions(&[session1, session2]);
    println!("   Merged session: {} messages", merged.request_count());
    println!("   Title: {}", merged.title());

    // Example 5: Filter sessions by timestamp
    println!("\n5. Filtering by timestamp range...");
    let sample_session = create_session_with_requests(
        "Sample",
        vec![("Old message", 1000), ("Recent message", 5000)],
    );

    if let Some((first, last)) = sample_session.timestamp_range() {
        println!("   Session spans: {} to {}", first, last);
        println!("   Duration: {} ms", last - first);
    }

    println!("\n=== Examples Complete ===");
    Ok(())
}

/// Create a sample chat session with realistic structure
fn create_sample_session() -> ChatSession {
    let session_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();

    ChatSession {
        version: 3,
        session_id: Some(session_id),
        requester_username: Some("user".to_string()),
        requester_avatar_icon_uri: None,
        responder_username: Some("GitHub Copilot".to_string()),
        initial_location: "panel".to_string(),
        creation_date: now,
        last_message_date: now,
        is_imported: false,
        custom_title: Some("Sample Session".to_string()),
        responder_avatar_icon_uri: None,
        requests: vec![ChatRequest {
            timestamp: Some(now),
            message: Some(ChatMessage {
                text: Some("Hello, how can I use CSM?".to_string()),
                parts: None,
            }),
            response: Some(serde_json::json!({
                "value": [{"value": "CSM helps you manage VS Code chat sessions!"}]
            })),
            request_id: Some(Uuid::new_v4().to_string()),
            response_id: Some(Uuid::new_v4().to_string()),
            model_id: Some("copilot/gpt-4".to_string()),
            variable_data: None,
            agent: None,
            result: None,
            followups: None,
            is_canceled: None,
            content_references: None,
            code_citations: None,
            response_markdown_info: None,
            source_session: None,
        }],
    }
}

/// Create a session with specific requests for testing
fn create_session_with_requests(title: &str, messages: Vec<(&str, i64)>) -> ChatSession {
    let requests: Vec<ChatRequest> = messages
        .into_iter()
        .map(|(text, timestamp)| ChatRequest {
            timestamp: Some(timestamp),
            message: Some(ChatMessage {
                text: Some(text.to_string()),
                parts: None,
            }),
            response: None,
            variable_data: None,
            request_id: None,
            response_id: None,
            model_id: None,
            agent: None,
            result: None,
            followups: None,
            is_canceled: None,
            content_references: None,
            code_citations: None,
            response_markdown_info: None,
            source_session: None,
        })
        .collect();

    ChatSession {
        version: 3,
        session_id: None,
        creation_date: 0,
        last_message_date: 0,
        is_imported: false,
        initial_location: "panel".to_string(),
        custom_title: Some(title.to_string()),
        requester_username: None,
        requester_avatar_icon_uri: None,
        responder_username: None,
        responder_avatar_icon_uri: None,
        requests,
    }
}

/// Merge multiple sessions chronologically
fn merge_sessions(sessions: &[ChatSession]) -> ChatSession {
    let mut all_requests: Vec<ChatRequest> =
        sessions.iter().flat_map(|s| s.requests.clone()).collect();

    // Sort by timestamp
    all_requests.sort_by_key(|r| r.timestamp.unwrap_or(0));

    let first_time = all_requests.first().and_then(|r| r.timestamp).unwrap_or(0);
    let last_time = all_requests.last().and_then(|r| r.timestamp).unwrap_or(0);

    ChatSession {
        version: 3,
        session_id: Some(Uuid::new_v4().to_string()),
        custom_title: Some(format!("Merged ({} sessions)", sessions.len())),
        creation_date: first_time,
        last_message_date: last_time,
        is_imported: false,
        initial_location: "panel".to_string(),
        requester_username: None,
        requester_avatar_icon_uri: None,
        responder_username: None,
        responder_avatar_icon_uri: None,
        requests: all_requests,
    }
}
