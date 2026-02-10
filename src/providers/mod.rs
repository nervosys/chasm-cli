// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! LLM Provider integrations for Chat System Manager
//!
//! Supports multiple chat providers:
//!
//! ## Local Providers
//! - VS Code Copilot Chat (default)
//! - Cursor
//! - Ollama
//! - vLLM
//! - Azure AI Foundry (Foundry Local)
//! - OpenAI API compatible servers
//! - LM Studio
//! - LocalAI
//!
//! ## Cloud Providers (conversation history import)
//! - ChatGPT (OpenAI)
//! - Claude (Anthropic)
//! - Perplexity
//! - DeepSeek
//! - Gemini (Google)
//! - Qwen (Alibaba)
//! - Mistral
//! - Cohere
//! - Groq
//! - Together AI

#[allow(dead_code)]
pub mod cloud;
pub mod config;
pub mod continuedev;
pub mod cursor;
#[allow(dead_code)]
pub mod discovery;
pub mod ollama;
pub mod openai_compat;
#[allow(dead_code)]
pub mod session_format;

#[allow(unused_imports)]
pub use cloud::{CloudConversation, CloudMessage, CloudProvider, FetchOptions};
pub use config::ProviderType;
#[allow(unused_imports)]
pub use config::{CsmConfig, ProviderConfig};
#[allow(unused_imports)]
pub use discovery::discover_all_providers;
#[allow(unused_imports)]
pub use session_format::{GenericMessage, GenericSession};

use crate::models::ChatSession;
use anyhow::Result;
use std::path::PathBuf;

/// Trait for LLM chat providers
pub trait ChatProvider: Send + Sync {
    /// Get the provider type
    fn provider_type(&self) -> ProviderType;

    /// Get the provider name for display
    fn name(&self) -> &str;

    /// Check if this provider is available/configured
    fn is_available(&self) -> bool;

    /// Get the base path where sessions are stored
    fn sessions_path(&self) -> Option<PathBuf>;

    /// List all chat sessions from this provider
    fn list_sessions(&self) -> Result<Vec<ChatSession>>;

    /// Import a session from this provider into CSM format
    fn import_session(&self, session_id: &str) -> Result<ChatSession>;

    /// Export a CSM session to this provider's format
    #[allow(dead_code)]
    fn export_session(&self, session: &ChatSession) -> Result<()>;
}

/// Registry of available providers
pub struct ProviderRegistry {
    providers: Vec<Box<dyn ChatProvider>>,
}

impl ProviderRegistry {
    /// Create a new provider registry with auto-discovered providers
    pub fn new() -> Self {
        let mut registry = Self {
            providers: Vec::new(),
        };
        registry.discover_providers();
        registry
    }

    /// Discover and register available providers
    fn discover_providers(&mut self) {
        // Add Cursor provider
        if let Some(provider) = cursor::CursorProvider::discover() {
            self.providers.push(Box::new(provider));
        }

        // Add Ollama provider
        if let Some(provider) = ollama::OllamaProvider::discover() {
            self.providers.push(Box::new(provider));
        }

        // Add OpenAI-compatible providers (vLLM, LM Studio, LocalAI, etc.)
        for provider in openai_compat::discover_openai_compatible_providers() {
            self.providers.push(Box::new(provider));
        }
    }

    /// Get all registered providers
    pub fn providers(&self) -> &[Box<dyn ChatProvider>] {
        &self.providers
    }

    /// Get available (configured and working) providers
    pub fn available_providers(&self) -> Vec<&dyn ChatProvider> {
        self.providers
            .iter()
            .filter(|p| p.is_available())
            .map(|p| p.as_ref())
            .collect()
    }

    /// Get a provider by type
    pub fn get_provider(&self, provider_type: ProviderType) -> Option<&dyn ChatProvider> {
        self.providers
            .iter()
            .find(|p| p.provider_type() == provider_type)
            .map(|p| p.as_ref())
    }

    /// List all sessions from all providers
    #[allow(dead_code)]
    pub fn list_all_sessions(&self) -> Result<Vec<(ProviderType, ChatSession)>> {
        let mut all_sessions = Vec::new();

        for provider in &self.providers {
            if provider.is_available() {
                if let Ok(sessions) = provider.list_sessions() {
                    for session in sessions {
                        all_sessions.push((provider.provider_type(), session));
                    }
                }
            }
        }

        Ok(all_sessions)
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
