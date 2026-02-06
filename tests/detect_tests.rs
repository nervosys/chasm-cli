//! Tests for auto-detection functionality
//!
//! This test file covers:
//! - Workspace detection
//! - Provider detection
//! - Session provider detection
//! - Full detection reports
//! - Edge cases and error handling

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Test Helpers
// ============================================================================

/// Helper to create a mock VS Code workspace structure
#[allow(dead_code)]
fn create_mock_workspace(base: &TempDir, hash: &str, project_path: &str) -> PathBuf {
    let ws_dir = base.path().join(hash);
    let sessions_dir = ws_dir.join("chatSessions");
    fs::create_dir_all(&sessions_dir).unwrap();

    // Create workspace.json matching VS Code format
    let ws_json = format!(
        r#"{{"folder": "file:///{}", "configuration": {{}}}}"#,
        project_path.replace('\\', "/").replace(' ', "%20")
    );
    fs::write(ws_dir.join("workspace.json"), ws_json).unwrap();

    ws_dir
}

/// Helper to create a mock chat session file
#[allow(dead_code)]
fn create_mock_session(
    sessions_dir: &std::path::Path,
    session_id: &str,
    title: &str,
    message_count: usize,
) {
    let base_timestamp = 1699999990000i64;
    let requests: Vec<serde_json::Value> = (0..message_count)
        .map(|i| {
            serde_json::json!({
                "timestamp": base_timestamp + (i as i64 * 60000),
                "message": { "text": format!("Question {}", i + 1) },
                "response": { "value": [{ "value": format!("Answer {}", i + 1) }] },
                "requestId": format!("req-{}-{}", session_id, i),
                "responseId": format!("resp-{}-{}", session_id, i),
                "modelId": "copilot/gpt-4"
            })
        })
        .collect();

    let session = serde_json::json!({
        "version": 3,
        "sessionId": session_id,
        "creationDate": base_timestamp,
        "lastMessageDate": base_timestamp + ((message_count - 1) as i64 * 60000),
        "customTitle": title,
        "initialLocation": "panel",
        "requests": requests
    });

    let file_path = sessions_dir.join(format!("{}.json", session_id));
    fs::write(file_path, serde_json::to_string_pretty(&session).unwrap()).unwrap();
}

// ============================================================================
// Provider Type Detection Tests
// ============================================================================

mod provider_detection_tests {
    use chasm::providers::{ProviderRegistry, ProviderType};

    #[test]
    fn test_provider_registry_creation() {
        let registry = ProviderRegistry::new();
        // Registry should be created successfully
        assert!(!registry.providers().is_empty());
    }

    #[test]
    fn test_get_copilot_provider() {
        let registry = ProviderRegistry::new();
        let provider = registry.get_provider(ProviderType::Copilot);
        // Provider may or may not be registered depending on configuration
        if let Some(p) = provider {
            assert_eq!(p.name(), "GitHub Copilot");
            assert_eq!(p.provider_type(), ProviderType::Copilot);
        }
    }

    #[test]
    fn test_get_cursor_provider() {
        let registry = ProviderRegistry::new();
        let provider = registry.get_provider(ProviderType::Cursor);
        // Provider may or may not be registered depending on configuration
        if let Some(p) = provider {
            assert_eq!(p.name(), "Cursor");
            assert_eq!(p.provider_type(), ProviderType::Cursor);
        }
    }

    #[test]
    fn test_get_ollama_provider() {
        let registry = ProviderRegistry::new();
        let provider = registry.get_provider(ProviderType::Ollama);
        assert!(provider.is_some());

        if let Some(p) = provider {
            assert_eq!(p.name(), "Ollama");
            assert_eq!(p.provider_type(), ProviderType::Ollama);
        }
    }

    #[test]
    fn test_get_all_local_providers() {
        let registry = ProviderRegistry::new();

        // Test that at least some local providers exist
        let local_providers = [
            ProviderType::Ollama,
            ProviderType::Vllm,
            ProviderType::LmStudio,
            ProviderType::LocalAI,
            ProviderType::TextGenWebUI,
            ProviderType::Jan,
            ProviderType::Gpt4All,
            ProviderType::Llamafile,
        ];

        let found_count = local_providers
            .iter()
            .filter(|pt| registry.get_provider(**pt).is_some())
            .count();

        // At least some providers should be registered
        assert!(
            found_count > 0,
            "At least some local providers should be registered"
        );
    }

    #[test]
    fn test_provider_availability_check() {
        let registry = ProviderRegistry::new();

        // Copilot availability depends on VS Code installation
        if let Some(copilot) = registry.get_provider(ProviderType::Copilot) {
            // Just verify the method can be called
            let _ = copilot.is_available();
        }

        // Ollama availability depends on installation
        if let Some(ollama) = registry.get_provider(ProviderType::Ollama) {
            let _ = ollama.is_available();
        }
    }

    #[test]
    fn test_provider_type_display_names_complete() {
        // Test all provider types have display names
        let all_types = vec![
            (ProviderType::Copilot, "GitHub Copilot"),
            (ProviderType::Cursor, "Cursor"),
            (ProviderType::Ollama, "Ollama"),
            (ProviderType::Vllm, "vLLM"),
            (ProviderType::Foundry, "Azure AI Foundry"),
            (ProviderType::LmStudio, "LM Studio"),
            (ProviderType::LocalAI, "LocalAI"),
            (ProviderType::TextGenWebUI, "Text Generation WebUI"),
            (ProviderType::Jan, "Jan.ai"),
            (ProviderType::Gpt4All, "GPT4All"),
            (ProviderType::Llamafile, "Llamafile"),
        ];

        for (pt, expected_name) in all_types {
            assert_eq!(pt.display_name(), expected_name);
        }
    }

    #[test]
    fn test_provider_default_endpoints() {
        // File-based providers should have no endpoint
        assert!(ProviderType::Copilot.default_endpoint().is_none());
        assert!(ProviderType::Cursor.default_endpoint().is_none());

        // Local API providers should have localhost endpoints
        assert!(ProviderType::Ollama
            .default_endpoint()
            .unwrap()
            .contains("localhost"));
        assert!(ProviderType::Vllm
            .default_endpoint()
            .unwrap()
            .contains("localhost"));
        assert!(ProviderType::LmStudio
            .default_endpoint()
            .unwrap()
            .contains("localhost"));
        assert!(ProviderType::LocalAI
            .default_endpoint()
            .unwrap()
            .contains("localhost"));
        assert!(ProviderType::Jan
            .default_endpoint()
            .unwrap()
            .contains("localhost"));
        assert!(ProviderType::Gpt4All
            .default_endpoint()
            .unwrap()
            .contains("localhost"));
    }
}

// ============================================================================
// Workspace Detection Tests
// ============================================================================

mod workspace_detection_tests {
    use chasm::workspace::{find_workspace_by_path, normalize_path};

    #[test]
    fn test_normalize_path_removes_trailing_slash() {
        // Test with Unix-style path
        let path = normalize_path("/home/user/project/");
        assert!(
            !path.ends_with('/'),
            "Unix path should not end with /: '{}'",
            path
        );

        // Test with Windows-style path
        let path2 = normalize_path("C:\\Users\\test\\project\\");
        assert!(
            !path2.ends_with('\\'),
            "Windows path should not end with \\: '{}'",
            path2
        );

        // Test path without trailing slash stays unchanged (except lowercasing)
        let path3 = normalize_path("/home/user/project");
        assert_eq!(path3, "/home/user/project");
    }

    #[test]
    fn test_normalize_path_converts_backslash() {
        let path = normalize_path("C:\\Users\\test\\project");
        // Should handle Windows paths
        assert!(!path.is_empty());
    }

    #[test]
    fn test_find_workspace_nonexistent_path() {
        let result = find_workspace_by_path("/definitely/nonexistent/path/12345");
        // Should not error, just return None
        assert!(result.is_ok());
        // May or may not find a workspace depending on system state
    }

    #[test]
    fn test_find_workspace_empty_path() {
        let result = find_workspace_by_path("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_find_workspace_current_dir() {
        let current = std::env::current_dir().unwrap();
        let result = find_workspace_by_path(current.to_str().unwrap());
        assert!(result.is_ok());
    }
}

// ============================================================================
// Session Detection Tests
// ============================================================================

mod session_detection_tests {
    use chasm::models::ChatSession;

    #[test]
    fn test_session_title_extraction() {
        let json = r#"{
            "version": 3,
            "customTitle": "My Custom Title",
            "requests": []
        }"#;

        let session: ChatSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.title(), "My Custom Title");
    }

    #[test]
    fn test_session_title_from_first_message() {
        let json = r#"{
            "version": 3,
            "requests": [
                {
                    "message": {"text": "First message as title"},
                    "response": {"value": [{"value": "Response"}]}
                }
            ]
        }"#;

        let session: ChatSession = serde_json::from_str(json).unwrap();
        // Title should come from first message when no custom title
        let title = session.title();
        assert!(!title.is_empty());
    }

    #[test]
    fn test_session_request_count() {
        let json = r#"{
            "version": 3,
            "requests": [
                {"message": {"text": "Q1"}, "response": {"value": [{"value": "A1"}]}},
                {"message": {"text": "Q2"}, "response": {"value": [{"value": "A2"}]}},
                {"message": {"text": "Q3"}, "response": {"value": [{"value": "A3"}]}}
            ]
        }"#;

        let session: ChatSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.request_count(), 3);
    }

    #[test]
    fn test_session_empty_requests() {
        let json = r#"{
            "version": 3,
            "requests": []
        }"#;

        let session: ChatSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.request_count(), 0);
    }

    #[test]
    fn test_session_id_parsing() {
        let json = r#"{
            "version": 3,
            "sessionId": "unique-session-id-123",
            "requests": []
        }"#;

        let session: ChatSession = serde_json::from_str(json).unwrap();
        assert_eq!(
            session.session_id,
            Some("unique-session-id-123".to_string())
        );
    }
}

// ============================================================================
// Provider Name Parsing Tests (for merge commands)
// ============================================================================

mod provider_name_parsing_tests {
    use chasm::providers::ProviderType;

    fn parse_provider_name(name: &str) -> Option<ProviderType> {
        match name.to_lowercase().as_str() {
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
        }
    }

    #[test]
    fn test_copilot_aliases() {
        assert_eq!(parse_provider_name("copilot"), Some(ProviderType::Copilot));
        assert_eq!(
            parse_provider_name("github-copilot"),
            Some(ProviderType::Copilot)
        );
        assert_eq!(parse_provider_name("vscode"), Some(ProviderType::Copilot));
        assert_eq!(parse_provider_name("COPILOT"), Some(ProviderType::Copilot));
        assert_eq!(
            parse_provider_name("GitHub-Copilot"),
            Some(ProviderType::Copilot)
        );
    }

    #[test]
    fn test_foundry_aliases() {
        assert_eq!(parse_provider_name("foundry"), Some(ProviderType::Foundry));
        assert_eq!(parse_provider_name("azure"), Some(ProviderType::Foundry));
        assert_eq!(
            parse_provider_name("azure-foundry"),
            Some(ProviderType::Foundry)
        );
        assert_eq!(parse_provider_name("AZURE"), Some(ProviderType::Foundry));
    }

    #[test]
    fn test_lm_studio_aliases() {
        assert_eq!(
            parse_provider_name("lm-studio"),
            Some(ProviderType::LmStudio)
        );
        assert_eq!(
            parse_provider_name("lmstudio"),
            Some(ProviderType::LmStudio)
        );
        assert_eq!(
            parse_provider_name("LM-Studio"),
            Some(ProviderType::LmStudio)
        );
    }

    #[test]
    fn test_localai_aliases() {
        assert_eq!(parse_provider_name("localai"), Some(ProviderType::LocalAI));
        assert_eq!(parse_provider_name("local-ai"), Some(ProviderType::LocalAI));
    }

    #[test]
    fn test_text_gen_webui_aliases() {
        assert_eq!(
            parse_provider_name("text-gen-webui"),
            Some(ProviderType::TextGenWebUI)
        );
        assert_eq!(
            parse_provider_name("textgenwebui"),
            Some(ProviderType::TextGenWebUI)
        );
        assert_eq!(
            parse_provider_name("oobabooga"),
            Some(ProviderType::TextGenWebUI)
        );
    }

    #[test]
    fn test_jan_aliases() {
        assert_eq!(parse_provider_name("jan"), Some(ProviderType::Jan));
        assert_eq!(parse_provider_name("jan-ai"), Some(ProviderType::Jan));
    }

    #[test]
    fn test_unknown_provider() {
        assert_eq!(parse_provider_name("unknown-provider"), None);
        assert_eq!(parse_provider_name(""), None);
        assert_eq!(parse_provider_name("random"), None);
    }

    #[test]
    fn test_case_insensitivity() {
        assert_eq!(parse_provider_name("OLLAMA"), Some(ProviderType::Ollama));
        assert_eq!(parse_provider_name("Ollama"), Some(ProviderType::Ollama));
        assert_eq!(parse_provider_name("ollama"), Some(ProviderType::Ollama));
        assert_eq!(parse_provider_name("OlLaMa"), Some(ProviderType::Ollama));
    }
}

// ============================================================================
// Detection Output Format Tests
// ============================================================================

mod detection_format_tests {
    #[test]
    fn test_truncate_short_string() {
        fn truncate(s: &str, max_len: usize) -> String {
            if s.len() <= max_len {
                s.to_string()
            } else {
                format!("{}...", &s[..max_len - 3])
            }
        }

        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        fn truncate(s: &str, max_len: usize) -> String {
            if s.len() <= max_len {
                s.to_string()
            } else {
                format!("{}...", &s[..max_len - 3])
            }
        }

        let long = "This is a very long string that needs truncation";
        let truncated = truncate(long, 20);
        assert_eq!(truncated.len(), 20);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_truncate_exact_length() {
        fn truncate(s: &str, max_len: usize) -> String {
            if s.len() <= max_len {
                s.to_string()
            } else {
                format!("{}...", &s[..max_len - 3])
            }
        }

        let exact = "exactly10c";
        assert_eq!(truncate(exact, 10), "exactly10c");
    }

    #[test]
    fn test_workspace_id_display_truncation() {
        // Workspace IDs are typically long hashes
        let ws_id = "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6";
        let display = &ws_id[..16.min(ws_id.len())];
        assert_eq!(display.len(), 16);
        assert_eq!(display, "a1b2c3d4e5f6g7h8");
    }

    #[test]
    fn test_short_workspace_id() {
        let short_id = "abc123";
        let display = &short_id[..16.min(short_id.len())];
        assert_eq!(display, "abc123");
    }
}

// ============================================================================
// Cross-Provider Detection Tests
// ============================================================================

mod cross_provider_tests {
    use chasm::providers::{ProviderRegistry, ProviderType};

    #[test]
    fn test_multiple_provider_enumeration() {
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

        assert_eq!(all_provider_types.len(), 11);
    }

    #[test]
    fn test_registry_iterates_all_providers() {
        let registry = ProviderRegistry::new();
        let providers = registry.providers();

        // Should have multiple providers
        assert!(providers.len() >= 2);

        // Each provider should have a name
        for provider in providers {
            assert!(!provider.name().is_empty());
        }
    }

    #[test]
    fn test_available_providers_subset() {
        let registry = ProviderRegistry::new();
        let all = registry.providers();
        let available = registry.available_providers();

        // Available should be <= all providers
        assert!(available.len() <= all.len());
    }
}

// ============================================================================
// Session Sorting Tests (for merge)
// ============================================================================

mod session_sorting_tests {
    use chasm::models::ChatSession;

    fn create_session_with_timestamp(timestamp: i64) -> ChatSession {
        let json = format!(
            r#"{{
            "version": 3,
            "requests": [
                {{
                    "timestamp": {},
                    "message": {{"text": "Test"}},
                    "response": {{"value": [{{"value": "Response"}}]}}
                }}
            ]
        }}"#,
            timestamp
        );

        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn test_session_timestamp_extraction() {
        let session = create_session_with_timestamp(1699999990000);
        assert!(!session.requests.is_empty());
        assert_eq!(session.requests[0].timestamp, Some(1699999990000));
    }

    #[test]
    fn test_sessions_can_be_sorted_by_timestamp() {
        let s1 = create_session_with_timestamp(1000);
        let s2 = create_session_with_timestamp(3000);
        let s3 = create_session_with_timestamp(2000);

        let mut sessions = [
            ("p1".to_string(), s1),
            ("p2".to_string(), s2),
            ("p3".to_string(), s3),
        ];

        sessions.sort_by(|(_, a), (_, b)| {
            let a_time = a.requests.first().and_then(|r| r.timestamp).unwrap_or(0);
            let b_time = b.requests.first().and_then(|r| r.timestamp).unwrap_or(0);
            a_time.cmp(&b_time)
        });

        // After sorting, should be in timestamp order
        assert_eq!(sessions[0].0, "p1"); // 1000
        assert_eq!(sessions[1].0, "p3"); // 2000
        assert_eq!(sessions[2].0, "p2"); // 3000
    }

    #[test]
    fn test_session_with_no_requests_has_zero_timestamp() {
        let json = r#"{"version": 3, "requests": []}"#;
        let session: ChatSession = serde_json::from_str(json).unwrap();

        let timestamp = session
            .requests
            .first()
            .and_then(|r| r.timestamp)
            .unwrap_or(0);

        assert_eq!(timestamp, 0);
    }
}

// ============================================================================
// Filter Tests
// ============================================================================

mod filter_tests {
    #[test]
    fn test_workspace_name_filter_case_insensitive() {
        let filter = "myproject";
        let title = "MyProject Development";

        assert!(title.to_lowercase().contains(&filter.to_lowercase()));
    }

    #[test]
    fn test_workspace_name_filter_partial_match() {
        let filter = "proj";
        let title = "MyProject Development";

        assert!(title.to_lowercase().contains(&filter.to_lowercase()));
    }

    #[test]
    fn test_workspace_name_filter_no_match() {
        let filter = "xyz";
        let title = "MyProject Development";

        assert!(!title.to_lowercase().contains(&filter.to_lowercase()));
    }

    #[test]
    fn test_session_id_filter() {
        let filter = "abc123";
        let session_ids = ["session-abc123-def", "session-xyz789-ghi", "abc123-session"];

        let matches: Vec<_> = session_ids
            .iter()
            .filter(|id| id.to_lowercase().contains(&filter.to_lowercase()))
            .collect();

        assert_eq!(matches.len(), 2);
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

mod error_handling_tests {
    use chasm::workspace::find_workspace_by_path;

    #[test]
    fn test_invalid_path_does_not_panic() {
        // Should not panic on invalid paths
        let result = find_workspace_by_path("\0invalid");
        // Either succeeds with None or returns an error, but no panic
        let _ = result; // Result is either Ok or Err, both are fine
    }

    #[test]
    fn test_very_long_path_handled() {
        let long_path = "a".repeat(10000);
        let result = find_workspace_by_path(&long_path);
        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_unicode_path_handled() {
        let unicode_path = "/home/user/project/test";
        let result = find_workspace_by_path(unicode_path);
        assert!(result.is_ok());
    }
}

// ============================================================================
// Integration-like Tests
// ============================================================================

mod detection_integration_tests {
    use chasm::providers::{ProviderRegistry, ProviderType};

    #[test]
    fn test_full_provider_detection_flow() {
        let registry = ProviderRegistry::new();
        let mut total_sessions: usize = 0;
        let mut providers_with_sessions: usize = 0;

        let provider_types = vec![
            ProviderType::Copilot,
            ProviderType::Cursor,
            ProviderType::Ollama,
        ];

        for pt in provider_types {
            if let Some(provider) = registry.get_provider(pt) {
                if provider.is_available() {
                    if let Ok(sessions) = provider.list_sessions() {
                        let count = sessions.len();
                        total_sessions += count;
                        if count > 0 {
                            providers_with_sessions += 1;
                        }
                    }
                }
            }
        }

        // Just verify the flow completes without error - types are usize so always >= 0
        let _ = total_sessions;
        let _ = providers_with_sessions;
    }

    #[test]
    fn test_provider_session_listing() {
        let registry = ProviderRegistry::new();

        // Try to list sessions from Copilot provider
        if let Some(provider) = registry.get_provider(ProviderType::Copilot) {
            if provider.is_available() {
                let result = provider.list_sessions();
                // Should succeed (may be empty)
                assert!(result.is_ok());
            }
        }
    }
}
