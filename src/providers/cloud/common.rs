// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Common types and traits for cloud providers

use crate::models::{ChatMessage, ChatRequest, ChatSession};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Options for fetching conversations from cloud providers
#[derive(Debug, Clone, Default)]
pub struct FetchOptions {
    /// Maximum number of conversations to fetch
    pub limit: Option<usize>,
    /// Fetch only conversations after this date
    pub after: Option<DateTime<Utc>>,
    /// Fetch only conversations before this date
    pub before: Option<DateTime<Utc>>,
    /// Include archived conversations
    pub include_archived: bool,
    /// Session token for web API authentication (alternative to API key)
    pub session_token: Option<String>,
}

/// Represents a conversation from a cloud provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConversation {
    /// Unique identifier from the cloud provider
    pub id: String,
    /// Title of the conversation
    pub title: Option<String>,
    /// When the conversation was created
    pub created_at: DateTime<Utc>,
    /// When the conversation was last updated
    pub updated_at: Option<DateTime<Utc>>,
    /// Model used in the conversation
    pub model: Option<String>,
    /// Messages in the conversation
    pub messages: Vec<CloudMessage>,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Represents a message in a cloud conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudMessage {
    /// Unique identifier
    pub id: Option<String>,
    /// Role: user, assistant, system
    pub role: String,
    /// Message content
    pub content: String,
    /// Timestamp
    pub timestamp: Option<DateTime<Utc>>,
    /// Model that generated this message (for assistant messages)
    pub model: Option<String>,
}

impl CloudConversation {
    /// Convert to a ChatSession for import (VS Code format)
    pub fn to_chat_session(&self, provider_name: &str) -> ChatSession {
        use uuid::Uuid;

        // Generate requests from message pairs
        let mut requests = Vec::new();
        let mut i = 0;
        while i < self.messages.len() {
            let msg = &self.messages[i];

            if msg.role == "user" {
                // Create a request with the user message
                let mut request = ChatRequest {
                    timestamp: msg.timestamp.map(|t| t.timestamp_millis()),
                    message: Some(ChatMessage {
                        text: Some(msg.content.clone()),
                        parts: None,
                    }),
                    response: None,
                    variable_data: None,
                    request_id: Some(msg.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string())),
                    response_id: None,
                    model_id: self.model.clone(),
                    agent: None,
                    result: None,
                    followups: None,
                    is_canceled: Some(false),
                    content_references: None,
                    code_citations: None,
                    response_markdown_info: None,
                    source_session: Some(format!("{}:{}", provider_name, self.id)),
                };

                // Check if next message is assistant response
                if i + 1 < self.messages.len() && self.messages[i + 1].role == "assistant" {
                    let assistant_msg = &self.messages[i + 1];
                    request.response = Some(serde_json::json!({
                        "text": assistant_msg.content,
                        "result": {
                            "metadata": {
                                "provider": provider_name,
                                "model": assistant_msg.model.clone().or_else(|| self.model.clone())
                            }
                        }
                    }));
                    request.response_id = Some(
                        assistant_msg
                            .id
                            .clone()
                            .unwrap_or_else(|| Uuid::new_v4().to_string()),
                    );
                    i += 1;
                }

                requests.push(request);
            } else if msg.role == "system" {
                // Skip system messages or add as metadata
                // Could be added to session metadata if needed
            }
            i += 1;
        }

        ChatSession {
            version: 3,
            session_id: Some(format!("{}:{}", provider_name, self.id)),
            creation_date: self.created_at.timestamp_millis(),
            last_message_date: self
                .updated_at
                .unwrap_or(self.created_at)
                .timestamp_millis(),
            is_imported: true,
            initial_location: "panel".to_string(),
            custom_title: self.title.clone(),
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: Some(provider_name.to_string()),
            responder_avatar_icon_uri: None,
            requests,
        }
    }
}

/// Trait for cloud provider implementations
pub trait CloudProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &'static str;

    /// Get the API base URL
    fn api_base_url(&self) -> &str;

    /// Check if the provider is authenticated
    fn is_authenticated(&self) -> bool;

    /// Set the API key or session token
    fn set_credentials(&mut self, api_key: Option<String>, session_token: Option<String>);

    /// List available conversations
    fn list_conversations(&self, options: &FetchOptions) -> Result<Vec<CloudConversation>>;

    /// Fetch a single conversation by ID
    fn fetch_conversation(&self, id: &str) -> Result<CloudConversation>;

    /// Fetch all conversations (with messages)
    fn fetch_all_conversations(&self, options: &FetchOptions) -> Result<Vec<ChatSession>> {
        let conversations = self.list_conversations(options)?;
        let mut sessions = Vec::new();

        for conv in conversations {
            // If messages are already populated, use them directly
            if !conv.messages.is_empty() {
                sessions.push(conv.to_chat_session(self.name()));
            } else {
                // Otherwise fetch the full conversation
                match self.fetch_conversation(&conv.id) {
                    Ok(full_conv) => sessions.push(full_conv.to_chat_session(self.name())),
                    Err(e) => {
                        eprintln!("Warning: Failed to fetch conversation {}: {}", conv.id, e);
                    }
                }
            }
        }

        Ok(sessions)
    }

    /// Get the environment variable name for the API key
    fn api_key_env_var(&self) -> &'static str;

    /// Attempt to load API key from environment
    fn load_api_key_from_env(&self) -> Option<String> {
        std::env::var(self.api_key_env_var()).ok()
    }
}

/// HTTP client configuration for cloud providers
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    pub timeout_secs: u64,
    pub user_agent: String,
    pub accept_invalid_certs: bool,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            user_agent: format!("csm/{}", env!("CARGO_PKG_VERSION")),
            accept_invalid_certs: false,
        }
    }
}

/// Build a configured HTTP client
pub fn build_http_client(config: &HttpClientConfig) -> Result<reqwest::blocking::Client> {
    use std::time::Duration;

    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs))
        .user_agent(&config.user_agent)
        .danger_accept_invalid_certs(config.accept_invalid_certs)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))
}
