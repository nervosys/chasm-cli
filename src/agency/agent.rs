// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Agent Definition and Builder
//!
//! Defines the core Agent structure and provides a fluent builder API.

#![allow(dead_code)]

use crate::agency::models::{ModelConfig, ModelProvider};
use crate::agency::tools::Tool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Agent status during execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    #[default]
    Idle,
    Thinking,
    Executing,
    WaitingForTool,
    WaitingForInput,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Idle => write!(f, "idle"),
            AgentStatus::Thinking => write!(f, "thinking"),
            AgentStatus::Executing => write!(f, "executing"),
            AgentStatus::WaitingForTool => write!(f, "waiting_for_tool"),
            AgentStatus::WaitingForInput => write!(f, "waiting_for_input"),
            AgentStatus::Completed => write!(f, "completed"),
            AgentStatus::Failed => write!(f, "failed"),
            AgentStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Agent role/specialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentRole {
    #[default]
    Assistant,
    Coordinator,
    Researcher,
    Coder,
    Reviewer,
    Analyst,
    Writer,
    Executor,
    /// Proactive household management agent
    Household,
    /// Proactive business/work agent
    Business,
    /// Tester agent
    Tester,
    Custom,
}

/// Agent configuration (serializable)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique agent name
    pub name: String,
    /// Agent description
    pub description: String,
    /// System instruction/prompt
    pub instruction: String,
    /// Agent role
    #[serde(default)]
    pub role: AgentRole,
    /// Model configuration
    #[serde(default)]
    pub model: ModelConfig,
    /// Registered tool names
    #[serde(default)]
    pub tools: Vec<String>,
    /// Sub-agent names (for hierarchical agents)
    #[serde(default)]
    pub sub_agents: Vec<String>,
    /// Output key for pipeline state
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_key: Option<String>,
    /// Maximum iterations for loop agents
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,
    /// Custom metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            name: String::new(),
            description: String::new(),
            instruction: String::new(),
            role: AgentRole::default(),
            model: ModelConfig::default(),
            tools: Vec::new(),
            sub_agents: Vec::new(),
            output_key: None,
            max_iterations: None,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// An AI Agent that can process messages and use tools
#[derive(Debug)]
pub struct Agent {
    /// Agent configuration
    pub config: AgentConfig,
    /// Registered tools (Arc for sharing across threads)
    pub registered_tools: Vec<Arc<Tool>>,
    /// Sub-agents for hierarchical execution
    pub sub_agents: Vec<Arc<Agent>>,
    /// Current status (interior mutability for thread-safe updates)
    status: RwLock<AgentStatus>,
}

impl Clone for Agent {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            registered_tools: self.registered_tools.clone(),
            sub_agents: self.sub_agents.clone(),
            status: RwLock::new(*self.status.read().unwrap()),
        }
    }
}

impl Agent {
    /// Create a new agent with the given configuration
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            registered_tools: Vec::new(),
            sub_agents: Vec::new(),
            status: RwLock::new(AgentStatus::Idle),
        }
    }

    /// Get agent name
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Get agent description
    pub fn description(&self) -> &str {
        &self.config.description
    }

    /// Get system instruction
    pub fn instruction(&self) -> &str {
        &self.config.instruction
    }

    /// Get model configuration
    pub fn model(&self) -> &ModelConfig {
        &self.config.model
    }

    /// Check if agent has tools
    pub fn has_tools(&self) -> bool {
        !self.registered_tools.is_empty()
    }

    /// Check if agent has sub-agents
    pub fn has_sub_agents(&self) -> bool {
        !self.sub_agents.is_empty()
    }

    /// Get tool by name
    pub fn get_tool(&self, name: &str) -> Option<&Arc<Tool>> {
        self.registered_tools.iter().find(|t| t.name == name)
    }

    /// Get current status
    pub fn status(&self) -> AgentStatus {
        *self.status.read().unwrap()
    }

    /// Update status (thread-safe)
    pub fn set_status(&self, status: AgentStatus) {
        *self.status.write().unwrap() = status;
    }

    /// Generate tool definitions for model API
    pub fn tool_definitions(&self) -> Vec<serde_json::Value> {
        self.registered_tools
            .iter()
            .map(|tool| tool.to_function_definition())
            .collect()
    }
}

/// Fluent builder for creating agents
#[derive(Default)]
pub struct AgentBuilder {
    config: AgentConfig,
    tools: Vec<Arc<Tool>>,
    sub_agents: Vec<Arc<Agent>>,
}

impl AgentBuilder {
    /// Create a new builder with the given agent name
    pub fn new(name: impl Into<String>) -> Self {
        let mut builder = Self::default();
        builder.config.name = name.into();
        builder.config.created_at = Utc::now();
        builder.config.updated_at = Utc::now();
        builder
    }

    /// Set agent description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.config.description = description.into();
        self
    }

    /// Set system instruction
    pub fn instruction(mut self, instruction: impl Into<String>) -> Self {
        self.config.instruction = instruction.into();
        self
    }

    /// Set agent role
    pub fn role(mut self, role: AgentRole) -> Self {
        self.config.role = role;
        self
    }

    /// Set model by name (uses default provider based on model name)
    pub fn model(mut self, model: impl Into<String>) -> Self {
        let model_name = model.into();
        let provider = infer_provider(&model_name);
        self.config.model = ModelConfig {
            model: model_name,
            provider,
            ..Default::default()
        };
        self
    }

    /// Set full model configuration
    pub fn model_config(mut self, config: ModelConfig) -> Self {
        self.config.model = config;
        self
    }

    /// Set temperature
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.config.model.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    /// Set max output tokens
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.config.model.max_tokens = Some(max_tokens);
        self
    }

    /// Add a tool
    pub fn tool(mut self, tool: Tool) -> Self {
        let name = tool.name.clone();
        self.tools.push(Arc::new(tool));
        self.config.tools.push(name);
        self
    }

    /// Add multiple tools
    pub fn tools(mut self, tools: impl IntoIterator<Item = Tool>) -> Self {
        for tool in tools {
            self = self.tool(tool);
        }
        self
    }

    /// Add a sub-agent
    pub fn sub_agent(mut self, agent: Agent) -> Self {
        let name = agent.config.name.clone();
        self.sub_agents.push(Arc::new(agent));
        self.config.sub_agents.push(name);
        self
    }

    /// Add multiple sub-agents
    pub fn sub_agents(mut self, agents: impl IntoIterator<Item = Agent>) -> Self {
        for agent in agents {
            self = self.sub_agent(agent);
        }
        self
    }

    /// Set output key for pipeline state
    pub fn output_key(mut self, key: impl Into<String>) -> Self {
        self.config.output_key = Some(key.into());
        self
    }

    /// Set max iterations (for loop agents)
    pub fn max_iterations(mut self, max: u32) -> Self {
        self.config.max_iterations = Some(max);
        self
    }

    /// Add custom metadata
    pub fn metadata(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.config.metadata.insert(key.into(), v);
        }
        self
    }

    /// Build the agent
    pub fn build(self) -> Agent {
        Agent {
            config: self.config,
            registered_tools: self.tools,
            sub_agents: self.sub_agents,
            status: RwLock::new(AgentStatus::Idle),
        }
    }
}

/// Infer provider from model name
fn infer_provider(model: &str) -> ModelProvider {
    let model_lower = model.to_lowercase();
    if model_lower.contains("gemini") || model_lower.contains("palm") {
        ModelProvider::Google
    } else if model_lower.contains("gpt")
        || model_lower.contains("o1")
        || model_lower.contains("davinci")
    {
        ModelProvider::OpenAI
    } else if model_lower.contains("claude") {
        ModelProvider::Anthropic
    } else if model_lower.contains("llama")
        || model_lower.contains("mistral")
        || model_lower.contains("codellama")
    {
        ModelProvider::Ollama
    } else {
        ModelProvider::OpenAICompatible
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_builder() {
        let agent = AgentBuilder::new("test_agent")
            .description("A test agent")
            .instruction("You are a helpful assistant.")
            .model("gemini-2.5-flash")
            .temperature(0.5)
            .build();

        assert_eq!(agent.name(), "test_agent");
        assert_eq!(agent.description(), "A test agent");
        assert_eq!(agent.config.model.model, "gemini-2.5-flash");
        assert_eq!(agent.config.model.temperature, 0.5);
        assert_eq!(agent.config.model.provider, ModelProvider::Google);
    }

    #[test]
    fn test_infer_provider() {
        assert_eq!(infer_provider("gemini-2.5-flash"), ModelProvider::Google);
        assert_eq!(infer_provider("gpt-4o"), ModelProvider::OpenAI);
        assert_eq!(infer_provider("claude-3-opus"), ModelProvider::Anthropic);
        assert_eq!(infer_provider("llama-3.2-8b"), ModelProvider::Ollama);
    }
}
