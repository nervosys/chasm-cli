//! Tests for cloud provider integrations
//!
//! Tests for the cloud provider module including:
//! - Provider type cloud detection
//! - Cloud provider configuration
//! - Export file parsing
//! - Conversation conversion

use chasm::providers::config::ProviderType;

// ============================================================================
// Cloud Provider Type Tests
// ============================================================================

mod cloud_provider_type_tests {
    use super::*;

    #[test]
    fn test_cloud_provider_detection() {
        // Cloud providers
        assert!(ProviderType::M365Copilot.is_cloud_provider());
        assert!(ProviderType::ChatGPT.is_cloud_provider());
        assert!(ProviderType::OpenAI.is_cloud_provider());
        assert!(ProviderType::Anthropic.is_cloud_provider());
        assert!(ProviderType::Perplexity.is_cloud_provider());
        assert!(ProviderType::DeepSeek.is_cloud_provider());
        assert!(ProviderType::Qwen.is_cloud_provider());
        assert!(ProviderType::Gemini.is_cloud_provider());
        assert!(ProviderType::Mistral.is_cloud_provider());
        assert!(ProviderType::Cohere.is_cloud_provider());
        assert!(ProviderType::Grok.is_cloud_provider());
        assert!(ProviderType::Groq.is_cloud_provider());
        assert!(ProviderType::Together.is_cloud_provider());
        assert!(ProviderType::Fireworks.is_cloud_provider());
        assert!(ProviderType::Replicate.is_cloud_provider());
        assert!(ProviderType::HuggingFace.is_cloud_provider());

        // Local providers should NOT be cloud
        assert!(!ProviderType::Copilot.is_cloud_provider());
        assert!(!ProviderType::Cursor.is_cloud_provider());
        assert!(!ProviderType::Ollama.is_cloud_provider());
        assert!(!ProviderType::Vllm.is_cloud_provider());
        assert!(!ProviderType::LmStudio.is_cloud_provider());
        assert!(!ProviderType::LocalAI.is_cloud_provider());
        assert!(!ProviderType::Jan.is_cloud_provider());
        assert!(!ProviderType::Gpt4All.is_cloud_provider());
        assert!(!ProviderType::Llamafile.is_cloud_provider());
        assert!(!ProviderType::TextGenWebUI.is_cloud_provider());
        assert!(!ProviderType::Custom.is_cloud_provider());
        assert!(!ProviderType::Foundry.is_cloud_provider());
    }

    #[test]
    fn test_cloud_provider_display_names() {
        assert_eq!(
            ProviderType::M365Copilot.display_name(),
            "Microsoft 365 Copilot"
        );
        assert_eq!(ProviderType::ChatGPT.display_name(), "ChatGPT");
        assert_eq!(ProviderType::Anthropic.display_name(), "Anthropic Claude");
        assert_eq!(ProviderType::Perplexity.display_name(), "Perplexity AI");
        assert_eq!(ProviderType::DeepSeek.display_name(), "DeepSeek");
        assert_eq!(ProviderType::Qwen.display_name(), "Qwen (Alibaba)");
        assert_eq!(ProviderType::Gemini.display_name(), "Google Gemini");
        assert_eq!(ProviderType::Mistral.display_name(), "Mistral AI");
        assert_eq!(ProviderType::Cohere.display_name(), "Cohere");
        assert_eq!(ProviderType::Grok.display_name(), "xAI Grok");
        assert_eq!(ProviderType::Groq.display_name(), "Groq");
        assert_eq!(ProviderType::Together.display_name(), "Together AI");
        assert_eq!(ProviderType::Fireworks.display_name(), "Fireworks AI");
        assert_eq!(ProviderType::Replicate.display_name(), "Replicate");
        assert_eq!(ProviderType::HuggingFace.display_name(), "HuggingFace");
    }

    #[test]
    fn test_cloud_provider_endpoints() {
        // Microsoft Graph API
        assert_eq!(
            ProviderType::M365Copilot.default_endpoint(),
            Some("https://graph.microsoft.com/v1.0")
        );

        // ChatGPT web interface
        assert_eq!(
            ProviderType::ChatGPT.default_endpoint(),
            Some("https://chat.openai.com")
        );

        // Standard API endpoints
        assert_eq!(
            ProviderType::OpenAI.default_endpoint(),
            Some("https://api.openai.com/v1")
        );
        assert_eq!(
            ProviderType::Anthropic.default_endpoint(),
            Some("https://api.anthropic.com/v1")
        );
        assert_eq!(
            ProviderType::Perplexity.default_endpoint(),
            Some("https://api.perplexity.ai")
        );
        assert_eq!(
            ProviderType::DeepSeek.default_endpoint(),
            Some("https://api.deepseek.com/v1")
        );
        assert_eq!(
            ProviderType::Qwen.default_endpoint(),
            Some("https://dashscope.aliyuncs.com/api/v1")
        );
        assert_eq!(
            ProviderType::Gemini.default_endpoint(),
            Some("https://generativelanguage.googleapis.com/v1beta")
        );
        assert_eq!(
            ProviderType::Mistral.default_endpoint(),
            Some("https://api.mistral.ai/v1")
        );
        assert_eq!(
            ProviderType::Cohere.default_endpoint(),
            Some("https://api.cohere.ai/v1")
        );
        assert_eq!(
            ProviderType::Grok.default_endpoint(),
            Some("https://api.x.ai/v1")
        );
        assert_eq!(
            ProviderType::Groq.default_endpoint(),
            Some("https://api.groq.com/openai/v1")
        );
        assert_eq!(
            ProviderType::Together.default_endpoint(),
            Some("https://api.together.xyz/v1")
        );
        assert_eq!(
            ProviderType::Fireworks.default_endpoint(),
            Some("https://api.fireworks.ai/inference/v1")
        );
        assert_eq!(
            ProviderType::Replicate.default_endpoint(),
            Some("https://api.replicate.com/v1")
        );
        assert_eq!(
            ProviderType::HuggingFace.default_endpoint(),
            Some("https://api-inference.huggingface.co")
        );
    }

    #[test]
    fn test_cloud_provider_openai_compatibility() {
        // These cloud providers use OpenAI-compatible APIs
        assert!(ProviderType::OpenAI.is_openai_compatible());
        assert!(ProviderType::DeepSeek.is_openai_compatible());
        assert!(ProviderType::Groq.is_openai_compatible());
        assert!(ProviderType::Together.is_openai_compatible());
        assert!(ProviderType::Fireworks.is_openai_compatible());

        // These have their own API formats
        assert!(!ProviderType::ChatGPT.is_openai_compatible()); // Web interface
        assert!(!ProviderType::Anthropic.is_openai_compatible());
        assert!(!ProviderType::Perplexity.is_openai_compatible());
        assert!(!ProviderType::Gemini.is_openai_compatible());
        assert!(!ProviderType::Cohere.is_openai_compatible());
        assert!(!ProviderType::Replicate.is_openai_compatible());
        assert!(!ProviderType::HuggingFace.is_openai_compatible());
    }

    #[test]
    fn test_cloud_provider_requires_api_key() {
        // All cloud providers should require API keys
        assert!(ProviderType::M365Copilot.requires_api_key());
        assert!(ProviderType::ChatGPT.requires_api_key());
        assert!(ProviderType::OpenAI.requires_api_key());
        assert!(ProviderType::Anthropic.requires_api_key());
        assert!(ProviderType::Perplexity.requires_api_key());
        assert!(ProviderType::DeepSeek.requires_api_key());
        assert!(ProviderType::Gemini.requires_api_key());

        // Local providers should not require API keys
        assert!(!ProviderType::Copilot.requires_api_key());
        assert!(!ProviderType::Cursor.requires_api_key());
        assert!(!ProviderType::Ollama.requires_api_key());
        assert!(!ProviderType::LmStudio.requires_api_key());
    }
}

// ============================================================================
// Cloud Provider Serialization Tests
// ============================================================================

mod cloud_provider_serialization_tests {
    use super::*;

    #[test]
    fn test_cloud_provider_type_serialization() {
        // Test kebab-case serialization
        let m365copilot = serde_json::to_string(&ProviderType::M365Copilot).unwrap();
        assert_eq!(m365copilot, "\"m365copilot\"");

        let chatgpt = serde_json::to_string(&ProviderType::ChatGPT).unwrap();
        assert_eq!(chatgpt, "\"chatgpt\"");

        let anthropic = serde_json::to_string(&ProviderType::Anthropic).unwrap();
        assert_eq!(anthropic, "\"anthropic\"");

        let perplexity = serde_json::to_string(&ProviderType::Perplexity).unwrap();
        assert_eq!(perplexity, "\"perplexity\"");

        let deepseek = serde_json::to_string(&ProviderType::DeepSeek).unwrap();
        assert_eq!(deepseek, "\"deepseek\"");

        let qwen = serde_json::to_string(&ProviderType::Qwen).unwrap();
        assert_eq!(qwen, "\"qwen\"");

        let gemini = serde_json::to_string(&ProviderType::Gemini).unwrap();
        assert_eq!(gemini, "\"gemini\"");

        let groq = serde_json::to_string(&ProviderType::Groq).unwrap();
        assert_eq!(groq, "\"groq\"");
    }

    #[test]
    fn test_cloud_provider_type_deserialization() {
        let m365copilot: ProviderType = serde_json::from_str("\"m365copilot\"").unwrap();
        assert_eq!(m365copilot, ProviderType::M365Copilot);

        let chatgpt: ProviderType = serde_json::from_str("\"chatgpt\"").unwrap();
        assert_eq!(chatgpt, ProviderType::ChatGPT);

        let anthropic: ProviderType = serde_json::from_str("\"anthropic\"").unwrap();
        assert_eq!(anthropic, ProviderType::Anthropic);

        let perplexity: ProviderType = serde_json::from_str("\"perplexity\"").unwrap();
        assert_eq!(perplexity, ProviderType::Perplexity);

        let deepseek: ProviderType = serde_json::from_str("\"deepseek\"").unwrap();
        assert_eq!(deepseek, ProviderType::DeepSeek);

        let gemini: ProviderType = serde_json::from_str("\"gemini\"").unwrap();
        assert_eq!(gemini, ProviderType::Gemini);
    }
}

// ============================================================================
// Cloud Provider Trait Tests
// ============================================================================

mod cloud_provider_trait_tests {
    use chasm::providers::cloud::anthropic::AnthropicProvider;
    use chasm::providers::cloud::chatgpt::ChatGPTProvider;
    use chasm::providers::cloud::common::CloudProvider;
    use chasm::providers::cloud::deepseek::DeepSeekProvider;
    use chasm::providers::cloud::gemini::GeminiProvider;
    use chasm::providers::cloud::m365copilot::M365CopilotProvider;
    use chasm::providers::cloud::perplexity::PerplexityProvider;

    #[test]
    fn test_m365_copilot_provider_creation() {
        let provider = M365CopilotProvider::new(Some("test-token".to_string()));
        assert_eq!(provider.name(), "Microsoft 365 Copilot");
        assert!(provider.is_authenticated());
        assert_eq!(provider.api_key_env_var(), "M365_COPILOT_ACCESS_TOKEN");
    }

    #[test]
    fn test_m365_copilot_provider_unauthenticated() {
        let provider = M365CopilotProvider::new(None);
        assert!(!provider.is_authenticated());
    }

    #[test]
    fn test_chatgpt_provider_creation() {
        let provider = ChatGPTProvider::new(Some("test-key".to_string()));
        assert_eq!(provider.name(), "ChatGPT");
        assert!(provider.is_authenticated());
        assert_eq!(provider.api_key_env_var(), "OPENAI_API_KEY");
    }

    #[test]
    fn test_chatgpt_provider_without_auth() {
        let provider = ChatGPTProvider::new(None);
        assert!(!provider.is_authenticated());
    }

    #[test]
    fn test_anthropic_provider_creation() {
        let provider = AnthropicProvider::new(Some("test-key".to_string()));
        assert_eq!(provider.name(), "Claude");
        assert!(provider.is_authenticated());
        assert_eq!(provider.api_key_env_var(), "ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_anthropic_provider_without_auth() {
        let provider = AnthropicProvider::new(None);
        assert!(!provider.is_authenticated());
    }

    #[test]
    fn test_perplexity_provider_creation() {
        let provider = PerplexityProvider::new(Some("test-key".to_string()));
        assert_eq!(provider.name(), "Perplexity");
        assert!(provider.is_authenticated());
        assert_eq!(provider.api_key_env_var(), "PERPLEXITY_API_KEY");
    }

    #[test]
    fn test_deepseek_provider_creation() {
        let provider = DeepSeekProvider::new(Some("test-key".to_string()));
        assert_eq!(provider.name(), "DeepSeek");
        assert!(provider.is_authenticated());
        assert_eq!(provider.api_key_env_var(), "DEEPSEEK_API_KEY");
    }

    #[test]
    fn test_gemini_provider_creation() {
        let provider = GeminiProvider::new(Some("test-key".to_string()));
        assert_eq!(provider.name(), "Gemini");
        assert!(provider.is_authenticated());
        assert_eq!(provider.api_key_env_var(), "GOOGLE_API_KEY");
    }
}

// ============================================================================
// Cloud Conversation Tests
// ============================================================================

mod cloud_conversation_tests {
    use chasm::providers::cloud::common::{CloudConversation, CloudMessage};
    use chrono::Utc;

    #[test]
    fn test_cloud_conversation_to_chat_session() {
        let conv = CloudConversation {
            id: "test-conv-123".to_string(),
            title: Some("Test Conversation".to_string()),
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
            model: Some("gpt-4".to_string()),
            messages: vec![
                CloudMessage {
                    id: Some("msg-1".to_string()),
                    role: "user".to_string(),
                    content: "Hello!".to_string(),
                    timestamp: Some(Utc::now()),
                    model: None,
                },
                CloudMessage {
                    id: Some("msg-2".to_string()),
                    role: "assistant".to_string(),
                    content: "Hi there! How can I help you?".to_string(),
                    timestamp: Some(Utc::now()),
                    model: Some("gpt-4".to_string()),
                },
            ],
            metadata: None,
        };

        let session = conv.to_chat_session("ChatGPT");

        assert!(session.session_id.is_some());
        assert!(session.session_id.as_ref().unwrap().starts_with("ChatGPT:"));
        assert_eq!(session.custom_title, Some("Test Conversation".to_string()));
        assert!(session.is_imported);
        assert_eq!(session.responder_username, Some("ChatGPT".to_string()));
        assert!(!session.requests.is_empty());
    }

    #[test]
    fn test_cloud_message_role_mapping() {
        let user_msg = CloudMessage {
            id: None,
            role: "user".to_string(),
            content: "test".to_string(),
            timestamp: None,
            model: None,
        };
        assert_eq!(user_msg.role, "user");

        let assistant_msg = CloudMessage {
            id: None,
            role: "assistant".to_string(),
            content: "test".to_string(),
            timestamp: None,
            model: None,
        };
        assert_eq!(assistant_msg.role, "assistant");
    }

    #[test]
    fn test_empty_conversation_conversion() {
        let conv = CloudConversation {
            id: "empty-conv".to_string(),
            title: None,
            created_at: Utc::now(),
            updated_at: None,
            model: None,
            messages: vec![],
            metadata: None,
        };

        let session = conv.to_chat_session("TestProvider");

        assert!(session.requests.is_empty());
        assert_eq!(session.custom_title, None);
    }
}

// ============================================================================
// Fetch Options Tests
// ============================================================================

mod fetch_options_tests {
    use chasm::providers::cloud::common::FetchOptions;
    use chrono::Utc;

    #[test]
    fn test_fetch_options_default() {
        let options = FetchOptions::default();

        assert!(options.limit.is_none());
        assert!(options.after.is_none());
        assert!(options.before.is_none());
        assert!(!options.include_archived);
        assert!(options.session_token.is_none());
    }

    #[test]
    fn test_fetch_options_custom() {
        let now = Utc::now();
        let options = FetchOptions {
            limit: Some(100),
            after: Some(now),
            before: None,
            include_archived: true,
            session_token: Some("token123".to_string()),
        };

        assert_eq!(options.limit, Some(100));
        assert_eq!(options.after, Some(now));
        assert!(options.include_archived);
        assert_eq!(options.session_token, Some("token123".to_string()));
    }
}

// ============================================================================
// HTTP Client Config Tests
// ============================================================================

mod http_client_tests {
    use chasm::providers::cloud::common::HttpClientConfig;

    #[test]
    fn test_http_client_config_default() {
        let config = HttpClientConfig::default();

        assert_eq!(config.timeout_secs, 30);
        assert!(config.user_agent.starts_with("csm/"));
        assert!(!config.accept_invalid_certs);
    }

    #[test]
    fn test_http_client_config_custom() {
        let config = HttpClientConfig {
            timeout_secs: 60,
            user_agent: "custom-agent/1.0".to_string(),
            accept_invalid_certs: true,
        };

        assert_eq!(config.timeout_secs, 60);
        assert_eq!(config.user_agent, "custom-agent/1.0");
        assert!(config.accept_invalid_certs);
    }
}

// ============================================================================
// Provider Registry Cloud Tests
// ============================================================================

mod provider_registry_cloud_tests {
    use super::*;

    #[test]
    fn test_all_provider_types_have_display_names() {
        // Ensure all provider types have non-empty display names
        let types = vec![
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
            ProviderType::ChatGPT,
            ProviderType::OpenAI,
            ProviderType::Anthropic,
            ProviderType::Perplexity,
            ProviderType::DeepSeek,
            ProviderType::Qwen,
            ProviderType::Gemini,
            ProviderType::Mistral,
            ProviderType::Cohere,
            ProviderType::Grok,
            ProviderType::Groq,
            ProviderType::Together,
            ProviderType::Fireworks,
            ProviderType::Replicate,
            ProviderType::HuggingFace,
            ProviderType::Custom,
        ];

        for provider in types {
            let name = provider.display_name();
            assert!(
                !name.is_empty(),
                "Provider {:?} has empty display name",
                provider
            );
        }
    }

    #[test]
    fn test_cloud_providers_have_endpoints() {
        let cloud_types = vec![
            ProviderType::M365Copilot,
            ProviderType::ChatGPT,
            ProviderType::OpenAI,
            ProviderType::Anthropic,
            ProviderType::Perplexity,
            ProviderType::DeepSeek,
            ProviderType::Qwen,
            ProviderType::Gemini,
            ProviderType::Mistral,
            ProviderType::Cohere,
            ProviderType::Grok,
            ProviderType::Groq,
            ProviderType::Together,
            ProviderType::Fireworks,
            ProviderType::Replicate,
            ProviderType::HuggingFace,
        ];

        for provider in cloud_types {
            let endpoint = provider.default_endpoint();
            assert!(
                endpoint.is_some(),
                "Cloud provider {:?} should have an endpoint",
                provider
            );
            assert!(
                endpoint.unwrap().starts_with("https://"),
                "Cloud provider {:?} should have HTTPS endpoint",
                provider
            );
        }
    }
}

// ============================================================================
// Microsoft 365 Copilot Export Parsing Tests
// ============================================================================

mod m365_copilot_export_tests {
    use chasm::providers::cloud::m365copilot::{get_friendly_app_name, parse_m365_copilot_export};

    #[test]
    fn test_parse_empty_array() {
        let json = "[]";
        let result = parse_m365_copilot_export(json).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_graph_response() {
        let json = r#"{
            "value": [
                {
                    "id": "1731701801008",
                    "sessionId": "session-123",
                    "requestId": "req-123",
                    "appClass": "IPM.SkypeTeams.Message.Copilot.BizChat",
                    "interactionType": "userPrompt",
                    "conversationType": "bizchat",
                    "createdDateTime": "2024-11-15T20:16:41.008Z",
                    "locale": "en-us",
                    "body": {
                        "contentType": "text",
                        "content": "What should be on my radar from emails last week?"
                    },
                    "attachments": [],
                    "links": [],
                    "contexts": []
                }
            ]
        }"#;

        let result = parse_m365_copilot_export(json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "session-123");
        assert_eq!(result[0].messages.len(), 1);
        assert_eq!(result[0].messages[0].role, "user");
    }

    #[test]
    fn test_parse_conversation_with_response() {
        let json = r#"[
            {
                "id": "1",
                "sessionId": "session-abc",
                "interactionType": "userPrompt",
                "appClass": "IPM.SkypeTeams.Message.Copilot.BizChat",
                "createdDateTime": "2024-11-15T10:00:00Z",
                "body": { "content": "Hello Copilot" }
            },
            {
                "id": "2",
                "sessionId": "session-abc",
                "interactionType": "aiResponse",
                "appClass": "IPM.SkypeTeams.Message.Copilot.BizChat",
                "createdDateTime": "2024-11-15T10:00:01Z",
                "body": { "content": "Hello! How can I help you today?" },
                "from": {
                    "application": {
                        "displayName": "Microsoft 365 Chat"
                    }
                }
            }
        ]"#;

        let result = parse_m365_copilot_export(json).unwrap();
        assert_eq!(result.len(), 1);

        let conv = &result[0];
        assert_eq!(conv.messages.len(), 2);
        assert_eq!(conv.messages[0].role, "user");
        assert_eq!(conv.messages[1].role, "assistant");
        assert_eq!(
            conv.messages[1].model,
            Some("Microsoft 365 Chat".to_string())
        );
    }

    #[test]
    fn test_parse_multiple_sessions() {
        let json = r#"[
            {
                "id": "1",
                "sessionId": "session-a",
                "interactionType": "userPrompt",
                "appClass": "IPM.SkypeTeams.Message.Copilot.Word",
                "createdDateTime": "2024-11-15T10:00:00Z",
                "body": { "content": "Draft an email" }
            },
            {
                "id": "2",
                "sessionId": "session-b",
                "interactionType": "userPrompt",
                "appClass": "IPM.SkypeTeams.Message.Copilot.Excel",
                "createdDateTime": "2024-11-15T11:00:00Z",
                "body": { "content": "Create a formula" }
            }
        ]"#;

        let result = parse_m365_copilot_export(json).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_friendly_app_names() {
        assert_eq!(
            get_friendly_app_name("IPM.SkypeTeams.Message.Copilot.BizChat"),
            "Microsoft 365 Copilot Chat"
        );
        assert_eq!(
            get_friendly_app_name("IPM.SkypeTeams.Message.Copilot.Teams"),
            "Copilot in Teams"
        );
        assert_eq!(
            get_friendly_app_name("IPM.SkypeTeams.Message.Copilot.Word"),
            "Copilot in Word"
        );
        assert_eq!(
            get_friendly_app_name("IPM.SkypeTeams.Message.Copilot.Excel"),
            "Copilot in Excel"
        );
        assert_eq!(
            get_friendly_app_name("IPM.SkypeTeams.Message.Copilot.PowerPoint"),
            "Copilot in PowerPoint"
        );
        assert_eq!(
            get_friendly_app_name("IPM.SkypeTeams.Message.Copilot.Outlook"),
            "Copilot in Outlook"
        );
        assert_eq!(get_friendly_app_name("unknown"), "Microsoft 365 Copilot");
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = "not valid json";
        let result = parse_m365_copilot_export(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_messages_sorted_by_timestamp() {
        let json = r#"[
            {
                "id": "3",
                "sessionId": "session-1",
                "interactionType": "aiResponse",
                "createdDateTime": "2024-11-15T10:00:02Z",
                "body": { "content": "Response" }
            },
            {
                "id": "1",
                "sessionId": "session-1",
                "interactionType": "userPrompt",
                "createdDateTime": "2024-11-15T10:00:00Z",
                "body": { "content": "Question" }
            }
        ]"#;

        let result = parse_m365_copilot_export(json).unwrap();
        assert_eq!(result.len(), 1);

        let conv = &result[0];
        assert_eq!(conv.messages.len(), 2);
        // Messages should be sorted by timestamp
        assert_eq!(conv.messages[0].content, "Question");
        assert_eq!(conv.messages[1].content, "Response");
    }

    #[test]
    fn test_conversation_title_generation() {
        let json = r#"[
            {
                "id": "1",
                "sessionId": "session-teams",
                "interactionType": "userPrompt",
                "appClass": "IPM.SkypeTeams.Message.Copilot.Teams",
                "createdDateTime": "2024-11-15T14:30:00Z",
                "body": { "content": "Summarize the meeting" }
            }
        ]"#;

        let result = parse_m365_copilot_export(json).unwrap();
        assert_eq!(result.len(), 1);

        let title = result[0].title.as_ref().unwrap();
        assert!(title.contains("Copilot in Teams"));
        assert!(title.contains("2024-11-15"));
    }
}

// ============================================================================
// Extended Cloud Provider Tests
// ============================================================================

mod extended_cloud_provider_tests {
    use chasm::providers::cloud::anthropic::AnthropicProvider;
    use chasm::providers::cloud::chatgpt::ChatGPTProvider;
    use chasm::providers::cloud::common::CloudProvider;
    use chasm::providers::cloud::deepseek::DeepSeekProvider;
    use chasm::providers::cloud::gemini::GeminiProvider;
    use chasm::providers::cloud::m365copilot::M365CopilotProvider;
    use chasm::providers::cloud::perplexity::PerplexityProvider;

    // ChatGPT Extended Tests
    #[test]
    fn test_chatgpt_name_is_chatgpt() {
        let provider = ChatGPTProvider::new(Some("key".to_string()));
        assert_eq!(provider.name(), "ChatGPT");
    }

    #[test]
    fn test_chatgpt_api_key_env_var() {
        let provider = ChatGPTProvider::new(None);
        assert_eq!(provider.api_key_env_var(), "OPENAI_API_KEY");
    }

    // Anthropic Extended Tests
    #[test]
    fn test_anthropic_name_is_claude() {
        let provider = AnthropicProvider::new(Some("key".to_string()));
        assert_eq!(provider.name(), "Claude");
    }

    #[test]
    fn test_anthropic_api_key_env_var() {
        let provider = AnthropicProvider::new(None);
        assert_eq!(provider.api_key_env_var(), "ANTHROPIC_API_KEY");
    }

    // Perplexity Extended Tests
    #[test]
    fn test_perplexity_name_is_perplexity() {
        let provider = PerplexityProvider::new(Some("key".to_string()));
        assert_eq!(provider.name(), "Perplexity");
    }

    #[test]
    fn test_perplexity_api_key_env_var() {
        let provider = PerplexityProvider::new(None);
        assert_eq!(provider.api_key_env_var(), "PERPLEXITY_API_KEY");
    }

    // DeepSeek Extended Tests
    #[test]
    fn test_deepseek_name_is_deepseek() {
        let provider = DeepSeekProvider::new(Some("key".to_string()));
        assert_eq!(provider.name(), "DeepSeek");
    }

    #[test]
    fn test_deepseek_api_key_env_var() {
        let provider = DeepSeekProvider::new(None);
        assert_eq!(provider.api_key_env_var(), "DEEPSEEK_API_KEY");
    }

    // Gemini Extended Tests
    #[test]
    fn test_gemini_name_is_gemini() {
        let provider = GeminiProvider::new(Some("key".to_string()));
        assert_eq!(provider.name(), "Gemini");
    }

    #[test]
    fn test_gemini_api_key_env_var() {
        let provider = GeminiProvider::new(None);
        assert_eq!(provider.api_key_env_var(), "GOOGLE_API_KEY");
    }

    // M365 Copilot Extended Tests
    #[test]
    fn test_m365_name_is_microsoft_365_copilot() {
        let provider = M365CopilotProvider::new(Some("token".to_string()));
        assert_eq!(provider.name(), "Microsoft 365 Copilot");
    }

    #[test]
    fn test_m365_api_key_env_var() {
        let provider = M365CopilotProvider::new(None);
        assert_eq!(provider.api_key_env_var(), "M365_COPILOT_ACCESS_TOKEN");
    }
}

// ============================================================================
// Cloud Conversation Edge Cases
// ============================================================================

mod cloud_conversation_edge_cases {
    use chasm::providers::cloud::common::{CloudConversation, CloudMessage};
    use chrono::Utc;

    #[test]
    fn test_conversation_with_single_message() {
        let conv = CloudConversation {
            id: "single-msg".to_string(),
            title: Some("Single Message".to_string()),
            created_at: Utc::now(),
            updated_at: None,
            model: None,
            messages: vec![CloudMessage {
                id: None,
                role: "user".to_string(),
                content: "Only user message".to_string(),
                timestamp: None,
                model: None,
            }],
            metadata: None,
        };

        let session = conv.to_chat_session("TestProvider");
        // A user message without response still creates a request (with no response)
        assert_eq!(session.requests.len(), 1);
        assert!(session.requests[0].response.is_none());
    }

    #[test]
    fn test_conversation_with_metadata() {
        let metadata = serde_json::json!({
            "source": "export",
            "version": "1.0",
            "custom_field": "custom_value"
        });

        let conv = CloudConversation {
            id: "with-meta".to_string(),
            title: None,
            created_at: Utc::now(),
            updated_at: None,
            model: None,
            messages: vec![],
            metadata: Some(metadata),
        };

        assert!(conv.metadata.is_some());
        let meta = conv.metadata.as_ref().unwrap();
        assert_eq!(meta["source"], "export");
    }

    #[test]
    fn test_conversation_multiple_models() {
        let conv = CloudConversation {
            id: "multi-model".to_string(),
            title: Some("Multi Model Chat".to_string()),
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
            model: Some("gpt-4".to_string()),
            messages: vec![
                CloudMessage {
                    id: Some("1".to_string()),
                    role: "user".to_string(),
                    content: "First question".to_string(),
                    timestamp: Some(Utc::now()),
                    model: None,
                },
                CloudMessage {
                    id: Some("2".to_string()),
                    role: "assistant".to_string(),
                    content: "Response from GPT-4".to_string(),
                    timestamp: Some(Utc::now()),
                    model: Some("gpt-4".to_string()),
                },
                CloudMessage {
                    id: Some("3".to_string()),
                    role: "user".to_string(),
                    content: "Follow up".to_string(),
                    timestamp: Some(Utc::now()),
                    model: None,
                },
                CloudMessage {
                    id: Some("4".to_string()),
                    role: "assistant".to_string(),
                    content: "Response from Claude".to_string(),
                    timestamp: Some(Utc::now()),
                    model: Some("claude-3-opus".to_string()),
                },
            ],
            metadata: None,
        };

        let session = conv.to_chat_session("TestProvider");
        assert_eq!(session.requests.len(), 2);
    }

    #[test]
    fn test_conversation_with_unicode() {
        let conv = CloudConversation {
            id: "unicode-conv".to_string(),
            title: Some("Japanese Chat".to_string()),
            created_at: Utc::now(),
            updated_at: None,
            model: None,
            messages: vec![
                CloudMessage {
                    id: None,
                    role: "user".to_string(),
                    content: "Hello!".to_string(),
                    timestamp: None,
                    model: None,
                },
                CloudMessage {
                    id: None,
                    role: "assistant".to_string(),
                    content: "Hello! How can I help?".to_string(),
                    timestamp: None,
                    model: None,
                },
            ],
            metadata: None,
        };

        let session = conv.to_chat_session("TestProvider");
        assert!(session.custom_title.as_ref().unwrap().contains("Japanese"));
    }

    #[test]
    fn test_conversation_long_content() {
        let long_content = "x".repeat(100000);
        let conv = CloudConversation {
            id: "long-content".to_string(),
            title: None,
            created_at: Utc::now(),
            updated_at: None,
            model: None,
            messages: vec![
                CloudMessage {
                    id: None,
                    role: "user".to_string(),
                    content: long_content.clone(),
                    timestamp: None,
                    model: None,
                },
                CloudMessage {
                    id: None,
                    role: "assistant".to_string(),
                    content: "Short response".to_string(),
                    timestamp: None,
                    model: None,
                },
            ],
            metadata: None,
        };

        let session = conv.to_chat_session("TestProvider");
        assert!(!session.requests.is_empty());
    }

    #[test]
    fn test_cloud_message_all_fields() {
        let msg = CloudMessage {
            id: Some("msg-123".to_string()),
            role: "assistant".to_string(),
            content: "Full message".to_string(),
            timestamp: Some(Utc::now()),
            model: Some("gpt-4-turbo".to_string()),
        };

        assert_eq!(msg.id, Some("msg-123".to_string()));
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "Full message");
        assert!(msg.timestamp.is_some());
        assert_eq!(msg.model, Some("gpt-4-turbo".to_string()));
    }
}

// ============================================================================
// Provider Type Method Tests
// ============================================================================

mod provider_type_methods_tests {
    use chasm::providers::config::ProviderType;

    #[test]
    fn test_all_cloud_providers_have_endpoints() {
        let cloud_types = vec![
            ProviderType::ChatGPT,
            ProviderType::OpenAI,
            ProviderType::Anthropic,
            ProviderType::Perplexity,
            ProviderType::DeepSeek,
            ProviderType::Qwen,
            ProviderType::Gemini,
            ProviderType::Mistral,
            ProviderType::Cohere,
            ProviderType::Grok,
            ProviderType::Groq,
            ProviderType::Together,
            ProviderType::Fireworks,
            ProviderType::Replicate,
            ProviderType::HuggingFace,
            ProviderType::M365Copilot,
        ];

        for pt in cloud_types {
            assert!(pt.is_cloud_provider(), "{:?} should be cloud provider", pt);
            assert!(
                pt.default_endpoint().is_some(),
                "{:?} should have endpoint",
                pt
            );
            assert!(pt.requires_api_key(), "{:?} should require API key", pt);
        }
    }

    #[test]
    fn test_all_local_providers_characteristics() {
        let local_types = vec![
            ProviderType::Ollama,
            ProviderType::Vllm,
            ProviderType::LmStudio,
            ProviderType::LocalAI,
            ProviderType::TextGenWebUI,
            ProviderType::Jan,
            ProviderType::Gpt4All,
            ProviderType::Llamafile,
        ];

        for pt in local_types {
            assert!(
                !pt.is_cloud_provider(),
                "{:?} should not be cloud provider",
                pt
            );
            assert!(
                pt.is_openai_compatible(),
                "{:?} should be OpenAI compatible",
                pt
            );
            assert!(
                !pt.requires_api_key(),
                "{:?} should not require API key",
                pt
            );
        }
    }

    #[test]
    fn test_file_storage_providers() {
        assert!(ProviderType::Copilot.uses_file_storage());
        assert!(ProviderType::Cursor.uses_file_storage());

        // API providers don't use file storage
        assert!(!ProviderType::Ollama.uses_file_storage());
        assert!(!ProviderType::ChatGPT.uses_file_storage());
    }
}

// ============================================================================
// Stress Tests for Cloud Providers
// ============================================================================

mod cloud_stress_tests {
    use chasm::providers::cloud::common::{CloudConversation, CloudMessage};
    use chrono::Utc;

    #[test]
    fn test_many_conversations() {
        let mut conversations = Vec::new();

        for i in 0..100 {
            conversations.push(CloudConversation {
                id: format!("conv-{}", i),
                title: Some(format!("Conversation {}", i)),
                created_at: Utc::now(),
                updated_at: None,
                model: Some("gpt-4".to_string()),
                messages: vec![
                    CloudMessage {
                        id: Some(format!("msg-{}-user", i)),
                        role: "user".to_string(),
                        content: format!("Question {}", i),
                        timestamp: None,
                        model: None,
                    },
                    CloudMessage {
                        id: Some(format!("msg-{}-assistant", i)),
                        role: "assistant".to_string(),
                        content: format!("Answer {}", i),
                        timestamp: None,
                        model: None,
                    },
                ],
                metadata: None,
            });
        }

        // Convert all to chat sessions
        for conv in conversations {
            let session = conv.to_chat_session("TestProvider");
            assert!(session.session_id.is_some());
        }
    }

    #[test]
    fn test_conversation_with_many_messages() {
        let mut messages = Vec::new();

        for i in 0..500 {
            messages.push(CloudMessage {
                id: Some(format!("{}", i)),
                role: if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
                content: format!("Message content {}", i),
                timestamp: Some(Utc::now()),
                model: None,
            });
        }

        let conv = CloudConversation {
            id: "many-messages".to_string(),
            title: Some("Long Conversation".to_string()),
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
            model: None,
            messages,
            metadata: None,
        };

        let session = conv.to_chat_session("TestProvider");
        assert_eq!(session.requests.len(), 250); // 500 messages = 250 pairs
    }
}
