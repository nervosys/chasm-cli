//! Extensive tests for session format conversions
//!
//! This file contains comprehensive unit tests for:
//! - GenericMessage struct
//! - GenericSession struct
//! - ChatSession to GenericSession conversion
//! - GenericSession to ChatSession conversion
//! - Markdown export/import
//! - Response text extraction

use chasm::models::{ChatMessage, ChatRequest, ChatSession};
use chasm::providers::session_format::{
    markdown_to_session, session_to_markdown, GenericMessage, GenericSession,
};

// ============================================================================
// GenericMessage Tests
// ============================================================================

mod generic_message_tests {
    use super::*;

    #[test]
    fn test_generic_message_user_role() {
        let msg = GenericMessage {
            role: "user".to_string(),
            content: "Hello, assistant!".to_string(),
            timestamp: Some(1700000000000),
            model: None,
        };
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello, assistant!");
    }

    #[test]
    fn test_generic_message_assistant_role() {
        let msg = GenericMessage {
            role: "assistant".to_string(),
            content: "Hello! How can I help?".to_string(),
            timestamp: Some(1700000000000),
            model: Some("gpt-4".to_string()),
        };
        assert_eq!(msg.role, "assistant");
        assert!(msg.model.is_some());
    }

    #[test]
    fn test_generic_message_system_role() {
        let msg = GenericMessage {
            role: "system".to_string(),
            content: "You are a helpful assistant.".to_string(),
            timestamp: None,
            model: None,
        };
        assert_eq!(msg.role, "system");
    }

    #[test]
    fn test_generic_message_serialization() {
        let msg = GenericMessage {
            role: "user".to_string(),
            content: "Test message".to_string(),
            timestamp: Some(1700000000000),
            model: Some("model-x".to_string()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Test message\""));
    }

    #[test]
    fn test_generic_message_deserialization() {
        let json = r#"{
            "role": "assistant",
            "content": "Response text",
            "timestamp": 1700000000000,
            "model": "claude-3"
        }"#;

        let msg: GenericMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.model, Some("claude-3".to_string()));
    }

    #[test]
    fn test_generic_message_optional_fields() {
        let json = r#"{"role": "user", "content": "Hello"}"#;
        let msg: GenericMessage = serde_json::from_str(json).unwrap();
        assert!(msg.timestamp.is_none());
        assert!(msg.model.is_none());
    }

    #[test]
    fn test_generic_message_clone() {
        let msg = GenericMessage {
            role: "user".to_string(),
            content: "Clone test".to_string(),
            timestamp: Some(1700000000000),
            model: Some("test-model".to_string()),
        };
        let cloned = msg.clone();
        assert_eq!(cloned.role, msg.role);
        assert_eq!(cloned.content, msg.content);
    }

    #[test]
    fn test_generic_message_empty_content() {
        let msg = GenericMessage {
            role: "user".to_string(),
            content: "".to_string(),
            timestamp: None,
            model: None,
        };
        assert!(msg.content.is_empty());
    }

    #[test]
    fn test_generic_message_long_content() {
        let long_content = "x".repeat(100000);
        let msg = GenericMessage {
            role: "user".to_string(),
            content: long_content.clone(),
            timestamp: None,
            model: None,
        };
        assert_eq!(msg.content.len(), 100000);
    }

    #[test]
    fn test_generic_message_unicode_content() {
        let msg = GenericMessage {
            role: "user".to_string(),
            content: "Hello World".to_string(),
            timestamp: None,
            model: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let restored: GenericMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.content, msg.content);
    }
}

// ============================================================================
// GenericSession Tests
// ============================================================================

mod generic_session_tests {
    use super::*;

    fn create_test_generic_session() -> GenericSession {
        GenericSession {
            id: "test-session-123".to_string(),
            title: Some("Test Session".to_string()),
            messages: vec![
                GenericMessage {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                    timestamp: Some(1700000000000),
                    model: None,
                },
                GenericMessage {
                    role: "assistant".to_string(),
                    content: "Hi there!".to_string(),
                    timestamp: Some(1700000001000),
                    model: Some("gpt-4".to_string()),
                },
            ],
            created_at: Some(1700000000000),
            updated_at: Some(1700000001000),
            provider: Some("OpenAI".to_string()),
            model: Some("gpt-4".to_string()),
        }
    }

    #[test]
    fn test_generic_session_creation() {
        let session = create_test_generic_session();
        assert_eq!(session.id, "test-session-123");
        assert_eq!(session.title, Some("Test Session".to_string()));
        assert_eq!(session.messages.len(), 2);
    }

    #[test]
    fn test_generic_session_serialization() {
        let session = create_test_generic_session();
        let json = serde_json::to_string(&session).unwrap();

        assert!(json.contains("\"id\":\"test-session-123\""));
        assert!(json.contains("\"title\":\"Test Session\""));
        assert!(json.contains("\"messages\""));
    }

    #[test]
    fn test_generic_session_deserialization() {
        let json = r#"{
            "id": "session-xyz",
            "title": "Deserialized Session",
            "messages": [
                {"role": "user", "content": "Question?"}
            ],
            "created_at": 1700000000000,
            "updated_at": 1700000001000,
            "provider": "TestProvider",
            "model": "test-model"
        }"#;

        let session: GenericSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.id, "session-xyz");
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.provider, Some("TestProvider".to_string()));
    }

    #[test]
    fn test_generic_session_minimal() {
        let json = r#"{"id": "min-session", "messages": []}"#;
        let session: GenericSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.id, "min-session");
        assert!(session.title.is_none());
        assert!(session.messages.is_empty());
    }

    #[test]
    fn test_generic_session_clone() {
        let session = create_test_generic_session();
        let cloned = session.clone();
        assert_eq!(cloned.id, session.id);
        assert_eq!(cloned.messages.len(), session.messages.len());
    }

    #[test]
    fn test_generic_session_many_messages() {
        let mut messages = Vec::new();
        for i in 0..100 {
            messages.push(GenericMessage {
                role: if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
                content: format!("Message {}", i),
                timestamp: Some(1700000000000 + i),
                model: None,
            });
        }

        let session = GenericSession {
            id: "many-messages".to_string(),
            title: None,
            messages,
            created_at: None,
            updated_at: None,
            provider: None,
            model: None,
        };

        assert_eq!(session.messages.len(), 100);
    }
}

// ============================================================================
// ChatSession to GenericSession Conversion Tests
// ============================================================================

mod chat_session_to_generic_tests {
    use super::*;

    fn create_chat_session_for_conversion() -> ChatSession {
        ChatSession {
            version: 3,
            session_id: Some("chat-123".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000010000,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: Some("Chat Session Title".to_string()),
            requester_username: Some("user".to_string()),
            requester_avatar_icon_uri: None,
            responder_username: Some("Copilot".to_string()),
            responder_avatar_icon_uri: None,
            requests: vec![ChatRequest {
                timestamp: Some(1700000000000),
                message: Some(ChatMessage {
                    text: Some("What is Rust?".to_string()),
                    parts: None,
                }),
                response: Some(serde_json::json!({
                    "value": [{"value": "Rust is a systems programming language."}]
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
        }
    }

    #[test]
    fn test_conversion_preserves_id() {
        let chat_session = create_chat_session_for_conversion();
        let generic: GenericSession = chat_session.into();
        assert_eq!(generic.id, "chat-123");
    }

    #[test]
    fn test_conversion_preserves_title() {
        let chat_session = create_chat_session_for_conversion();
        let generic: GenericSession = chat_session.into();
        assert_eq!(generic.title, Some("Chat Session Title".to_string()));
    }

    #[test]
    fn test_conversion_creates_message_pairs() {
        let chat_session = create_chat_session_for_conversion();
        let generic: GenericSession = chat_session.into();

        // Should have user message + assistant response
        assert_eq!(generic.messages.len(), 2);
        assert_eq!(generic.messages[0].role, "user");
        assert_eq!(generic.messages[1].role, "assistant");
    }

    #[test]
    fn test_conversion_preserves_timestamps() {
        let chat_session = create_chat_session_for_conversion();
        let generic: GenericSession = chat_session.into();

        assert_eq!(generic.created_at, Some(1700000000000));
        assert_eq!(generic.updated_at, Some(1700000010000));
    }

    #[test]
    fn test_conversion_preserves_provider() {
        let chat_session = create_chat_session_for_conversion();
        let generic: GenericSession = chat_session.into();
        assert_eq!(generic.provider, Some("Copilot".to_string()));
    }

    #[test]
    fn test_conversion_without_session_id() {
        let mut chat_session = create_chat_session_for_conversion();
        chat_session.session_id = None;

        let generic: GenericSession = chat_session.into();
        // Should generate a UUID
        assert!(!generic.id.is_empty());
        assert!(generic.id.len() >= 32); // UUID length
    }

    #[test]
    fn test_conversion_empty_requests() {
        let mut chat_session = create_chat_session_for_conversion();
        chat_session.requests.clear();

        let generic: GenericSession = chat_session.into();
        assert!(generic.messages.is_empty());
    }

    #[test]
    fn test_conversion_request_without_response() {
        let mut chat_session = create_chat_session_for_conversion();
        chat_session.requests[0].response = None;

        let generic: GenericSession = chat_session.into();
        // Should only have user message, no assistant response
        assert_eq!(generic.messages.len(), 1);
        assert_eq!(generic.messages[0].role, "user");
    }

    #[test]
    fn test_conversion_request_without_message() {
        let mut chat_session = create_chat_session_for_conversion();
        chat_session.requests[0].message = None;

        let generic: GenericSession = chat_session.into();
        // No user message, but assistant response should still be present
        assert_eq!(generic.messages.len(), 1);
        assert_eq!(generic.messages[0].role, "assistant");
    }
}

// ============================================================================
// GenericSession to ChatSession Conversion Tests
// ============================================================================

mod generic_to_chat_session_tests {
    use super::*;

    fn create_generic_for_conversion() -> GenericSession {
        GenericSession {
            id: "generic-123".to_string(),
            title: Some("Generic Title".to_string()),
            messages: vec![
                GenericMessage {
                    role: "user".to_string(),
                    content: "Hello!".to_string(),
                    timestamp: Some(1700000000000),
                    model: None,
                },
                GenericMessage {
                    role: "assistant".to_string(),
                    content: "Hi there!".to_string(),
                    timestamp: Some(1700000001000),
                    model: Some("gpt-4".to_string()),
                },
            ],
            created_at: Some(1700000000000),
            updated_at: Some(1700000001000),
            provider: Some("TestProvider".to_string()),
            model: Some("gpt-4".to_string()),
        }
    }

    #[test]
    fn test_conversion_to_chat_session_id() {
        let generic = create_generic_for_conversion();
        let chat_session: ChatSession = generic.into();
        assert_eq!(chat_session.session_id, Some("generic-123".to_string()));
    }

    #[test]
    fn test_conversion_to_chat_session_title() {
        let generic = create_generic_for_conversion();
        let chat_session: ChatSession = generic.into();
        assert_eq!(chat_session.custom_title, Some("Generic Title".to_string()));
    }

    #[test]
    fn test_conversion_to_chat_session_version() {
        let generic = create_generic_for_conversion();
        let chat_session: ChatSession = generic.into();
        assert_eq!(chat_session.version, 3);
    }

    #[test]
    fn test_conversion_to_chat_session_imported() {
        let generic = create_generic_for_conversion();
        let chat_session: ChatSession = generic.into();
        assert!(chat_session.is_imported);
    }

    #[test]
    fn test_conversion_to_chat_session_requests() {
        let generic = create_generic_for_conversion();
        let chat_session: ChatSession = generic.into();

        // User+Assistant pair should create one request
        assert_eq!(chat_session.requests.len(), 1);

        let request = &chat_session.requests[0];
        assert!(request.message.is_some());
        assert!(request.response.is_some());
    }

    #[test]
    fn test_conversion_preserves_responder_username() {
        let generic = create_generic_for_conversion();
        let chat_session: ChatSession = generic.into();
        assert_eq!(
            chat_session.responder_username,
            Some("TestProvider".to_string())
        );
    }

    #[test]
    fn test_conversion_creates_request_ids() {
        let generic = create_generic_for_conversion();
        let chat_session: ChatSession = generic.into();

        let request = &chat_session.requests[0];
        assert!(request.request_id.is_some());
        assert!(request.response_id.is_some());
    }

    #[test]
    fn test_conversion_unpaired_messages() {
        let generic = GenericSession {
            id: "unpaired".to_string(),
            title: None,
            messages: vec![
                GenericMessage {
                    role: "user".to_string(),
                    content: "First question".to_string(),
                    timestamp: None,
                    model: None,
                },
                GenericMessage {
                    role: "user".to_string(),
                    content: "Second question".to_string(),
                    timestamp: None,
                    model: None,
                },
                GenericMessage {
                    role: "assistant".to_string(),
                    content: "Response".to_string(),
                    timestamp: None,
                    model: None,
                },
            ],
            created_at: None,
            updated_at: None,
            provider: None,
            model: None,
        };

        let chat_session: ChatSession = generic.into();
        // First user message is orphaned, second user + assistant form a pair
        // This depends on implementation - the conversion pairs consecutive user/assistant
        assert!(!chat_session.requests.is_empty());
    }

    #[test]
    fn test_conversion_only_user_messages() {
        let generic = GenericSession {
            id: "users-only".to_string(),
            title: None,
            messages: vec![
                GenericMessage {
                    role: "user".to_string(),
                    content: "Question 1".to_string(),
                    timestamp: None,
                    model: None,
                },
                GenericMessage {
                    role: "user".to_string(),
                    content: "Question 2".to_string(),
                    timestamp: None,
                    model: None,
                },
            ],
            created_at: None,
            updated_at: None,
            provider: None,
            model: None,
        };

        let chat_session: ChatSession = generic.into();
        // No assistant responses means no complete pairs
        assert!(chat_session.requests.is_empty());
    }

    #[test]
    fn test_conversion_only_assistant_messages() {
        let generic = GenericSession {
            id: "assistants-only".to_string(),
            title: None,
            messages: vec![GenericMessage {
                role: "assistant".to_string(),
                content: "Response without question".to_string(),
                timestamp: None,
                model: None,
            }],
            created_at: None,
            updated_at: None,
            provider: None,
            model: None,
        };

        let chat_session: ChatSession = generic.into();
        // No user message to pair with
        assert!(chat_session.requests.is_empty());
    }
}

// ============================================================================
// Session to Markdown Tests
// ============================================================================

mod session_to_markdown_tests {
    use super::*;

    fn create_chat_session_for_markdown() -> ChatSession {
        ChatSession {
            version: 3,
            session_id: Some("md-session-123".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000001000,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: Some("Markdown Test Session".to_string()),
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: None,
            responder_avatar_icon_uri: None,
            requests: vec![ChatRequest {
                timestamp: Some(1700000000000),
                message: Some(ChatMessage {
                    text: Some("What is Rust?".to_string()),
                    parts: None,
                }),
                response: Some(serde_json::json!({
                    "value": [{"value": "Rust is a systems programming language focused on safety."}]
                })),
                variable_data: None,
                request_id: None,
                response_id: None,
                model_id: Some("gpt-4".to_string()),
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

    #[test]
    fn test_markdown_contains_title() {
        let session = create_chat_session_for_markdown();
        let md = session_to_markdown(&session);
        assert!(md.contains("# Markdown Test Session"));
    }

    #[test]
    fn test_markdown_contains_session_id() {
        let session = create_chat_session_for_markdown();
        let md = session_to_markdown(&session);
        assert!(md.contains("md-session-123"));
    }

    #[test]
    fn test_markdown_contains_user_message() {
        let session = create_chat_session_for_markdown();
        let md = session_to_markdown(&session);
        assert!(md.contains("What is Rust?"));
    }

    #[test]
    fn test_markdown_contains_assistant_response() {
        let session = create_chat_session_for_markdown();
        let md = session_to_markdown(&session);
        assert!(md.contains("systems programming language"));
    }

    #[test]
    fn test_markdown_section_headers() {
        let session = create_chat_session_for_markdown();
        let md = session_to_markdown(&session);
        assert!(md.contains("## User"));
        assert!(md.contains("## gpt-4") || md.contains("## Assistant"));
    }

    #[test]
    fn test_markdown_separators() {
        let session = create_chat_session_for_markdown();
        let md = session_to_markdown(&session);
        assert!(md.contains("---"));
    }

    #[test]
    fn test_markdown_empty_session() {
        let mut session = create_chat_session_for_markdown();
        session.requests.clear();

        let md = session_to_markdown(&session);
        assert!(md.contains("# Markdown Test Session"));
        // Should still have header but no message sections
    }

    #[test]
    fn test_markdown_multiple_messages() {
        let mut session = create_chat_session_for_markdown();
        session.requests.push(ChatRequest {
            timestamp: Some(1700000002000),
            message: Some(ChatMessage {
                text: Some("Can you give an example?".to_string()),
                parts: None,
            }),
            response: Some(serde_json::json!({
                "value": [{"value": "Here's a Hello World in Rust:\n```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```"}]
            })),
            variable_data: None,
            request_id: None,
            response_id: None,
            model_id: Some("gpt-4".to_string()),
            agent: None,
            result: None,
            followups: None,
            is_canceled: None,
            content_references: None,
            code_citations: None,
            response_markdown_info: None,
            source_session: None,
        });

        let md = session_to_markdown(&session);
        assert!(md.contains("What is Rust?"));
        assert!(md.contains("Can you give an example?"));
        assert!(md.contains("Hello World"));
    }

    #[test]
    fn test_markdown_preserves_code_blocks() {
        let mut session = create_chat_session_for_markdown();
        session.requests[0].response = Some(serde_json::json!({
            "value": [{"value": "```rust\nfn main() {}\n```"}]
        }));

        let md = session_to_markdown(&session);
        assert!(md.contains("```rust"));
        assert!(md.contains("fn main()"));
    }
}

// ============================================================================
// Markdown to Session Tests
// ============================================================================

mod markdown_to_session_tests {
    use super::*;

    #[test]
    fn test_parse_simple_markdown() {
        let md = r#"# Test Session

Session ID: `test-123`

Created: 2023-11-15 00:00:00
Last Updated: 2023-11-15 00:00:01

---

## User (1)

What is Rust?

## gpt-4 (1)

Rust is a programming language.

---
"#;

        let session = markdown_to_session(md, Some("Test Session".to_string()));
        assert!(session.session_id.is_some());
        assert!(session.is_imported);
    }

    #[test]
    fn test_parse_markdown_extracts_messages() {
        let md = r#"## User (1)

Hello, world!

## Assistant (1)

Hi there!

---
"#;

        let session = markdown_to_session(md, None);
        assert!(!session.requests.is_empty());
    }

    #[test]
    fn test_parse_markdown_multiple_exchanges() {
        let md = r#"## User (1)

First question

## Assistant (1)

First answer

---

## User (2)

Second question

## Assistant (2)

Second answer

---
"#;

        let session = markdown_to_session(md, None);
        assert_eq!(session.requests.len(), 2);
    }

    #[test]
    fn test_parse_markdown_preserves_title() {
        let md = "## User\n\nTest\n\n## Assistant\n\nResponse\n\n---";
        let session = markdown_to_session(md, Some("Custom Title".to_string()));
        assert_eq!(session.custom_title, Some("Custom Title".to_string()));
    }

    #[test]
    fn test_parse_markdown_sets_imported_flag() {
        let md = "## User\n\nTest\n\n## Assistant\n\nResponse\n\n---";
        let session = markdown_to_session(md, None);
        assert!(session.is_imported);
    }

    #[test]
    fn test_parse_empty_markdown() {
        let md = "";
        let session = markdown_to_session(md, None);
        assert!(session.requests.is_empty());
    }

    #[test]
    fn test_parse_markdown_only_user() {
        let md = "## User\n\nQuestion without answer\n\n---";
        let _session = markdown_to_session(md, None);
        // User without assistant response - depends on implementation
    }

    #[test]
    fn test_parse_markdown_with_code_blocks() {
        let md = r#"## User (1)

Show me Rust code

## Assistant (1)

Here's an example:

```rust
fn main() {
    println!("Hello, world!");
}
```

---
"#;

        let session = markdown_to_session(md, None);
        assert!(!session.requests.is_empty());

        if let Some(response) = &session.requests[0].response {
            let text = response.to_string();
            assert!(text.contains("Hello") || text.contains("rust"));
        }
    }
}

// ============================================================================
// Roundtrip Tests
// ============================================================================

mod roundtrip_tests {
    use super::*;

    fn create_complex_chat_session() -> ChatSession {
        ChatSession {
            version: 3,
            session_id: Some("roundtrip-test".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000010000,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: Some("Roundtrip Test".to_string()),
            requester_username: Some("user".to_string()),
            requester_avatar_icon_uri: None,
            responder_username: Some("assistant".to_string()),
            responder_avatar_icon_uri: None,
            requests: vec![
                ChatRequest {
                    timestamp: Some(1700000000000),
                    message: Some(ChatMessage {
                        text: Some("Question 1".to_string()),
                        parts: None,
                    }),
                    response: Some(serde_json::json!({
                        "value": [{"value": "Answer 1"}]
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
                },
                ChatRequest {
                    timestamp: Some(1700000010000),
                    message: Some(ChatMessage {
                        text: Some("Question 2".to_string()),
                        parts: None,
                    }),
                    response: Some(serde_json::json!({
                        "value": [{"value": "Answer 2"}]
                    })),
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
    fn test_chat_to_generic_to_chat() {
        let original = create_complex_chat_session();
        let generic: GenericSession = original.clone().into();
        let restored: ChatSession = generic.into();

        // Key properties should be preserved
        assert_eq!(restored.session_id, original.session_id);
        assert_eq!(restored.custom_title, original.custom_title);
    }

    #[test]
    fn test_generic_to_chat_to_generic() {
        let original = GenericSession {
            id: "generic-roundtrip".to_string(),
            title: Some("Generic Roundtrip".to_string()),
            messages: vec![
                GenericMessage {
                    role: "user".to_string(),
                    content: "User message".to_string(),
                    timestamp: Some(1700000000000),
                    model: None,
                },
                GenericMessage {
                    role: "assistant".to_string(),
                    content: "Assistant response".to_string(),
                    timestamp: Some(1700000001000),
                    model: Some("model-x".to_string()),
                },
            ],
            created_at: Some(1700000000000),
            updated_at: Some(1700000001000),
            provider: Some("TestProvider".to_string()),
            model: Some("model-x".to_string()),
        };

        let chat: ChatSession = original.clone().into();
        let restored: GenericSession = chat.into();

        assert_eq!(restored.id, original.id);
        assert_eq!(restored.title, original.title);
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_conversion_with_empty_message_text() {
        let session = ChatSession {
            version: 3,
            session_id: Some("empty-text".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000000000,
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
                    text: Some("".to_string()),
                    parts: None,
                }),
                response: Some(serde_json::json!({
                    "value": [{"value": "Response"}]
                })),
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

        let _generic: GenericSession = session.into();
        // Empty text should still be included or skipped gracefully
    }

    #[test]
    fn test_conversion_with_null_response_values() {
        let session = ChatSession {
            version: 3,
            session_id: Some("null-response".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000000000,
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
                    text: Some("Question".to_string()),
                    parts: None,
                }),
                response: Some(serde_json::json!(null)),
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

        let _generic: GenericSession = session.into();
        // Should handle null response gracefully
    }

    #[test]
    fn test_markdown_with_special_characters() {
        let session = ChatSession {
            version: 3,
            session_id: Some("special-chars".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000000000,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: Some("Session with *bold* and _italic_".to_string()),
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: None,
            responder_avatar_icon_uri: None,
            requests: vec![ChatRequest {
                timestamp: Some(1700000000000),
                message: Some(ChatMessage {
                    text: Some("What about `code` and **bold**?".to_string()),
                    parts: None,
                }),
                response: Some(serde_json::json!({
                    "value": [{"value": "Response with # header and - list"}]
                })),
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

        let md = session_to_markdown(&session);
        assert!(md.contains("code"));
        assert!(md.contains("bold"));
    }

    #[test]
    fn test_generic_session_with_system_messages() {
        let generic = GenericSession {
            id: "with-system".to_string(),
            title: None,
            messages: vec![
                GenericMessage {
                    role: "system".to_string(),
                    content: "You are a helpful assistant.".to_string(),
                    timestamp: None,
                    model: None,
                },
                GenericMessage {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                    timestamp: None,
                    model: None,
                },
                GenericMessage {
                    role: "assistant".to_string(),
                    content: "Hi!".to_string(),
                    timestamp: None,
                    model: None,
                },
            ],
            created_at: None,
            updated_at: None,
            provider: None,
            model: None,
        };

        let _chat: ChatSession = generic.into();
        // System message should be ignored in conversion
    }
}
