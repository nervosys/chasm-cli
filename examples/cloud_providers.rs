//! Cloud provider examples for CSM library
//!
//! Demonstrates working with cloud LLM providers:
//! - ChatGPT (OpenAI)
//! - Claude (Anthropic)
//! - Perplexity AI
//! - DeepSeek
//! - Google Gemini
//! - Microsoft 365 Copilot
//!
//! Run with: cargo run --example cloud_providers

use chasm::providers::cloud::anthropic::parse_claude_export;
use chasm::providers::cloud::chatgpt::parse_chatgpt_export;
#[allow(unused_imports)]
use chasm::providers::cloud::deepseek::parse_deepseek_export;
#[allow(unused_imports)]
use chasm::providers::cloud::gemini::parse_gemini_export;
use chasm::providers::cloud::m365copilot::{get_friendly_app_name, parse_m365_copilot_export};
#[allow(unused_imports)]
use chasm::providers::cloud::perplexity::parse_perplexity_export;
use chasm::providers::{
    CloudConversation, CloudMessage, FetchOptions, GenericMessage, GenericSession, ProviderType,
};
use chrono::{TimeZone, Utc};

fn main() -> anyhow::Result<()> {
    println!("=== CSM Cloud Provider Examples ===\n");

    // ========================================================================
    // Example 1: List all cloud providers
    // ========================================================================
    println!("1. Available cloud providers:");

    let cloud_providers = vec![
        (ProviderType::ChatGPT, "OPENAI_API_KEY"),
        (ProviderType::Anthropic, "ANTHROPIC_API_KEY"),
        (ProviderType::Perplexity, "PERPLEXITY_API_KEY"),
        (ProviderType::DeepSeek, "DEEPSEEK_API_KEY"),
        (ProviderType::Gemini, "GOOGLE_API_KEY"),
        (ProviderType::M365Copilot, "MICROSOFT_GRAPH_TOKEN"),
        (ProviderType::Mistral, "MISTRAL_API_KEY"),
        (ProviderType::Cohere, "COHERE_API_KEY"),
        (ProviderType::Groq, "GROQ_API_KEY"),
        (ProviderType::Together, "TOGETHER_API_KEY"),
    ];

    for (provider, env_var) in &cloud_providers {
        let has_key = std::env::var(env_var).is_ok();
        let status = if has_key { "+" } else { "o" };
        println!("   {} {} ({})", status, provider.display_name(), env_var);
    }

    // ========================================================================
    // Example 2: Parse ChatGPT export data
    // ========================================================================
    println!("\n2. Parsing ChatGPT export:");

    // ChatGPT export format uses "mapping" with nested messages
    let chatgpt_export = r#"[
        {
            "id": "conv-001",
            "title": "Rust Programming",
            "create_time": 1700000000.0,
            "update_time": 1700001000.0,
            "mapping": {
                "node-1": {
                    "id": "node-1",
                    "message": {
                        "id": "msg-1",
                        "author": {"role": "user"},
                        "content": {"parts": ["How do I use iterators in Rust?"]},
                        "create_time": 1700000000.0
                    }
                },
                "node-2": {
                    "id": "node-2",
                    "message": {
                        "id": "msg-2",
                        "author": {"role": "assistant"},
                        "content": {"parts": ["Iterators in Rust are lazy and powerful..."]},
                        "create_time": 1700000100.0
                    }
                }
            }
        }
    ]"#;

    match parse_chatgpt_export(chatgpt_export) {
        Ok(conversations) => {
            println!("   Parsed {} conversation(s)", conversations.len());
            for conv in &conversations {
                println!(
                    "   - {} ({} messages)",
                    conv.title.as_deref().unwrap_or("Untitled"),
                    conv.messages.len()
                );
            }
        }
        Err(e) => println!("   Parse error: {}", e),
    }

    // ========================================================================
    // Example 3: Parse Claude export data
    // ========================================================================
    println!("\n3. Parsing Claude export:");

    // Claude export format uses "chat_messages" array
    let claude_export = r#"[
        {
            "uuid": "claude-conv-001",
            "name": "Python Data Analysis",
            "created_at": "2024-01-15T10:30:00Z",
            "updated_at": "2024-01-15T11:00:00Z",
            "chat_messages": [
                {
                    "uuid": "msg-001",
                    "sender": "human",
                    "text": "How do I read a CSV file in pandas?",
                    "created_at": "2024-01-15T10:30:00Z"
                },
                {
                    "uuid": "msg-002",
                    "sender": "assistant",
                    "text": "You can use pd.read_csv('file.csv')...",
                    "created_at": "2024-01-15T10:30:30Z"
                }
            ]
        }
    ]"#;

    match parse_claude_export(claude_export) {
        Ok(conversations) => {
            println!("   Parsed {} conversation(s)", conversations.len());
            for conv in &conversations {
                println!(
                    "   - {} ({} messages)",
                    conv.title.as_deref().unwrap_or("Untitled"),
                    conv.messages.len()
                );
            }
        }
        Err(e) => println!("   Parse error: {}", e),
    }

    // ========================================================================
    // Example 4: Parse M365 Copilot Graph API response
    // ========================================================================
    println!("\n4. Parsing Microsoft 365 Copilot data:");

    // M365 Copilot uses Microsoft Graph API response format
    let m365_export = r#"{
        "value": [
            {
                "id": "interaction-001",
                "createdDateTime": "2024-01-15T10:00:00Z",
                "appClass": "Office.Word",
                "body": {
                    "content": "<p>Help me write an introduction</p>"
                },
                "from": {
                    "user": {"displayName": "John Doe"}
                }
            },
            {
                "id": "interaction-002",
                "createdDateTime": "2024-01-15T10:00:30Z",
                "appClass": "Office.Word",
                "body": {
                    "content": "<p>Here's a professional introduction...</p>"
                },
                "from": {
                    "application": {"displayName": "Copilot"}
                }
            }
        ]
    }"#;

    match parse_m365_copilot_export(m365_export) {
        Ok(conversations) => {
            println!("   Parsed {} conversation(s)", conversations.len());
            for conv in &conversations {
                let app_name = get_friendly_app_name(
                    conv.metadata
                        .as_ref()
                        .and_then(|m| m.get("app_class"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown"),
                );
                println!(
                    "   - {} in {} ({} messages)",
                    conv.title.as_deref().unwrap_or("Untitled"),
                    app_name,
                    conv.messages.len()
                );
            }
        }
        Err(e) => println!("   Parse error: {}", e),
    }

    // Show friendly app names
    println!("\n   M365 Copilot application contexts:");
    let app_classes = [
        "Office.Word",
        "Office.Excel",
        "Office.PowerPoint",
        "Office.Outlook",
        "Teams",
        "Bing",
    ];
    for app in &app_classes {
        println!("     {} -> {}", app, get_friendly_app_name(app));
    }

    // ========================================================================
    // Example 5: Create CloudConversation and convert to ChatSession
    // ========================================================================
    println!("\n5. CloudConversation to ChatSession conversion:");

    let cloud_conv = CloudConversation {
        id: "cloud-123".to_string(),
        title: Some("API Design Discussion".to_string()),
        created_at: Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap(),
        updated_at: Some(Utc.with_ymd_and_hms(2024, 1, 15, 11, 0, 0).unwrap()),
        model: Some("gpt-4".to_string()),
        messages: vec![
            CloudMessage {
                id: Some("msg-1".to_string()),
                role: "user".to_string(),
                content: "What are REST API best practices?".to_string(),
                timestamp: Some(Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap()),
                model: None,
            },
            CloudMessage {
                id: Some("msg-2".to_string()),
                role: "assistant".to_string(),
                content: "Here are key REST API best practices:\n\n1. Use nouns for resources\n2. Use HTTP methods correctly\n3. Version your API...".to_string(),
                timestamp: Some(Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 30).unwrap()),
                model: Some("gpt-4".to_string()),
            },
        ],
        metadata: Some(serde_json::json!({
            "source": "chatgpt",
            "exported_at": "2024-01-20"
        })),
    };

    println!("   Cloud conversation:");
    println!("     ID: {}", cloud_conv.id);
    println!("     Title: {:?}", cloud_conv.title);
    println!("     Model: {:?}", cloud_conv.model);
    println!("     Messages: {}", cloud_conv.messages.len());

    // Convert to VS Code format
    let chat_session = cloud_conv.to_chat_session("ChatGPT");
    println!("\n   Converted ChatSession:");
    println!("     Session ID: {:?}", chat_session.session_id);
    println!("     Is imported: {}", chat_session.is_imported);
    println!("     Requests: {}", chat_session.requests.len());
    println!("     Responder: {:?}", chat_session.responder_username);

    // ========================================================================
    // Example 6: FetchOptions for filtering cloud conversations
    // ========================================================================
    println!("\n6. FetchOptions configuration:");

    // Default options
    let default_opts = FetchOptions::default();
    println!("   Default options:");
    println!("     Limit: {:?}", default_opts.limit);
    println!("     Include archived: {}", default_opts.include_archived);

    // Custom options for date range
    let custom_opts = FetchOptions {
        limit: Some(100),
        after: Some(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()),
        before: Some(Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap()),
        include_archived: true,
        session_token: None,
    };

    println!("\n   Custom options (2024 conversations only):");
    println!("     Limit: {:?}", custom_opts.limit);
    println!("     After: {:?}", custom_opts.after);
    println!("     Before: {:?}", custom_opts.before);
    println!("     Include archived: {}", custom_opts.include_archived);

    // ========================================================================
    // Example 7: Export format detection
    // ========================================================================
    println!("\n7. Export format detection:");

    fn detect_export_format(json_data: &str) -> &'static str {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_data) {
            // M365 Copilot (Graph API response)
            if value.get("value").is_some() && value.get("@odata.context").is_some() {
                return "Microsoft 365 Copilot (Graph API)";
            }

            // M365 with just "value" array
            if value.get("value").is_some() {
                return "Microsoft 365 Copilot (Graph API)";
            }

            // Array format
            if let Some(arr) = value.as_array() {
                if let Some(first) = arr.first() {
                    // ChatGPT has "mapping" field
                    if first.get("mapping").is_some() {
                        return "ChatGPT";
                    }
                    // Claude has "chat_messages" field
                    if first.get("chat_messages").is_some() {
                        return "Claude (Anthropic)";
                    }
                    // Perplexity has "entries" field
                    if first.get("entries").is_some() {
                        return "Perplexity";
                    }
                    // DeepSeek has specific structure
                    if first.get("messages").is_some() && first.get("title").is_some() {
                        return "DeepSeek";
                    }
                    // Gemini structure
                    if first.get("contents").is_some() {
                        return "Google Gemini";
                    }
                }
            }
        }
        "Unknown format"
    }

    let test_formats = [
        (r#"[{"mapping": {}, "title": "Test"}]"#, "ChatGPT-like"),
        (r#"[{"chat_messages": [], "uuid": "123"}]"#, "Claude-like"),
        (r#"{"value": []}"#, "M365-like"),
    ];

    for (data, expected) in &test_formats {
        let detected = detect_export_format(data);
        println!("   {} -> {}", expected, detected);
    }

    // ========================================================================
    // Example 8: Cloud to Generic to ChatSession pipeline
    // ========================================================================
    println!("\n8. Full import pipeline:");

    // Simulate importing from cloud provider
    let imported_messages = vec![
        GenericMessage {
            role: "user".to_string(),
            content: "Explain async/await in JavaScript".to_string(),
            timestamp: Some(Utc::now().timestamp_millis()),
            model: None,
        },
        GenericMessage {
            role: "assistant".to_string(),
            content: "Async/await is syntactic sugar for Promises...".to_string(),
            timestamp: Some(Utc::now().timestamp_millis()),
            model: Some("claude-3-opus".to_string()),
        },
    ];

    let generic_session = GenericSession {
        id: "imported-001".to_string(),
        title: Some("JavaScript Async Help".to_string()),
        messages: imported_messages,
        created_at: Some(Utc::now().timestamp_millis()),
        updated_at: Some(Utc::now().timestamp_millis()),
        provider: Some("Claude".to_string()),
        model: Some("claude-3-opus".to_string()),
    };

    println!("   Step 1: Generic session created");
    println!("     Source: {:?}", generic_session.provider);
    println!("     Messages: {}", generic_session.messages.len());

    // Convert to ChatSession (VS Code format)
    let chat_session: chasm::models::ChatSession = generic_session.into();

    println!("   Step 2: Converted to ChatSession");
    println!("     Session ID: {:?}", chat_session.session_id);
    println!("     Is imported: {}", chat_session.is_imported);

    // Serialize to JSON (ready for VS Code)
    let session_json = serde_json::to_string_pretty(&chat_session)?;
    println!("   Step 3: Ready for VS Code import");
    println!("     JSON size: {} bytes", session_json.len());
    println!("     Preview:");
    for line in session_json.lines().take(8) {
        println!("       {}", line);
    }
    println!("       ...");

    // ========================================================================
    // Example 9: Provider endpoints and compatibility
    // ========================================================================
    println!("\n9. Cloud provider API information:");

    let providers_info = [
        (
            ProviderType::ChatGPT,
            "https://chat.openai.com/backend-api",
            "Session token",
        ),
        (
            ProviderType::Anthropic,
            "https://claude.ai/api",
            "Session token",
        ),
        (
            ProviderType::Perplexity,
            "https://www.perplexity.ai/api",
            "Session token",
        ),
        (
            ProviderType::DeepSeek,
            "https://chat.deepseek.com/api",
            "API key",
        ),
        (
            ProviderType::Gemini,
            "https://generativelanguage.googleapis.com/v1",
            "API key",
        ),
        (
            ProviderType::M365Copilot,
            "https://graph.microsoft.com/v1.0",
            "OAuth token",
        ),
    ];

    for (provider, endpoint, auth) in &providers_info {
        println!("   {}:", provider.display_name());
        println!("     Endpoint: {}", endpoint);
        println!("     Auth: {}", auth);
    }

    println!("\n=== Cloud Provider Examples Complete ===");
    Ok(())
}
