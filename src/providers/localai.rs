// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! LocalAI provider for multi-backend local inference
//!
//! LocalAI is a drop-in OpenAI replacement that supports multiple
//! backends including llama.cpp, whisper.cpp, GPT-J, and more.

#![allow(dead_code)]

use super::{ChatProvider, ProviderType};
use crate::models::{ChatMessage, ChatSession};
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// LocalAI provider for local multi-backend inference
///
/// LocalAI typically runs as a Docker container or standalone binary
/// with an OpenAI-compatible API on port 8080.
pub struct LocalAiProvider {
    /// API endpoint
    endpoint: String,
    /// Whether LocalAI API is available
    available: bool,
    /// List of available models
    models: Vec<String>,
    /// Models directory path
    models_path: Option<PathBuf>,
}

/// LocalAI chat request (OpenAI-compatible)
#[derive(Debug, Serialize)]
struct LocalAiChatRequest {
    model: String,
    messages: Vec<LocalAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

/// LocalAI message format
#[derive(Debug, Deserialize, Serialize, Clone)]
struct LocalAiMessage {
    role: String,
    content: String,
}

/// LocalAI chat response
#[derive(Debug, Deserialize)]
struct LocalAiChatResponse {
    id: Option<String>,
    object: String,
    created: i64,
    model: String,
    choices: Vec<LocalAiChoice>,
    #[serde(default)]
    usage: Option<LocalAiUsage>,
}

#[derive(Debug, Deserialize)]
struct LocalAiChoice {
    index: i32,
    message: LocalAiMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalAiUsage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

/// LocalAI model info
#[derive(Debug, Deserialize)]
struct LocalAiModel {
    id: String,
    object: String,
    #[serde(default)]
    owned_by: Option<String>,
}

/// LocalAI models list response
#[derive(Debug, Deserialize)]
struct LocalAiModelsResponse {
    object: String,
    data: Vec<LocalAiModel>,
}

/// LocalAI backend info
#[derive(Debug, Deserialize)]
struct LocalAiBackendInfo {
    backend: String,
    #[serde(default)]
    available: bool,
}

impl LocalAiProvider {
    /// Discover LocalAI installation
    pub fn discover() -> Option<Self> {
        let endpoint = Self::find_api_endpoint();
        let (available, models) = Self::check_api(&endpoint);
        let models_path = Self::find_models_path();

        Some(Self {
            endpoint,
            available,
            models,
            models_path,
        })
    }

    /// Find API endpoint
    fn find_api_endpoint() -> String {
        // Check environment variable
        if let Ok(endpoint) = std::env::var("LOCALAI_API") {
            return endpoint;
        }

        // Check common alternate endpoints
        if let Ok(endpoint) = std::env::var("OPENAI_API_BASE") {
            if endpoint.contains("localhost") || endpoint.contains("127.0.0.1") {
                return endpoint;
            }
        }

        // Default endpoint
        "http://localhost:8080/v1".to_string()
    }

    /// Check API availability
    fn check_api(endpoint: &str) -> (bool, Vec<String>) {
        let url = format!("{}/models", endpoint);
        match ureq::get(&url)
            .timeout(std::time::Duration::from_secs(3))
            .call()
        {
            Ok(response) if response.status() == 200 => {
                if let Ok(models_resp) = response.into_json::<LocalAiModelsResponse>() {
                    let models: Vec<String> =
                        models_resp.data.iter().map(|m| m.id.clone()).collect();
                    return (true, models);
                }
                (true, Vec::new())
            }
            _ => (false, Vec::new()),
        }
    }

    /// Find models directory
    fn find_models_path() -> Option<PathBuf> {
        // Check environment variable
        if let Ok(path) = std::env::var("LOCALAI_MODELS") {
            let path = PathBuf::from(path);
            if path.exists() {
                return Some(path);
            }
        }

        // Check common locations
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".local-ai").join("models");
            if path.exists() {
                return Some(path);
            }

            let path = home.join("localai").join("models");
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    /// List available models
    pub fn list_models(&self) -> Result<Vec<String>> {
        if !self.available {
            return Ok(self.models.clone());
        }

        let url = format!("{}/models", self.endpoint);
        let response: LocalAiModelsResponse = ureq::get(&url).call()?.into_json()?;

        Ok(response.data.iter().map(|m| m.id.clone()).collect())
    }

    /// Get system info/status
    pub fn get_system_info(&self) -> Result<serde_json::Value> {
        let url = self.endpoint.replace("/v1", "/system");
        let response: serde_json::Value = ureq::get(&url).call()?.into_json()?;
        Ok(response)
    }

    /// Send a chat message
    pub fn chat(&self, model: &str, messages: &[ChatMessage]) -> Result<String> {
        if !self.available {
            return Err(anyhow!("LocalAI API not available"));
        }

        let api_messages: Vec<LocalAiMessage> = messages
            .iter()
            .map(|m| LocalAiMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let request = LocalAiChatRequest {
            model: model.to_string(),
            messages: api_messages,
            temperature: Some(0.7),
            max_tokens: Some(2048),
            stream: Some(false),
            stop: None,
        };

        let url = format!("{}/chat/completions", self.endpoint);
        let response: LocalAiChatResponse = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(&request)?
            .into_json()?;

        response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow!("No response from LocalAI"))
    }

    /// Generate text completion (non-chat)
    pub fn complete(&self, model: &str, prompt: &str) -> Result<String> {
        if !self.available {
            return Err(anyhow!("LocalAI API not available"));
        }

        #[derive(Serialize)]
        struct CompletionRequest {
            model: String,
            prompt: String,
            max_tokens: i32,
            temperature: f32,
        }

        #[derive(Deserialize)]
        struct CompletionResponse {
            choices: Vec<CompletionChoice>,
        }

        #[derive(Deserialize)]
        struct CompletionChoice {
            text: String,
        }

        let request = CompletionRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            max_tokens: 512,
            temperature: 0.7,
        };

        let url = format!("{}/completions", self.endpoint);
        let response: CompletionResponse = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(&request)?
            .into_json()?;

        response
            .choices
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| anyhow!("No completion from LocalAI"))
    }

    /// Generate embeddings
    pub fn embeddings(&self, model: &str, input: &[String]) -> Result<Vec<Vec<f32>>> {
        if !self.available {
            return Err(anyhow!("LocalAI API not available"));
        }

        #[derive(Serialize)]
        struct EmbeddingsRequest {
            model: String,
            input: Vec<String>,
        }

        #[derive(Deserialize)]
        struct EmbeddingsResponse {
            data: Vec<EmbeddingData>,
        }

        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
        }

        let request = EmbeddingsRequest {
            model: model.to_string(),
            input: input.to_vec(),
        };

        let url = format!("{}/embeddings", self.endpoint);
        let response: EmbeddingsResponse = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(&request)?
            .into_json()?;

        Ok(response.data.iter().map(|d| d.embedding.clone()).collect())
    }

    /// Transcribe audio (Whisper backend)
    pub fn transcribe(&self, model: &str, audio_path: &str) -> Result<String> {
        if !self.available {
            return Err(anyhow!("LocalAI API not available"));
        }

        let audio_data = std::fs::read(audio_path)?;
        let file_name = std::path::Path::new(audio_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.wav");

        let url = format!("{}/audio/transcriptions", self.endpoint);

        // Create multipart form
        use std::io::Cursor;
        let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
        let mut body = Vec::new();

        // Add model field
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"model\"\r\n\r\n");
        body.extend_from_slice(model.as_bytes());
        body.extend_from_slice(b"\r\n");

        // Add file field
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n",
                file_name
            )
            .as_bytes(),
        );
        body.extend_from_slice(b"Content-Type: audio/wav\r\n\r\n");
        body.extend_from_slice(&audio_data);
        body.extend_from_slice(b"\r\n");

        // End boundary
        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        #[derive(Deserialize)]
        struct TranscriptionResponse {
            text: String,
        }

        let response: TranscriptionResponse = ureq::post(&url)
            .set(
                "Content-Type",
                &format!("multipart/form-data; boundary={}", boundary),
            )
            .send_bytes(&body)?
            .into_json()?;

        Ok(response.text)
    }
}

impl ChatProvider for LocalAiProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::LocalAi
    }

    fn name(&self) -> &str {
        "LocalAI"
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn sessions_path(&self) -> Option<PathBuf> {
        // LocalAI is stateless - no session storage
        None
    }

    fn list_sessions(&self) -> Result<Vec<ChatSession>> {
        // LocalAI doesn't store conversations
        Ok(Vec::new())
    }

    fn import_session(&self, _session_id: &str) -> Result<ChatSession> {
        Err(anyhow!("LocalAI does not store conversation history"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover() {
        let provider = LocalAiProvider::discover();
        println!("LocalAI discovered: {:?}", provider.is_some());
    }

    #[test]
    fn test_endpoint_format() {
        let endpoint = LocalAiProvider::find_api_endpoint();
        assert!(endpoint.contains("localhost") || endpoint.contains("127.0.0.1"));
    }
}
