// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! LlamaFile provider for single-file LLM execution
//!
//! LlamaFile bundles models into single executable files that
//! run on any platform and expose an OpenAI-compatible API.

#![allow(dead_code)]

use super::{ChatProvider, ProviderType};
use crate::models::{ChatMessage, ChatSession};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// LlamaFile provider for single-file LLM execution
///
/// LlamaFile exposes an OpenAI-compatible API typically on port 8080.
/// Since it's a single binary, there's no persistent conversation storage.
pub struct LlamaFileProvider {
    /// API endpoint
    endpoint: String,
    /// Whether LlamaFile is running
    available: bool,
    /// Current model name (from running instance)
    model_name: Option<String>,
}

/// LlamaFile model info from API
#[derive(Debug, Deserialize)]
struct LlamaFileModel {
    id: String,
    object: String,
    owned_by: Option<String>,
}

/// LlamaFile API chat request (OpenAI-compatible)
#[derive(Debug, Serialize)]
struct LlamaFileChatRequest {
    model: Option<String>,
    messages: Vec<LlamaFileMessage>,
    temperature: Option<f32>,
    max_tokens: Option<i32>,
    stream: Option<bool>,
    stop: Option<Vec<String>>,
}

/// LlamaFile message format
#[derive(Debug, Serialize, Deserialize, Clone)]
struct LlamaFileMessage {
    role: String,
    content: String,
}

/// LlamaFile API chat response
#[derive(Debug, Deserialize)]
struct LlamaFileChatResponse {
    id: Option<String>,
    object: String,
    created: i64,
    model: Option<String>,
    choices: Vec<LlamaFileChoice>,
    usage: Option<LlamaFileUsage>,
}

#[derive(Debug, Deserialize)]
struct LlamaFileChoice {
    index: i32,
    message: LlamaFileMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LlamaFileUsage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

/// LlamaFile completion request (for non-chat completions)
#[derive(Debug, Serialize)]
struct LlamaFileCompletionRequest {
    prompt: String,
    temperature: Option<f32>,
    n_predict: Option<i32>,
    stop: Option<Vec<String>>,
}

/// LlamaFile completion response
#[derive(Debug, Deserialize)]
struct LlamaFileCompletionResponse {
    content: String,
    model: Option<String>,
    prompt: Option<String>,
    stop: bool,
    tokens_predicted: Option<i32>,
}

/// LlamaFile health response
#[derive(Debug, Deserialize)]
struct LlamaFileHealth {
    status: String,
    slots_idle: Option<i32>,
    slots_processing: Option<i32>,
}

impl LlamaFileProvider {
    /// Discover running LlamaFile instance
    pub fn discover() -> Option<Self> {
        let endpoints = Self::possible_endpoints();

        for endpoint in endpoints {
            if let Some(model_name) = Self::check_endpoint(&endpoint) {
                return Some(Self {
                    endpoint,
                    available: true,
                    model_name: Some(model_name),
                });
            }
        }

        // Return unavailable provider so users can start one
        Some(Self {
            endpoint: "http://localhost:8080".to_string(),
            available: false,
            model_name: None,
        })
    }

    /// Create provider with custom endpoint
    pub fn with_endpoint(endpoint: &str) -> Self {
        let model_name = Self::check_endpoint(endpoint);
        Self {
            endpoint: endpoint.to_string(),
            available: model_name.is_some(),
            model_name,
        }
    }

    /// Get possible endpoint addresses
    fn possible_endpoints() -> Vec<String> {
        let mut endpoints = Vec::new();

        // Check environment variable first
        if let Ok(endpoint) = std::env::var("LLAMAFILE_API") {
            endpoints.push(endpoint);
        }

        // Common ports for llamafile
        endpoints.push("http://localhost:8080".to_string()); // Default
        endpoints.push("http://127.0.0.1:8080".to_string());
        endpoints.push("http://localhost:8081".to_string()); // Alternative
        endpoints.push("http://localhost:8000".to_string()); // Alternative

        endpoints
    }

    /// Check if endpoint is available and get model name
    fn check_endpoint(endpoint: &str) -> Option<String> {
        // Try /v1/models first (OpenAI-compatible)
        let models_url = format!("{}/v1/models", endpoint);
        if let Ok(response) = ureq::get(&models_url)
            .timeout(std::time::Duration::from_secs(2))
            .call()
        {
            if response.status() == 200 {
                if let Ok(json) = response.into_json::<serde_json::Value>() {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        if let Some(model) = data.first() {
                            if let Some(id) = model.get("id").and_then(|id| id.as_str()) {
                                return Some(id.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Try /health endpoint (llama.cpp native)
        let health_url = format!("{}/health", endpoint);
        if let Ok(response) = ureq::get(&health_url)
            .timeout(std::time::Duration::from_secs(2))
            .call()
        {
            if response.status() == 200 {
                return Some("llamafile".to_string());
            }
        }

        None
    }

    /// Get current model name
    pub fn model_name(&self) -> Option<&str> {
        self.model_name.as_deref()
    }

    /// Check health status
    pub fn health(&self) -> Result<LlamaFileHealth> {
        let url = format!("{}/health", self.endpoint);
        let response: LlamaFileHealth = ureq::get(&url).call()?.into_json()?;
        Ok(response)
    }

    /// Send a chat message
    pub fn chat(&self, messages: &[ChatMessage]) -> Result<String> {
        if !self.available {
            return Err(anyhow!("LlamaFile is not running"));
        }

        let api_messages: Vec<LlamaFileMessage> = messages
            .iter()
            .map(|m| LlamaFileMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let request = LlamaFileChatRequest {
            model: self.model_name.clone(),
            messages: api_messages,
            temperature: Some(0.7),
            max_tokens: Some(2048),
            stream: Some(false),
            stop: None,
        };

        let url = format!("{}/v1/chat/completions", self.endpoint);
        let response: LlamaFileChatResponse = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(&request)?
            .into_json()?;

        response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow!("No response from LlamaFile"))
    }

    /// Send a completion request (non-chat)
    pub fn complete(&self, prompt: &str) -> Result<String> {
        if !self.available {
            return Err(anyhow!("LlamaFile is not running"));
        }

        let request = LlamaFileCompletionRequest {
            prompt: prompt.to_string(),
            temperature: Some(0.7),
            n_predict: Some(2048),
            stop: None,
        };

        let url = format!("{}/completion", self.endpoint);
        let response: LlamaFileCompletionResponse = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(&request)?
            .into_json()?;

        Ok(response.content)
    }
}

impl ChatProvider for LlamaFileProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::LlamaFile
    }

    fn name(&self) -> &str {
        "LlamaFile"
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn sessions_path(&self) -> Option<PathBuf> {
        // LlamaFile doesn't store sessions persistently
        None
    }

    fn list_sessions(&self) -> Result<Vec<ChatSession>> {
        // LlamaFile is stateless - no persistent sessions
        Ok(Vec::new())
    }

    fn import_session(&self, _session_id: &str) -> Result<ChatSession> {
        Err(anyhow!("LlamaFile does not store sessions"))
    }

    fn export_session(&self, _session: &ChatSession) -> Result<()> {
        Err(anyhow!("LlamaFile does not support session export"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover() {
        let provider = LlamaFileProvider::discover();
        println!("LlamaFile discovered: {:?}", provider.is_some());
        if let Some(p) = provider {
            println!("Available: {}", p.available);
            println!("Model: {:?}", p.model_name);
        }
    }
}
