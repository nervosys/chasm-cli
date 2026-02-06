//! Integration tests for Chat System Manager (csm)
//!
//! These tests validate the CLI, workspace operations, and end-to-end functionality.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// =============================================================================
// Test Helpers
// =============================================================================

/// Helper to create a test workspace structure with proper VS Code layout
fn create_test_workspace(base: &TempDir, hash: &str, project_path: &str) -> PathBuf {
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

/// Helper to create a test chat session file with realistic structure
fn create_test_session(
    sessions_dir: &std::path::Path,
    session_id: &str,
    title: &str,
    messages: Vec<(&str, i64)>,
) {
    let requests: Vec<serde_json::Value> = messages
        .iter()
        .map(|(text, timestamp)| {
            serde_json::json!({
                "timestamp": timestamp,
                "message": { "text": text },
                "response": { "value": [{ "value": "AI response" }] },
                "requestId": format!("req-{}", uuid::Uuid::new_v4()),
                "responseId": format!("resp-{}", uuid::Uuid::new_v4()),
                "modelId": "copilot/gpt-4"
            })
        })
        .collect();

    let session = serde_json::json!({
        "version": 3,
        "sessionId": session_id,
        "creationDate": messages.first().map(|(_, t)| *t).unwrap_or(0),
        "lastMessageDate": messages.last().map(|(_, t)| *t).unwrap_or(0),
        "customTitle": title,
        "initialLocation": "panel",
        "requests": requests
    });

    let file_path = sessions_dir.join(format!("{}.json", session_id));
    fs::write(file_path, serde_json::to_string_pretty(&session).unwrap()).unwrap();
}

// =============================================================================
// Model Tests - Data structure serialization/deserialization
// =============================================================================

mod model_tests {
    use chasm::models::{ChatMessage, ChatRequest, ChatSession};

    #[test]
    fn test_chat_session_full_deserialization() {
        let json = r#"{
            "version": 3,
            "sessionId": "abc-123-def",
            "requesterUsername": "test_user",
            "responderUsername": "GitHub Copilot",
            "initialLocation": "panel",
            "creationDate": 1699999990000,
            "lastMessageDate": 1699999999000,
            "isImported": false,
            "customTitle": "My Session",
            "requests": [
                {
                    "timestamp": 1699999990000,
                    "message": {"text": "Hello"},
                    "response": {"value": [{"value": "Hi there!"}]},
                    "requestId": "req-1",
                    "responseId": "resp-1"
                }
            ]
        }"#;

        let session: ChatSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.version, 3);
        assert_eq!(session.session_id, Some("abc-123-def".to_string()));
        assert_eq!(session.custom_title, Some("My Session".to_string()));
        assert_eq!(session.requests.len(), 1);
    }

    #[test]
    fn test_chat_session_minimal_deserialization() {
        let json = r#"{
            "version": 3,
            "creationDate": 1699999990000,
            "lastMessageDate": 1699999999000,
            "requests": []
        }"#;

        let session: ChatSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.version, 3);
        assert!(session.session_id.is_none());
        assert!(session.requests.is_empty());
    }

    #[test]
    fn test_chat_session_serialization_roundtrip() {
        let session = ChatSession {
            version: 3,
            session_id: Some("test-session-123".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000001000,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: Some("Test Conversation".to_string()),
            requester_username: Some("user".to_string()),
            requester_avatar_icon_uri: None,
            responder_username: Some("assistant".to_string()),
            responder_avatar_icon_uri: None,
            requests: vec![ChatRequest {
                timestamp: Some(1700000000000),
                message: Some(ChatMessage {
                    text: Some("Hello".to_string()),
                    parts: None,
                }),
                response: Some(serde_json::json!({
                    "value": [{"value": "Hi there!"}]
                })),
                variable_data: None,
                request_id: Some("req-1".to_string()),
                response_id: Some("resp-1".to_string()),
                model_id: Some("gpt-4".to_string()),
                agent: None,
                result: None,
                followups: None,
                is_canceled: Some(false),
                content_references: None,
                code_citations: None,
                response_markdown_info: None,
                source_session: None,
            }],
        };

        // Serialize
        let json = serde_json::to_string(&session).unwrap();

        // Deserialize
        let restored: ChatSession = serde_json::from_str(&json).unwrap();

        assert_eq!(session.session_id, restored.session_id);
        assert_eq!(session.custom_title, restored.custom_title);
        assert_eq!(session.requests.len(), restored.requests.len());
    }

    #[test]
    fn test_chat_message_with_parts() {
        let json = r#"{
            "text": "Hello",
            "parts": [
                {"text": "Hello"},
                {"code": "println!(\"world\");"}
            ]
        }"#;

        let msg: ChatMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.text, Some("Hello".to_string()));
        assert!(msg.parts.is_some());
    }

    #[test]
    fn test_chat_session_title_method() {
        // Session with custom title
        let with_title = ChatSession {
            version: 3,
            session_id: Some("test-1".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000001000,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: Some("My Custom Title".to_string()),
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: None,
            responder_avatar_icon_uri: None,
            requests: vec![],
        };
        assert_eq!(with_title.title(), "My Custom Title");

        // Session without custom title (should fall back to first message or default)
        let without_title = ChatSession {
            version: 3,
            session_id: Some("test-2".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000001000,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: None,
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: None,
            responder_avatar_icon_uri: None,
            requests: vec![ChatRequest {
                timestamp: Some(1700000000000),
                message: Some(ChatMessage {
                    text: Some("What is Rust programming?".to_string()),
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
            }],
        };
        // Title should be first message text, truncated
        let title = without_title.title();
        assert!(title.starts_with("What is Rust"));
    }
}

// =============================================================================
// CLI Tests - Command-line interface parsing
// =============================================================================

mod cli_tests {
    use chasm::cli::{Cli, Commands};
    use clap::Parser;

    #[test]
    fn test_cli_run_tui_command() {
        let cli = Cli::try_parse_from(["csm", "run", "tui"]).unwrap();
        assert!(matches!(cli.command, Commands::Run { .. }));
    }

    #[test]
    fn test_cli_list_workspaces_command() {
        let cli = Cli::try_parse_from(["csm", "list", "workspaces"]).unwrap();
        assert!(matches!(cli.command, Commands::List { .. }));
    }

    #[test]
    fn test_cli_list_sessions_command() {
        let cli = Cli::try_parse_from(["csm", "list", "sessions", "--project-path", "/test/path"])
            .unwrap();
        assert!(matches!(cli.command, Commands::List { .. }));
    }

    #[test]
    fn test_cli_find_workspace_command() {
        let cli = Cli::try_parse_from(["csm", "find", "workspace", "my_project"]).unwrap();
        assert!(matches!(cli.command, Commands::Find { .. }));
    }

    #[test]
    fn test_cli_find_session_command() {
        let cli = Cli::try_parse_from(["csm", "find", "session", "rust"]).unwrap();
        assert!(matches!(cli.command, Commands::Find { .. }));
    }

    #[test]
    fn test_cli_show_path_command() {
        let cli = Cli::try_parse_from(["csm", "show", "path", "/path/to/project"]).unwrap();
        assert!(matches!(cli.command, Commands::Show { .. }));
    }

    #[test]
    fn test_cli_fetch_path_command() {
        let cli = Cli::try_parse_from(["csm", "fetch", "path", "/path/to/project"]).unwrap();
        assert!(matches!(cli.command, Commands::Fetch { .. }));
    }

    #[test]
    fn test_cli_merge_path_command() {
        let cli = Cli::try_parse_from(["csm", "merge", "path", "/path/to/project"]).unwrap();
        assert!(matches!(cli.command, Commands::Merge { .. }));
    }

    #[test]
    fn test_cli_show_workspace_command() {
        let cli = Cli::try_parse_from(["csm", "show", "workspace", "test-project"]).unwrap();
        assert!(matches!(cli.command, Commands::Show { .. }));
    }

    #[test]
    fn test_cli_show_session_command() {
        let cli = Cli::try_parse_from(["csm", "show", "session", "session-id-123"]).unwrap();
        assert!(matches!(cli.command, Commands::Show { .. }));
    }

    #[test]
    fn test_cli_export_sessions_command() {
        let cli = Cli::try_parse_from([
            "csm",
            "export",
            "sessions",
            "/dest/path",
            "session1",
            "session2",
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::Export { .. }));
    }

    #[test]
    fn test_cli_import_sessions_command() {
        let cli =
            Cli::try_parse_from(["csm", "import", "sessions", "/src/path/session.json"]).unwrap();
        assert!(matches!(cli.command, Commands::Import { .. }));
    }

    #[test]
    fn test_cli_move_sessions_command() {
        let cli = Cli::try_parse_from(["csm", "move", "sessions", "abc123", "/dest/path"]).unwrap();
        assert!(matches!(cli.command, Commands::Move { .. }));
    }

    #[test]
    fn test_cli_git_init_command() {
        let cli = Cli::try_parse_from(["csm", "git", "init", "/path/to/project"]).unwrap();
        assert!(matches!(cli.command, Commands::Git { .. }));
    }

    #[test]
    fn test_cli_git_add_command() {
        let cli = Cli::try_parse_from(["csm", "git", "add", "/path/to/project"]).unwrap();
        assert!(matches!(cli.command, Commands::Git { .. }));
    }

    #[test]
    fn test_cli_git_status_command() {
        let cli = Cli::try_parse_from(["csm", "git", "status", "/path/to/project"]).unwrap();
        assert!(matches!(cli.command, Commands::Git { .. }));
    }

    #[test]
    fn test_cli_git_snapshot_command() {
        let cli = Cli::try_parse_from([
            "csm",
            "git",
            "snapshot",
            "/path/to/project",
            "--message",
            "Test snapshot",
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::Git { .. }));
    }

    #[test]
    fn test_cli_migration_create_command() {
        let cli =
            Cli::try_parse_from(["csm", "migration", "create", "/output/path", "--all"]).unwrap();
        assert!(matches!(cli.command, Commands::Migration { .. }));
    }

    #[test]
    fn test_cli_migration_restore_command() {
        let cli =
            Cli::try_parse_from(["csm", "migration", "restore", "/package/path", "--dry-run"])
                .unwrap();
        assert!(matches!(cli.command, Commands::Migration { .. }));
    }

    #[test]
    fn test_cli_provider_list_command() {
        let cli = Cli::try_parse_from(["csm", "provider", "list"]).unwrap();
        assert!(matches!(cli.command, Commands::Provider { .. }));
    }

    #[test]
    fn test_cli_provider_info_command() {
        let cli = Cli::try_parse_from(["csm", "provider", "info", "ollama"]).unwrap();
        assert!(matches!(cli.command, Commands::Provider { .. }));
    }

    #[test]
    fn test_cli_provider_config_command() {
        let cli = Cli::try_parse_from([
            "csm",
            "provider",
            "config",
            "ollama",
            "--endpoint",
            "http://localhost:11434",
        ])
        .unwrap();
        assert!(matches!(cli.command, Commands::Provider { .. }));
    }

    #[test]
    fn test_cli_provider_test_command() {
        let cli = Cli::try_parse_from(["csm", "provider", "test", "ollama"]).unwrap();
        assert!(matches!(cli.command, Commands::Provider { .. }));
    }

    #[test]
    fn test_cli_help_flag() {
        // --help should cause an error (early exit), but we can test it parses partially
        let result = Cli::try_parse_from(["csm", "--help"]);
        assert!(result.is_err()); // Help flag causes early exit
    }

    #[test]
    fn test_cli_version_flag() {
        let result = Cli::try_parse_from(["csm", "--version"]);
        assert!(result.is_err()); // Version flag causes early exit
    }
}

// =============================================================================
// Workspace Tests - Workspace discovery and path handling
// =============================================================================

mod workspace_tests {
    use super::*;
    use chasm::workspace::normalize_path;

    #[test]
    fn test_normalize_path_basic() {
        let path = normalize_path("/home/user/project");
        assert!(!path.is_empty());
    }

    #[test]
    fn test_normalize_path_with_trailing_slash() {
        // normalize_path preserves trailing slashes
        let with_slash = normalize_path("/home/user/project/");
        let without_slash = normalize_path("/home/user/project");

        // Both should be non-empty valid paths
        assert!(!with_slash.is_empty());
        assert!(!without_slash.is_empty());
    }

    #[test]
    fn test_create_workspace_structure() {
        let temp_dir = TempDir::new().unwrap();
        let ws_dir = create_test_workspace(&temp_dir, "abc123def", "/home/user/my_project");

        assert!(ws_dir.exists());
        assert!(ws_dir.join("workspace.json").exists());
        assert!(ws_dir.join("chatSessions").exists());
    }

    #[test]
    fn test_workspace_json_content() {
        let temp_dir = TempDir::new().unwrap();
        let ws_dir = create_test_workspace(&temp_dir, "test123", "/home/user/test_project");

        let json_content = fs::read_to_string(ws_dir.join("workspace.json")).unwrap();
        assert!(json_content.contains("folder"));
        assert!(json_content.contains("test_project"));
    }
}

// =============================================================================
// Session Tests - Session creation and manipulation
// =============================================================================

mod session_tests {
    use super::*;

    #[test]
    fn test_create_session_file() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("chatSessions");
        fs::create_dir_all(&sessions_dir).unwrap();

        create_test_session(
            &sessions_dir,
            "session-abc-123",
            "Test Session",
            vec![
                ("Hello, how are you?", 1700000000000),
                ("What is Rust?", 1700000001000),
            ],
        );

        let session_path = sessions_dir.join("session-abc-123.json");
        assert!(session_path.exists());

        let content = fs::read_to_string(&session_path).unwrap();
        let session: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(session["sessionId"], "session-abc-123");
        assert_eq!(session["customTitle"], "Test Session");
        assert_eq!(session["requests"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_session_timestamps_ordering() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("chatSessions");
        fs::create_dir_all(&sessions_dir).unwrap();

        let messages = vec![
            ("First message", 1700000001000),
            ("Second message", 1700000002000),
            ("Third message", 1700000003000),
        ];

        create_test_session(&sessions_dir, "ordered-session", "Ordered", messages);

        let content = fs::read_to_string(sessions_dir.join("ordered-session.json")).unwrap();
        let session: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(session["creationDate"], 1700000001000i64);
        assert_eq!(session["lastMessageDate"], 1700000003000i64);
    }
}

// =============================================================================
// Error Tests - Error type formatting and display
// =============================================================================

mod error_tests {
    use chasm::error::CsmError;

    #[test]
    fn test_error_display_workspace_not_found() {
        let error = CsmError::WorkspaceNotFound("/test/path".to_string());
        let msg = format!("{}", error);
        assert!(msg.contains("Workspace not found"));
        assert!(msg.contains("/test/path"));
    }

    #[test]
    fn test_error_display_session_not_found() {
        let error = CsmError::SessionNotFound("session-123".to_string());
        let msg = format!("{}", error);
        assert!(msg.contains("Session not found"));
        assert!(msg.contains("session-123"));
    }

    #[test]
    fn test_error_display_vscode_running() {
        let error = CsmError::VSCodeRunning;
        let msg = format!("{}", error);
        assert!(msg.to_lowercase().contains("vs code") || msg.to_lowercase().contains("running"));
    }
}
