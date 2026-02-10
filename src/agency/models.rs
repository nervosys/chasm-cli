// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Agency Data Models
//!
//! Core data structures for the Agent Development Kit.

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Message in an Agency conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgencyMessage {
    /// Unique message ID
    pub id: String,
    /// Role: user, assistant, system, tool
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Optional tool calls in this message
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// Optional tool result (if role is tool)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<ToolResult>,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Token count (if available)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u32>,
    /// Associated agent name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Message role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::System => write!(f, "system"),
            MessageRole::Tool => write!(f, "tool"),
        }
    }
}

/// Tool call request from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique call ID
    pub id: String,
    /// Tool name
    pub name: String,
    /// Tool arguments as JSON
    pub arguments: serde_json::Value,
    /// Call timestamp
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
}

/// Result from tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Associated tool call ID
    pub call_id: String,
    /// Tool name
    pub name: String,
    /// Whether execution succeeded
    pub success: bool,
    /// Result content (or error message)
    pub content: String,
    /// Execution duration in milliseconds
    #[serde(default)]
    pub duration_ms: u64,
    /// Additional output data
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Event emitted during agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgencyEvent {
    /// Event type
    pub event_type: EventType,
    /// Associated agent name
    pub agent_name: String,
    /// Event data
    pub data: serde_json::Value,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Session ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Types of events during execution - matches csm-shared AgencyEventType
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Agent started processing
    AgentStarted,
    /// Agent is thinking
    AgentThinking,
    /// Agent is executing
    AgentExecuting,
    /// Agent completed processing
    AgentCompleted,
    /// Agent failed
    AgentFailed,
    /// Tool call started
    ToolCallStarted,
    /// Tool call completed
    ToolCallCompleted,
    /// Tool call failed
    ToolCallFailed,
    /// Message created
    MessageCreated,
    /// Message delta (streaming)
    MessageDelta,
    /// Task created
    TaskCreated,
    /// Task started
    TaskStarted,
    /// Task completed
    TaskCompleted,
    /// Task failed
    TaskFailed,
    /// Swarm started
    SwarmStarted,
    /// Agent joined swarm
    SwarmAgentJoined,
    /// Swarm completed
    SwarmCompleted,
    /// Swarm failed
    SwarmFailed,
    /// Agent handoff to another agent
    Handoff,
    /// Error occurred
    Error,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::AgentStarted => write!(f, "agent_started"),
            EventType::AgentThinking => write!(f, "agent_thinking"),
            EventType::AgentExecuting => write!(f, "agent_executing"),
            EventType::AgentCompleted => write!(f, "agent_completed"),
            EventType::AgentFailed => write!(f, "agent_failed"),
            EventType::ToolCallStarted => write!(f, "tool_call_started"),
            EventType::ToolCallCompleted => write!(f, "tool_call_completed"),
            EventType::ToolCallFailed => write!(f, "tool_call_failed"),
            EventType::MessageCreated => write!(f, "message_created"),
            EventType::MessageDelta => write!(f, "message_delta"),
            EventType::TaskCreated => write!(f, "task_created"),
            EventType::TaskStarted => write!(f, "task_started"),
            EventType::TaskCompleted => write!(f, "task_completed"),
            EventType::TaskFailed => write!(f, "task_failed"),
            EventType::SwarmStarted => write!(f, "swarm_started"),
            EventType::SwarmAgentJoined => write!(f, "swarm_agent_joined"),
            EventType::SwarmCompleted => write!(f, "swarm_completed"),
            EventType::SwarmFailed => write!(f, "swarm_failed"),
            EventType::Handoff => write!(f, "handoff"),
            EventType::Error => write!(f, "error"),
        }
    }
}

/// Token usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Prompt/input tokens
    pub prompt_tokens: u32,
    /// Completion/output tokens
    pub completion_tokens: u32,
    /// Total tokens
    pub total_tokens: u32,
}

impl TokenUsage {
    pub fn new(prompt: u32, completion: u32) -> Self {
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
        }
    }

    pub fn add(&mut self, other: &TokenUsage) {
        self.prompt_tokens += other.prompt_tokens;
        self.completion_tokens += other.completion_tokens;
        self.total_tokens += other.total_tokens;
    }
}

/// Model configuration for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model identifier (e.g., "gemini-2.5-flash", "gpt-4o")
    pub model: String,
    /// Provider type
    #[serde(default)]
    pub provider: ModelProvider,
    /// API endpoint (if custom)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    /// API key (if not using environment variable)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Temperature (0.0 - 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Max output tokens
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Top-p sampling
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

fn default_temperature() -> f32 {
    0.7
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model: "gemini-2.5-flash".to_string(),
            provider: ModelProvider::Google,
            endpoint: None,
            api_key: None,
            temperature: 0.7,
            max_tokens: None,
            top_p: None,
        }
    }
}

/// Supported model providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelProvider {
    // Cloud Providers
    #[default]
    Google,
    OpenAI,
    Anthropic,
    Azure,
    Groq,
    Together,
    Fireworks,
    DeepSeek,
    Mistral,
    Cohere,
    Perplexity,

    // Local Providers
    Ollama,
    LMStudio,
    Jan,
    GPT4All,
    LocalAI,
    Llamafile,
    TextGenWebUI,
    VLLM,
    KoboldCpp,
    TabbyML,
    Exo,

    // Generic
    OpenAICompatible,
    Custom,
}

impl ModelProvider {
    /// Get the default endpoint for this provider
    pub fn default_endpoint(&self) -> Option<&'static str> {
        match self {
            // Cloud Providers
            ModelProvider::Google => Some("https://generativelanguage.googleapis.com/v1"),
            ModelProvider::OpenAI => Some("https://api.openai.com/v1"),
            ModelProvider::Anthropic => Some("https://api.anthropic.com/v1"),
            ModelProvider::Azure => None, // Requires custom endpoint
            ModelProvider::Groq => Some("https://api.groq.com/openai/v1"),
            ModelProvider::Together => Some("https://api.together.xyz/v1"),
            ModelProvider::Fireworks => Some("https://api.fireworks.ai/inference/v1"),
            ModelProvider::DeepSeek => Some("https://api.deepseek.com/v1"),
            ModelProvider::Mistral => Some("https://api.mistral.ai/v1"),
            ModelProvider::Cohere => Some("https://api.cohere.ai/v1"),
            ModelProvider::Perplexity => Some("https://api.perplexity.ai"),

            // Local Providers
            ModelProvider::Ollama => Some("http://localhost:11434"),
            ModelProvider::LMStudio => Some("http://localhost:1234/v1"),
            ModelProvider::Jan => Some("http://localhost:1337/v1"),
            ModelProvider::GPT4All => Some("http://localhost:4891/v1"),
            ModelProvider::LocalAI => Some("http://localhost:8080/v1"),
            ModelProvider::Llamafile => Some("http://localhost:8080/v1"),
            ModelProvider::TextGenWebUI => Some("http://localhost:5000/v1"),
            ModelProvider::VLLM => Some("http://localhost:8000/v1"),
            ModelProvider::KoboldCpp => Some("http://localhost:5001/v1"),
            ModelProvider::TabbyML => Some("http://localhost:8080/v1"),
            ModelProvider::Exo => Some("http://localhost:52415/v1"),

            // Generic
            ModelProvider::OpenAICompatible => None, // Requires custom endpoint
            ModelProvider::Custom => None,
        }
    }

    /// Check if this provider is a local provider
    pub fn is_local(&self) -> bool {
        matches!(
            self,
            ModelProvider::Ollama
                | ModelProvider::LMStudio
                | ModelProvider::Jan
                | ModelProvider::GPT4All
                | ModelProvider::LocalAI
                | ModelProvider::Llamafile
                | ModelProvider::TextGenWebUI
                | ModelProvider::VLLM
                | ModelProvider::KoboldCpp
                | ModelProvider::TabbyML
                | ModelProvider::Exo
        )
    }

    /// Check if this provider uses OpenAI-compatible API
    pub fn is_openai_compatible(&self) -> bool {
        matches!(
            self,
            ModelProvider::OpenAI
                | ModelProvider::Azure
                | ModelProvider::Groq
                | ModelProvider::Together
                | ModelProvider::Fireworks
                | ModelProvider::DeepSeek
                | ModelProvider::Mistral
                | ModelProvider::Perplexity
                | ModelProvider::LMStudio
                | ModelProvider::Jan
                | ModelProvider::GPT4All
                | ModelProvider::LocalAI
                | ModelProvider::Llamafile
                | ModelProvider::TextGenWebUI
                | ModelProvider::VLLM
                | ModelProvider::KoboldCpp
                | ModelProvider::TabbyML
                | ModelProvider::Exo
                | ModelProvider::OpenAICompatible
        )
    }
}

impl std::fmt::Display for ModelProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Cloud Providers
            ModelProvider::Google => write!(f, "google"),
            ModelProvider::OpenAI => write!(f, "openai"),
            ModelProvider::Anthropic => write!(f, "anthropic"),
            ModelProvider::Azure => write!(f, "azure"),
            ModelProvider::Groq => write!(f, "groq"),
            ModelProvider::Together => write!(f, "together"),
            ModelProvider::Fireworks => write!(f, "fireworks"),
            ModelProvider::DeepSeek => write!(f, "deepseek"),
            ModelProvider::Mistral => write!(f, "mistral"),
            ModelProvider::Cohere => write!(f, "cohere"),
            ModelProvider::Perplexity => write!(f, "perplexity"),
            // Local Providers
            ModelProvider::Ollama => write!(f, "ollama"),
            ModelProvider::LMStudio => write!(f, "lmstudio"),
            ModelProvider::Jan => write!(f, "jan"),
            ModelProvider::GPT4All => write!(f, "gpt4all"),
            ModelProvider::LocalAI => write!(f, "localai"),
            ModelProvider::Llamafile => write!(f, "llamafile"),
            ModelProvider::TextGenWebUI => write!(f, "textgenwebui"),
            ModelProvider::VLLM => write!(f, "vllm"),
            ModelProvider::KoboldCpp => write!(f, "koboldcpp"),
            ModelProvider::TabbyML => write!(f, "tabbyml"),
            ModelProvider::Exo => write!(f, "exo"),
            // Generic
            ModelProvider::OpenAICompatible => write!(f, "openai_compatible"),
            ModelProvider::Custom => write!(f, "custom"),
        }
    }
}
