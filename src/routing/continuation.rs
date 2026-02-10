// Copyright (c) 2024-2027 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Conversation continuation across providers
//!
//! Enables seamless continuation of conversations when switching between
//! different AI providers or models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// Conversation State
// ============================================================================

/// Normalized message format for cross-provider continuity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedMessage {
    /// Unique message ID
    pub id: Uuid,
    /// Role (user, assistant, system, tool)
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Original provider
    pub source_provider: String,
    /// Original model
    pub source_model: Option<String>,
    /// Attachments (images, files)
    pub attachments: Vec<Attachment>,
    /// Tool calls made
    pub tool_calls: Vec<ToolCall>,
    /// Token count
    pub token_count: Option<usize>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Metadata
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

/// Attachment (image, file, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// Attachment ID
    pub id: Uuid,
    /// Attachment type
    pub attachment_type: AttachmentType,
    /// File name
    pub name: Option<String>,
    /// MIME type
    pub mime_type: String,
    /// Content (base64 encoded for binary)
    pub content: String,
    /// URL if hosted
    pub url: Option<String>,
}

/// Attachment type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentType {
    Image,
    File,
    Code,
    Audio,
    Video,
}

/// Tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool call ID
    pub id: String,
    /// Tool name
    pub name: String,
    /// Arguments as JSON
    pub arguments: serde_json::Value,
    /// Result if available
    pub result: Option<String>,
}

// ============================================================================
// Conversation Context
// ============================================================================

/// Portable conversation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    /// Conversation ID
    pub id: Uuid,
    /// Title
    pub title: String,
    /// System prompt
    pub system_prompt: Option<String>,
    /// Normalized messages
    pub messages: Vec<NormalizedMessage>,
    /// Summary for context compression
    pub summary: Option<ConversationSummary>,
    /// Available tools
    pub tools: Vec<ToolDefinition>,
    /// Provider history (list of providers used)
    pub provider_history: Vec<ProviderSwitch>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated
    pub updated_at: DateTime<Utc>,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Conversation summary for context compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    /// Summary text
    pub text: String,
    /// Key topics discussed
    pub topics: Vec<String>,
    /// Important entities mentioned
    pub entities: Vec<String>,
    /// User's apparent goals
    pub goals: Vec<String>,
    /// Messages summarized up to
    pub up_to_message_id: Uuid,
    /// Generated at
    pub generated_at: DateTime<Utc>,
}

/// Tool definition for portable tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    /// Description
    pub description: String,
    /// Parameters schema (JSON Schema)
    pub parameters: serde_json::Value,
}

/// Record of provider switch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSwitch {
    /// From provider
    pub from_provider: String,
    /// From model
    pub from_model: Option<String>,
    /// To provider
    pub to_provider: String,
    /// To model
    pub to_model: Option<String>,
    /// Reason for switch
    pub reason: Option<String>,
    /// Timestamp
    pub switched_at: DateTime<Utc>,
}

// ============================================================================
// Provider Adapters
// ============================================================================

/// Provider-specific message format adapter
pub trait ProviderAdapter: Send + Sync {
    /// Provider name
    fn provider_name(&self) -> &str;

    /// Convert normalized messages to provider format
    fn to_provider_format(&self, context: &ConversationContext) -> ProviderMessages;

    /// Convert provider response to normalized format
    fn from_provider_format(&self, response: &ProviderResponse) -> NormalizedMessage;

    /// Get supported features
    fn capabilities(&self) -> ProviderCapabilities;

    /// Estimate tokens for messages
    fn estimate_tokens(&self, messages: &[NormalizedMessage]) -> usize;
}

/// Provider-specific messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMessages {
    /// Messages in provider format
    pub messages: Vec<serde_json::Value>,
    /// System message (if separate)
    pub system: Option<String>,
    /// Tools in provider format
    pub tools: Option<Vec<serde_json::Value>>,
}

/// Provider response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponse {
    /// Provider name
    pub provider: String,
    /// Model used
    pub model: String,
    /// Response content
    pub content: String,
    /// Tool calls
    pub tool_calls: Vec<ToolCall>,
    /// Usage statistics
    pub usage: Option<UsageStats>,
    /// Raw response
    pub raw: serde_json::Value,
}

/// Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// Provider capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    /// Supports vision/images
    pub vision: bool,
    /// Supports tool/function calling
    pub tools: bool,
    /// Supports system messages
    pub system_messages: bool,
    /// Maximum context length
    pub max_context: usize,
    /// Supports streaming
    pub streaming: bool,
}

// ============================================================================
// OpenAI Adapter
// ============================================================================

/// OpenAI format adapter
pub struct OpenAIAdapter;

impl ProviderAdapter for OpenAIAdapter {
    fn provider_name(&self) -> &str {
        "openai"
    }

    fn to_provider_format(&self, context: &ConversationContext) -> ProviderMessages {
        let mut messages: Vec<serde_json::Value> = vec![];

        // Add system message
        if let Some(ref system) = context.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system
            }));
        }

        // Convert messages
        for msg in &context.messages {
            let role = match msg.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::System => "system",
                MessageRole::Tool => "tool",
            };

            let mut message = serde_json::json!({
                "role": role,
                "content": msg.content
            });

            // Add tool calls for assistant messages
            if !msg.tool_calls.is_empty() && msg.role == MessageRole::Assistant {
                message["tool_calls"] = serde_json::json!(msg.tool_calls.iter().map(|tc| {
                    serde_json::json!({
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments.to_string()
                        }
                    })
                }).collect::<Vec<_>>());
            }

            // Add images for vision
            if !msg.attachments.is_empty() {
                let content_parts: Vec<serde_json::Value> = std::iter::once(
                    serde_json::json!({ "type": "text", "text": msg.content })
                ).chain(msg.attachments.iter().filter(|a| a.attachment_type == AttachmentType::Image).map(|a| {
                    if let Some(ref url) = a.url {
                        serde_json::json!({
                            "type": "image_url",
                            "image_url": { "url": url }
                        })
                    } else {
                        serde_json::json!({
                            "type": "image_url",
                            "image_url": { "url": format!("data:{};base64,{}", a.mime_type, a.content) }
                        })
                    }
                })).collect();

                message["content"] = serde_json::json!(content_parts);
            }

            messages.push(message);
        }

        // Convert tools
        let tools = if !context.tools.is_empty() {
            Some(context.tools.iter().map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters
                    }
                })
            }).collect())
        } else {
            None
        };

        ProviderMessages {
            messages,
            system: None, // Included in messages
            tools,
        }
    }

    fn from_provider_format(&self, response: &ProviderResponse) -> NormalizedMessage {
        NormalizedMessage {
            id: Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: response.content.clone(),
            source_provider: "openai".to_string(),
            source_model: Some(response.model.clone()),
            attachments: vec![],
            tool_calls: response.tool_calls.clone(),
            token_count: response.usage.as_ref().map(|u| u.completion_tokens),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            vision: true,
            tools: true,
            system_messages: true,
            max_context: 128000,
            streaming: true,
        }
    }

    fn estimate_tokens(&self, messages: &[NormalizedMessage]) -> usize {
        // Rough estimation: ~4 chars per token
        messages.iter().map(|m| m.content.len() / 4).sum()
    }
}

// ============================================================================
// Anthropic Adapter
// ============================================================================

/// Anthropic Claude format adapter
pub struct AnthropicAdapter;

impl ProviderAdapter for AnthropicAdapter {
    fn provider_name(&self) -> &str {
        "anthropic"
    }

    fn to_provider_format(&self, context: &ConversationContext) -> ProviderMessages {
        let mut messages: Vec<serde_json::Value> = vec![];

        for msg in &context.messages {
            // Anthropic uses "user" and "assistant" roles only
            let role = match msg.role {
                MessageRole::User | MessageRole::Tool => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::System => continue, // System handled separately
            };

            let mut content_parts: Vec<serde_json::Value> = vec![];

            // Add text content
            content_parts.push(serde_json::json!({
                "type": "text",
                "text": msg.content
            }));

            // Add images
            for attachment in &msg.attachments {
                if attachment.attachment_type == AttachmentType::Image {
                    content_parts.push(serde_json::json!({
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": attachment.mime_type,
                            "data": attachment.content
                        }
                    }));
                }
            }

            messages.push(serde_json::json!({
                "role": role,
                "content": content_parts
            }));
        }

        // Convert tools to Anthropic format
        let tools = if !context.tools.is_empty() {
            Some(context.tools.iter().map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters
                })
            }).collect())
        } else {
            None
        };

        ProviderMessages {
            messages,
            system: context.system_prompt.clone(),
            tools,
        }
    }

    fn from_provider_format(&self, response: &ProviderResponse) -> NormalizedMessage {
        NormalizedMessage {
            id: Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: response.content.clone(),
            source_provider: "anthropic".to_string(),
            source_model: Some(response.model.clone()),
            attachments: vec![],
            tool_calls: response.tool_calls.clone(),
            token_count: response.usage.as_ref().map(|u| u.completion_tokens),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            vision: true,
            tools: true,
            system_messages: true,
            max_context: 200000,
            streaming: true,
        }
    }

    fn estimate_tokens(&self, messages: &[NormalizedMessage]) -> usize {
        messages.iter().map(|m| m.content.len() / 4).sum()
    }
}

// ============================================================================
// Continuation Manager
// ============================================================================

/// Manages conversation continuation across providers
pub struct ContinuationManager {
    /// Provider adapters
    adapters: HashMap<String, Box<dyn ProviderAdapter>>,
    /// Active contexts
    contexts: HashMap<Uuid, ConversationContext>,
}

impl ContinuationManager {
    /// Create a new continuation manager
    pub fn new() -> Self {
        let mut adapters: HashMap<String, Box<dyn ProviderAdapter>> = HashMap::new();
        adapters.insert("openai".to_string(), Box::new(OpenAIAdapter));
        adapters.insert("anthropic".to_string(), Box::new(AnthropicAdapter));

        Self {
            adapters,
            contexts: HashMap::new(),
        }
    }

    /// Register a provider adapter
    pub fn register_adapter(&mut self, adapter: Box<dyn ProviderAdapter>) {
        self.adapters.insert(adapter.provider_name().to_string(), adapter);
    }

    /// Create a new conversation context
    pub fn create_context(&mut self, title: &str, system_prompt: Option<&str>) -> Uuid {
        let id = Uuid::new_v4();
        let context = ConversationContext {
            id,
            title: title.to_string(),
            system_prompt: system_prompt.map(String::from),
            messages: vec![],
            summary: None,
            tools: vec![],
            provider_history: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: HashMap::new(),
        };
        self.contexts.insert(id, context);
        id
    }

    /// Add a message to the context
    pub fn add_message(&mut self, context_id: Uuid, message: NormalizedMessage) -> bool {
        if let Some(context) = self.contexts.get_mut(&context_id) {
            context.messages.push(message);
            context.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Switch provider for a conversation
    pub fn switch_provider(
        &mut self,
        context_id: Uuid,
        to_provider: &str,
        to_model: Option<&str>,
        reason: Option<&str>,
    ) -> Option<ProviderMessages> {
        let context = self.contexts.get_mut(&context_id)?;
        let adapter = self.adapters.get(to_provider)?;

        // Record the switch
        let last_provider = context.provider_history.last();
        let switch = ProviderSwitch {
            from_provider: last_provider.map(|p| p.to_provider.clone()).unwrap_or_default(),
            from_model: last_provider.and_then(|p| p.to_model.clone()),
            to_provider: to_provider.to_string(),
            to_model: to_model.map(String::from),
            reason: reason.map(String::from),
            switched_at: Utc::now(),
        };
        context.provider_history.push(switch);
        context.updated_at = Utc::now();

        // Convert to new provider format
        Some(adapter.to_provider_format(context))
    }

    /// Get context for a provider
    pub fn get_provider_messages(&self, context_id: Uuid, provider: &str) -> Option<ProviderMessages> {
        let context = self.contexts.get(&context_id)?;
        let adapter = self.adapters.get(provider)?;
        Some(adapter.to_provider_format(context))
    }

    /// Process a response from a provider
    pub fn process_response(&mut self, context_id: Uuid, response: &ProviderResponse) -> Option<NormalizedMessage> {
        let adapter = self.adapters.get(&response.provider)?;
        let message = adapter.from_provider_format(response);
        
        if let Some(context) = self.contexts.get_mut(&context_id) {
            context.messages.push(message.clone());
            context.updated_at = Utc::now();
        }

        Some(message)
    }

    /// Get a context
    pub fn get_context(&self, context_id: Uuid) -> Option<&ConversationContext> {
        self.contexts.get(&context_id)
    }

    /// Estimate tokens for a context on a provider
    pub fn estimate_tokens(&self, context_id: Uuid, provider: &str) -> Option<usize> {
        let context = self.contexts.get(&context_id)?;
        let adapter = self.adapters.get(provider)?;
        Some(adapter.estimate_tokens(&context.messages))
    }

    /// Compress context by generating a summary
    pub fn compress_context(&mut self, context_id: Uuid, summary_text: &str, topics: Vec<String>) -> bool {
        if let Some(context) = self.contexts.get_mut(&context_id) {
            let last_message_id = context.messages.last().map(|m| m.id).unwrap_or(Uuid::nil());
            context.summary = Some(ConversationSummary {
                text: summary_text.to_string(),
                topics,
                entities: vec![],
                goals: vec![],
                up_to_message_id: last_message_id,
                generated_at: Utc::now(),
            });
            context.updated_at = Utc::now();
            true
        } else {
            false
        }
    }
}

impl Default for ContinuationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_context() {
        let mut manager = ContinuationManager::new();
        let id = manager.create_context("Test Conversation", Some("You are helpful."));
        
        let context = manager.get_context(id).unwrap();
        assert_eq!(context.title, "Test Conversation");
        assert_eq!(context.system_prompt.as_deref(), Some("You are helpful."));
    }

    #[test]
    fn test_add_message() {
        let mut manager = ContinuationManager::new();
        let id = manager.create_context("Test", None);

        let message = NormalizedMessage {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: "Hello!".to_string(),
            source_provider: "openai".to_string(),
            source_model: Some("gpt-4".to_string()),
            attachments: vec![],
            tool_calls: vec![],
            token_count: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        assert!(manager.add_message(id, message));
        assert_eq!(manager.get_context(id).unwrap().messages.len(), 1);
    }

    #[test]
    fn test_provider_switch() {
        let mut manager = ContinuationManager::new();
        let id = manager.create_context("Test", Some("System prompt"));

        // Add a message
        let message = NormalizedMessage {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: "Hello!".to_string(),
            source_provider: "openai".to_string(),
            source_model: None,
            attachments: vec![],
            tool_calls: vec![],
            token_count: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };
        manager.add_message(id, message);

        // Switch to Anthropic
        let messages = manager.switch_provider(id, "anthropic", Some("claude-sonnet-4-20250514"), Some("Better for writing"));
        assert!(messages.is_some());

        let context = manager.get_context(id).unwrap();
        assert_eq!(context.provider_history.len(), 1);
        assert_eq!(context.provider_history[0].to_provider, "anthropic");
    }
}
