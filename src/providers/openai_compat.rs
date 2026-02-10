// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! OpenAI-compatible provider support
//!
//! Supports servers that implement the OpenAI Chat Completions API:
//! - vLLM
//! - LM Studio
//! - LocalAI
//! - Text Generation WebUI
//! - Jan.ai
//! - GPT4All
//! - Llamafile
//! - Azure AI Foundry (Foundry Local)
//! - Any custom OpenAI-compatible endpoint

#![allow(dead_code)]

use super::{ChatProvider, ProviderType};
use crate::models::{ChatMessage, ChatRequest, ChatSession};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// OpenAI-compatible API provider
pub struct OpenAICompatProvider {
    /// Provider type
    provider_type: ProviderType,
    /// Display name
    name: String,
    /// API endpoint URL
    endpoint: String,
    /// API key (if required)
    api_key: Option<String>,
    /// Default model
    model: Option<String>,
    /// Whether the endpoint is available
    available: bool,
    /// Local data path (if any)
    data_path: Option<PathBuf>,
}

/// OpenAI chat message format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatMessage {
    pub role: String,
    pub content: String,
}

/// OpenAI chat completion request
#[derive(Debug, Serialize)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<OpenAIChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// OpenAI chat completion response
#[derive(Debug, Deserialize)]
pub struct OpenAIChatResponse {
    pub id: String,
    pub choices: Vec<OpenAIChatChoice>,
    #[allow(dead_code)]
    pub model: String,
}

/// OpenAI chat completion choice
#[derive(Debug, Deserialize)]
pub struct OpenAIChatChoice {
    pub message: OpenAIChatMessage,
    #[allow(dead_code)]
    pub finish_reason: Option<String>,
}

impl OpenAICompatProvider {
    /// Create a new OpenAI-compatible provider
    pub fn new(
        provider_type: ProviderType,
        name: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Self {
        let endpoint = endpoint.into();
        Self {
            provider_type,
            name: name.into(),
            endpoint: endpoint.clone(),
            api_key: None,
            model: None,
            available: Self::check_availability(&endpoint),
            data_path: None,
        }
    }

    /// Set API key
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set default model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set local data path
    pub fn with_data_path(mut self, path: PathBuf) -> Self {
        self.data_path = Some(path);
        self
    }

    /// Check if the endpoint is available
    fn check_availability(endpoint: &str) -> bool {
        // Basic check - would use HTTP client in production
        !endpoint.is_empty()
    }

    /// Convert CSM session to OpenAI message format
    pub fn session_to_messages(session: &ChatSession) -> Vec<OpenAIChatMessage> {
        let mut messages = Vec::new();

        for request in &session.requests {
            // Add user message
            if let Some(msg) = &request.message {
                if let Some(text) = &msg.text {
                    messages.push(OpenAIChatMessage {
                        role: "user".to_string(),
                        content: text.clone(),
                    });
                }
            }

            // Add assistant response
            if let Some(response) = &request.response {
                if let Some(text) = extract_response_text(response) {
                    messages.push(OpenAIChatMessage {
                        role: "assistant".to_string(),
                        content: text,
                    });
                }
            }
        }

        messages
    }

    /// Convert OpenAI messages to CSM session
    pub fn messages_to_session(
        messages: Vec<OpenAIChatMessage>,
        model: &str,
        provider_name: &str,
    ) -> ChatSession {
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
                            model_id: Some(model.to_string()),
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
                "system" => {
                    // System messages could be stored as metadata
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
            initial_location: "api".to_string(),
            custom_title: Some(format!("{} Chat", provider_name)),
            requester_username: Some("user".to_string()),
            requester_avatar_icon_uri: None,
            responder_username: Some(format!("{}/{}", provider_name, model)),
            responder_avatar_icon_uri: None,
            requests,
        }
    }
}

impl ChatProvider for OpenAICompatProvider {
    fn provider_type(&self) -> ProviderType {
        self.provider_type
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn sessions_path(&self) -> Option<PathBuf> {
        self.data_path.clone()
    }

    fn list_sessions(&self) -> Result<Vec<ChatSession>> {
        // OpenAI-compatible APIs don't persist sessions
        // This would need a local history storage layer
        Ok(Vec::new())
    }

    fn import_session(&self, _session_id: &str) -> Result<ChatSession> {
        anyhow::bail!("{} does not persist chat sessions", self.name)
    }

    fn export_session(&self, _session: &ChatSession) -> Result<()> {
        // Could implement by sending messages to recreate context
        anyhow::bail!("Export to {} not yet implemented", self.name)
    }
}

/// Discover available OpenAI-compatible providers
pub fn discover_openai_compatible_providers() -> Vec<OpenAICompatProvider> {
    let mut providers = Vec::new();

    // vLLM (default port 8000)
    if let Some(provider) = discover_vllm() {
        providers.push(provider);
    }

    // LM Studio (default port 1234)
    if let Some(provider) = discover_lm_studio() {
        providers.push(provider);
    }

    // LocalAI (default port 8080)
    if let Some(provider) = discover_localai() {
        providers.push(provider);
    }

    // Text Generation WebUI (default port 5000)
    if let Some(provider) = discover_text_gen_webui() {
        providers.push(provider);
    }

    // Jan.ai (default port 1337)
    if let Some(provider) = discover_jan() {
        providers.push(provider);
    }

    // GPT4All (default port 4891)
    if let Some(provider) = discover_gpt4all() {
        providers.push(provider);
    }

    // Azure AI Foundry / Foundry Local (default port 5272)
    if let Some(provider) = discover_foundry() {
        providers.push(provider);
    }

    providers
}

fn discover_vllm() -> Option<OpenAICompatProvider> {
    let endpoint =
        std::env::var("VLLM_ENDPOINT").unwrap_or_else(|_| "http://localhost:8000/v1".to_string());

    Some(OpenAICompatProvider::new(
        ProviderType::Vllm,
        "vLLM",
        endpoint,
    ))
}

fn discover_lm_studio() -> Option<OpenAICompatProvider> {
    let endpoint = std::env::var("LM_STUDIO_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:1234/v1".to_string());

    // Check for LM Studio data directory
    let data_path = find_lm_studio_data();

    let mut provider = OpenAICompatProvider::new(ProviderType::LmStudio, "LM Studio", endpoint);

    if let Some(path) = data_path {
        provider = provider.with_data_path(path);
    }

    Some(provider)
}

fn discover_localai() -> Option<OpenAICompatProvider> {
    let endpoint = std::env::var("LOCALAI_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:8080/v1".to_string());

    Some(OpenAICompatProvider::new(
        ProviderType::LocalAI,
        "LocalAI",
        endpoint,
    ))
}

fn discover_text_gen_webui() -> Option<OpenAICompatProvider> {
    let endpoint = std::env::var("TEXT_GEN_WEBUI_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:5000/v1".to_string());

    Some(OpenAICompatProvider::new(
        ProviderType::TextGenWebUI,
        "Text Generation WebUI",
        endpoint,
    ))
}

fn discover_jan() -> Option<OpenAICompatProvider> {
    let endpoint =
        std::env::var("JAN_ENDPOINT").unwrap_or_else(|_| "http://localhost:1337/v1".to_string());

    // Check for Jan data directory
    let data_path = find_jan_data();

    let mut provider = OpenAICompatProvider::new(ProviderType::Jan, "Jan.ai", endpoint);

    if let Some(path) = data_path {
        provider = provider.with_data_path(path);
    }

    Some(provider)
}

fn discover_gpt4all() -> Option<OpenAICompatProvider> {
    let endpoint = std::env::var("GPT4ALL_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4891/v1".to_string());

    // Check for GPT4All data directory
    let data_path = find_gpt4all_data();

    let mut provider = OpenAICompatProvider::new(ProviderType::Gpt4All, "GPT4All", endpoint);

    if let Some(path) = data_path {
        provider = provider.with_data_path(path);
    }

    Some(provider)
}

fn discover_foundry() -> Option<OpenAICompatProvider> {
    // Azure AI Foundry Local / Foundry Local
    let endpoint = std::env::var("FOUNDRY_LOCAL_ENDPOINT")
        .or_else(|_| std::env::var("AI_FOUNDRY_ENDPOINT"))
        .unwrap_or_else(|_| "http://localhost:5272/v1".to_string());

    Some(OpenAICompatProvider::new(
        ProviderType::Foundry,
        "Azure AI Foundry",
        endpoint,
    ))
}

// Helper functions to find application data directories

fn find_lm_studio_data() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let home = dirs::home_dir()?;
        let path = home.join(".cache").join("lm-studio");
        if path.exists() {
            return Some(path);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir()?;
        let path = home.join(".cache").join("lm-studio");
        if path.exists() {
            return Some(path);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(cache_dir) = dirs::cache_dir() {
            let path = cache_dir.join("lm-studio");
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

fn find_jan_data() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let home = dirs::home_dir()?;
        let path = home.join("jan");
        if path.exists() {
            return Some(path);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir()?;
        let path = home.join("jan");
        if path.exists() {
            return Some(path);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir()?;
        let path = home.join("jan");
        if path.exists() {
            return Some(path);
        }
    }

    None
}

fn find_gpt4all_data() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let local_app_data = dirs::data_local_dir()?;
        let path = local_app_data.join("nomic.ai").join("GPT4All");
        if path.exists() {
            return Some(path);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir()?;
        let path = home
            .join("Library")
            .join("Application Support")
            .join("nomic.ai")
            .join("GPT4All");
        if path.exists() {
            return Some(path);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(data_dir) = dirs::data_dir() {
            let path = data_dir.join("nomic.ai").join("GPT4All");
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

/// Extract text from various response formats
fn extract_response_text(response: &serde_json::Value) -> Option<String> {
    // Try direct text field
    if let Some(text) = response.get("text").and_then(|v| v.as_str()) {
        return Some(text.to_string());
    }

    // Try value array format (VS Code Copilot format)
    if let Some(value) = response.get("value").and_then(|v| v.as_array()) {
        let parts: Vec<String> = value
            .iter()
            .filter_map(|v| v.get("value").and_then(|v| v.as_str()))
            .map(String::from)
            .collect();
        if !parts.is_empty() {
            return Some(parts.join("\n"));
        }
    }

    // Try content field (OpenAI format)
    if let Some(content) = response.get("content").and_then(|v| v.as_str()) {
        return Some(content.to_string());
    }

    None
}
