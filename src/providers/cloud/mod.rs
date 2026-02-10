// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Cloud-based LLM provider integrations
//!
//! This module provides clients for fetching conversation histories from
//! cloud-based LLM services like ChatGPT, Claude, Perplexity, etc.
//!
//! ## Supported Providers
//!
//! - **Microsoft 365 Copilot** - Enterprise AI assistant in Office apps
//! - **ChatGPT** - OpenAI's ChatGPT web interface conversations
//! - **Anthropic** - Claude conversations
//! - **Perplexity** - Perplexity AI conversations
//! - **DeepSeek** - DeepSeek chat history
//! - **Qwen** - Alibaba Qwen conversations
//! - **Gemini** - Google Gemini conversations
//! - **Mistral** - Mistral AI conversations
//! - **Cohere** - Cohere chat history
//! - **Grok** - xAI Grok conversations
//! - **Groq** - Groq conversations
//! - **Together** - Together AI conversations
//! - **Fireworks** - Fireworks AI conversations
//!
//! ## Authentication
//!
//! Most providers require API keys or session tokens. These can be provided via:
//! - Environment variables (e.g., `OPENAI_API_KEY`)
//! - Configuration file (`~/.config/csm/config.json`)
//! - Command-line arguments

pub mod anthropic;
pub mod chatgpt;
pub mod common;
pub mod deepseek;
pub mod gemini;
pub mod m365copilot;
pub mod perplexity;

pub use anthropic::AnthropicProvider;
pub use chatgpt::ChatGPTProvider;
pub use common::{CloudConversation, CloudMessage, CloudProvider, FetchOptions};
pub use deepseek::DeepSeekProvider;
pub use gemini::GeminiProvider;
pub use m365copilot::M365CopilotProvider;
pub use perplexity::PerplexityProvider;

use super::config::ProviderType;
use crate::models::ChatSession;
use anyhow::Result;

/// Get a cloud provider by type
pub fn get_cloud_provider(
    provider_type: ProviderType,
    api_key: Option<String>,
) -> Option<Box<dyn CloudProvider>> {
    match provider_type {
        ProviderType::M365Copilot => Some(Box::new(M365CopilotProvider::new(api_key))),
        ProviderType::ChatGPT => Some(Box::new(ChatGPTProvider::new(api_key))),
        ProviderType::Anthropic => Some(Box::new(AnthropicProvider::new(api_key))),
        ProviderType::Perplexity => Some(Box::new(PerplexityProvider::new(api_key))),
        ProviderType::DeepSeek => Some(Box::new(DeepSeekProvider::new(api_key))),
        ProviderType::Gemini => Some(Box::new(GeminiProvider::new(api_key))),
        _ => None,
    }
}

/// Fetch all conversations from a cloud provider
pub fn fetch_conversations(
    provider_type: ProviderType,
    api_key: Option<String>,
    options: &FetchOptions,
) -> Result<Vec<ChatSession>> {
    let provider = get_cloud_provider(provider_type, api_key)
        .ok_or_else(|| anyhow::anyhow!("Unsupported cloud provider: {}", provider_type))?;

    provider.fetch_all_conversations(options)
}
