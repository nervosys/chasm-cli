// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Provider configuration and types

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported LLM provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderType {
    // ========================================================================
    // Local/File-based Providers
    // ========================================================================
    /// VS Code GitHub Copilot Chat (default)
    Copilot,
    /// Cursor IDE chat
    Cursor,
    /// Continue.dev VS Code extension
    #[serde(rename = "continuedev")]
    ContinueDev,

    // ========================================================================
    // Local API Providers
    // ========================================================================
    /// Ollama local models
    Ollama,
    /// vLLM server
    Vllm,
    /// Azure AI Foundry / Foundry Local
    Foundry,
    /// LM Studio
    LmStudio,
    /// LocalAI
    #[serde(rename = "localai")]
    LocalAI,
    /// Text Generation WebUI (oobabooga)
    #[serde(rename = "text-gen-webui")]
    TextGenWebUI,
    /// Jan.ai
    Jan,
    /// GPT4All
    #[serde(rename = "gpt4all")]
    Gpt4All,
    /// Llamafile
    Llamafile,

    // ========================================================================
    // Cloud API Providers (with conversation history APIs)
    // ========================================================================
    /// Microsoft 365 Copilot (enterprise)
    #[serde(rename = "m365copilot")]
    M365Copilot,
    /// OpenAI ChatGPT (cloud)
    #[serde(rename = "chatgpt")]
    ChatGPT,
    /// OpenAI API (for local/custom deployments)
    #[serde(rename = "openai")]
    OpenAI,
    /// Anthropic Claude
    #[serde(rename = "anthropic")]
    Anthropic,
    /// Perplexity AI
    #[serde(rename = "perplexity")]
    Perplexity,
    /// DeepSeek
    #[serde(rename = "deepseek")]
    DeepSeek,
    /// Qwen (Alibaba Cloud)
    #[serde(rename = "qwen")]
    Qwen,
    /// Google Gemini
    #[serde(rename = "gemini")]
    Gemini,
    /// Mistral AI
    #[serde(rename = "mistral")]
    Mistral,
    /// Cohere
    #[serde(rename = "cohere")]
    Cohere,
    /// xAI Grok
    #[serde(rename = "grok")]
    Grok,
    /// Groq
    #[serde(rename = "groq")]
    Groq,
    /// Together AI
    #[serde(rename = "together")]
    Together,
    /// Fireworks AI
    #[serde(rename = "fireworks")]
    Fireworks,
    /// Replicate
    #[serde(rename = "replicate")]
    Replicate,
    /// HuggingFace Inference API
    #[serde(rename = "huggingface")]
    HuggingFace,

    /// Custom OpenAI-compatible endpoint
    Custom,
}

impl ProviderType {
    /// Get the display name for this provider
    pub fn display_name(&self) -> &'static str {
        match self {
            // Local/File-based
            Self::Copilot => "GitHub Copilot",
            Self::Cursor => "Cursor",
            Self::ContinueDev => "Continue.dev",
            // Local API
            Self::Ollama => "Ollama",
            Self::Vllm => "vLLM",
            Self::Foundry => "Azure AI Foundry",
            Self::LmStudio => "LM Studio",
            Self::LocalAI => "LocalAI",
            Self::TextGenWebUI => "Text Generation WebUI",
            Self::Jan => "Jan.ai",
            Self::Gpt4All => "GPT4All",
            Self::Llamafile => "Llamafile",
            // Cloud API
            Self::M365Copilot => "Microsoft 365 Copilot",
            Self::ChatGPT => "ChatGPT",
            Self::OpenAI => "OpenAI API",
            Self::Anthropic => "Anthropic Claude",
            Self::Perplexity => "Perplexity AI",
            Self::DeepSeek => "DeepSeek",
            Self::Qwen => "Qwen (Alibaba)",
            Self::Gemini => "Google Gemini",
            Self::Mistral => "Mistral AI",
            Self::Cohere => "Cohere",
            Self::Grok => "xAI Grok",
            Self::Groq => "Groq",
            Self::Together => "Together AI",
            Self::Fireworks => "Fireworks AI",
            Self::Replicate => "Replicate",
            Self::HuggingFace => "HuggingFace",
            Self::Custom => "Custom",
        }
    }

    /// Get the default API endpoint for this provider
    pub fn default_endpoint(&self) -> Option<&'static str> {
        match self {
            // Local/File-based (no API endpoint)
            Self::Copilot => None,
            Self::Cursor => None,
            Self::ContinueDev => None,
            // Local API
            Self::Ollama => Some("http://localhost:11434"),
            Self::Vllm => Some("http://localhost:8000"),
            Self::Foundry => Some("http://localhost:5272"),
            Self::LmStudio => Some("http://localhost:1234/v1"),
            Self::LocalAI => Some("http://localhost:8080/v1"),
            Self::TextGenWebUI => Some("http://localhost:5000/v1"),
            Self::Jan => Some("http://localhost:1337/v1"),
            Self::Gpt4All => Some("http://localhost:4891/v1"),
            Self::Llamafile => Some("http://localhost:8080/v1"),
            // Cloud API
            Self::M365Copilot => Some("https://graph.microsoft.com/v1.0"),
            Self::ChatGPT => Some("https://chat.openai.com"),
            Self::OpenAI => Some("https://api.openai.com/v1"),
            Self::Anthropic => Some("https://api.anthropic.com/v1"),
            Self::Perplexity => Some("https://api.perplexity.ai"),
            Self::DeepSeek => Some("https://api.deepseek.com/v1"),
            Self::Qwen => Some("https://dashscope.aliyuncs.com/api/v1"),
            Self::Gemini => Some("https://generativelanguage.googleapis.com/v1beta"),
            Self::Mistral => Some("https://api.mistral.ai/v1"),
            Self::Cohere => Some("https://api.cohere.ai/v1"),
            Self::Grok => Some("https://api.x.ai/v1"),
            Self::Groq => Some("https://api.groq.com/openai/v1"),
            Self::Together => Some("https://api.together.xyz/v1"),
            Self::Fireworks => Some("https://api.fireworks.ai/inference/v1"),
            Self::Replicate => Some("https://api.replicate.com/v1"),
            Self::HuggingFace => Some("https://api-inference.huggingface.co"),
            Self::Custom => None,
        }
    }

    /// Check if this provider uses local file storage for sessions
    pub fn uses_file_storage(&self) -> bool {
        matches!(self, Self::Copilot | Self::Cursor | Self::ContinueDev)
    }

    /// Check if this provider is a cloud-based service with conversation history API
    pub fn is_cloud_provider(&self) -> bool {
        matches!(
            self,
            Self::M365Copilot
                | Self::ChatGPT
                | Self::OpenAI
                | Self::Anthropic
                | Self::Perplexity
                | Self::DeepSeek
                | Self::Qwen
                | Self::Gemini
                | Self::Mistral
                | Self::Cohere
                | Self::Grok
                | Self::Groq
                | Self::Together
                | Self::Fireworks
                | Self::Replicate
                | Self::HuggingFace
        )
    }

    /// Check if this provider supports the OpenAI API format
    pub fn is_openai_compatible(&self) -> bool {
        matches!(
            self,
            Self::Ollama
                | Self::Vllm
                | Self::Foundry
                | Self::OpenAI
                | Self::LmStudio
                | Self::LocalAI
                | Self::TextGenWebUI
                | Self::Jan
                | Self::Gpt4All
                | Self::Llamafile
                | Self::DeepSeek  // DeepSeek uses OpenAI-compatible API
                | Self::Groq      // Groq uses OpenAI-compatible API
                | Self::Together  // Together uses OpenAI-compatible API
                | Self::Fireworks // Fireworks uses OpenAI-compatible API
                | Self::Custom
        )
    }

    /// Check if this provider requires an API key
    pub fn requires_api_key(&self) -> bool {
        self.is_cloud_provider()
    }
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Configuration for a single provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider type
    pub provider_type: ProviderType,

    /// Whether this provider is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// API endpoint URL (for server-based providers)
    pub endpoint: Option<String>,

    /// API key (if required)
    pub api_key: Option<String>,

    /// Model to use (if configurable)
    pub model: Option<String>,

    /// Custom name for this provider instance
    pub name: Option<String>,

    /// Path to session storage (for file-based providers)
    pub storage_path: Option<PathBuf>,

    /// Additional provider-specific settings
    #[serde(default)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

fn default_true() -> bool {
    true
}

impl ProviderConfig {
    /// Create a new provider config with default settings
    pub fn new(provider_type: ProviderType) -> Self {
        Self {
            provider_type,
            enabled: true,
            endpoint: provider_type.default_endpoint().map(String::from),
            api_key: None,
            model: None,
            name: None,
            storage_path: None,
            extra: std::collections::HashMap::new(),
        }
    }

    /// Get the display name for this provider
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| self.provider_type.display_name().to_string())
    }
}

/// Global CSM configuration including all providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsmConfig {
    /// Configured providers
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,

    /// Default provider for new sessions
    pub default_provider: Option<ProviderType>,

    /// Whether to auto-discover providers
    #[serde(default = "default_true")]
    pub auto_discover: bool,
}

impl Default for CsmConfig {
    fn default() -> Self {
        Self {
            providers: Vec::new(),
            default_provider: None,
            auto_discover: true, // Important: enable auto-discovery by default
        }
    }
}

impl CsmConfig {
    /// Load configuration from the default location
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Self = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to the default location
    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Get the configuration file path
    pub fn config_path() -> anyhow::Result<PathBuf> {
        let config_dir =
            dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        Ok(config_dir.join("csm").join("config.json"))
    }

    /// Get a provider config by type
    pub fn get_provider(&self, provider_type: ProviderType) -> Option<&ProviderConfig> {
        self.providers
            .iter()
            .find(|p| p.provider_type == provider_type)
    }

    /// Add or update a provider config
    pub fn set_provider(&mut self, config: ProviderConfig) {
        if let Some(existing) = self
            .providers
            .iter_mut()
            .find(|p| p.provider_type == config.provider_type)
        {
            *existing = config;
        } else {
            self.providers.push(config);
        }
    }
}
