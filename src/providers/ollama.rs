// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Ollama provider for local LLM inference

#![allow(dead_code)]

use super::{ChatProvider, ProviderType};
use crate::models::{ChatMessage, ChatRequest, ChatSession};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Ollama API provider
///
/// Ollama runs local LLMs and provides an API at http://localhost:11434
/// It can also be configured to save conversation history.
pub struct OllamaProvider {
    /// API endpoint URL
    endpoint: String,
    /// Whether Ollama is available
    available: bool,
    /// Path to Ollama's data directory (for history)
    data_path: Option<PathBuf>,
}

/// Ollama API response for listing models
#[derive(Debug, Deserialize)]
struct OllamaModelsResponse {
    models: Vec<OllamaModel>,
}

/// Ollama model info
#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
    modified_at: Option<String>,
    size: Option<u64>,
}

/// Ollama chat message format
#[derive(Debug, Serialize, Deserialize)]
struct OllamaChatMessage {
    role: String,
    content: String,
}

/// Ollama chat request
#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaChatMessage>,
    stream: bool,
}

/// Ollama chat response
#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaChatMessage,
    done: bool,
}

impl OllamaProvider {
    /// Discover Ollama installation and create provider
    pub fn discover() -> Option<Self> {
        let endpoint =
            std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());

        let data_path = Self::find_ollama_data();

        // Check if Ollama is running
        let available = Self::check_availability(&endpoint);

        Some(Self {
            endpoint,
            available,
            data_path,
        })
    }

    /// Find Ollama's data directory
    fn find_ollama_data() -> Option<PathBuf> {
        // Check OLLAMA_MODELS environment variable first
        if let Ok(models_path) = std::env::var("OLLAMA_MODELS") {
            return Some(PathBuf::from(models_path));
        }

        #[cfg(target_os = "windows")]
        {
            let home = dirs::home_dir()?;
            let path = home.join(".ollama");
            if path.exists() {
                return Some(path);
            }
        }

        #[cfg(target_os = "macos")]
        {
            let home = dirs::home_dir()?;
            let path = home.join(".ollama");
            if path.exists() {
                return Some(path);
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Check XDG data dir first
            if let Some(data_dir) = dirs::data_dir() {
                let path = data_dir.join("ollama");
                if path.exists() {
                    return Some(path);
                }
            }
            // Fall back to home directory
            let home = dirs::home_dir()?;
            let path = home.join(".ollama");
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    /// Check if Ollama API is available
    fn check_availability(endpoint: &str) -> bool {
        // Try to connect to Ollama API
        // We use a simple blocking check here
        let _url = format!("{}/api/tags", endpoint);

        // Use ureq for simple HTTP requests (add to Cargo.toml if needed)
        // For now, we'll just check if the endpoint looks valid
        // and assume it's available if configured
        !endpoint.is_empty()
    }

    /// List available models from Ollama
    pub fn list_models(&self) -> Result<Vec<String>> {
        if !self.available {
            return Ok(Vec::new());
        }

        // This would make an HTTP request to /api/tags
        // For now, return empty list - implement with reqwest/ureq later
        Ok(Vec::new())
    }

    /// Convert Ollama chat history to CSM session format
    fn convert_to_session(&self, messages: Vec<OllamaChatMessage>, model: &str) -> ChatSession {
        let now = chrono::Utc::now().timestamp_millis();
        let session_id = uuid::Uuid::new_v4().to_string();

        let mut requests = Vec::new();
        let mut user_msg: Option<String> = None;

        for msg in messages {
            match msg.role.as_str() {
                "user" => {
                    user_msg = Some(msg.content);
                }
                "assistant" => {
                    if let Some(user_text) = user_msg.take() {
                        requests.push(ChatRequest {
                            timestamp: Some(now),
                            message: Some(ChatMessage {
                                text: Some(user_text),
                                parts: None,
                            }),
                            response: Some(serde_json::json!({
                                "value": [{"value": msg.content}]
                            })),
                            variable_data: None,
                            request_id: Some(uuid::Uuid::new_v4().to_string()),
                            response_id: Some(uuid::Uuid::new_v4().to_string()),
                            model_id: Some(format!("ollama/{}", model)),
                            agent: None,
                            result: None,
                            followups: None,
                            is_canceled: Some(false),
                            content_references: None,
                            code_citations: None,
                            response_markdown_info: None,
                            source_session: None,
                        });
                    }
                }
                _ => {}
            }
        }

        ChatSession {
            version: 3,
            session_id: Some(session_id),
            creation_date: now,
            last_message_date: now,
            is_imported: true,
            initial_location: "ollama".to_string(),
            custom_title: Some(format!("Ollama Chat ({})", model)),
            requester_username: Some("user".to_string()),
            requester_avatar_icon_uri: None,
            responder_username: Some(format!("Ollama/{}", model)),
            responder_avatar_icon_uri: None,
            requests,
        }
    }
}

impl ChatProvider for OllamaProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Ollama
    }

    fn name(&self) -> &str {
        "Ollama"
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn sessions_path(&self) -> Option<PathBuf> {
        self.data_path.clone()
    }

    fn list_sessions(&self) -> Result<Vec<ChatSession>> {
        // Ollama doesn't persist chat history by default
        // This would need integration with Ollama's history feature
        // or a custom persistence layer
        Ok(Vec::new())
    }

    fn import_session(&self, _session_id: &str) -> Result<ChatSession> {
        anyhow::bail!("Ollama does not persist chat sessions by default")
    }

    fn export_session(&self, _session: &ChatSession) -> Result<()> {
        // Could implement by sending messages to Ollama to recreate context
        anyhow::bail!("Export to Ollama not yet implemented")
    }
}

/// Create an Ollama chat session from a conversation
pub fn create_ollama_session(
    messages: Vec<(String, String)>, // (user_msg, assistant_msg) pairs
    model: &str,
) -> ChatSession {
    let provider = OllamaProvider {
        endpoint: String::new(),
        available: false,
        data_path: None,
    };

    let ollama_messages: Vec<OllamaChatMessage> = messages
        .into_iter()
        .flat_map(|(user, assistant)| {
            vec![
                OllamaChatMessage {
                    role: "user".to_string(),
                    content: user,
                },
                OllamaChatMessage {
                    role: "assistant".to_string(),
                    content: assistant,
                },
            ]
        })
        .collect();

    provider.convert_to_session(ollama_messages, model)
}
