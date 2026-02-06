// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! LM Studio provider for local model inference
//!
//! LM Studio provides a GUI for running local LLMs and exposes
//! an OpenAI-compatible API on port 1234.

#![allow(dead_code)]

use super::{ChatProvider, ProviderType};
use crate::models::{ChatMessage, ChatSession};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// LM Studio provider for local model inference
///
/// LM Studio stores conversation history locally and provides
/// an OpenAI-compatible API typically on port 1234.
pub struct LmStudioProvider {
    /// Path to LM Studio data directory
    data_path: Option<PathBuf>,
    /// API endpoint
    endpoint: String,
    /// Whether LM Studio API is available
    available: bool,
    /// Loaded model name
    model_name: Option<String>,
    /// Path to chat history
    history_path: Option<PathBuf>,
}

/// LM Studio conversation history entry
#[derive(Debug, Deserialize, Serialize)]
struct LmStudioConversation {
    id: String,
    title: Option<String>,
    model: Option<String>,
    created_at: String,
    updated_at: Option<String>,
    messages: Vec<LmStudioMessage>,
}

/// LM Studio message format
#[derive(Debug, Deserialize, Serialize, Clone)]
struct LmStudioMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<String>,
}

/// LM Studio API chat request (OpenAI-compatible)
#[derive(Debug, Serialize)]
struct LmStudioChatRequest {
    model: String,
    messages: Vec<LmStudioMessage>,
    temperature: Option<f32>,
    max_tokens: Option<i32>,
    stream: Option<bool>,
}

/// LM Studio API chat response
#[derive(Debug, Deserialize)]
struct LmStudioChatResponse {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<LmStudioChoice>,
    usage: Option<LmStudioUsage>,
}

#[derive(Debug, Deserialize)]
struct LmStudioChoice {
    index: i32,
    message: LmStudioMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LmStudioUsage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

/// LM Studio model info
#[derive(Debug, Deserialize)]
struct LmStudioModel {
    id: String,
    object: String,
    owned_by: Option<String>,
}

impl LmStudioProvider {
    /// Discover LM Studio installation
    pub fn discover() -> Option<Self> {
        let data_path = Self::find_lm_studio_data();
        let history_path = data_path.as_ref().and_then(|p| Self::find_history_path(p));
        let endpoint = Self::find_api_endpoint();
        let (available, model_name) = Self::check_api(&endpoint);

        Some(Self {
            data_path,
            endpoint,
            available,
            model_name,
            history_path,
        })
    }

    /// Find LM Studio data directory
    fn find_lm_studio_data() -> Option<PathBuf> {
        // Check environment variable
        if let Ok(path) = std::env::var("LMSTUDIO_PATH") {
            let path = PathBuf::from(path);
            if path.exists() {
                return Some(path);
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(home) = dirs::home_dir() {
                let path = home.join(".lmstudio");
                if path.exists() {
                    return Some(path);
                }
            }
            if let Some(app_data) = dirs::data_local_dir() {
                let path = app_data.join("LM Studio");
                if path.exists() {
                    return Some(path);
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Some(home) = dirs::home_dir() {
                let path = home.join(".lmstudio");
                if path.exists() {
                    return Some(path);
                }
                let app_support = home
                    .join("Library")
                    .join("Application Support")
                    .join("LM Studio");
                if app_support.exists() {
                    return Some(app_support);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(home) = dirs::home_dir() {
                let path = home.join(".lmstudio");
                if path.exists() {
                    return Some(path);
                }
            }
        }

        None
    }

    /// Find chat history path
    fn find_history_path(data_path: &PathBuf) -> Option<PathBuf> {
        let history_path = data_path.join("chat-history");
        if history_path.exists() {
            return Some(history_path);
        }

        let alt_path = data_path.join("conversations");
        if alt_path.exists() {
            return Some(alt_path);
        }

        None
    }

    /// Find API endpoint
    fn find_api_endpoint() -> String {
        // Check environment variable
        if let Ok(endpoint) = std::env::var("LMSTUDIO_API") {
            return endpoint;
        }

        // Default endpoint
        "http://localhost:1234/v1".to_string()
    }

    /// Check API availability and get loaded model
    fn check_api(endpoint: &str) -> (bool, Option<String>) {
        let url = format!("{}/models", endpoint);
        match ureq::get(&url)
            .timeout(std::time::Duration::from_secs(2))
            .call()
        {
            Ok(response) if response.status() == 200 => {
                if let Ok(json) = response.into_json::<serde_json::Value>() {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        let model = data
                            .first()
                            .and_then(|m| m.get("id"))
                            .and_then(|id| id.as_str())
                            .map(String::from);
                        return (true, model);
                    }
                }
                (true, None)
            }
            _ => (false, None),
        }
    }

    /// List available models
    pub fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/models", self.endpoint);
        let response: serde_json::Value = ureq::get(&url).call()?.into_json()?;

        if let Some(data) = response.get("data").and_then(|d| d.as_array()) {
            return Ok(data
                .iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()))
                .map(String::from)
                .collect());
        }

        Ok(Vec::new())
    }

    /// Load conversations from history
    fn load_conversations(&self) -> Result<Vec<LmStudioConversation>> {
        let history_path = self
            .history_path
            .as_ref()
            .ok_or_else(|| anyhow!("History path not found"))?;

        let mut conversations = Vec::new();

        for entry in std::fs::read_dir(history_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let content = std::fs::read_to_string(&path)?;
                if let Ok(conv) = serde_json::from_str::<LmStudioConversation>(&content) {
                    conversations.push(conv);
                }
            }
        }

        // Sort by updated_at/created_at descending
        conversations.sort_by(|a, b| {
            let a_time = a.updated_at.as_ref().unwrap_or(&a.created_at);
            let b_time = b.updated_at.as_ref().unwrap_or(&b.created_at);
            b_time.cmp(a_time)
        });

        Ok(conversations)
    }

    /// Convert LM Studio conversation to ChatSession
    fn convert_to_session(&self, conv: &LmStudioConversation) -> Result<ChatSession> {
        let chat_messages: Vec<ChatMessage> = conv
            .messages
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                let timestamp = msg
                    .timestamp
                    .as_ref()
                    .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                ChatMessage {
                    id: Some(format!("{}_{}", conv.id, i)),
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                    timestamp,
                    metadata: None,
                }
            })
            .collect();

        let created_at = DateTime::parse_from_rfc3339(&conv.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = conv
            .updated_at
            .as_ref()
            .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or(created_at);

        Ok(ChatSession {
            id: conv.id.clone(),
            title: conv
                .title
                .clone()
                .unwrap_or_else(|| "LM Studio Chat".to_string()),
            provider: "lmstudio".to_string(),
            model: conv.model.clone(),
            messages: chat_messages,
            created_at,
            updated_at,
            workspace_id: None,
            metadata: None,
            tags: Vec::new(),
        })
    }

    /// Send a chat message
    pub fn chat(&self, model: &str, messages: &[ChatMessage]) -> Result<String> {
        if !self.available {
            return Err(anyhow!("LM Studio API not available"));
        }

        let api_messages: Vec<LmStudioMessage> = messages
            .iter()
            .map(|m| LmStudioMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                timestamp: None,
            })
            .collect();

        let request = LmStudioChatRequest {
            model: model.to_string(),
            messages: api_messages,
            temperature: Some(0.7),
            max_tokens: Some(2048),
            stream: Some(false),
        };

        let url = format!("{}/chat/completions", self.endpoint);
        let response: LmStudioChatResponse = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(&request)?
            .into_json()?;

        response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow!("No response from LM Studio"))
    }
}

impl ChatProvider for LmStudioProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::LmStudio
    }

    fn name(&self) -> &str {
        "LM Studio"
    }

    fn is_available(&self) -> bool {
        self.available || self.history_path.is_some()
    }

    fn sessions_path(&self) -> Option<PathBuf> {
        self.history_path.clone()
    }

    fn list_sessions(&self) -> Result<Vec<ChatSession>> {
        if self.history_path.is_none() {
            return Ok(Vec::new());
        }

        let conversations = self.load_conversations()?;
        conversations
            .iter()
            .map(|conv| self.convert_to_session(conv))
            .collect()
    }

    fn import_session(&self, session_id: &str) -> Result<ChatSession> {
        let conversations = self.load_conversations()?;

        let conv = conversations
            .iter()
            .find(|c| c.id == session_id)
            .ok_or_else(|| anyhow!("Conversation not found: {}", session_id))?;

        self.convert_to_session(conv)
    }

    fn export_session(&self, session: &ChatSession) -> Result<()> {
        let history_path = self
            .history_path
            .as_ref()
            .ok_or_else(|| anyhow!("History path not found"))?;

        let conv = LmStudioConversation {
            id: session.id.clone(),
            title: Some(session.title.clone()),
            model: session.model.clone(),
            created_at: session.created_at.to_rfc3339(),
            updated_at: Some(session.updated_at.to_rfc3339()),
            messages: session
                .messages
                .iter()
                .map(|m| LmStudioMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                    timestamp: m.timestamp.map(|t| t.to_rfc3339()),
                })
                .collect(),
        };

        let file_path = history_path.join(format!("{}.json", session.id));
        let json = serde_json::to_string_pretty(&conv)?;
        std::fs::write(file_path, json)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover() {
        let provider = LmStudioProvider::discover();
        println!("LM Studio discovered: {:?}", provider.is_some());
    }
}
