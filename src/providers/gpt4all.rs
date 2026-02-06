// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! GPT4All provider for local LLM inference
//!
//! GPT4All is a free-to-use, locally running, privacy-aware chatbot.
//! It supports many models and stores conversation history locally.

#![allow(dead_code)]

use super::{ChatProvider, ProviderType};
use crate::models::{ChatMessage, ChatSession};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// GPT4All provider for local LLM chat
///
/// GPT4All stores conversations in a local SQLite database and
/// provides a REST API when running.
pub struct Gpt4AllProvider {
    /// Path to GPT4All data directory
    data_path: PathBuf,
    /// API endpoint (when GPT4All API server is running)
    endpoint: Option<String>,
    /// Whether GPT4All is available
    available: bool,
    /// Path to conversation database
    db_path: Option<PathBuf>,
}

/// GPT4All conversation from database
#[derive(Debug, Deserialize, Serialize)]
struct Gpt4AllConversation {
    id: i64,
    name: Option<String>,
    created_at: String,
    updated_at: String,
    model: Option<String>,
}

/// GPT4All message from database
#[derive(Debug, Deserialize, Serialize)]
struct Gpt4AllMessage {
    id: i64,
    conversation_id: i64,
    role: String,
    content: String,
    created_at: String,
    model: Option<String>,
    tokens: Option<i64>,
}

/// GPT4All API chat request
#[derive(Debug, Serialize)]
struct Gpt4AllChatRequest {
    model: String,
    messages: Vec<Gpt4AllApiMessage>,
    temperature: f32,
    max_tokens: i32,
}

/// GPT4All API message format
#[derive(Debug, Serialize, Deserialize)]
struct Gpt4AllApiMessage {
    role: String,
    content: String,
}

/// GPT4All API response
#[derive(Debug, Deserialize)]
struct Gpt4AllChatResponse {
    choices: Vec<Gpt4AllChoice>,
    usage: Option<Gpt4AllUsage>,
}

#[derive(Debug, Deserialize)]
struct Gpt4AllChoice {
    message: Gpt4AllApiMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Gpt4AllUsage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

/// GPT4All model info
#[derive(Debug, Deserialize)]
struct Gpt4AllModel {
    id: String,
    object: String,
    name: Option<String>,
}

impl Gpt4AllProvider {
    /// Discover GPT4All installation and create provider
    pub fn discover() -> Option<Self> {
        let data_path = Self::find_gpt4all_data()?;
        let db_path = Self::find_database(&data_path);
        let endpoint = Self::find_api_endpoint();
        let available = db_path.is_some() || endpoint.is_some();

        Some(Self {
            data_path,
            endpoint,
            available,
            db_path,
        })
    }

    /// Find GPT4All's data directory
    fn find_gpt4all_data() -> Option<PathBuf> {
        // Check environment variable first
        if let Ok(path) = std::env::var("GPT4ALL_DATA") {
            let path = PathBuf::from(path);
            if path.exists() {
                return Some(path);
            }
        }

        // Platform-specific default locations
        #[cfg(target_os = "windows")]
        {
            if let Some(local_app_data) = dirs::data_local_dir() {
                let path = local_app_data.join("nomic.ai").join("GPT4All");
                if path.exists() {
                    return Some(path);
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Some(home) = dirs::home_dir() {
                let path = home
                    .join("Library")
                    .join("Application Support")
                    .join("nomic.ai")
                    .join("GPT4All");
                if path.exists() {
                    return Some(path);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(home) = dirs::home_dir() {
                // Check ~/.local/share/nomic.ai/GPT4All
                let path = home
                    .join(".local")
                    .join("share")
                    .join("nomic.ai")
                    .join("GPT4All");
                if path.exists() {
                    return Some(path);
                }

                // Alternative: ~/.gpt4all
                let alt_path = home.join(".gpt4all");
                if alt_path.exists() {
                    return Some(alt_path);
                }
            }
        }

        None
    }

    /// Find conversation database
    fn find_database(data_path: &PathBuf) -> Option<PathBuf> {
        // GPT4All stores conversations in a SQLite database
        let db_path = data_path.join("chats.db");
        if db_path.exists() {
            return Some(db_path);
        }

        // Alternative location
        let alt_db = data_path.join("conversations.db");
        if alt_db.exists() {
            return Some(alt_db);
        }

        None
    }

    /// Find API endpoint if GPT4All server is running
    fn find_api_endpoint() -> Option<String> {
        // Check environment variable
        if let Ok(endpoint) = std::env::var("GPT4ALL_API") {
            return Some(endpoint);
        }

        // Default API endpoint
        let default_endpoint = "http://localhost:4891/v1";

        // Check if API is responding
        if Self::check_api_availability(default_endpoint) {
            return Some(default_endpoint.to_string());
        }

        None
    }

    /// Check if API endpoint is available
    fn check_api_availability(endpoint: &str) -> bool {
        // Simple connectivity check
        let url = format!("{}/models", endpoint);
        match ureq::get(&url)
            .timeout(std::time::Duration::from_secs(2))
            .call()
        {
            Ok(response) => response.status() == 200,
            Err(_) => false,
        }
    }

    /// List available models
    pub fn list_models(&self) -> Result<Vec<String>> {
        if let Some(ref endpoint) = self.endpoint {
            let url = format!("{}/models", endpoint);
            let response: serde_json::Value = ureq::get(&url).call()?.into_json()?;

            if let Some(data) = response.get("data").and_then(|d| d.as_array()) {
                return Ok(data
                    .iter()
                    .filter_map(|m| m.get("id").and_then(|id| id.as_str()))
                    .map(String::from)
                    .collect());
            }
        }

        // Fall back to scanning models directory
        let models_dir = self.data_path.join("models");
        if models_dir.exists() {
            let models: Vec<String> = std::fs::read_dir(models_dir)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .map(|e| e == "gguf" || e == "bin")
                        .unwrap_or(false)
                })
                .filter_map(|entry| {
                    entry
                        .path()
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                })
                .collect();
            return Ok(models);
        }

        Ok(Vec::new())
    }

    /// Load conversations from database
    fn load_conversations(&self) -> Result<Vec<Gpt4AllConversation>> {
        let db_path = self
            .db_path
            .as_ref()
            .ok_or_else(|| anyhow!("No database found"))?;

        let conn = rusqlite::Connection::open(db_path)?;

        let mut stmt = conn.prepare(
            "SELECT id, name, created_at, updated_at, model FROM conversations ORDER BY updated_at DESC",
        )?;

        let conversations = stmt
            .query_map([], |row| {
                Ok(Gpt4AllConversation {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                    model: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(conversations)
    }

    /// Load messages for a conversation
    fn load_messages(&self, conversation_id: i64) -> Result<Vec<Gpt4AllMessage>> {
        let db_path = self
            .db_path
            .as_ref()
            .ok_or_else(|| anyhow!("No database found"))?;

        let conn = rusqlite::Connection::open(db_path)?;

        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, role, content, created_at, model, tokens 
             FROM messages 
             WHERE conversation_id = ? 
             ORDER BY created_at ASC",
        )?;

        let messages = stmt
            .query_map([conversation_id], |row| {
                Ok(Gpt4AllMessage {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    created_at: row.get(4)?,
                    model: row.get(5)?,
                    tokens: row.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(messages)
    }

    /// Convert GPT4All conversation to ChatSession
    fn convert_to_session(&self, conv: &Gpt4AllConversation) -> Result<ChatSession> {
        let messages = self.load_messages(conv.id)?;

        let chat_messages: Vec<ChatMessage> = messages
            .iter()
            .map(|msg| ChatMessage {
                id: Some(msg.id.to_string()),
                role: msg.role.clone(),
                content: msg.content.clone(),
                timestamp: DateTime::parse_from_rfc3339(&msg.created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok(),
                metadata: None,
            })
            .collect();

        let created_at = DateTime::parse_from_rfc3339(&conv.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = DateTime::parse_from_rfc3339(&conv.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(ChatSession {
            id: conv.id.to_string(),
            title: conv
                .name
                .clone()
                .unwrap_or_else(|| "GPT4All Chat".to_string()),
            provider: "gpt4all".to_string(),
            model: conv.model.clone(),
            messages: chat_messages,
            created_at,
            updated_at,
            workspace_id: None,
            metadata: None,
            tags: Vec::new(),
        })
    }

    /// Send a chat message via API
    pub fn chat(&self, model: &str, messages: &[ChatMessage]) -> Result<String> {
        let endpoint = self
            .endpoint
            .as_ref()
            .ok_or_else(|| anyhow!("GPT4All API not available"))?;

        let api_messages: Vec<Gpt4AllApiMessage> = messages
            .iter()
            .map(|m| Gpt4AllApiMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let request = Gpt4AllChatRequest {
            model: model.to_string(),
            messages: api_messages,
            temperature: 0.7,
            max_tokens: 2048,
        };

        let url = format!("{}/chat/completions", endpoint);
        let response: Gpt4AllChatResponse = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(&request)?
            .into_json()?;

        response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow!("No response from GPT4All"))
    }
}

impl ChatProvider for Gpt4AllProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Gpt4All
    }

    fn name(&self) -> &str {
        "GPT4All"
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn sessions_path(&self) -> Option<PathBuf> {
        Some(self.data_path.clone())
    }

    fn list_sessions(&self) -> Result<Vec<ChatSession>> {
        let conversations = self.load_conversations()?;
        conversations
            .iter()
            .map(|conv| self.convert_to_session(conv))
            .collect()
    }

    fn import_session(&self, session_id: &str) -> Result<ChatSession> {
        let conv_id: i64 = session_id.parse()?;
        let conversations = self.load_conversations()?;

        let conv = conversations
            .iter()
            .find(|c| c.id == conv_id)
            .ok_or_else(|| anyhow!("Conversation not found: {}", session_id))?;

        self.convert_to_session(conv)
    }

    fn export_session(&self, _session: &ChatSession) -> Result<()> {
        // GPT4All doesn't support importing external conversations easily
        Err(anyhow!("Export to GPT4All is not supported"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover() {
        // This test will only pass if GPT4All is installed
        let provider = Gpt4AllProvider::discover();
        // Provider discovery should not panic
        println!("GPT4All discovered: {:?}", provider.is_some());
    }
}
