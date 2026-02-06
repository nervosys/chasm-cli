// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Jan provider for local AI assistant
//!
//! Jan is a free, open-source AI assistant that runs offline.
//! It stores conversations locally and provides an OpenAI-compatible API.

#![allow(dead_code)]

use super::{ChatProvider, ProviderType};
use crate::models::{ChatMessage, ChatSession};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Jan provider for local AI chat
///
/// Jan stores conversations as JSON files in a threads directory
/// and provides an OpenAI-compatible API on port 1337.
pub struct JanProvider {
    /// Path to Jan data directory
    data_path: PathBuf,
    /// API endpoint
    endpoint: Option<String>,
    /// Whether Jan is available
    available: bool,
    /// Path to threads (conversations) directory
    threads_path: Option<PathBuf>,
}

/// Jan thread (conversation) metadata
#[derive(Debug, Deserialize, Serialize)]
struct JanThread {
    id: String,
    object: String,
    title: Option<String>,
    created: i64,
    updated: Option<i64>,
    metadata: Option<serde_json::Value>,
}

/// Jan message format
#[derive(Debug, Deserialize, Serialize)]
struct JanMessage {
    id: String,
    object: String,
    thread_id: String,
    role: String,
    content: Vec<JanMessageContent>,
    created: i64,
    metadata: Option<serde_json::Value>,
}

/// Jan message content block
#[derive(Debug, Deserialize, Serialize)]
struct JanMessageContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<JanTextContent>,
}

#[derive(Debug, Deserialize, Serialize)]
struct JanTextContent {
    value: String,
    annotations: Option<Vec<serde_json::Value>>,
}

/// Jan model info
#[derive(Debug, Deserialize)]
struct JanModel {
    id: String,
    object: String,
    name: Option<String>,
    version: Option<String>,
}

/// Jan API chat request (OpenAI-compatible)
#[derive(Debug, Serialize)]
struct JanChatRequest {
    model: String,
    messages: Vec<JanApiMessage>,
    temperature: Option<f32>,
    max_tokens: Option<i32>,
    stream: Option<bool>,
}

/// Jan API message format
#[derive(Debug, Serialize, Deserialize)]
struct JanApiMessage {
    role: String,
    content: String,
}

/// Jan API chat response
#[derive(Debug, Deserialize)]
struct JanChatResponse {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<JanChoice>,
    usage: Option<JanUsage>,
}

#[derive(Debug, Deserialize)]
struct JanChoice {
    index: i32,
    message: JanApiMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JanUsage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

impl JanProvider {
    /// Discover Jan installation and create provider
    pub fn discover() -> Option<Self> {
        let data_path = Self::find_jan_data()?;
        let threads_path = Self::find_threads_path(&data_path);
        let endpoint = Self::find_api_endpoint();
        let available = threads_path.is_some() || endpoint.is_some();

        Some(Self {
            data_path,
            endpoint,
            available,
            threads_path,
        })
    }

    /// Find Jan's data directory
    fn find_jan_data() -> Option<PathBuf> {
        // Check environment variable first
        if let Ok(path) = std::env::var("JAN_DATA_FOLDER") {
            let path = PathBuf::from(path);
            if path.exists() {
                return Some(path);
            }
        }

        // Platform-specific default locations
        #[cfg(target_os = "windows")]
        {
            if let Some(home) = dirs::home_dir() {
                let path = home.join("jan");
                if path.exists() {
                    return Some(path);
                }
            }
            if let Some(app_data) = dirs::data_local_dir() {
                let path = app_data.join("Jan");
                if path.exists() {
                    return Some(path);
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Some(home) = dirs::home_dir() {
                let path = home.join("jan");
                if path.exists() {
                    return Some(path);
                }
                let app_support = home.join("Library").join("Application Support").join("Jan");
                if app_support.exists() {
                    return Some(app_support);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(home) = dirs::home_dir() {
                let path = home.join("jan");
                if path.exists() {
                    return Some(path);
                }
                let config_path = home.join(".config").join("jan");
                if config_path.exists() {
                    return Some(config_path);
                }
            }
        }

        None
    }

    /// Find threads (conversations) directory
    fn find_threads_path(data_path: &PathBuf) -> Option<PathBuf> {
        let threads_path = data_path.join("threads");
        if threads_path.exists() {
            Some(threads_path)
        } else {
            None
        }
    }

    /// Find API endpoint if Jan server is running
    fn find_api_endpoint() -> Option<String> {
        // Check environment variable
        if let Ok(endpoint) = std::env::var("JAN_API_HOST") {
            return Some(endpoint);
        }

        // Default API endpoint
        let default_endpoint = "http://localhost:1337/v1";

        // Check if API is responding
        if Self::check_api_availability(default_endpoint) {
            return Some(default_endpoint.to_string());
        }

        None
    }

    /// Check if API endpoint is available
    fn check_api_availability(endpoint: &str) -> bool {
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
                .filter(|entry| entry.path().is_dir())
                .filter_map(|entry| {
                    entry
                        .path()
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                })
                .collect();
            return Ok(models);
        }

        Ok(Vec::new())
    }

    /// Load all threads (conversations)
    fn load_threads(&self) -> Result<Vec<JanThread>> {
        let threads_path = self
            .threads_path
            .as_ref()
            .ok_or_else(|| anyhow!("Threads directory not found"))?;

        let mut threads = Vec::new();

        for entry in std::fs::read_dir(threads_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let thread_file = path.join("thread.json");
                if thread_file.exists() {
                    let content = std::fs::read_to_string(&thread_file)?;
                    if let Ok(thread) = serde_json::from_str::<JanThread>(&content) {
                        threads.push(thread);
                    }
                }
            }
        }

        // Sort by updated/created time descending
        threads.sort_by(|a, b| {
            let a_time = a.updated.unwrap_or(a.created);
            let b_time = b.updated.unwrap_or(b.created);
            b_time.cmp(&a_time)
        });

        Ok(threads)
    }

    /// Load messages for a thread
    fn load_messages(&self, thread_id: &str) -> Result<Vec<JanMessage>> {
        let threads_path = self
            .threads_path
            .as_ref()
            .ok_or_else(|| anyhow!("Threads directory not found"))?;

        let messages_file = threads_path.join(thread_id).join("messages.json");
        if !messages_file.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&messages_file)?;
        let messages: Vec<JanMessage> = serde_json::from_str(&content)?;

        Ok(messages)
    }

    /// Convert Jan thread to ChatSession
    fn convert_to_session(&self, thread: &JanThread) -> Result<ChatSession> {
        let messages = self.load_messages(&thread.id)?;

        let chat_messages: Vec<ChatMessage> = messages
            .iter()
            .map(|msg| {
                let content = msg
                    .content
                    .iter()
                    .filter_map(|c| c.text.as_ref().map(|t| t.value.clone()))
                    .collect::<Vec<_>>()
                    .join("\n");

                ChatMessage {
                    id: Some(msg.id.clone()),
                    role: msg.role.clone(),
                    content,
                    timestamp: Some(DateTime::from_timestamp(msg.created, 0).unwrap_or(Utc::now())),
                    metadata: None,
                }
            })
            .collect();

        let created_at = DateTime::from_timestamp(thread.created, 0).unwrap_or(Utc::now());
        let updated_at = thread
            .updated
            .and_then(|ts| DateTime::from_timestamp(ts, 0))
            .unwrap_or(created_at);

        Ok(ChatSession {
            id: thread.id.clone(),
            title: thread
                .title
                .clone()
                .unwrap_or_else(|| "Jan Chat".to_string()),
            provider: "jan".to_string(),
            model: None,
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
            .ok_or_else(|| anyhow!("Jan API not available"))?;

        let api_messages: Vec<JanApiMessage> = messages
            .iter()
            .map(|m| JanApiMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let request = JanChatRequest {
            model: model.to_string(),
            messages: api_messages,
            temperature: Some(0.7),
            max_tokens: Some(2048),
            stream: Some(false),
        };

        let url = format!("{}/chat/completions", endpoint);
        let response: JanChatResponse = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(&request)?
            .into_json()?;

        response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow!("No response from Jan"))
    }
}

impl ChatProvider for JanProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Jan
    }

    fn name(&self) -> &str {
        "Jan"
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn sessions_path(&self) -> Option<PathBuf> {
        self.threads_path.clone()
    }

    fn list_sessions(&self) -> Result<Vec<ChatSession>> {
        let threads = self.load_threads()?;
        threads
            .iter()
            .map(|thread| self.convert_to_session(thread))
            .collect()
    }

    fn import_session(&self, session_id: &str) -> Result<ChatSession> {
        let threads = self.load_threads()?;

        let thread = threads
            .iter()
            .find(|t| t.id == session_id)
            .ok_or_else(|| anyhow!("Thread not found: {}", session_id))?;

        self.convert_to_session(thread)
    }

    fn export_session(&self, session: &ChatSession) -> Result<()> {
        let threads_path = self
            .threads_path
            .as_ref()
            .ok_or_else(|| anyhow!("Threads directory not found"))?;

        let thread_dir = threads_path.join(&session.id);
        std::fs::create_dir_all(&thread_dir)?;

        // Create thread.json
        let thread = JanThread {
            id: session.id.clone(),
            object: "thread".to_string(),
            title: Some(session.title.clone()),
            created: session.created_at.timestamp(),
            updated: Some(session.updated_at.timestamp()),
            metadata: None,
        };
        let thread_json = serde_json::to_string_pretty(&thread)?;
        std::fs::write(thread_dir.join("thread.json"), thread_json)?;

        // Create messages.json
        let jan_messages: Vec<JanMessage> = session
            .messages
            .iter()
            .enumerate()
            .map(|(i, msg)| JanMessage {
                id: msg.id.clone().unwrap_or_else(|| format!("msg_{}", i)),
                object: "thread.message".to_string(),
                thread_id: session.id.clone(),
                role: msg.role.clone(),
                content: vec![JanMessageContent {
                    content_type: "text".to_string(),
                    text: Some(JanTextContent {
                        value: msg.content.clone(),
                        annotations: None,
                    }),
                }],
                created: msg.timestamp.map(|t| t.timestamp()).unwrap_or(0),
                metadata: None,
            })
            .collect();
        let messages_json = serde_json::to_string_pretty(&jan_messages)?;
        std::fs::write(thread_dir.join("messages.json"), messages_json)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover() {
        let provider = JanProvider::discover();
        println!("Jan discovered: {:?}", provider.is_some());
    }
}
