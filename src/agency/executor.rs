// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Agent Execution
//!
//! Handles the execution of individual agents with tool calling.

#![allow(dead_code)]

use crate::agency::agent::{Agent, AgentStatus};
use crate::agency::error::{AgencyError, AgencyResult};
use crate::agency::models::{
    AgencyEvent, AgencyMessage, EventType, MessageRole, TokenUsage, ToolCall, ToolResult,
};
use crate::agency::session::{generate_message_id, Session};
use crate::agency::tools::ToolRegistry;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Execution context passed to tools
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Session ID
    pub session_id: String,
    /// Agent name
    pub agent_name: String,
    /// User ID
    pub user_id: Option<String>,
    /// Current state
    pub state: HashMap<String, serde_json::Value>,
    /// Whether to allow tool execution
    pub allow_tools: bool,
    /// Maximum tool calls per turn
    pub max_tool_calls: u32,
    /// Event sender for streaming
    pub event_sender: Option<mpsc::Sender<AgencyEvent>>,
}

impl ExecutionContext {
    pub fn new(session: &Session) -> Self {
        Self {
            session_id: session.id.clone(),
            agent_name: session.agent_name.clone(),
            user_id: session.user_id.clone(),
            state: session.state.data.clone(),
            allow_tools: true,
            max_tool_calls: 10,
            event_sender: None,
        }
    }

    /// Send an event to listeners
    pub async fn emit(&self, event: AgencyEvent) {
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(event).await;
        }
    }
}

/// Result of agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Final response text
    pub response: String,
    /// Messages generated during execution
    pub messages: Vec<AgencyMessage>,
    /// Events emitted
    pub events: Vec<AgencyEvent>,
    /// Token usage
    pub token_usage: TokenUsage,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Whether execution completed successfully
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Agent executor handles running an agent with optional tool calling
pub struct Executor {
    tool_registry: Arc<ToolRegistry>,
}

impl Executor {
    /// Create a new executor with the given tool registry
    pub fn new(tool_registry: Arc<ToolRegistry>) -> Self {
        Self { tool_registry }
    }

    /// Execute an agent with a user message
    pub async fn execute(
        &self,
        agent: &Agent,
        session: &mut Session,
        user_message: &str,
        ctx: &mut ExecutionContext,
    ) -> AgencyResult<ExecutionResult> {
        let start_time = std::time::Instant::now();
        let mut messages = Vec::new();
        let mut events = Vec::new();
        let mut token_usage = TokenUsage::default();

        // Set agent status
        agent.set_status(AgentStatus::Thinking);

        // Emit start event
        let start_event = AgencyEvent {
            event_type: EventType::AgentStarted,
            agent_name: agent.name().to_string(),
            data: serde_json::json!({ "message": user_message }),
            timestamp: Utc::now(),
            session_id: Some(session.id.clone()),
        };
        events.push(start_event.clone());
        ctx.emit(start_event).await;

        // Add user message
        let user_msg = AgencyMessage {
            id: generate_message_id(),
            role: MessageRole::User,
            content: user_message.to_string(),
            tool_calls: vec![],
            tool_result: None,
            timestamp: Utc::now(),
            tokens: None,
            agent_name: Some(agent.name().to_string()),
            metadata: HashMap::new(),
        };
        session.add_message(user_msg.clone());
        messages.push(user_msg);

        // Execute with tool loop
        let mut tool_call_count = 0;
        #[allow(unused_assignments)]
        let mut final_response = String::new();

        loop {
            // Call the model
            agent.set_status(AgentStatus::Thinking);
            let thinking_event = AgencyEvent {
                event_type: EventType::AgentThinking,
                agent_name: agent.name().to_string(),
                data: serde_json::json!({}),
                timestamp: Utc::now(),
                session_id: Some(session.id.clone()),
            };
            events.push(thinking_event.clone());
            ctx.emit(thinking_event).await;

            // Call the model with the current session context
            let model_response = self.call_model(agent, session).await?;

            token_usage.add(&model_response.usage);

            // Check for tool calls
            if !model_response.tool_calls.is_empty() && ctx.allow_tools {
                agent.set_status(AgentStatus::WaitingForTool);

                for tool_call in &model_response.tool_calls {
                    tool_call_count += 1;
                    if tool_call_count > ctx.max_tool_calls {
                        return Err(AgencyError::MaxIterationsExceeded(ctx.max_tool_calls));
                    }

                    // Emit tool call event
                    let call_event = AgencyEvent {
                        event_type: EventType::ToolCallStarted,
                        agent_name: agent.name().to_string(),
                        data: serde_json::json!({
                            "tool": tool_call.name,
                            "arguments": tool_call.arguments
                        }),
                        timestamp: Utc::now(),
                        session_id: Some(session.id.clone()),
                    };
                    events.push(call_event.clone());
                    ctx.emit(call_event).await;

                    // Execute tool
                    agent.set_status(AgentStatus::Executing);
                    let tool_result = self.execute_tool(tool_call).await;

                    // Emit tool result event
                    let result_event = AgencyEvent {
                        event_type: EventType::ToolCallCompleted,
                        agent_name: agent.name().to_string(),
                        data: serde_json::json!({
                            "tool": tool_call.name,
                            "success": tool_result.success,
                            "content": tool_result.content
                        }),
                        timestamp: Utc::now(),
                        session_id: Some(session.id.clone()),
                    };
                    events.push(result_event.clone());
                    ctx.emit(result_event).await;

                    // Add tool result message
                    let tool_msg = AgencyMessage {
                        id: generate_message_id(),
                        role: MessageRole::Tool,
                        content: tool_result.content.clone(),
                        tool_calls: vec![],
                        tool_result: Some(tool_result),
                        timestamp: Utc::now(),
                        tokens: None,
                        agent_name: Some(agent.name().to_string()),
                        metadata: HashMap::new(),
                    };
                    session.add_message(tool_msg.clone());
                    messages.push(tool_msg);
                }

                // Continue loop to get model response after tool results
                continue;
            }

            // No tool calls - we have the final response
            final_response = model_response.content.clone();

            // Add assistant message
            let assistant_msg = AgencyMessage {
                id: generate_message_id(),
                role: MessageRole::Assistant,
                content: model_response.content,
                tool_calls: model_response.tool_calls,
                tool_result: None,
                timestamp: Utc::now(),
                tokens: Some(model_response.usage.completion_tokens),
                agent_name: Some(agent.name().to_string()),
                metadata: HashMap::new(),
            };
            session.add_message(assistant_msg.clone());
            messages.push(assistant_msg);

            break;
        }

        // Emit end event
        agent.set_status(AgentStatus::Completed);
        let end_event = AgencyEvent {
            event_type: EventType::AgentCompleted,
            agent_name: agent.name().to_string(),
            data: serde_json::json!({ "response": final_response }),
            timestamp: Utc::now(),
            session_id: Some(session.id.clone()),
        };
        events.push(end_event.clone());
        ctx.emit(end_event).await;

        Ok(ExecutionResult {
            response: final_response,
            messages,
            events,
            token_usage,
            duration_ms: start_time.elapsed().as_millis() as u64,
            success: true,
            error: None,
        })
    }

    /// Call the model using the appropriate provider
    async fn call_model(&self, agent: &Agent, session: &Session) -> AgencyResult<ModelResponse> {
        use crate::agency::models::ModelProvider;

        let messages = session.to_api_messages();
        let tools = agent.tool_definitions();
        let model_config = agent.model();

        // Build request body
        let mut request_body = serde_json::json!({
            "model": model_config.model,
            "messages": messages,
            "temperature": model_config.temperature,
        });

        if let Some(max_tokens) = model_config.max_tokens {
            request_body["max_tokens"] = serde_json::json!(max_tokens);
        }

        if !tools.is_empty() {
            request_body["tools"] = serde_json::json!(tools);
        }

        // Determine endpoint based on provider
        let endpoint = match model_config.provider {
            // Cloud Providers
            ModelProvider::OpenAI => "https://api.openai.com/v1/chat/completions".to_string(),
            ModelProvider::Anthropic => "https://api.anthropic.com/v1/messages".to_string(),
            ModelProvider::Google => format!(
                "https://generativelanguage.googleapis.com/v1/models/{}:generateContent",
                model_config.model
            ),
            ModelProvider::Groq => "https://api.groq.com/openai/v1/chat/completions".to_string(),
            ModelProvider::Together => "https://api.together.xyz/v1/chat/completions".to_string(),
            ModelProvider::Fireworks => {
                "https://api.fireworks.ai/inference/v1/chat/completions".to_string()
            }
            ModelProvider::DeepSeek => "https://api.deepseek.com/v1/chat/completions".to_string(),
            ModelProvider::Mistral => "https://api.mistral.ai/v1/chat/completions".to_string(),
            ModelProvider::Cohere => "https://api.cohere.ai/v1/chat".to_string(),
            ModelProvider::Perplexity => "https://api.perplexity.ai/chat/completions".to_string(),
            ModelProvider::Azure => model_config.endpoint.clone().unwrap_or_default(),

            // Local Providers (OpenAI-compatible)
            ModelProvider::Ollama => model_config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:11434/api/chat".to_string()),
            ModelProvider::LMStudio => model_config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:1234/v1/chat/completions".to_string()),
            ModelProvider::Jan => model_config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:1337/v1/chat/completions".to_string()),
            ModelProvider::GPT4All => model_config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:4891/v1/chat/completions".to_string()),
            ModelProvider::LocalAI => model_config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:8080/v1/chat/completions".to_string()),
            ModelProvider::Llamafile => model_config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:8080/v1/chat/completions".to_string()),
            ModelProvider::TextGenWebUI => model_config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:5000/v1/chat/completions".to_string()),
            ModelProvider::VLLM => model_config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:8000/v1/chat/completions".to_string()),
            ModelProvider::KoboldCpp => model_config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:5001/v1/chat/completions".to_string()),
            ModelProvider::TabbyML => model_config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:8080/v1/chat/completions".to_string()),
            ModelProvider::Exo => model_config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:52415/v1/chat/completions".to_string()),

            // Generic
            ModelProvider::OpenAICompatible | ModelProvider::Custom => model_config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:8080/v1/chat/completions".to_string()),
        };

        if endpoint.is_empty() {
            return Err(AgencyError::ConfigError(
                "No endpoint configured for model provider".to_string(),
            ));
        }

        // Make HTTP request
        let client = reqwest::Client::new();
        let mut request = client.post(&endpoint).json(&request_body);

        // Add authentication
        if let Some(api_key) = &model_config.api_key {
            match model_config.provider {
                ModelProvider::Anthropic => {
                    request = request.header("x-api-key", api_key);
                    request = request.header("anthropic-version", "2023-06-01");
                }
                ModelProvider::Google => {
                    // Google uses query parameter for API key
                    request = client
                        .post(format!("{}?key={}", endpoint, api_key))
                        .json(&request_body);
                }
                _ => {
                    request = request.header("Authorization", format!("Bearer {}", api_key));
                }
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| AgencyError::NetworkError(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body: String = response.text().await.unwrap_or_default();
            return Err(AgencyError::ModelError(format!(
                "Model API error ({}): {}",
                status, body
            )));
        }

        let response_body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AgencyError::ModelError(format!("Failed to parse response: {}", e)))?;

        // Parse response based on provider format
        let (content, tool_calls, usage) =
            Self::parse_model_response(&response_body, &model_config.provider)?;

        Ok(ModelResponse {
            content,
            tool_calls,
            usage,
        })
    }

    /// Parse model response based on provider format
    fn parse_model_response(
        response: &serde_json::Value,
        provider: &crate::agency::models::ModelProvider,
    ) -> AgencyResult<(String, Vec<ToolCall>, TokenUsage)> {
        use crate::agency::models::ModelProvider;

        match provider {
            ModelProvider::Anthropic => {
                // Anthropic format
                let content = response["content"][0]["text"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let usage = TokenUsage::new(
                    response["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
                    response["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
                );
                // Parse tool_use blocks for Anthropic
                let mut tool_calls = vec![];
                if let Some(content_blocks) = response["content"].as_array() {
                    for block in content_blocks {
                        if block["type"].as_str() == Some("tool_use") {
                            tool_calls.push(ToolCall {
                                id: block["id"].as_str().unwrap_or("").to_string(),
                                name: block["name"].as_str().unwrap_or("").to_string(),
                                arguments: block["input"].clone(),
                                timestamp: Utc::now(),
                            });
                        }
                    }
                }
                Ok((content, tool_calls, usage))
            }
            ModelProvider::Google => {
                // Google Gemini format
                let content = response["candidates"][0]["content"]["parts"][0]["text"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let usage = TokenUsage::new(
                    response["usageMetadata"]["promptTokenCount"]
                        .as_u64()
                        .unwrap_or(0) as u32,
                    response["usageMetadata"]["candidatesTokenCount"]
                        .as_u64()
                        .unwrap_or(0) as u32,
                );
                // Parse function calls for Google
                let mut tool_calls = vec![];
                if let Some(parts) = response["candidates"][0]["content"]["parts"].as_array() {
                    for part in parts {
                        if let Some(fn_call) = part.get("functionCall") {
                            tool_calls.push(ToolCall {
                                id: uuid::Uuid::new_v4().to_string(),
                                name: fn_call["name"].as_str().unwrap_or("").to_string(),
                                arguments: fn_call["args"].clone(),
                                timestamp: Utc::now(),
                            });
                        }
                    }
                }
                Ok((content, tool_calls, usage))
            }
            _ => {
                // OpenAI-compatible format (OpenAI, Ollama, Azure, OpenAICompatible, Custom)
                let choice = &response["choices"][0];
                let content = choice["message"]["content"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                let mut tool_calls = vec![];
                if let Some(calls) = choice["message"]["tool_calls"].as_array() {
                    for call in calls {
                        tool_calls.push(ToolCall {
                            id: call["id"].as_str().unwrap_or("").to_string(),
                            name: call["function"]["name"].as_str().unwrap_or("").to_string(),
                            arguments: serde_json::from_str(
                                call["function"]["arguments"].as_str().unwrap_or("{}"),
                            )
                            .unwrap_or_default(),
                            timestamp: Utc::now(),
                        });
                    }
                }

                let usage = TokenUsage::new(
                    response["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                    response["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
                );

                Ok((content, tool_calls, usage))
            }
        }
    }

    /// Execute a tool
    async fn execute_tool(&self, tool_call: &ToolCall) -> ToolResult {
        let start = std::time::Instant::now();

        // Check if tool exists
        if let Some(executor) = self.tool_registry.get_executor(&tool_call.name) {
            match executor.execute(tool_call.arguments.clone()).await {
                Ok(result) => result,
                Err(e) => ToolResult {
                    call_id: tool_call.id.clone(),
                    name: tool_call.name.clone(),
                    success: false,
                    content: format!("Tool execution failed: {}", e),
                    duration_ms: start.elapsed().as_millis() as u64,
                    data: None,
                },
            }
        } else {
            // Tool not found - return error result
            ToolResult {
                call_id: tool_call.id.clone(),
                name: tool_call.name.clone(),
                success: false,
                content: format!("Tool '{}' not found in registry", tool_call.name),
                duration_ms: start.elapsed().as_millis() as u64,
                data: None,
            }
        }
    }
}

/// Response from model API
struct ModelResponse {
    content: String,
    tool_calls: Vec<ToolCall>,
    usage: TokenUsage,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agency::agent::AgentBuilder;

    #[tokio::test]
    #[ignore = "Integration test - requires API credentials"]
    async fn test_executor() {
        let tool_registry = Arc::new(ToolRegistry::new());
        let executor = Executor::new(tool_registry);

        let mut agent = AgentBuilder::new("test_agent")
            .description("Test agent")
            .instruction("You are a helpful assistant.")
            .model("gemini-2.5-flash")
            .build();

        let mut session = Session::new("test_agent", None);
        let mut ctx = ExecutionContext::new(&session);

        let result = executor
            .execute(&mut agent, &mut session, "Hello!", &mut ctx)
            .await
            .unwrap();

        assert!(result.success);
        assert!(!result.response.is_empty());
        assert!(!result.messages.is_empty());
    }
}
