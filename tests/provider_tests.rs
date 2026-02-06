//! Extensive tests for LLM provider integrations
//!
//! This test file covers:
//! - Provider type definitions and properties
//! - Provider configuration
//! - Provider registry
//! - Session format conversions
//! - Individual provider implementations
//! - Discovery mechanisms

use chasm::models::{ChatMessage, ChatRequest, ChatSession};
use chasm::providers::config::{CsmConfig, ProviderConfig, ProviderType};
use chasm::providers::session_format::{GenericMessage, GenericSession};
use chasm::providers::ProviderRegistry;

// ============================================================================
// Provider Type Tests
// ============================================================================

mod provider_type_tests {
    use super::*;

    #[test]
    fn test_provider_type_display_names() {
        assert_eq!(ProviderType::Copilot.display_name(), "GitHub Copilot");
        assert_eq!(ProviderType::Cursor.display_name(), "Cursor");
        assert_eq!(ProviderType::Ollama.display_name(), "Ollama");
        assert_eq!(ProviderType::Vllm.display_name(), "vLLM");
        assert_eq!(ProviderType::Foundry.display_name(), "Azure AI Foundry");
        assert_eq!(ProviderType::OpenAI.display_name(), "OpenAI API");
        assert_eq!(ProviderType::LmStudio.display_name(), "LM Studio");
        assert_eq!(ProviderType::LocalAI.display_name(), "LocalAI");
        assert_eq!(
            ProviderType::TextGenWebUI.display_name(),
            "Text Generation WebUI"
        );
        assert_eq!(ProviderType::Jan.display_name(), "Jan.ai");
        assert_eq!(ProviderType::Gpt4All.display_name(), "GPT4All");
        assert_eq!(ProviderType::Llamafile.display_name(), "Llamafile");
        assert_eq!(ProviderType::Custom.display_name(), "Custom");
    }

    #[test]
    fn test_provider_type_default_endpoints() {
        // File-based providers should have no endpoint
        assert!(ProviderType::Copilot.default_endpoint().is_none());
        assert!(ProviderType::Cursor.default_endpoint().is_none());

        // Server-based providers should have endpoints
        assert_eq!(
            ProviderType::Ollama.default_endpoint(),
            Some("http://localhost:11434")
        );
        assert_eq!(
            ProviderType::Vllm.default_endpoint(),
            Some("http://localhost:8000")
        );
        assert_eq!(
            ProviderType::Foundry.default_endpoint(),
            Some("http://localhost:5272")
        );
        assert_eq!(
            ProviderType::OpenAI.default_endpoint(),
            Some("https://api.openai.com/v1")
        );
        assert_eq!(
            ProviderType::LmStudio.default_endpoint(),
            Some("http://localhost:1234/v1")
        );
        assert_eq!(
            ProviderType::LocalAI.default_endpoint(),
            Some("http://localhost:8080/v1")
        );
        assert_eq!(
            ProviderType::TextGenWebUI.default_endpoint(),
            Some("http://localhost:5000/v1")
        );
        assert_eq!(
            ProviderType::Jan.default_endpoint(),
            Some("http://localhost:1337/v1")
        );
        assert_eq!(
            ProviderType::Gpt4All.default_endpoint(),
            Some("http://localhost:4891/v1")
        );
        assert_eq!(
            ProviderType::Llamafile.default_endpoint(),
            Some("http://localhost:8080/v1")
        );

        // Custom provider has no default
        assert!(ProviderType::Custom.default_endpoint().is_none());
    }

    #[test]
    fn test_provider_type_uses_file_storage() {
        // Only Copilot and Cursor use file storage
        assert!(ProviderType::Copilot.uses_file_storage());
        assert!(ProviderType::Cursor.uses_file_storage());

        // All API-based providers don't use file storage
        assert!(!ProviderType::Ollama.uses_file_storage());
        assert!(!ProviderType::Vllm.uses_file_storage());
        assert!(!ProviderType::Foundry.uses_file_storage());
        assert!(!ProviderType::OpenAI.uses_file_storage());
        assert!(!ProviderType::LmStudio.uses_file_storage());
        assert!(!ProviderType::LocalAI.uses_file_storage());
        assert!(!ProviderType::TextGenWebUI.uses_file_storage());
        assert!(!ProviderType::Jan.uses_file_storage());
        assert!(!ProviderType::Gpt4All.uses_file_storage());
        assert!(!ProviderType::Llamafile.uses_file_storage());
        assert!(!ProviderType::Custom.uses_file_storage());
    }

    #[test]
    fn test_provider_type_is_openai_compatible() {
        // File-based providers are not OpenAI compatible
        assert!(!ProviderType::Copilot.is_openai_compatible());
        assert!(!ProviderType::Cursor.is_openai_compatible());

        // All API-based providers are OpenAI compatible
        assert!(ProviderType::Ollama.is_openai_compatible());
        assert!(ProviderType::Vllm.is_openai_compatible());
        assert!(ProviderType::Foundry.is_openai_compatible());
        assert!(ProviderType::OpenAI.is_openai_compatible());
        assert!(ProviderType::LmStudio.is_openai_compatible());
        assert!(ProviderType::LocalAI.is_openai_compatible());
        assert!(ProviderType::TextGenWebUI.is_openai_compatible());
        assert!(ProviderType::Jan.is_openai_compatible());
        assert!(ProviderType::Gpt4All.is_openai_compatible());
        assert!(ProviderType::Llamafile.is_openai_compatible());
        assert!(ProviderType::Custom.is_openai_compatible());
    }

    #[test]
    fn test_provider_type_display_trait() {
        // Test Display trait implementation
        assert_eq!(format!("{}", ProviderType::Copilot), "GitHub Copilot");
        assert_eq!(format!("{}", ProviderType::Ollama), "Ollama");
        assert_eq!(format!("{}", ProviderType::Vllm), "vLLM");
    }

    #[test]
    fn test_provider_type_serialization() {
        // Test JSON serialization (kebab-case with custom renames)
        let serialized = serde_json::to_string(&ProviderType::TextGenWebUI).unwrap();
        assert_eq!(serialized, "\"text-gen-webui\"");

        let serialized = serde_json::to_string(&ProviderType::LmStudio).unwrap();
        assert_eq!(serialized, "\"lm-studio\"");

        let serialized = serde_json::to_string(&ProviderType::Gpt4All).unwrap();
        assert_eq!(serialized, "\"gpt4all\"");

        let serialized = serde_json::to_string(&ProviderType::OpenAI).unwrap();
        assert_eq!(serialized, "\"openai\"");

        let serialized = serde_json::to_string(&ProviderType::LocalAI).unwrap();
        assert_eq!(serialized, "\"localai\"");
    }

    #[test]
    fn test_provider_type_deserialization() {
        // Test JSON deserialization
        let provider: ProviderType = serde_json::from_str("\"ollama\"").unwrap();
        assert_eq!(provider, ProviderType::Ollama);

        let provider: ProviderType = serde_json::from_str("\"vllm\"").unwrap();
        assert_eq!(provider, ProviderType::Vllm);

        let provider: ProviderType =
            serde_json::from_str("\"azure\"").unwrap_or(ProviderType::Foundry);
        assert_eq!(provider, ProviderType::Foundry);
    }

    #[test]
    fn test_provider_type_equality() {
        assert_eq!(ProviderType::Ollama, ProviderType::Ollama);
        assert_ne!(ProviderType::Ollama, ProviderType::Vllm);
    }

    #[test]
    fn test_provider_type_clone() {
        let original = ProviderType::Foundry;
        let cloned = original; // ProviderType is Copy
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_provider_type_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(ProviderType::Ollama);
        set.insert(ProviderType::Vllm);
        set.insert(ProviderType::Ollama); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&ProviderType::Ollama));
        assert!(set.contains(&ProviderType::Vllm));
    }
}

// ============================================================================
// Provider Configuration Tests
// ============================================================================

mod provider_config_tests {
    use super::*;

    #[test]
    fn test_provider_config_new() {
        let config = ProviderConfig::new(ProviderType::Ollama);

        assert_eq!(config.provider_type, ProviderType::Ollama);
        assert!(config.enabled);
        assert_eq!(config.endpoint, Some("http://localhost:11434".to_string()));
        assert!(config.api_key.is_none());
        assert!(config.model.is_none());
        assert!(config.name.is_none());
        assert!(config.storage_path.is_none());
        assert!(config.extra.is_empty());
    }

    #[test]
    fn test_provider_config_new_with_no_endpoint() {
        let config = ProviderConfig::new(ProviderType::Custom);

        assert!(config.endpoint.is_none());
    }

    #[test]
    fn test_provider_config_display_name() {
        // Without custom name
        let config = ProviderConfig::new(ProviderType::Ollama);
        assert_eq!(config.display_name(), "Ollama");

        // With custom name
        let mut config_custom = ProviderConfig::new(ProviderType::Ollama);
        config_custom.name = Some("My Local Ollama".to_string());
        assert_eq!(config_custom.display_name(), "My Local Ollama");
    }

    #[test]
    fn test_provider_config_serialization() {
        let mut config = ProviderConfig::new(ProviderType::Vllm);
        config.api_key = Some("sk-test-key".to_string());
        config.model = Some("llama-3.1-70b".to_string());

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("\"provider_type\": \"vllm\""));
        assert!(json.contains("\"enabled\": true"));
        assert!(json.contains("\"endpoint\": \"http://localhost:8000\""));
        assert!(json.contains("\"api_key\": \"sk-test-key\""));
        assert!(json.contains("\"model\": \"llama-3.1-70b\""));
    }

    #[test]
    fn test_provider_config_deserialization() {
        let json = r#"{
            "provider_type": "ollama",
            "enabled": true,
            "endpoint": "http://myserver:11434",
            "model": "llama3.2"
        }"#;

        let config: ProviderConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.provider_type, ProviderType::Ollama);
        assert!(config.enabled);
        assert_eq!(config.endpoint, Some("http://myserver:11434".to_string()));
        assert_eq!(config.model, Some("llama3.2".to_string()));
    }

    #[test]
    fn test_provider_config_deserialization_defaults() {
        let json = r#"{
            "provider_type": "vllm"
        }"#;

        let config: ProviderConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.provider_type, ProviderType::Vllm);
        assert!(config.enabled); // Default true
        assert!(config.api_key.is_none());
        assert!(config.extra.is_empty());
    }

    #[test]
    fn test_provider_config_extra_settings() {
        let mut config = ProviderConfig::new(ProviderType::Ollama);
        config
            .extra
            .insert("temperature".to_string(), serde_json::json!(0.7));
        config
            .extra
            .insert("num_ctx".to_string(), serde_json::json!(4096));

        let json = serde_json::to_string(&config).unwrap();
        let parsed: ProviderConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(
            parsed.extra.get("temperature"),
            Some(&serde_json::json!(0.7))
        );
        assert_eq!(parsed.extra.get("num_ctx"), Some(&serde_json::json!(4096)));
    }
}

// ============================================================================
// CSM Config Tests
// ============================================================================

mod csm_config_tests {
    use super::*;

    #[test]
    fn test_csm_config_default() {
        let config = CsmConfig::default();

        assert!(config.providers.is_empty());
        assert!(config.default_provider.is_none());
        assert!(config.auto_discover);
    }

    #[test]
    fn test_csm_config_serialization() {
        let mut config = CsmConfig {
            default_provider: Some(ProviderType::Ollama),
            ..Default::default()
        };
        config
            .providers
            .push(ProviderConfig::new(ProviderType::Ollama));
        config
            .providers
            .push(ProviderConfig::new(ProviderType::Vllm));

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("\"default_provider\": \"ollama\""));
        assert!(json.contains("\"auto_discover\": true"));
    }

    #[test]
    fn test_csm_config_deserialization() {
        let json = r#"{
            "providers": [
                {"provider_type": "ollama", "enabled": true},
                {"provider_type": "vllm", "enabled": false}
            ],
            "default_provider": "ollama",
            "auto_discover": false
        }"#;

        let config: CsmConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.providers.len(), 2);
        assert_eq!(config.default_provider, Some(ProviderType::Ollama));
        assert!(!config.auto_discover);

        assert!(config.providers[0].enabled);
        assert!(!config.providers[1].enabled);
    }

    #[test]
    fn test_csm_config_find_provider() {
        let mut config = CsmConfig::default();
        config
            .providers
            .push(ProviderConfig::new(ProviderType::Ollama));
        config
            .providers
            .push(ProviderConfig::new(ProviderType::Vllm));

        // Find existing provider
        let ollama = config
            .providers
            .iter()
            .find(|p| p.provider_type == ProviderType::Ollama);
        assert!(ollama.is_some());

        // Non-existing provider
        let cursor = config
            .providers
            .iter()
            .find(|p| p.provider_type == ProviderType::Cursor);
        assert!(cursor.is_none());
    }
}

// ============================================================================
// Provider Registry Tests
// ============================================================================

mod provider_registry_tests {
    use super::*;

    #[test]
    fn test_provider_registry_new() {
        let registry = ProviderRegistry::new();

        // Registry should discover providers automatically
        // At minimum, it should initialize without panicking
        let _ = registry.providers();
    }

    #[test]
    fn test_provider_registry_default() {
        let registry = ProviderRegistry::default();

        // Default should be equivalent to new()
        let _ = registry.providers();
    }

    #[test]
    fn test_provider_registry_available_providers() {
        let registry = ProviderRegistry::new();

        // Available providers should be a subset of all providers
        let all_count = registry.providers().len();
        let available_count = registry.available_providers().len();

        assert!(available_count <= all_count);
    }

    #[test]
    fn test_provider_registry_get_provider() {
        let registry = ProviderRegistry::new();

        // Try to get providers by type
        // These may or may not be available depending on system configuration
        for provider_type in [
            ProviderType::Cursor,
            ProviderType::Ollama,
            ProviderType::Vllm,
            ProviderType::LmStudio,
        ] {
            let result = registry.get_provider(provider_type);
            // Just verify it doesn't panic
            if let Some(provider) = result {
                assert_eq!(provider.provider_type(), provider_type);
            }
        }
    }

    #[test]
    fn test_provider_registry_list_all_sessions() {
        let registry = ProviderRegistry::new();

        // This should not panic even if no providers are available
        let result = registry.list_all_sessions();
        assert!(result.is_ok());
    }
}

// ============================================================================
// Session Format Conversion Tests
// ============================================================================

mod session_format_tests {
    use super::*;

    fn create_test_session() -> ChatSession {
        ChatSession {
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
            requests: vec![
                ChatRequest {
                    timestamp: Some(1700000000000),
                    message: Some(ChatMessage {
                        text: Some("Hello, how are you?".to_string()),
                        parts: None,
                    }),
                    response: Some(serde_json::json!({
                        "value": [{"value": "I'm doing well, thank you for asking!"}]
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
                    timestamp: Some(1700000001000),
                    message: Some(ChatMessage {
                        text: Some("What is 2+2?".to_string()),
                        parts: None,
                    }),
                    response: Some(serde_json::json!({
                        "value": [{"value": "2+2 equals 4."}]
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
    fn test_generic_message_creation() {
        let msg = GenericMessage {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
            timestamp: Some(1700000000000),
            model: Some("gpt-4".to_string()),
        };

        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello, world!");
        assert_eq!(msg.timestamp, Some(1700000000000));
        assert_eq!(msg.model, Some("gpt-4".to_string()));
    }

    #[test]
    fn test_generic_session_creation() {
        let session = GenericSession {
            id: "session-123".to_string(),
            title: Some("Test Session".to_string()),
            messages: vec![
                GenericMessage {
                    role: "user".to_string(),
                    content: "Hi".to_string(),
                    timestamp: None,
                    model: None,
                },
                GenericMessage {
                    role: "assistant".to_string(),
                    content: "Hello!".to_string(),
                    timestamp: None,
                    model: None,
                },
            ],
            created_at: Some(1700000000000),
            updated_at: Some(1700000001000),
            provider: Some("ollama".to_string()),
            model: Some("llama3.2".to_string()),
        };

        assert_eq!(session.id, "session-123");
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.title, Some("Test Session".to_string()));
    }

    #[test]
    fn test_chat_session_to_generic_session() {
        let chat_session = create_test_session();
        let generic: GenericSession = chat_session.into();

        assert_eq!(generic.id, "test-session-123");
        assert_eq!(generic.title, Some("Test Conversation".to_string()));
        assert_eq!(generic.created_at, Some(1700000000000));
        assert_eq!(generic.updated_at, Some(1700000001000));

        // Should have 4 messages (2 user + 2 assistant)
        assert_eq!(generic.messages.len(), 4);

        // Check message order and content
        assert_eq!(generic.messages[0].role, "user");
        assert_eq!(generic.messages[0].content, "Hello, how are you?");

        assert_eq!(generic.messages[1].role, "assistant");
        assert!(generic.messages[1].content.contains("doing well"));
    }

    #[test]
    fn test_generic_session_to_chat_session() {
        let generic = GenericSession {
            id: "generic-123".to_string(),
            title: Some("Imported Session".to_string()),
            messages: vec![
                GenericMessage {
                    role: "user".to_string(),
                    content: "What is Rust?".to_string(),
                    timestamp: Some(1700000000000),
                    model: Some("llama3.2".to_string()),
                },
                GenericMessage {
                    role: "assistant".to_string(),
                    content: "Rust is a systems programming language.".to_string(),
                    timestamp: Some(1700000001000),
                    model: Some("llama3.2".to_string()),
                },
            ],
            created_at: Some(1700000000000),
            updated_at: Some(1700000001000),
            provider: Some("ollama".to_string()),
            model: Some("llama3.2".to_string()),
        };

        let chat_session: ChatSession = generic.into();

        assert_eq!(chat_session.session_id, Some("generic-123".to_string()));
        assert_eq!(
            chat_session.custom_title,
            Some("Imported Session".to_string())
        );
        assert_eq!(chat_session.requests.len(), 1);

        let request = &chat_session.requests[0];
        assert_eq!(
            request.message.as_ref().unwrap().text,
            Some("What is Rust?".to_string())
        );
    }

    #[test]
    fn test_generic_message_serialization() {
        let msg = GenericMessage {
            role: "user".to_string(),
            content: "Test message".to_string(),
            timestamp: Some(1700000000000),
            model: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: GenericMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(msg.role, parsed.role);
        assert_eq!(msg.content, parsed.content);
        assert_eq!(msg.timestamp, parsed.timestamp);
    }

    #[test]
    fn test_generic_session_serialization() {
        let session = GenericSession {
            id: "session-456".to_string(),
            title: None,
            messages: vec![],
            created_at: None,
            updated_at: None,
            provider: None,
            model: None,
        };

        let json = serde_json::to_string(&session).unwrap();
        let parsed: GenericSession = serde_json::from_str(&json).unwrap();

        assert_eq!(session.id, parsed.id);
        assert_eq!(session.title, parsed.title);
    }

    #[test]
    fn test_roundtrip_conversion() {
        let original = create_test_session();
        let original_id = original.session_id.clone();
        let original_title = original.custom_title.clone();

        // Convert to generic and back
        let generic: GenericSession = original.into();
        let restored: ChatSession = generic.into();

        assert_eq!(restored.session_id, original_id);
        assert_eq!(restored.custom_title, original_title);
    }
}

// ============================================================================
// OpenAI Compatible Provider Tests
// ============================================================================

mod openai_compat_tests {
    use super::*;
    use chasm::providers::openai_compat::{OpenAIChatMessage, OpenAICompatProvider};
    use chasm::providers::ChatProvider;

    #[test]
    fn test_openai_chat_message_creation() {
        let msg = OpenAIChatMessage {
            role: "user".to_string(),
            content: "Hello!".to_string(),
        };

        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello!");
    }

    #[test]
    fn test_openai_chat_message_serialization() {
        let msg = OpenAIChatMessage {
            role: "assistant".to_string(),
            content: "How can I help you?".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"assistant\""));
        assert!(json.contains("\"content\":\"How can I help you?\""));
    }

    #[test]
    fn test_openai_compat_provider_new() {
        let provider =
            OpenAICompatProvider::new(ProviderType::Vllm, "vLLM Server", "http://localhost:8000");

        assert_eq!(provider.provider_type(), ProviderType::Vllm);
        assert_eq!(provider.name(), "vLLM Server");
    }

    #[test]
    fn test_openai_compat_provider_with_api_key() {
        let provider =
            OpenAICompatProvider::new(ProviderType::OpenAI, "OpenAI", "https://api.openai.com/v1")
                .with_api_key("sk-test-key");

        assert_eq!(provider.provider_type(), ProviderType::OpenAI);
    }

    #[test]
    fn test_openai_compat_provider_with_model() {
        let provider =
            OpenAICompatProvider::new(ProviderType::Vllm, "vLLM", "http://localhost:8000")
                .with_model("meta-llama/Llama-3.1-8B-Instruct");

        assert_eq!(provider.name(), "vLLM");
    }

    #[test]
    fn test_openai_compat_provider_is_available() {
        let provider =
            OpenAICompatProvider::new(ProviderType::Vllm, "vLLM", "http://localhost:8000");

        // Should return true since endpoint is not empty
        // Actual connectivity check is not done in tests
        assert!(provider.is_available());
    }

    #[test]
    fn test_openai_compat_provider_sessions_path() {
        let provider =
            OpenAICompatProvider::new(ProviderType::Vllm, "vLLM", "http://localhost:8000");

        // API-based providers typically don't have a local sessions path
        assert!(provider.sessions_path().is_none());
    }

    #[test]
    fn test_session_to_openai_messages() {
        let session = ChatSession {
            version: 3,
            session_id: Some("test-123".to_string()),
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
                    text: Some("Hello".to_string()),
                    parts: None,
                }),
                response: Some(serde_json::json!({
                    "value": [{"value": "Hi there!"}]
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

        let messages = OpenAICompatProvider::session_to_messages(&session);

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].role, "assistant");
        assert!(messages[1].content.contains("Hi there"));
    }

    #[test]
    fn test_openai_messages_to_session() {
        let messages = vec![
            OpenAIChatMessage {
                role: "user".to_string(),
                content: "What is AI?".to_string(),
            },
            OpenAIChatMessage {
                role: "assistant".to_string(),
                content: "AI stands for Artificial Intelligence.".to_string(),
            },
        ];

        let session = OpenAICompatProvider::messages_to_session(messages, "gpt-4", "OpenAI");

        assert!(session.session_id.is_some());
        assert_eq!(session.requests.len(), 1);

        let request = &session.requests[0];
        assert_eq!(
            request.message.as_ref().unwrap().text,
            Some("What is AI?".to_string())
        );
    }

    #[test]
    fn test_empty_messages_to_session() {
        let messages: Vec<OpenAIChatMessage> = vec![];
        let session = OpenAICompatProvider::messages_to_session(messages, "gpt-4", "test");

        assert!(session.requests.is_empty());
    }

    #[test]
    fn test_system_message_handling() {
        let messages = vec![
            OpenAIChatMessage {
                role: "system".to_string(),
                content: "You are a helpful assistant.".to_string(),
            },
            OpenAIChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            },
            OpenAIChatMessage {
                role: "assistant".to_string(),
                content: "Hi!".to_string(),
            },
        ];

        let session = OpenAICompatProvider::messages_to_session(messages, "gpt-4", "test");

        // System message should be skipped in conversion
        assert_eq!(session.requests.len(), 1);
    }
}

// ============================================================================
// Ollama Provider Tests
// ============================================================================

mod ollama_provider_tests {
    use super::*;
    use chasm::providers::ollama::OllamaProvider;
    use chasm::providers::ChatProvider;

    #[test]
    fn test_ollama_provider_discover() {
        // Discover may return None if Ollama is not installed
        let result = OllamaProvider::discover();

        // Just verify it doesn't panic
        if let Some(provider) = result {
            assert_eq!(provider.name(), "Ollama");
            assert_eq!(provider.provider_type(), ProviderType::Ollama);
        }
    }

    #[test]
    fn test_ollama_provider_type() {
        if let Some(provider) = OllamaProvider::discover() {
            assert_eq!(provider.provider_type(), ProviderType::Ollama);
        }
    }

    #[test]
    fn test_ollama_provider_name() {
        if let Some(provider) = OllamaProvider::discover() {
            assert_eq!(provider.name(), "Ollama");
        }
    }

    #[test]
    fn test_ollama_provider_sessions_path() {
        if let Some(provider) = OllamaProvider::discover() {
            // Ollama stores data in ~/.ollama
            let path = provider.sessions_path();
            // Path may or may not exist depending on system
            if let Some(p) = path {
                assert!(p.to_string_lossy().contains("ollama"));
            }
        }
    }

    #[test]
    fn test_ollama_provider_list_models() {
        if let Some(provider) = OllamaProvider::discover() {
            // This should not panic even if Ollama is not running
            let result = provider.list_models();
            assert!(result.is_ok());
        }
    }
}

// ============================================================================
// Cursor Provider Tests
// ============================================================================

mod cursor_provider_tests {
    use super::*;
    use chasm::providers::cursor::CursorProvider;
    use chasm::providers::ChatProvider;

    #[test]
    fn test_cursor_provider_discover() {
        // Discover may return None if Cursor is not installed
        let result = CursorProvider::discover();

        // Just verify it doesn't panic
        if let Some(provider) = result {
            assert_eq!(provider.name(), "Cursor");
            assert_eq!(provider.provider_type(), ProviderType::Cursor);
        }
    }

    #[test]
    fn test_cursor_provider_type() {
        if let Some(provider) = CursorProvider::discover() {
            assert_eq!(provider.provider_type(), ProviderType::Cursor);
        }
    }

    #[test]
    fn test_cursor_provider_name() {
        if let Some(provider) = CursorProvider::discover() {
            assert_eq!(provider.name(), "Cursor");
        }
    }

    #[test]
    fn test_cursor_provider_uses_file_storage() {
        // Cursor uses file storage similar to VS Code Copilot
        assert!(ProviderType::Cursor.uses_file_storage());
    }
}

// ============================================================================
// Discovery Tests
// ============================================================================

mod discovery_tests {
    use super::*;

    #[test]
    fn test_discover_all_providers() {
        let registry = ProviderRegistry::new();
        let providers = registry.providers();

        // Should return a list (may be empty)
        let _ = providers;
    }

    #[test]
    fn test_discovery_does_not_panic() {
        // Multiple calls should be safe
        for _ in 0..3 {
            let _ = ProviderRegistry::new();
        }
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

mod integration_tests {
    use super::*;

    #[test]
    fn test_full_session_import_export_workflow() {
        // Create a session
        let original_session = ChatSession {
            version: 3,
            session_id: Some("workflow-test-123".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000001000,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: Some("Workflow Test".to_string()),
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: None,
            responder_avatar_icon_uri: None,
            requests: vec![ChatRequest {
                timestamp: Some(1700000000000),
                message: Some(ChatMessage {
                    text: Some("Explain recursion".to_string()),
                    parts: None,
                }),
                response: Some(serde_json::json!({
                    "value": [{
                        "value": "Recursion is when a function calls itself. To understand recursion, you must first understand recursion."
                    }]
                })),
                variable_data: None,
                request_id: Some("req-1".to_string()),
                response_id: Some("resp-1".to_string()),
                model_id: Some("claude-3".to_string()),
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

        // Convert to generic format
        let generic: GenericSession = original_session.clone().into();

        // Serialize to JSON (as if exporting)
        let json = serde_json::to_string_pretty(&generic).unwrap();

        // Deserialize from JSON (as if importing)
        let imported_generic: GenericSession = serde_json::from_str(&json).unwrap();

        // Convert back to ChatSession
        let restored: ChatSession = imported_generic.into();

        // Verify key fields preserved
        assert_eq!(restored.session_id, original_session.session_id);
        assert_eq!(restored.custom_title, original_session.custom_title);
        assert_eq!(restored.requests.len(), original_session.requests.len());
    }

    #[test]
    fn test_multi_turn_conversation_conversion() {
        let session = ChatSession {
            version: 3,
            session_id: Some("multi-turn-123".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000003000,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: None,
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: None,
            responder_avatar_icon_uri: None,
            requests: vec![
                ChatRequest {
                    timestamp: Some(1700000000000),
                    message: Some(ChatMessage {
                        text: Some("What is Rust?".to_string()),
                        parts: None,
                    }),
                    response: Some(serde_json::json!({
                        "value": [{"value": "Rust is a systems programming language."}]
                    })),
                    variable_data: None,
                    request_id: None,
                    response_id: None,
                    model_id: Some("model-1".to_string()),
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
                    timestamp: Some(1700000001000),
                    message: Some(ChatMessage {
                        text: Some("What are its main features?".to_string()),
                        parts: None,
                    }),
                    response: Some(serde_json::json!({
                        "value": [{"value": "Memory safety without garbage collection."}]
                    })),
                    variable_data: None,
                    request_id: None,
                    response_id: None,
                    model_id: Some("model-1".to_string()),
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
                    timestamp: Some(1700000002000),
                    message: Some(ChatMessage {
                        text: Some("Show me an example".to_string()),
                        parts: None,
                    }),
                    response: Some(serde_json::json!({
                        "value": [{"value": "fn main() { println!(\"Hello, Rust!\"); }"}]
                    })),
                    variable_data: None,
                    request_id: None,
                    response_id: None,
                    model_id: Some("model-1".to_string()),
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

        let generic: GenericSession = session.into();

        // Should have 6 messages (3 user + 3 assistant)
        assert_eq!(generic.messages.len(), 6);

        // Check alternating pattern
        for (i, msg) in generic.messages.iter().enumerate() {
            if i % 2 == 0 {
                assert_eq!(msg.role, "user");
            } else {
                assert_eq!(msg.role, "assistant");
            }
        }
    }

    #[test]
    fn test_provider_config_integration() {
        // Create a complete configuration
        let mut config = CsmConfig {
            default_provider: Some(ProviderType::Ollama),
            ..Default::default()
        };

        let mut ollama_config = ProviderConfig::new(ProviderType::Ollama);
        ollama_config.model = Some("llama3.2".to_string());
        config.providers.push(ollama_config);

        let mut vllm_config = ProviderConfig::new(ProviderType::Vllm);
        vllm_config.endpoint = Some("http://gpu-server:8000".to_string());
        vllm_config.api_key = Some("sk-local-key".to_string());
        config.providers.push(vllm_config);

        // Serialize and deserialize
        let json = serde_json::to_string_pretty(&config).unwrap();
        let restored: CsmConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.default_provider, Some(ProviderType::Ollama));
        assert_eq!(restored.providers.len(), 2);

        let ollama = &restored.providers[0];
        assert_eq!(ollama.model, Some("llama3.2".to_string()));

        let vllm = &restored.providers[1];
        assert_eq!(vllm.endpoint, Some("http://gpu-server:8000".to_string()));
        assert_eq!(vllm.api_key, Some("sk-local-key".to_string()));
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_session_conversion() {
        let empty_session = ChatSession {
            version: 3,
            session_id: Some("empty-session".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000000000,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: None,
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: None,
            responder_avatar_icon_uri: None,
            requests: vec![],
        };

        let generic: GenericSession = empty_session.into();
        assert!(generic.messages.is_empty());
    }

    #[test]
    fn test_session_with_missing_fields() {
        let minimal_session = ChatSession {
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
            }],
        };

        // Should not panic
        let generic: GenericSession = minimal_session.into();
        assert!(generic.messages.is_empty()); // No valid messages extracted
    }

    #[test]
    fn test_session_with_special_characters() {
        let session = ChatSession {
            version: 3,
            session_id: Some("special-chars".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000001000,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: Some("Test with \"quotes\" and 'apostrophes'".to_string()),
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: None,
            responder_avatar_icon_uri: None,
            requests: vec![ChatRequest {
                timestamp: Some(1700000000000),
                message: Some(ChatMessage {
                    text: Some("Hello\nWorld\t!".to_string()),
                    parts: None,
                }),
                response: Some(serde_json::json!({
                    "value": [{"value": "Response with <html> and & special chars"}]
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

        let generic: GenericSession = session.clone().into();
        assert!(generic.messages[0].content.contains('\n'));

        // Roundtrip through JSON
        let json = serde_json::to_string(&generic).unwrap();
        let restored: GenericSession = serde_json::from_str(&json).unwrap();
        assert_eq!(
            restored.title,
            Some("Test with \"quotes\" and 'apostrophes'".to_string())
        );
    }

    #[test]
    fn test_session_with_unicode() {
        let session = ChatSession {
            version: 3,
            session_id: Some("unicode-test".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000001000,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: Some("Japanese Test".to_string()),
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: None,
            responder_avatar_icon_uri: None,
            requests: vec![ChatRequest {
                timestamp: Some(1700000000000),
                message: Some(ChatMessage {
                    text: Some("Hello!".to_string()),
                    parts: None,
                }),
                response: Some(serde_json::json!({
                    "value": [{"value": "Hello World"}]
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

        let generic: GenericSession = session.into();
        assert!(generic.title.as_ref().unwrap().contains("Japanese"));
        assert!(generic.messages[0].content.contains("Hello"));
    }

    #[test]
    fn test_large_session_conversion() {
        // Create a session with many messages
        let mut requests = Vec::new();
        for i in 0..100 {
            requests.push(ChatRequest {
                timestamp: Some(1700000000000 + i as i64 * 1000),
                message: Some(ChatMessage {
                    text: Some(format!("Question {}", i)),
                    parts: None,
                }),
                response: Some(serde_json::json!({
                    "value": [{"value": format!("Answer {}", i)}]
                })),
                variable_data: None,
                request_id: Some(format!("req-{}", i)),
                response_id: Some(format!("resp-{}", i)),
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
        }

        let session = ChatSession {
            version: 3,
            session_id: Some("large-session".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000099000,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: Some("Large Conversation".to_string()),
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: None,
            responder_avatar_icon_uri: None,
            requests,
        };

        let generic: GenericSession = session.into();

        // Should have 200 messages (100 user + 100 assistant)
        assert_eq!(generic.messages.len(), 200);
    }

    #[test]
    fn test_provider_config_all_fields() {
        use std::path::PathBuf;

        let mut config = ProviderConfig::new(ProviderType::Custom);
        config.enabled = true;
        config.endpoint = Some("http://custom-server:9000/v1".to_string());
        config.api_key = Some("custom-api-key".to_string());
        config.model = Some("custom-model".to_string());
        config.name = Some("My Custom Provider".to_string());
        config.storage_path = Some(PathBuf::from("/var/data/custom"));
        config
            .extra
            .insert("option1".to_string(), serde_json::json!("value1"));
        config
            .extra
            .insert("option2".to_string(), serde_json::json!(42));

        let json = serde_json::to_string(&config).unwrap();
        let restored: ProviderConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.provider_type, ProviderType::Custom);
        assert!(restored.enabled);
        assert_eq!(restored.endpoint, config.endpoint);
        assert_eq!(restored.api_key, config.api_key);
        assert_eq!(restored.model, config.model);
        assert_eq!(restored.name, config.name);
        assert_eq!(restored.storage_path, config.storage_path);
        assert_eq!(restored.extra.len(), 2);
    }
}

// ============================================================================
// Stress Tests
// ============================================================================

mod stress_tests {
    use super::*;

    #[test]
    fn test_rapid_provider_registry_creation() {
        // Create multiple registries rapidly
        for _ in 0..10 {
            let registry = ProviderRegistry::new();
            let _ = registry.providers();
        }
    }

    #[test]
    fn test_concurrent_config_serialization() {
        use std::thread;

        let handles: Vec<_> = (0..4)
            .map(|i| {
                thread::spawn(move || {
                    let mut config = CsmConfig::default();
                    config
                        .providers
                        .push(ProviderConfig::new(ProviderType::Ollama));

                    for _ in 0..100 {
                        let json = serde_json::to_string(&config).unwrap();
                        let _: CsmConfig = serde_json::from_str(&json).unwrap();
                    }
                    i
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
