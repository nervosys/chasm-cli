// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! vLLM provider for high-performance inference
//!
//! vLLM is a high-throughput and memory-efficient inference engine
//! with an OpenAI-compatible API server.

#![allow(dead_code)]

use super::{ChatProvider, ProviderType};
use crate::models::{ChatMessage, ChatSession};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// vLLM provider for high-performance inference
///
/// vLLM provides PagedAttention for efficient memory management,
/// continuous batching, and high throughput. Exposes OpenAI-compatible API.
pub struct VllmProvider {
    /// API endpoint
    endpoint: String,
    /// Whether vLLM API is available
    available: bool,
    /// Available models
    models: Vec<String>,
    /// Server metrics endpoint
    metrics_endpoint: Option<String>,
}

/// vLLM chat request (OpenAI-compatible)
#[derive(Debug, Serialize)]
struct VllmChatRequest {
    model: String,
    messages: Vec<VllmMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    best_of: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_beam_search: Option<bool>,
}

/// vLLM message format
#[derive(Debug, Deserialize, Serialize, Clone)]
struct VllmMessage {
    role: String,
    content: String,
}

/// vLLM chat response
#[derive(Debug, Deserialize)]
struct VllmChatResponse {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<VllmChoice>,
    usage: Option<VllmUsage>,
}

#[derive(Debug, Deserialize)]
struct VllmChoice {
    index: i32,
    message: VllmMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VllmUsage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

/// vLLM completion request
#[derive(Debug, Serialize)]
struct VllmCompletionRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    best_of: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_beam_search: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    logprobs: Option<i32>,
}

/// vLLM completion response
#[derive(Debug, Deserialize)]
struct VllmCompletionResponse {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<VllmCompletionChoice>,
    usage: Option<VllmUsage>,
}

#[derive(Debug, Deserialize)]
struct VllmCompletionChoice {
    index: i32,
    text: String,
    finish_reason: Option<String>,
    logprobs: Option<serde_json::Value>,
}

/// vLLM model info
#[derive(Debug, Deserialize)]
struct VllmModel {
    id: String,
    object: String,
    #[serde(default)]
    owned_by: Option<String>,
    #[serde(default)]
    root: Option<String>,
    #[serde(default)]
    parent: Option<String>,
}

/// vLLM models list response
#[derive(Debug, Deserialize)]
struct VllmModelsResponse {
    object: String,
    data: Vec<VllmModel>,
}

/// vLLM server health status
#[derive(Debug, Deserialize)]
struct VllmHealthResponse {
    status: String,
}

impl VllmProvider {
    /// Discover vLLM server
    pub fn discover() -> Option<Self> {
        let endpoint = Self::find_api_endpoint();
        let metrics_endpoint = Self::find_metrics_endpoint(&endpoint);
        let (available, models) = Self::check_api(&endpoint);

        Some(Self {
            endpoint,
            available,
            models,
            metrics_endpoint,
        })
    }

    /// Find API endpoint
    fn find_api_endpoint() -> String {
        // Check environment variable
        if let Ok(endpoint) = std::env::var("VLLM_API") {
            return endpoint;
        }

        // Check OpenAI-compatible env var
        if let Ok(endpoint) = std::env::var("OPENAI_API_BASE") {
            if endpoint.contains("vllm") || endpoint.contains(":8000") {
                return endpoint;
            }
        }

        // Default endpoint (vLLM server default)
        "http://localhost:8000/v1".to_string()
    }

    /// Find metrics endpoint
    fn find_metrics_endpoint(endpoint: &str) -> Option<String> {
        let base = endpoint.trim_end_matches("/v1");
        let metrics_url = format!("{}/metrics", base);

        // Verify metrics endpoint exists
        if ureq::get(&metrics_url)
            .timeout(std::time::Duration::from_secs(1))
            .call()
            .is_ok()
        {
            return Some(metrics_url);
        }

        None
    }

    /// Check API availability
    fn check_api(endpoint: &str) -> (bool, Vec<String>) {
        // Try health endpoint first
        let base = endpoint.trim_end_matches("/v1");
        let health_url = format!("{}/health", base);
        let health_ok = ureq::get(&health_url)
            .timeout(std::time::Duration::from_secs(2))
            .call()
            .map(|r| r.status() == 200)
            .unwrap_or(false);

        if !health_ok {
            // Fallback: try models endpoint
            let url = format!("{}/models", endpoint);
            match ureq::get(&url)
                .timeout(std::time::Duration::from_secs(2))
                .call()
            {
                Ok(response) if response.status() == 200 => {
                    if let Ok(models_resp) = response.into_json::<VllmModelsResponse>() {
                        let models: Vec<String> =
                            models_resp.data.iter().map(|m| m.id.clone()).collect();
                        return (true, models);
                    }
                    return (true, Vec::new());
                }
                _ => return (false, Vec::new()),
            }
        }

        // Get models
        let url = format!("{}/models", endpoint);
        let models = ureq::get(&url)
            .timeout(std::time::Duration::from_secs(2))
            .call()
            .ok()
            .and_then(|r| r.into_json::<VllmModelsResponse>().ok())
            .map(|resp| resp.data.iter().map(|m| m.id.clone()).collect())
            .unwrap_or_default();

        (true, models)
    }

    /// List available models
    pub fn list_models(&self) -> Result<Vec<String>> {
        if !self.available {
            return Ok(self.models.clone());
        }

        let url = format!("{}/models", self.endpoint);
        let response: VllmModelsResponse = ureq::get(&url).call()?.into_json()?;

        Ok(response.data.iter().map(|m| m.id.clone()).collect())
    }

    /// Check server health
    pub fn health_check(&self) -> Result<bool> {
        let base = self.endpoint.trim_end_matches("/v1");
        let url = format!("{}/health", base);

        let response = ureq::get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .call()?;

        Ok(response.status() == 200)
    }

    /// Get server metrics (Prometheus format)
    pub fn get_metrics(&self) -> Result<String> {
        let metrics_url = self
            .metrics_endpoint
            .as_ref()
            .ok_or_else(|| anyhow!("Metrics endpoint not available"))?;

        let response = ureq::get(metrics_url).call()?;
        Ok(response.into_string()?)
    }

    /// Parse metrics into structured data
    pub fn parse_metrics(&self) -> Result<VllmMetrics> {
        let raw_metrics = self.get_metrics()?;
        VllmMetrics::parse(&raw_metrics)
    }

    /// Send a chat message
    pub fn chat(&self, model: &str, messages: &[ChatMessage]) -> Result<String> {
        if !self.available {
            return Err(anyhow!("vLLM API not available"));
        }

        let api_messages: Vec<VllmMessage> = messages
            .iter()
            .map(|m| VllmMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let request = VllmChatRequest {
            model: model.to_string(),
            messages: api_messages,
            temperature: Some(0.7),
            top_p: Some(0.9),
            max_tokens: Some(2048),
            stream: Some(false),
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            n: Some(1),
            best_of: None,
            use_beam_search: None,
        };

        let url = format!("{}/chat/completions", self.endpoint);
        let response: VllmChatResponse = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(&request)?
            .into_json()?;

        response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow!("No response from vLLM"))
    }

    /// Generate text completion (non-chat)
    pub fn complete(&self, model: &str, prompt: &str) -> Result<String> {
        if !self.available {
            return Err(anyhow!("vLLM API not available"));
        }

        let request = VllmCompletionRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            max_tokens: Some(512),
            temperature: Some(0.7),
            top_p: Some(0.9),
            stream: Some(false),
            stop: None,
            n: Some(1),
            best_of: None,
            use_beam_search: None,
            logprobs: None,
        };

        let url = format!("{}/completions", self.endpoint);
        let response: VllmCompletionResponse = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(&request)?
            .into_json()?;

        response
            .choices
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| anyhow!("No completion from vLLM"))
    }

    /// Generate with beam search
    pub fn generate_beam_search(
        &self,
        model: &str,
        prompt: &str,
        num_beams: i32,
    ) -> Result<Vec<String>> {
        if !self.available {
            return Err(anyhow!("vLLM API not available"));
        }

        let request = VllmCompletionRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            max_tokens: Some(256),
            temperature: Some(0.0), // Deterministic for beam search
            top_p: None,
            stream: Some(false),
            stop: None,
            n: Some(num_beams),
            best_of: Some(num_beams),
            use_beam_search: Some(true),
            logprobs: None,
        };

        let url = format!("{}/completions", self.endpoint);
        let response: VllmCompletionResponse = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(&request)?
            .into_json()?;

        Ok(response.choices.iter().map(|c| c.text.clone()).collect())
    }
}

impl ChatProvider for VllmProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Vllm
    }

    fn name(&self) -> &str {
        "vLLM"
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn sessions_path(&self) -> Option<PathBuf> {
        // vLLM is stateless - no session storage
        None
    }

    fn list_sessions(&self) -> Result<Vec<ChatSession>> {
        // vLLM doesn't store conversations
        Ok(Vec::new())
    }

    fn import_session(&self, _session_id: &str) -> Result<ChatSession> {
        Err(anyhow!("vLLM does not store conversation history"))
    }
}

/// Parsed vLLM metrics
#[derive(Debug, Default)]
pub struct VllmMetrics {
    pub num_running_requests: Option<i64>,
    pub num_waiting_requests: Option<i64>,
    pub gpu_cache_usage_percent: Option<f64>,
    pub cpu_cache_usage_percent: Option<f64>,
    pub avg_generation_throughput: Option<f64>,
    pub avg_prompt_throughput: Option<f64>,
}

impl VllmMetrics {
    /// Parse Prometheus-format metrics
    fn parse(raw: &str) -> Result<Self> {
        let mut metrics = VllmMetrics::default();

        for line in raw.lines() {
            if line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            let name = parts[0].split('{').next().unwrap_or(parts[0]);
            let value = parts.last().and_then(|v| v.parse::<f64>().ok());

            match name {
                "vllm:num_requests_running" => {
                    metrics.num_running_requests = value.map(|v| v as i64);
                }
                "vllm:num_requests_waiting" => {
                    metrics.num_waiting_requests = value.map(|v| v as i64);
                }
                "vllm:gpu_cache_usage_perc" => {
                    metrics.gpu_cache_usage_percent = value;
                }
                "vllm:cpu_cache_usage_perc" => {
                    metrics.cpu_cache_usage_percent = value;
                }
                "vllm:avg_generation_throughput_toks_per_s" => {
                    metrics.avg_generation_throughput = value;
                }
                "vllm:avg_prompt_throughput_toks_per_s" => {
                    metrics.avg_prompt_throughput = value;
                }
                _ => {}
            }
        }

        Ok(metrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover() {
        let provider = VllmProvider::discover();
        println!("vLLM discovered: {:?}", provider.is_some());
    }

    #[test]
    fn test_metrics_parsing() {
        let raw = r#"
# HELP vllm:num_requests_running Number of requests being processed
# TYPE vllm:num_requests_running gauge
vllm:num_requests_running 5
# HELP vllm:gpu_cache_usage_perc GPU KV cache usage
# TYPE vllm:gpu_cache_usage_perc gauge
vllm:gpu_cache_usage_perc 0.45
"#;

        let metrics = VllmMetrics::parse(raw).unwrap();
        assert_eq!(metrics.num_running_requests, Some(5));
        assert_eq!(metrics.gpu_cache_usage_percent, Some(0.45));
    }
}
