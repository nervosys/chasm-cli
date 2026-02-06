// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Text Generation WebUI provider for local inference
//!
//! Oobabooga's Text Generation WebUI provides a web interface
//! for running local LLMs with multiple backends and an API.

#![allow(dead_code)]

use super::{ChatProvider, ProviderType};
use crate::models::{ChatMessage, ChatSession};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Text Generation WebUI provider
///
/// Text Gen WebUI runs locally and exposes APIs for chat and completion.
/// Supports multiple backends and has character/persona features.
pub struct TextGenWebUiProvider {
    /// API endpoint
    endpoint: String,
    /// OpenAI-compatible endpoint
    openai_endpoint: Option<String>,
    /// Whether API is available
    available: bool,
    /// Data directory path
    data_path: Option<PathBuf>,
    /// Characters directory
    characters_path: Option<PathBuf>,
    /// Chat logs directory
    logs_path: Option<PathBuf>,
}

/// Text Gen WebUI native chat request
#[derive(Debug, Serialize)]
struct TextGenChatRequest {
    user_input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_new_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_max_new_tokens: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    character: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    instruction_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    your_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    regenerate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    continue_: Option<bool>,
    history: TextGenHistory,
}

/// Text Gen WebUI chat history
#[derive(Debug, Serialize, Deserialize, Clone)]
struct TextGenHistory {
    internal: Vec<Vec<String>>,
    visible: Vec<Vec<String>>,
}

/// Text Gen WebUI chat response
#[derive(Debug, Deserialize)]
struct TextGenChatResponse {
    results: Vec<TextGenResult>,
}

#[derive(Debug, Deserialize)]
struct TextGenResult {
    history: TextGenHistory,
}

/// Text Gen WebUI OpenAI-compatible message
#[derive(Debug, Deserialize, Serialize, Clone)]
struct TextGenMessage {
    role: String,
    content: String,
}

/// Text Gen WebUI OpenAI-compatible chat request
#[derive(Debug, Serialize)]
struct TextGenOpenAiRequest {
    messages: Vec<TextGenMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    character: Option<String>,
}

/// Text Gen WebUI OpenAI-compatible response
#[derive(Debug, Deserialize)]
struct TextGenOpenAiResponse {
    id: Option<String>,
    choices: Vec<TextGenOpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct TextGenOpenAiChoice {
    message: TextGenMessage,
    finish_reason: Option<String>,
}

/// Saved chat log format
#[derive(Debug, Deserialize, Serialize)]
struct TextGenChatLog {
    character: Option<String>,
    mode: Option<String>,
    chat: Vec<TextGenChatTurn>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TextGenChatTurn {
    user: String,
    assistant: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<String>,
}

impl TextGenWebUiProvider {
    /// Discover Text Generation WebUI installation
    pub fn discover() -> Option<Self> {
        let data_path = Self::find_data_path();
        let characters_path = data_path
            .as_ref()
            .map(|p| p.join("characters"))
            .filter(|p| p.exists());
        let logs_path = data_path
            .as_ref()
            .map(|p| p.join("logs").join("chat"))
            .filter(|p| p.exists());
        let endpoint = Self::find_api_endpoint();
        let openai_endpoint = Self::find_openai_endpoint();
        let available = Self::check_api(&endpoint);

        Some(Self {
            endpoint,
            openai_endpoint,
            available,
            data_path,
            characters_path,
            logs_path,
        })
    }

    /// Find data directory
    fn find_data_path() -> Option<PathBuf> {
        // Check environment variable
        if let Ok(path) = std::env::var("TEXTGEN_PATH") {
            let path = PathBuf::from(path);
            if path.exists() {
                return Some(path);
            }
        }

        // Common installation paths
        let candidates = [
            dirs::home_dir().map(|h| h.join("text-generation-webui")),
            dirs::home_dir().map(|h| h.join("oobabooga").join("text-generation-webui")),
            dirs::data_dir().map(|d| d.join("text-generation-webui")),
            Some(PathBuf::from("/opt/text-generation-webui")),
        ];

        for candidate in candidates.iter().flatten() {
            if candidate.exists() {
                return Some(candidate.clone());
            }
        }

        None
    }

    /// Find native API endpoint
    fn find_api_endpoint() -> String {
        if let Ok(endpoint) = std::env::var("TEXTGEN_API") {
            return endpoint;
        }
        // Default native API port
        "http://localhost:5000/api".to_string()
    }

    /// Find OpenAI-compatible endpoint
    fn find_openai_endpoint() -> Option<String> {
        if let Ok(endpoint) = std::env::var("TEXTGEN_OPENAI_API") {
            return Some(endpoint);
        }

        // Default OpenAI-compatible endpoint (requires --extensions openai flag)
        // Try common ports
        for port in [5001, 5000, 7860] {
            let url = format!("http://localhost:{}/v1", port);
            if ureq::get(&format!("{}/models", url))
                .timeout(std::time::Duration::from_millis(500))
                .call()
                .is_ok()
            {
                return Some(url);
            }
        }

        None
    }

    /// Check API availability
    fn check_api(endpoint: &str) -> bool {
        // Try native API health check
        let url = format!("{}/v1/model", endpoint.trim_end_matches("/api"));
        if let Ok(response) = ureq::get(&url)
            .timeout(std::time::Duration::from_secs(2))
            .call()
        {
            return response.status() == 200;
        }

        // Fallback: try base URL
        let base_url = endpoint.trim_end_matches("/api");
        ureq::get(base_url)
            .timeout(std::time::Duration::from_secs(2))
            .call()
            .map(|r| r.status() == 200)
            .unwrap_or(false)
    }

    /// Get current model info
    pub fn get_model_info(&self) -> Result<serde_json::Value> {
        let url = format!("{}/v1/model", self.endpoint.trim_end_matches("/api"));
        let response: serde_json::Value = ureq::get(&url).call()?.into_json()?;
        Ok(response)
    }

    /// List available characters
    pub fn list_characters(&self) -> Result<Vec<String>> {
        let characters_path = self
            .characters_path
            .as_ref()
            .ok_or_else(|| anyhow!("Characters path not found"))?;

        let mut characters = Vec::new();
        for entry in std::fs::read_dir(characters_path)? {
            let entry = entry?;
            let path = entry.path();
            if path
                .extension()
                .map(|e| e == "yaml" || e == "json")
                .unwrap_or(false)
            {
                if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                    characters.push(name.to_string());
                }
            }
        }

        Ok(characters)
    }

    /// Load chat logs
    fn load_chat_logs(&self) -> Result<Vec<(String, TextGenChatLog, std::time::SystemTime)>> {
        let logs_path = self
            .logs_path
            .as_ref()
            .ok_or_else(|| anyhow!("Chat logs path not found"))?;

        let mut logs = Vec::new();
        Self::scan_logs_recursive(logs_path, &mut logs)?;

        // Sort by modification time (newest first)
        logs.sort_by(|(_, _, a), (_, _, b)| b.cmp(a));

        Ok(logs)
    }

    /// Recursively scan for chat logs
    fn scan_logs_recursive(
        path: &PathBuf,
        logs: &mut Vec<(String, TextGenChatLog, std::time::SystemTime)>,
    ) -> Result<()> {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                Self::scan_logs_recursive(&path, logs)?;
            } else if path.extension().map(|e| e == "json").unwrap_or(false) {
                let metadata = entry.metadata()?;
                let modified = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);

                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(log) = serde_json::from_str::<TextGenChatLog>(&content) {
                        let id = path
                            .file_stem()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        logs.push((id, log, modified));
                    }
                }
            }
        }
        Ok(())
    }

    /// Convert chat log to ChatSession
    fn convert_to_session(
        &self,
        id: &str,
        log: &TextGenChatLog,
        modified: std::time::SystemTime,
    ) -> Result<ChatSession> {
        let mut messages = Vec::new();

        for (i, turn) in log.chat.iter().enumerate() {
            // User message
            messages.push(ChatMessage {
                id: Some(format!("{}_{}_user", id, i)),
                role: "user".to_string(),
                content: turn.user.clone(),
                timestamp: turn
                    .timestamp
                    .as_ref()
                    .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                metadata: None,
            });

            // Assistant message
            messages.push(ChatMessage {
                id: Some(format!("{}_{}_assistant", id, i)),
                role: "assistant".to_string(),
                content: turn.assistant.clone(),
                timestamp: turn
                    .timestamp
                    .as_ref()
                    .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                metadata: None,
            });
        }

        let created_at = messages
            .first()
            .and_then(|m| m.timestamp)
            .unwrap_or_else(Utc::now);

        let updated_at = DateTime::<Utc>::from(modified);

        let title = log
            .character
            .clone()
            .unwrap_or_else(|| "Text Gen Chat".to_string());

        Ok(ChatSession {
            id: id.to_string(),
            title,
            provider: "textgen_webui".to_string(),
            model: None,
            messages,
            created_at,
            updated_at,
            workspace_id: None,
            metadata: Some(serde_json::json!({
                "character": log.character,
                "mode": log.mode,
            })),
            tags: Vec::new(),
        })
    }

    /// Send a chat message using OpenAI-compatible API
    pub fn chat(&self, messages: &[ChatMessage], character: Option<&str>) -> Result<String> {
        let endpoint = self
            .openai_endpoint
            .as_ref()
            .ok_or_else(|| anyhow!("OpenAI-compatible API not available"))?;

        let api_messages: Vec<TextGenMessage> = messages
            .iter()
            .map(|m| TextGenMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let request = TextGenOpenAiRequest {
            messages: api_messages,
            model: None,
            max_tokens: Some(2048),
            temperature: Some(0.7),
            stream: Some(false),
            mode: Some("chat".to_string()),
            character: character.map(String::from),
        };

        let url = format!("{}/chat/completions", endpoint);
        let response: TextGenOpenAiResponse = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(&request)?
            .into_json()?;

        response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow!("No response from Text Gen WebUI"))
    }

    /// Send a chat message using native API
    pub fn chat_native(
        &self,
        user_input: &str,
        history: Option<TextGenHistory>,
        character: Option<&str>,
    ) -> Result<(String, TextGenHistory)> {
        if !self.available {
            return Err(anyhow!("Text Gen WebUI API not available"));
        }

        let history = history.unwrap_or_else(|| TextGenHistory {
            internal: Vec::new(),
            visible: Vec::new(),
        });

        let request = TextGenChatRequest {
            user_input: user_input.to_string(),
            max_new_tokens: Some(2048),
            auto_max_new_tokens: Some(false),
            mode: Some("chat".to_string()),
            character: character.map(String::from),
            instruction_template: None,
            your_name: None,
            regenerate: Some(false),
            continue_: Some(false),
            history,
        };

        let url = format!("{}/v1/chat", self.endpoint.trim_end_matches("/api"));
        let response: TextGenChatResponse = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(&request)?
            .into_json()?;

        let result = response
            .results
            .first()
            .ok_or_else(|| anyhow!("No response from Text Gen WebUI"))?;

        let assistant_response = result
            .history
            .visible
            .last()
            .and_then(|turn| turn.get(1))
            .cloned()
            .unwrap_or_default();

        Ok((assistant_response, result.history.clone()))
    }
}

impl ChatProvider for TextGenWebUiProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::TextGenWebUi
    }

    fn name(&self) -> &str {
        "Text Generation WebUI"
    }

    fn is_available(&self) -> bool {
        self.available || self.logs_path.is_some()
    }

    fn sessions_path(&self) -> Option<PathBuf> {
        self.logs_path.clone()
    }

    fn list_sessions(&self) -> Result<Vec<ChatSession>> {
        if self.logs_path.is_none() {
            return Ok(Vec::new());
        }

        let logs = self.load_chat_logs()?;
        logs.iter()
            .map(|(id, log, modified)| self.convert_to_session(id, log, *modified))
            .collect()
    }

    fn import_session(&self, session_id: &str) -> Result<ChatSession> {
        let logs = self.load_chat_logs()?;

        let (id, log, modified) = logs
            .iter()
            .find(|(id, _, _)| id == session_id)
            .ok_or_else(|| anyhow!("Chat log not found: {}", session_id))?;

        self.convert_to_session(id, log, *modified)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover() {
        let provider = TextGenWebUiProvider::discover();
        println!("Text Gen WebUI discovered: {:?}", provider.is_some());
    }
}
