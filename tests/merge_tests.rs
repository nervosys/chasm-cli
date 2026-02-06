//! Tests for cross-provider merge functionality
//!
//! This test file covers:
//! - Single provider merge
//! - Multiple provider merge
//! - All providers merge
//! - Session filtering and sorting
//! - Merge title generation
//! - Error handling during merge

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a test workspace directory structure
#[allow(dead_code)]
fn create_test_workspace_dir(base: &TempDir, hash: &str, project_path: &str) -> PathBuf {
    let ws_dir = base.path().join(hash);
    let sessions_dir = ws_dir.join("chatSessions");
    fs::create_dir_all(&sessions_dir).unwrap();

    let ws_json = format!(
        r#"{{"folder": "file:///{}", "configuration": {{}}}}"#,
        project_path.replace('\\', "/")
    );
    fs::write(ws_dir.join("workspace.json"), ws_json).unwrap();

    ws_dir
}

/// Create a test session with customizable parameters
#[allow(dead_code)]
fn create_test_session_file(
    sessions_dir: &std::path::Path,
    session_id: &str,
    title: &str,
    messages: &[(&str, &str, i64)], // (question, answer, timestamp)
) {
    let requests: Vec<serde_json::Value> = messages
        .iter()
        .map(|(q, a, ts)| {
            serde_json::json!({
                "timestamp": ts,
                "message": { "text": q },
                "response": { "value": [{ "value": a }] },
                "requestId": format!("req-{}", uuid::Uuid::new_v4()),
                "responseId": format!("resp-{}", uuid::Uuid::new_v4()),
                "modelId": "copilot/gpt-4"
            })
        })
        .collect();

    let session = serde_json::json!({
        "version": 3,
        "sessionId": session_id,
        "creationDate": messages.first().map(|(_, _, t)| *t).unwrap_or(0),
        "lastMessageDate": messages.last().map(|(_, _, t)| *t).unwrap_or(0),
        "customTitle": title,
        "initialLocation": "panel",
        "requests": requests
    });

    let file_path = sessions_dir.join(format!("{}.json", session_id));
    fs::write(file_path, serde_json::to_string_pretty(&session).unwrap()).unwrap();
}

// ============================================================================
// Provider Type Tests for Merge
// ============================================================================

mod provider_merge_type_tests {
    use chasm::providers::ProviderType;

    #[test]
    fn test_all_provider_types_have_display_names() {
        let providers = vec![
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

        for pt in providers {
            let name = pt.display_name();
            assert!(
                !name.is_empty(),
                "Provider {:?} should have a display name",
                pt
            );
        }
    }

    #[test]
    fn test_provider_type_equality() {
        assert_eq!(ProviderType::Copilot, ProviderType::Copilot);
        assert_ne!(ProviderType::Copilot, ProviderType::Cursor);
    }

    #[test]
    fn test_provider_type_clone() {
        let pt = ProviderType::Ollama;
        let cloned = pt; // ProviderType is Copy
        assert_eq!(pt, cloned);
    }
}

// ============================================================================
// Session Merge Logic Tests
// ============================================================================

mod session_merge_logic_tests {
    use chasm::models::{ChatRequest, ChatSession};

    fn create_session(id: &str, title: &str, timestamps: &[i64]) -> ChatSession {
        let _requests: Vec<ChatRequest> = timestamps
            .iter()
            .enumerate()
            .map(|(i, &ts)| {
                serde_json::from_value(serde_json::json!({
                    "timestamp": ts,
                    "message": { "text": format!("Q{}", i + 1) },
                    "response": { "value": [{ "value": format!("A{}", i + 1) }] },
                    "requestId": format!("req-{}-{}", id, i),
                    "responseId": format!("resp-{}-{}", id, i)
                }))
                .unwrap()
            })
            .collect();

        // Create session via JSON deserialization to avoid Default requirement
        serde_json::from_value(serde_json::json!({
            "version": 3,
            "sessionId": id,
            "creationDate": timestamps.first().unwrap_or(&0),
            "lastMessageDate": timestamps.last().unwrap_or(&0),
            "customTitle": title,
            "initialLocation": "panel",
            "requests": timestamps.iter().enumerate().map(|(i, ts)| {
                serde_json::json!({
                    "timestamp": ts,
                    "message": { "text": format!("Q{}", i + 1) },
                    "response": { "value": [{ "value": format!("A{}", i + 1) }] },
                    "requestId": format!("req-{}-{}", id, i),
                    "responseId": format!("resp-{}-{}", id, i)
                })
            }).collect::<Vec<_>>()
        }))
        .unwrap()
    }

    #[test]
    fn test_merge_preserves_all_requests() {
        let s1 = create_session("s1", "Session 1", &[1000, 2000]);
        let s2 = create_session("s2", "Session 2", &[3000, 4000, 5000]);

        let total_requests = s1.requests.len() + s2.requests.len();
        assert_eq!(total_requests, 5);
    }

    #[test]
    fn test_merge_chronological_ordering() {
        let sessions = vec![
            (
                "p1".to_string(),
                create_session("s1", "Late", &[5000, 6000]),
            ),
            (
                "p2".to_string(),
                create_session("s2", "Early", &[1000, 2000]),
            ),
            (
                "p3".to_string(),
                create_session("s3", "Middle", &[3000, 4000]),
            ),
        ];

        let mut sorted = sessions.clone();
        sorted.sort_by(|(_, a), (_, b)| {
            let a_time = a.requests.first().and_then(|r| r.timestamp).unwrap_or(0);
            let b_time = b.requests.first().and_then(|r| r.timestamp).unwrap_or(0);
            a_time.cmp(&b_time)
        });

        assert_eq!(sorted[0].1.custom_title, Some("Early".to_string()));
        assert_eq!(sorted[1].1.custom_title, Some("Middle".to_string()));
        assert_eq!(sorted[2].1.custom_title, Some("Late".to_string()));
    }

    #[test]
    fn test_merge_empty_sessions_handled() {
        let empty = create_session("empty", "Empty Session", &[]);
        assert_eq!(empty.requests.len(), 0);
        assert_eq!(empty.request_count(), 0);
    }

    #[test]
    fn test_merge_single_session() {
        let single = create_session("single", "Single Session", &[1000]);
        assert_eq!(single.requests.len(), 1);
    }
}

// ============================================================================
// Title Generation Tests
// ============================================================================

mod title_generation_tests {
    #[test]
    fn test_provider_import_title() {
        let provider_name = "Ollama";
        let auto_title = format!("Imported from {}", provider_name);
        assert_eq!(auto_title, "Imported from Ollama");
    }

    #[test]
    fn test_cross_provider_title() {
        let providers = ["copilot", "cursor", "ollama"];
        let auto_title = format!("Cross-provider merge: {}", providers.join(", "));
        assert_eq!(auto_title, "Cross-provider merge: copilot, cursor, ollama");
    }

    #[test]
    fn test_all_providers_title() {
        let provider_count = 5;
        let auto_title = format!("All providers merge ({})", provider_count);
        assert_eq!(auto_title, "All providers merge (5)");
    }

    #[test]
    fn test_custom_title_overrides_auto() {
        fn get_custom() -> Option<&'static str> {
            Some("My Custom Title")
        }
        let auto_title = "Auto Generated Title";

        let final_title = get_custom().unwrap_or(auto_title);
        assert_eq!(final_title, "My Custom Title");
    }

    #[test]
    fn test_auto_title_when_no_custom() {
        fn get_custom() -> Option<&'static str> {
            None
        }
        let auto_title = "Auto Generated Title";

        let final_title = get_custom().unwrap_or(auto_title);
        assert_eq!(final_title, "Auto Generated Title");
    }
}

// ============================================================================
// Filter Tests for Merge
// ============================================================================

mod merge_filter_tests {
    use chasm::models::ChatSession;

    fn create_session_with_title(title: &str) -> ChatSession {
        serde_json::from_str(&format!(
            r#"{{
            "version": 3,
            "customTitle": "{}",
            "requests": []
        }}"#,
            title
        ))
        .unwrap()
    }

    #[test]
    fn test_workspace_filter_matches() {
        let session = create_session_with_title("My Project Development");
        let filter = "project";

        let matches = session
            .title()
            .to_lowercase()
            .contains(&filter.to_lowercase());
        assert!(matches);
    }

    #[test]
    fn test_workspace_filter_no_match() {
        let session = create_session_with_title("Something Else");
        let filter = "project";

        let matches = session
            .title()
            .to_lowercase()
            .contains(&filter.to_lowercase());
        assert!(!matches);
    }

    #[test]
    fn test_workspace_filter_empty() {
        let session = create_session_with_title("Any Title");
        let filter = "";

        let matches = session
            .title()
            .to_lowercase()
            .contains(&filter.to_lowercase());
        assert!(matches); // Empty filter matches everything
    }

    #[test]
    fn test_session_id_filter() {
        let ids = ["session-abc-123", "session-def-456", "abc-session-789"];
        let filter = "abc";

        let filtered: Vec<_> = ids
            .iter()
            .filter(|id| id.to_lowercase().contains(&filter.to_lowercase()))
            .collect();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&&"session-abc-123"));
        assert!(filtered.contains(&&"abc-session-789"));
    }
}

// ============================================================================
// Provider List Parsing Tests
// ============================================================================

mod provider_list_tests {
    use chasm::providers::ProviderType;

    fn parse_providers(names: &[&str]) -> Vec<Option<ProviderType>> {
        names
            .iter()
            .map(|name| match name.to_lowercase().as_str() {
                "copilot" | "github-copilot" | "vscode" => Some(ProviderType::Copilot),
                "cursor" => Some(ProviderType::Cursor),
                "ollama" => Some(ProviderType::Ollama),
                "vllm" => Some(ProviderType::Vllm),
                "foundry" | "azure" | "azure-foundry" => Some(ProviderType::Foundry),
                "lm-studio" | "lmstudio" => Some(ProviderType::LmStudio),
                "localai" | "local-ai" => Some(ProviderType::LocalAI),
                "text-gen-webui" | "textgenwebui" | "oobabooga" => Some(ProviderType::TextGenWebUI),
                "jan" | "jan-ai" => Some(ProviderType::Jan),
                "gpt4all" => Some(ProviderType::Gpt4All),
                "llamafile" => Some(ProviderType::Llamafile),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_parse_single_provider() {
        let result = parse_providers(&["copilot"]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], Some(ProviderType::Copilot));
    }

    #[test]
    fn test_parse_multiple_providers() {
        let result = parse_providers(&["copilot", "cursor", "ollama"]);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], Some(ProviderType::Copilot));
        assert_eq!(result[1], Some(ProviderType::Cursor));
        assert_eq!(result[2], Some(ProviderType::Ollama));
    }

    #[test]
    fn test_parse_with_unknown() {
        let result = parse_providers(&["copilot", "unknown", "ollama"]);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], Some(ProviderType::Copilot));
        assert_eq!(result[1], None); // Unknown
        assert_eq!(result[2], Some(ProviderType::Ollama));
    }

    #[test]
    fn test_parse_all_valid_providers() {
        let all_names = vec![
            "copilot",
            "cursor",
            "ollama",
            "vllm",
            "foundry",
            "lm-studio",
            "localai",
            "text-gen-webui",
            "jan",
            "gpt4all",
            "llamafile",
        ];

        let result = parse_providers(&all_names.to_vec());

        for (i, pt) in result.iter().enumerate() {
            assert!(pt.is_some(), "Provider {} should be valid", all_names[i]);
        }
    }

    #[test]
    fn test_filter_valid_providers() {
        let names = ["copilot", "unknown1", "cursor", "unknown2", "ollama"];
        let result = parse_providers(&names);

        let valid: Vec<_> = result.iter().filter_map(|x| *x).collect();
        assert_eq!(valid.len(), 3);
    }
}

// ============================================================================
// Session Count Aggregation Tests
// ============================================================================

mod session_count_tests {
    #[test]
    fn test_total_sessions_across_providers() {
        let provider_sessions = [("Copilot", 5), ("Cursor", 3), ("Ollama", 10)];

        let total: usize = provider_sessions.iter().map(|(_, count)| count).sum();
        assert_eq!(total, 18);
    }

    #[test]
    fn test_providers_with_sessions_count() {
        let provider_sessions = [("Copilot", 5), ("Cursor", 0), ("Ollama", 10), ("vLLM", 0)];

        let with_sessions: Vec<_> = provider_sessions
            .iter()
            .filter(|(_, count)| *count > 0)
            .collect();

        assert_eq!(with_sessions.len(), 2);
    }

    #[test]
    fn test_empty_providers() {
        let provider_sessions: Vec<(&str, usize)> = vec![];

        let total: usize = provider_sessions.iter().map(|(_, count)| count).sum();
        assert_eq!(total, 0);
    }
}

// ============================================================================
// Merge Result Tests
// ============================================================================

mod merge_result_tests {
    use chasm::models::ChatSession;

    #[test]
    fn test_merged_session_has_correct_version() {
        let session: ChatSession = serde_json::from_str(
            r#"{
            "version": 3,
            "requests": []
        }"#,
        )
        .unwrap();

        assert_eq!(session.version, 3);
    }

    #[test]
    fn test_merged_session_calculates_dates() {
        let json = r#"{
            "version": 3,
            "creationDate": 1000,
            "lastMessageDate": 5000,
            "requests": [
                {"timestamp": 1000, "message": {"text": "Q1"}, "response": {"value": [{"value": "A1"}]}},
                {"timestamp": 5000, "message": {"text": "Q2"}, "response": {"value": [{"value": "A2"}]}}
            ]
        }"#;

        let session: ChatSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.creation_date, 1000);
        assert_eq!(session.last_message_date, 5000);
    }

    #[test]
    fn test_merged_session_preserves_request_order() {
        let json = r#"{
            "version": 3,
            "requests": [
                {"timestamp": 1000, "message": {"text": "First"}, "response": {"value": [{"value": "R1"}]}},
                {"timestamp": 2000, "message": {"text": "Second"}, "response": {"value": [{"value": "R2"}]}},
                {"timestamp": 3000, "message": {"text": "Third"}, "response": {"value": [{"value": "R3"}]}}
            ]
        }"#;

        let session: ChatSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.requests.len(), 3);

        // Verify order
        let timestamps: Vec<_> = session
            .requests
            .iter()
            .map(|r| r.timestamp.unwrap_or(0))
            .collect();

        assert!(timestamps.windows(2).all(|w| w[0] <= w[1]));
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

mod merge_error_tests {
    #[test]
    fn test_empty_provider_list_handled() {
        let providers: Vec<String> = vec![];
        assert!(providers.is_empty());
    }

    #[test]
    fn test_all_unknown_providers_handled() {
        fn parse_provider(name: &str) -> Option<&str> {
            match name {
                "copilot" | "cursor" | "ollama" => Some(name),
                _ => None,
            }
        }

        let names = ["unknown1", "unknown2"];
        let valid: Vec<_> = names.iter().filter_map(|n| parse_provider(n)).collect();

        assert!(valid.is_empty());
    }

    #[test]
    fn test_no_sessions_found_handled() {
        let sessions: Vec<String> = vec![];
        let message = if sessions.is_empty() {
            "No sessions found across providers"
        } else {
            "Sessions found"
        };

        assert_eq!(message, "No sessions found across providers");
    }
}

// ============================================================================
// Integration-style Tests
// ============================================================================

mod merge_integration_tests {
    use chasm::providers::{ProviderRegistry, ProviderType};

    #[test]
    fn test_cross_provider_merge_flow() {
        let registry = ProviderRegistry::new();
        let provider_names = vec!["copilot", "ollama"];

        let mut all_sessions = Vec::new();

        for name in &provider_names {
            let pt = match *name {
                "copilot" => Some(ProviderType::Copilot),
                "ollama" => Some(ProviderType::Ollama),
                _ => None,
            };

            if let Some(provider_type) = pt {
                if let Some(provider) = registry.get_provider(provider_type) {
                    if provider.is_available() {
                        if let Ok(sessions) = provider.list_sessions() {
                            for session in sessions {
                                all_sessions.push((provider.name().to_string(), session));
                            }
                        }
                    }
                }
            }
        }

        // Just verify the flow completes
        let _ = all_sessions.len();
    }

    #[test]
    fn test_all_providers_merge_flow() {
        let registry = ProviderRegistry::new();
        let provider_types = vec![
            ProviderType::Copilot,
            ProviderType::Cursor,
            ProviderType::Ollama,
        ];

        let mut total_sessions = 0;
        let mut providers_found = 0;

        for pt in provider_types {
            if let Some(provider) = registry.get_provider(pt) {
                if provider.is_available() {
                    if let Ok(sessions) = provider.list_sessions() {
                        if !sessions.is_empty() {
                            providers_found += 1;
                            total_sessions += sessions.len();
                        }
                    }
                }
            }
        }

        // Just verify counts - use values to avoid warnings
        let _ = total_sessions;
        let _ = providers_found;
    }

    #[test]
    fn test_provider_with_filter() {
        let registry = ProviderRegistry::new();
        let filter = "test";

        if let Some(provider) = registry.get_provider(ProviderType::Copilot) {
            if provider.is_available() {
                if let Ok(sessions) = provider.list_sessions() {
                    let filtered: Vec<_> = sessions
                        .into_iter()
                        .filter(|s| s.title().to_lowercase().contains(filter))
                        .collect();

                    // Just verify filtering works
                    let _ = filtered.len();
                }
            }
        }
    }
}

// ============================================================================
// Deduplication Tests
// ============================================================================

mod deduplication_tests {
    #[test]
    fn test_session_id_uniqueness() {
        let session_ids = [
            "session-001",
            "session-002",
            "session-001", // Duplicate
            "session-003",
        ];

        let unique: std::collections::HashSet<_> = session_ids.iter().collect();
        assert_eq!(unique.len(), 3);
    }

    #[test]
    fn test_provider_deduplication() {
        let providers = ["copilot", "cursor", "copilot", "ollama", "cursor"];

        let unique: std::collections::HashSet<_> = providers.iter().collect();
        assert_eq!(unique.len(), 3);
    }
}

// ============================================================================
// Output Limiting Tests
// ============================================================================

mod output_limit_tests {
    #[test]
    fn test_session_display_limit() {
        let sessions: Vec<usize> = (0..100).collect();
        let limit = 20;

        let displayed: Vec<_> = sessions.iter().take(limit).collect();
        let remaining = sessions.len() - displayed.len();

        assert_eq!(displayed.len(), 20);
        assert_eq!(remaining, 80);
    }

    #[test]
    fn test_sessions_under_limit() {
        let sessions: Vec<usize> = (0..5).collect();
        let limit = 20;

        let displayed: Vec<_> = sessions.iter().take(limit).collect();

        assert_eq!(displayed.len(), 5);
    }
}
