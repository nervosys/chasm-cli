//! Extensive tests for data models
//!
//! This file contains comprehensive unit tests for:
//! - ChatSession struct and methods
//! - ChatRequest struct
//! - ChatMessage struct and methods
//! - Workspace struct
//! - ChatSessionIndex and ChatSessionIndexEntry
//! - SessionWithPath
//! - WorkspaceJson

use chasm::models::{
    ChatMessage, ChatRequest, ChatSession, ChatSessionIndex, ChatSessionIndexEntry,
    SessionWithPath, Workspace, WorkspaceJson,
};
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// ChatMessage Tests
// ============================================================================

mod chat_message_tests {
    use super::*;

    #[test]
    fn test_chat_message_get_text_with_text() {
        let msg = ChatMessage {
            text: Some("Hello, world!".to_string()),
            parts: None,
        };
        assert_eq!(msg.get_text(), "Hello, world!");
    }

    #[test]
    fn test_chat_message_get_text_empty() {
        let msg = ChatMessage {
            text: None,
            parts: None,
        };
        assert_eq!(msg.get_text(), "");
    }

    #[test]
    fn test_chat_message_get_text_empty_string() {
        let msg = ChatMessage {
            text: Some("".to_string()),
            parts: None,
        };
        assert_eq!(msg.get_text(), "");
    }

    #[test]
    fn test_chat_message_with_parts() {
        let msg = ChatMessage {
            text: Some("Main text".to_string()),
            parts: Some(vec![
                serde_json::json!({"type": "text", "content": "part 1"}),
            ]),
        };
        assert!(msg.parts.is_some());
        assert_eq!(msg.get_text(), "Main text");
    }

    #[test]
    fn test_chat_message_serialization() {
        let msg = ChatMessage {
            text: Some("Test message".to_string()),
            parts: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"text\":\"Test message\""));
    }

    #[test]
    fn test_chat_message_deserialization() {
        let json = r#"{"text": "Deserialized message", "parts": null}"#;
        let msg: ChatMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.text, Some("Deserialized message".to_string()));
    }

    #[test]
    fn test_chat_message_deserialization_with_content_alias() {
        // ChatMessage has alias for "content" field
        let json = r#"{"content": "Content alias test"}"#;
        let msg: ChatMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.text, Some("Content alias test".to_string()));
    }

    #[test]
    fn test_chat_message_clone() {
        let msg = ChatMessage {
            text: Some("Clone test".to_string()),
            parts: Some(vec![serde_json::json!({"part": 1})]),
        };
        let cloned = msg.clone();
        assert_eq!(cloned.text, msg.text);
        assert_eq!(
            cloned.parts.as_ref().unwrap().len(),
            msg.parts.as_ref().unwrap().len()
        );
    }

    #[test]
    fn test_chat_message_with_unicode() {
        let msg = ChatMessage {
            text: Some("Hello World".to_string()),
            parts: None,
        };
        assert_eq!(msg.get_text(), "Hello World");
    }

    #[test]
    fn test_chat_message_with_special_characters() {
        let msg = ChatMessage {
            text: Some("Line1\nLine2\tTab\"Quote".to_string()),
            parts: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.text, msg.text);
    }
}

// ============================================================================
// ChatRequest Tests
// ============================================================================

mod chat_request_tests {
    use super::*;

    fn create_test_request() -> ChatRequest {
        ChatRequest {
            timestamp: Some(1700000000000),
            message: Some(ChatMessage {
                text: Some("Test question".to_string()),
                parts: None,
            }),
            response: Some(serde_json::json!({
                "value": [{"value": "Test response"}]
            })),
            variable_data: None,
            request_id: Some("req-123".to_string()),
            response_id: Some("resp-123".to_string()),
            model_id: Some("gpt-4".to_string()),
            agent: None,
            result: None,
            followups: None,
            is_canceled: Some(false),
            content_references: None,
            code_citations: None,
            response_markdown_info: None,
            source_session: None,
        }
    }

    #[test]
    fn test_chat_request_creation() {
        let req = create_test_request();
        assert_eq!(req.timestamp, Some(1700000000000));
        assert!(req.message.is_some());
    }

    #[test]
    fn test_chat_request_serialization() {
        let req = create_test_request();
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("timestamp"));
        assert!(json.contains("message"));
        assert!(json.contains("modelId")); // camelCase
    }

    #[test]
    fn test_chat_request_deserialization() {
        let json = r#"{
            "timestamp": 1700000000000,
            "message": {"text": "Hello"},
            "requestId": "req-1",
            "responseId": "resp-1",
            "modelId": "gpt-4",
            "isCanceled": false
        }"#;
        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.timestamp, Some(1700000000000));
        assert_eq!(req.request_id, Some("req-1".to_string()));
        assert_eq!(req.model_id, Some("gpt-4".to_string()));
        assert_eq!(req.is_canceled, Some(false));
    }

    #[test]
    fn test_chat_request_with_all_fields() {
        let req = ChatRequest {
            timestamp: Some(1700000000000),
            message: Some(ChatMessage {
                text: Some("Test".to_string()),
                parts: None,
            }),
            response: Some(serde_json::json!({"value": [{"value": "Response"}]})),
            variable_data: Some(serde_json::json!({"files": ["test.rs"]})),
            request_id: Some("req-full".to_string()),
            response_id: Some("resp-full".to_string()),
            model_id: Some("claude-3".to_string()),
            agent: Some(serde_json::json!({"name": "workspace"})),
            result: Some(serde_json::json!({"status": "success"})),
            followups: Some(vec![serde_json::json!({"text": "Follow up?"})]),
            is_canceled: Some(false),
            content_references: Some(vec![serde_json::json!({"uri": "file://test.rs"})]),
            code_citations: Some(vec![serde_json::json!({"license": "MIT"})]),
            response_markdown_info: Some(vec![serde_json::json!({"rendered": true})]),
            source_session: Some("source-session-123".to_string()),
        };

        let json = serde_json::to_string(&req).unwrap();
        let deserialized: ChatRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(
            deserialized.source_session,
            Some("source-session-123".to_string())
        );
        assert!(deserialized.variable_data.is_some());
        assert!(deserialized.agent.is_some());
    }

    #[test]
    fn test_chat_request_minimal() {
        let json = r#"{}"#;
        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert!(req.timestamp.is_none());
        assert!(req.message.is_none());
        assert!(req.response.is_none());
    }

    #[test]
    fn test_chat_request_clone() {
        let req = create_test_request();
        let cloned = req.clone();
        assert_eq!(cloned.timestamp, req.timestamp);
        assert_eq!(cloned.request_id, req.request_id);
    }

    #[test]
    fn test_chat_request_source_session_skip_serialize() {
        // source_session should not serialize when None
        let req = ChatRequest {
            timestamp: None,
            message: None,
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
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("_sourceSession"));
    }
}

// ============================================================================
// ChatSession Tests
// ============================================================================

mod chat_session_tests {
    use super::*;

    fn create_test_session() -> ChatSession {
        ChatSession {
            version: 3,
            session_id: Some("test-session-123".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000010000,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: Some("Test Conversation".to_string()),
            requester_username: Some("user".to_string()),
            requester_avatar_icon_uri: None,
            responder_username: Some("assistant".to_string()),
            responder_avatar_icon_uri: None,
            requests: vec![
                ChatRequest {
                    timestamp: Some(1700000000000),
                    message: Some(ChatMessage {
                        text: Some("First question".to_string()),
                        parts: None,
                    }),
                    response: Some(serde_json::json!({"value": [{"value": "First answer"}]})),
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
                },
                ChatRequest {
                    timestamp: Some(1700000010000),
                    message: Some(ChatMessage {
                        text: Some("Second question".to_string()),
                        parts: None,
                    }),
                    response: Some(serde_json::json!({"value": [{"value": "Second answer"}]})),
                    variable_data: None,
                    request_id: Some("req-2".to_string()),
                    response_id: Some("resp-2".to_string()),
                    model_id: Some("gpt-4".to_string()),
                    agent: None,
                    result: None,
                    followups: None,
                    is_canceled: Some(false),
                    content_references: None,
                    code_citations: None,
                    response_markdown_info: None,
                    source_session: None,
                },
            ],
        }
    }

    #[test]
    fn test_chat_session_title_with_custom_title() {
        let session = create_test_session();
        assert_eq!(session.title(), "Test Conversation");
    }

    #[test]
    fn test_chat_session_title_from_first_message() {
        let mut session = create_test_session();
        session.custom_title = None;
        let title = session.title();
        assert!(title.starts_with("First question"));
    }

    #[test]
    fn test_chat_session_title_truncation() {
        let mut session = create_test_session();
        session.custom_title = None;
        session.requests[0].message = Some(ChatMessage {
            text: Some("A".repeat(100)),
            parts: None,
        });
        let title = session.title();
        assert!(title.len() <= 53); // 50 chars + "..."
        assert!(title.ends_with("..."));
    }

    #[test]
    fn test_chat_session_title_untitled() {
        let session = ChatSession {
            version: 3,
            session_id: None,
            creation_date: 0,
            last_message_date: 0,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: None,
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: None,
            responder_avatar_icon_uri: None,
            requests: vec![],
        };
        assert_eq!(session.title(), "Untitled");
    }

    #[test]
    fn test_chat_session_title_empty_custom_title() {
        let mut session = create_test_session();
        session.custom_title = Some("".to_string());
        // Empty custom_title should fall back to first message
        assert!(session.title().starts_with("First"));
    }

    #[test]
    fn test_chat_session_is_empty() {
        let mut session = create_test_session();
        assert!(!session.is_empty());

        session.requests.clear();
        assert!(session.is_empty());
    }

    #[test]
    fn test_chat_session_request_count() {
        let session = create_test_session();
        assert_eq!(session.request_count(), 2);
    }

    #[test]
    fn test_chat_session_request_count_empty() {
        let mut session = create_test_session();
        session.requests.clear();
        assert_eq!(session.request_count(), 0);
    }

    #[test]
    fn test_chat_session_timestamp_range() {
        let session = create_test_session();
        let range = session.timestamp_range();
        assert!(range.is_some());
        let (min, max) = range.unwrap();
        assert_eq!(min, 1700000000000);
        assert_eq!(max, 1700000010000);
    }

    #[test]
    fn test_chat_session_timestamp_range_empty() {
        let mut session = create_test_session();
        session.requests.clear();
        assert!(session.timestamp_range().is_none());
    }

    #[test]
    fn test_chat_session_timestamp_range_no_timestamps() {
        let mut session = create_test_session();
        for req in &mut session.requests {
            req.timestamp = None;
        }
        assert!(session.timestamp_range().is_none());
    }

    #[test]
    fn test_chat_session_get_session_id() {
        let session = create_test_session();
        assert_eq!(session.get_session_id(), "test-session-123");
    }

    #[test]
    fn test_chat_session_get_session_id_none() {
        let mut session = create_test_session();
        session.session_id = None;
        assert_eq!(session.get_session_id(), "unknown");
    }

    #[test]
    fn test_chat_session_serialization() {
        let session = create_test_session();
        let json = serde_json::to_string(&session).unwrap();

        // Check camelCase serialization
        assert!(json.contains("\"sessionId\""));
        assert!(json.contains("\"creationDate\""));
        assert!(json.contains("\"lastMessageDate\""));
        assert!(json.contains("\"customTitle\""));
        assert!(json.contains("\"initialLocation\""));
    }

    #[test]
    fn test_chat_session_deserialization() {
        let json = r#"{
            "version": 3,
            "sessionId": "session-xyz",
            "creationDate": 1700000000000,
            "lastMessageDate": 1700000001000,
            "isImported": true,
            "initialLocation": "editor",
            "customTitle": "My Session",
            "requesterUsername": "user1",
            "responderUsername": "copilot",
            "requests": []
        }"#;

        let session: ChatSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.version, 3);
        assert_eq!(session.session_id, Some("session-xyz".to_string()));
        assert!(session.is_imported);
        assert_eq!(session.initial_location, "editor");
    }

    #[test]
    fn test_chat_session_deserialization_minimal() {
        let json = r#"{"version": 3, "creationDate": 0, "lastMessageDate": 0, "requests": []}"#;
        let session: ChatSession = serde_json::from_str(json).unwrap();
        assert!(session.session_id.is_none());
        assert!(session.custom_title.is_none());
    }

    #[test]
    fn test_chat_session_default_version() {
        let json = r#"{"creationDate": 0, "lastMessageDate": 0, "requests": []}"#;
        let session: ChatSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.version, 3); // default_version()
    }

    #[test]
    fn test_chat_session_default_location() {
        let json = r#"{"version": 3, "creationDate": 0, "lastMessageDate": 0, "requests": []}"#;
        let session: ChatSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.initial_location, "panel"); // default_location()
    }

    #[test]
    fn test_chat_session_clone() {
        let session = create_test_session();
        let cloned = session.clone();
        assert_eq!(cloned.session_id, session.session_id);
        assert_eq!(cloned.requests.len(), session.requests.len());
    }

    #[test]
    fn test_chat_session_roundtrip() {
        let session = create_test_session();
        let json = serde_json::to_string_pretty(&session).unwrap();
        let restored: ChatSession = serde_json::from_str(&json).unwrap();

        assert_eq!(session.version, restored.version);
        assert_eq!(session.session_id, restored.session_id);
        assert_eq!(session.custom_title, restored.custom_title);
        assert_eq!(session.requests.len(), restored.requests.len());
    }

    #[test]
    fn test_chat_session_locations() {
        for location in ["panel", "terminal", "notebook", "editor", "inline"] {
            let session = ChatSession {
                version: 3,
                session_id: None,
                creation_date: 0,
                last_message_date: 0,
                is_imported: false,
                initial_location: location.to_string(),
                custom_title: None,
                requester_username: None,
                requester_avatar_icon_uri: None,
                responder_username: None,
                responder_avatar_icon_uri: None,
                requests: vec![],
            };
            assert_eq!(session.initial_location, location);
        }
    }
}

// ============================================================================
// ChatSessionIndex Tests
// ============================================================================

mod chat_session_index_tests {
    use super::*;

    #[test]
    fn test_chat_session_index_default() {
        let index = ChatSessionIndex::default();
        assert_eq!(index.version, 1);
        assert!(index.entries.is_empty());
    }

    #[test]
    fn test_chat_session_index_with_entries() {
        let mut entries = HashMap::new();
        entries.insert(
            "session-1".to_string(),
            ChatSessionIndexEntry {
                session_id: "session-1".to_string(),
                title: "First Session".to_string(),
                last_message_date: 1700000000000,
                is_imported: false,
                initial_location: "panel".to_string(),
                is_empty: false,
            },
        );

        let index = ChatSessionIndex {
            version: 1,
            entries,
        };

        assert_eq!(index.entries.len(), 1);
        assert!(index.entries.contains_key("session-1"));
    }

    #[test]
    fn test_chat_session_index_serialization() {
        let mut entries = HashMap::new();
        entries.insert(
            "sess-abc".to_string(),
            ChatSessionIndexEntry {
                session_id: "sess-abc".to_string(),
                title: "Test".to_string(),
                last_message_date: 1700000000000,
                is_imported: false,
                initial_location: "panel".to_string(),
                is_empty: false,
            },
        );

        let index = ChatSessionIndex {
            version: 1,
            entries,
        };
        let json = serde_json::to_string(&index).unwrap();

        assert!(json.contains("\"version\":1"));
        assert!(json.contains("\"entries\""));
        assert!(json.contains("sess-abc"));
    }

    #[test]
    fn test_chat_session_index_deserialization() {
        let json = r#"{
            "version": 1,
            "entries": {
                "session-xyz": {
                    "sessionId": "session-xyz",
                    "title": "Test Session",
                    "lastMessageDate": 1700000000000,
                    "isImported": true,
                    "initialLocation": "terminal",
                    "isEmpty": false
                }
            }
        }"#;

        let index: ChatSessionIndex = serde_json::from_str(json).unwrap();
        assert_eq!(index.version, 1);
        assert_eq!(index.entries.len(), 1);

        let entry = index.entries.get("session-xyz").unwrap();
        assert_eq!(entry.title, "Test Session");
        assert!(entry.is_imported);
    }
}

// ============================================================================
// ChatSessionIndexEntry Tests
// ============================================================================

mod chat_session_index_entry_tests {
    use super::*;

    #[test]
    fn test_entry_creation() {
        let entry = ChatSessionIndexEntry {
            session_id: "test-id".to_string(),
            title: "Test Title".to_string(),
            last_message_date: 1700000000000,
            is_imported: false,
            initial_location: "panel".to_string(),
            is_empty: false,
        };

        assert_eq!(entry.session_id, "test-id");
        assert_eq!(entry.title, "Test Title");
    }

    #[test]
    fn test_entry_serialization() {
        let entry = ChatSessionIndexEntry {
            session_id: "entry-1".to_string(),
            title: "Entry Title".to_string(),
            last_message_date: 1700000000000,
            is_imported: true,
            initial_location: "editor".to_string(),
            is_empty: true,
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"sessionId\":\"entry-1\""));
        assert!(json.contains("\"lastMessageDate\""));
        assert!(json.contains("\"isImported\":true"));
        assert!(json.contains("\"isEmpty\":true"));
    }

    #[test]
    fn test_entry_deserialization_with_defaults() {
        // Test that defaults are applied correctly
        let json = r#"{
            "sessionId": "sess-1",
            "title": "Title",
            "lastMessageDate": 1000
        }"#;

        let entry: ChatSessionIndexEntry = serde_json::from_str(json).unwrap();
        assert!(!entry.is_imported); // default
        assert_eq!(entry.initial_location, "panel"); // default
        assert!(!entry.is_empty); // default
    }

    #[test]
    fn test_entry_clone() {
        let entry = ChatSessionIndexEntry {
            session_id: "clone-test".to_string(),
            title: "Clone".to_string(),
            last_message_date: 1700000000000,
            is_imported: true,
            initial_location: "terminal".to_string(),
            is_empty: false,
        };

        let cloned = entry.clone();
        assert_eq!(cloned.session_id, entry.session_id);
        assert_eq!(cloned.is_imported, entry.is_imported);
    }
}

// ============================================================================
// SessionWithPath Tests
// ============================================================================

mod session_with_path_tests {
    use super::*;

    fn create_session_with_path(filename: &str) -> SessionWithPath {
        SessionWithPath {
            path: PathBuf::from(format!("/test/sessions/{}.json", filename)),
            session: ChatSession {
                version: 3,
                session_id: None,
                creation_date: 1700000000000,
                last_message_date: 1700000000000,
                is_imported: false,
                initial_location: "panel".to_string(),
                custom_title: Some("Test".to_string()),
                requester_username: None,
                requester_avatar_icon_uri: None,
                responder_username: None,
                responder_avatar_icon_uri: None,
                requests: vec![],
            },
        }
    }

    #[test]
    fn test_session_with_path_get_session_id_from_session() {
        let mut swp = create_session_with_path("my-session");
        swp.session.session_id = Some("explicit-id".to_string());
        assert_eq!(swp.get_session_id(), "explicit-id");
    }

    #[test]
    fn test_session_with_path_get_session_id_from_filename() {
        let swp = create_session_with_path("filename-based-id");
        assert_eq!(swp.get_session_id(), "filename-based-id");
    }

    #[test]
    fn test_session_with_path_clone() {
        let swp = create_session_with_path("clone-test");
        let cloned = swp.clone();
        assert_eq!(cloned.path, swp.path);
        assert_eq!(cloned.session.custom_title, swp.session.custom_title);
    }
}

// ============================================================================
// WorkspaceJson Tests
// ============================================================================

mod workspace_json_tests {
    use super::*;

    #[test]
    fn test_workspace_json_with_folder() {
        let ws = WorkspaceJson {
            folder: Some("file:///home/user/project".to_string()),
        };
        assert_eq!(ws.folder, Some("file:///home/user/project".to_string()));
    }

    #[test]
    fn test_workspace_json_no_folder() {
        let ws = WorkspaceJson { folder: None };
        assert!(ws.folder.is_none());
    }

    #[test]
    fn test_workspace_json_serialization() {
        let ws = WorkspaceJson {
            folder: Some("file:///test".to_string()),
        };
        let json = serde_json::to_string(&ws).unwrap();
        assert!(json.contains("\"folder\":\"file:///test\""));
    }

    #[test]
    fn test_workspace_json_deserialization() {
        let json = r#"{"folder": "file:///C:/Users/test/project"}"#;
        let ws: WorkspaceJson = serde_json::from_str(json).unwrap();
        assert_eq!(ws.folder, Some("file:///C:/Users/test/project".to_string()));
    }

    #[test]
    fn test_workspace_json_deserialization_no_folder() {
        let json = r#"{}"#;
        let ws: WorkspaceJson = serde_json::from_str(json).unwrap();
        assert!(ws.folder.is_none());
    }

    #[test]
    fn test_workspace_json_clone() {
        let ws = WorkspaceJson {
            folder: Some("test".to_string()),
        };
        let cloned = ws.clone();
        assert_eq!(cloned.folder, ws.folder);
    }
}

// ============================================================================
// Workspace Struct Tests
// ============================================================================

mod workspace_struct_tests {
    use super::*;

    fn create_test_workspace() -> Workspace {
        Workspace {
            hash: "abc123def456".to_string(),
            project_path: Some("/home/user/myproject".to_string()),
            workspace_path: PathBuf::from("/vscode/workspaceStorage/abc123def456"),
            chat_sessions_path: PathBuf::from("/vscode/workspaceStorage/abc123def456/chatSessions"),
            chat_session_count: 5,
            has_chat_sessions: true,
            last_modified: None,
        }
    }

    #[test]
    fn test_workspace_fields() {
        let ws = create_test_workspace();
        assert_eq!(ws.hash, "abc123def456");
        assert_eq!(ws.project_path, Some("/home/user/myproject".to_string()));
        assert_eq!(ws.chat_session_count, 5);
        assert!(ws.has_chat_sessions);
    }

    #[test]
    fn test_workspace_no_project_path() {
        let mut ws = create_test_workspace();
        ws.project_path = None;
        assert!(ws.project_path.is_none());
    }

    #[test]
    fn test_workspace_no_chat_sessions() {
        let mut ws = create_test_workspace();
        ws.has_chat_sessions = false;
        ws.chat_session_count = 0;
        assert!(!ws.has_chat_sessions);
        assert_eq!(ws.chat_session_count, 0);
    }

    #[test]
    fn test_workspace_with_last_modified() {
        let mut ws = create_test_workspace();
        ws.last_modified = Some(chrono::Utc::now());
        assert!(ws.last_modified.is_some());
    }

    #[test]
    fn test_workspace_clone() {
        let ws = create_test_workspace();
        let cloned = ws.clone();
        assert_eq!(cloned.hash, ws.hash);
        assert_eq!(cloned.project_path, ws.project_path);
        assert_eq!(cloned.chat_session_count, ws.chat_session_count);
    }

    #[test]
    fn test_workspace_debug() {
        let ws = create_test_workspace();
        let debug_str = format!("{:?}", ws);
        assert!(debug_str.contains("abc123def456"));
        assert!(debug_str.contains("myproject"));
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_extremely_long_message() {
        let long_text = "x".repeat(1_000_000);
        let msg = ChatMessage {
            text: Some(long_text.clone()),
            parts: None,
        };
        assert_eq!(msg.get_text().len(), 1_000_000);
    }

    #[test]
    fn test_extremely_long_title() {
        let mut session = ChatSession {
            version: 3,
            session_id: None,
            creation_date: 0,
            last_message_date: 0,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: Some("x".repeat(10000)),
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: None,
            responder_avatar_icon_uri: None,
            requests: vec![],
        };

        // Custom title should be used as-is
        assert_eq!(session.title().len(), 10000);

        // But if from message, should be truncated
        session.custom_title = None;
        session.requests.push(ChatRequest {
            timestamp: None,
            message: Some(ChatMessage {
                text: Some("x".repeat(10000)),
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
        });

        assert!(session.title().len() <= 53);
    }

    #[test]
    fn test_many_requests() {
        let mut session = ChatSession {
            version: 3,
            session_id: None,
            creation_date: 0,
            last_message_date: 0,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: None,
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: None,
            responder_avatar_icon_uri: None,
            requests: vec![],
        };

        for i in 0..1000 {
            session.requests.push(ChatRequest {
                timestamp: Some(i),
                message: Some(ChatMessage {
                    text: Some(format!("Message {}", i)),
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
            });
        }

        assert_eq!(session.request_count(), 1000);
        let (min, max) = session.timestamp_range().unwrap();
        assert_eq!(min, 0);
        assert_eq!(max, 999);
    }

    #[test]
    fn test_zero_timestamp() {
        let session = ChatSession {
            version: 3,
            session_id: None,
            creation_date: 0,
            last_message_date: 0,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: None,
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: None,
            responder_avatar_icon_uri: None,
            requests: vec![ChatRequest {
                timestamp: Some(0),
                message: None,
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

        let range = session.timestamp_range();
        assert!(range.is_some());
        assert_eq!(range.unwrap(), (0, 0));
    }

    #[test]
    fn test_negative_timestamp() {
        // Timestamps before Unix epoch
        let session = ChatSession {
            version: 3,
            session_id: None,
            creation_date: -1000,
            last_message_date: -500,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: None,
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: None,
            responder_avatar_icon_uri: None,
            requests: vec![ChatRequest {
                timestamp: Some(-1000),
                message: None,
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

        let range = session.timestamp_range();
        assert!(range.is_some());
    }

    #[test]
    fn test_mixed_timestamp_presence() {
        let session = ChatSession {
            version: 3,
            session_id: None,
            creation_date: 0,
            last_message_date: 0,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: None,
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: None,
            responder_avatar_icon_uri: None,
            requests: vec![
                ChatRequest {
                    timestamp: Some(100),
                    message: None,
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
                },
                ChatRequest {
                    timestamp: None, // No timestamp
                    message: None,
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
                },
                ChatRequest {
                    timestamp: Some(300),
                    message: None,
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
                },
            ],
        };

        let range = session.timestamp_range();
        assert!(range.is_some());
        let (min, max) = range.unwrap();
        assert_eq!(min, 100);
        assert_eq!(max, 300);
    }
}
