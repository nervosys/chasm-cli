// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Session format conversion utilities
//!
//! Converts between different chat session formats:
//! - VS Code Copilot Chat format
//! - OpenAI API format
//! - Ollama format
//! - Generic markdown format

use crate::models::{ChatMessage, ChatRequest, ChatSession};
use serde::{Deserialize, Serialize};

/// Generic message format for import/export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericMessage {
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub timestamp: Option<i64>,
    #[serde(default)]
    pub model: Option<String>,
}

/// Generic session format for import/export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericSession {
    pub id: String,
    pub title: Option<String>,
    pub messages: Vec<GenericMessage>,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub updated_at: Option<i64>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

impl From<ChatSession> for GenericSession {
    fn from(session: ChatSession) -> Self {
        let mut messages = Vec::new();

        for request in session.requests {
            // Add user message
            if let Some(msg) = &request.message {
                if let Some(text) = &msg.text {
                    messages.push(GenericMessage {
                        role: "user".to_string(),
                        content: text.clone(),
                        timestamp: request.timestamp,
                        model: request.model_id.clone(),
                    });
                }
            }

            // Add assistant response
            if let Some(response) = &request.response {
                if let Some(text) = extract_response_text(response) {
                    messages.push(GenericMessage {
                        role: "assistant".to_string(),
                        content: text,
                        timestamp: request.timestamp,
                        model: request.model_id.clone(),
                    });
                }
            }
        }

        GenericSession {
            id: session
                .session_id
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            title: session.custom_title,
            messages,
            created_at: Some(session.creation_date),
            updated_at: Some(session.last_message_date),
            provider: session.responder_username,
            model: None,
        }
    }
}

impl From<GenericSession> for ChatSession {
    fn from(generic: GenericSession) -> Self {
        let now = chrono::Utc::now().timestamp_millis();

        let mut requests = Vec::new();
        let mut user_msg: Option<(String, Option<i64>, Option<String>)> = None;

        for msg in generic.messages {
            match msg.role.as_str() {
                "user" => {
                    user_msg = Some((msg.content, msg.timestamp, msg.model));
                }
                "assistant" => {
                    if let Some((user_text, timestamp, model)) = user_msg.take() {
                        requests.push(ChatRequest {
                            timestamp: timestamp.or(Some(now)),
                            message: Some(ChatMessage {
                                text: Some(user_text),
                                parts: None,
                            }),
                            response: Some(serde_json::json!({
                                "value": [{"value": msg.content}]
                            })),
                            variable_data: None,
                            request_id: Some(uuid::Uuid::new_v4().to_string()),
                            response_id: Some(uuid::Uuid::new_v4().to_string()),
                            model_id: model.or(msg.model),
                            agent: None,
                            result: None,
                            followups: None,
                            is_canceled: Some(false),
                            content_references: None,
                            code_citations: None,
                            response_markdown_info: None,
                            source_session: None,
                        });
                    }
                }
                _ => {}
            }
        }

        ChatSession {
            version: 3,
            session_id: Some(generic.id),
            creation_date: generic.created_at.unwrap_or(now),
            last_message_date: generic.updated_at.unwrap_or(now),
            is_imported: true,
            initial_location: "imported".to_string(),
            custom_title: generic.title,
            requester_username: Some("user".to_string()),
            requester_avatar_icon_uri: None,
            responder_username: generic.provider,
            responder_avatar_icon_uri: None,
            requests,
        }
    }
}

/// Convert a session to markdown format
pub fn session_to_markdown(session: &ChatSession) -> String {
    let mut md = String::new();

    // Header
    md.push_str(&format!("# {}\n\n", session.title()));

    if let Some(id) = &session.session_id {
        md.push_str(&format!("Session ID: `{}`\n\n", id));
    }

    md.push_str(&format!(
        "Created: {}\n",
        format_timestamp(session.creation_date)
    ));
    md.push_str(&format!(
        "Last Updated: {}\n\n",
        format_timestamp(session.last_message_date)
    ));

    md.push_str("---\n\n");

    // Messages
    for (i, request) in session.requests.iter().enumerate() {
        // User message
        if let Some(msg) = &request.message {
            if let Some(text) = &msg.text {
                md.push_str(&format!("## User ({})\n\n", i + 1));
                md.push_str(text);
                md.push_str("\n\n");
            }
        }

        // Assistant response
        if let Some(response) = &request.response {
            if let Some(text) = extract_response_text(response) {
                let model = request.model_id.as_deref().unwrap_or("Assistant");
                md.push_str(&format!("## {} ({})\n\n", model, i + 1));
                md.push_str(&text);
                md.push_str("\n\n");
            }
        }

        md.push_str("---\n\n");
    }

    md
}

/// Parse a markdown file into a session
pub fn markdown_to_session(markdown: &str, title: Option<String>) -> ChatSession {
    let now = chrono::Utc::now().timestamp_millis();
    let session_id = uuid::Uuid::new_v4().to_string();

    // Simple parsing - look for ## User and ## Assistant sections
    let mut requests = Vec::new();
    let mut current_user: Option<String> = None;
    let mut current_assistant: Option<String> = None;
    let mut in_user = false;
    let mut in_assistant = false;
    let mut content = String::new();

    for line in markdown.lines() {
        if line.starts_with("## User") {
            // Save previous pair
            if let Some(user) = current_user.take() {
                requests.push(create_request(
                    user,
                    current_assistant.take().unwrap_or_default(),
                    now,
                    None,
                ));
            }
            in_user = true;
            in_assistant = false;
            content.clear();
        } else if line.starts_with("## ") && !line.starts_with("## User") {
            // Assistant or model response
            if in_user {
                current_user = Some(content.trim().to_string());
            }
            in_user = false;
            in_assistant = true;
            content.clear();
        } else if line == "---" {
            if in_assistant {
                current_assistant = Some(content.trim().to_string());
            }
            // Save pair
            if let Some(user) = current_user.take() {
                requests.push(create_request(
                    user,
                    current_assistant.take().unwrap_or_default(),
                    now,
                    None,
                ));
            }
            in_user = false;
            in_assistant = false;
            content.clear();
        } else {
            content.push_str(line);
            content.push('\n');
        }
    }

    // Handle final pair
    if in_user {
        current_user = Some(content.trim().to_string());
    } else if in_assistant {
        current_assistant = Some(content.trim().to_string());
    }
    if let Some(user) = current_user.take() {
        requests.push(create_request(
            user,
            current_assistant.take().unwrap_or_default(),
            now,
            None,
        ));
    }

    ChatSession {
        version: 3,
        session_id: Some(session_id),
        creation_date: now,
        last_message_date: now,
        is_imported: true,
        initial_location: "markdown".to_string(),
        custom_title: title,
        requester_username: Some("user".to_string()),
        requester_avatar_icon_uri: None,
        responder_username: Some("Imported".to_string()),
        responder_avatar_icon_uri: None,
        requests,
    }
}

/// Create a ChatRequest from user/assistant text
fn create_request(
    user_text: String,
    assistant_text: String,
    timestamp: i64,
    model: Option<String>,
) -> ChatRequest {
    ChatRequest {
        timestamp: Some(timestamp),
        message: Some(ChatMessage {
            text: Some(user_text),
            parts: None,
        }),
        response: Some(serde_json::json!({
            "value": [{"value": assistant_text}]
        })),
        variable_data: None,
        request_id: Some(uuid::Uuid::new_v4().to_string()),
        response_id: Some(uuid::Uuid::new_v4().to_string()),
        model_id: model,
        agent: None,
        result: None,
        followups: None,
        is_canceled: Some(false),
        content_references: None,
        code_citations: None,
        response_markdown_info: None,
        source_session: None,
    }
}

/// Extract text from various response formats
fn extract_response_text(response: &serde_json::Value) -> Option<String> {
    // Try direct text field
    if let Some(text) = response.get("text").and_then(|v| v.as_str()) {
        return Some(text.to_string());
    }

    // Try value array format (VS Code Copilot format)
    if let Some(value) = response.get("value").and_then(|v| v.as_array()) {
        let parts: Vec<String> = value
            .iter()
            .filter_map(|v| v.get("value").and_then(|v| v.as_str()))
            .map(String::from)
            .collect();
        if !parts.is_empty() {
            return Some(parts.join("\n"));
        }
    }

    // Try content field (OpenAI format)
    if let Some(content) = response.get("content").and_then(|v| v.as_str()) {
        return Some(content.to_string());
    }

    None
}

/// Format a timestamp for display
fn format_timestamp(timestamp: i64) -> String {
    use chrono::{TimeZone, Utc};

    if timestamp == 0 {
        return "Unknown".to_string();
    }

    let dt = Utc.timestamp_millis_opt(timestamp);
    match dt {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => "Invalid".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_to_markdown() {
        let session = ChatSession {
            version: 3,
            session_id: Some("test-123".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000000000,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: Some("Test Session".to_string()),
            requester_username: Some("user".to_string()),
            requester_avatar_icon_uri: None,
            responder_username: Some("assistant".to_string()),
            responder_avatar_icon_uri: None,
            requests: vec![ChatRequest {
                timestamp: Some(1700000000000),
                message: Some(ChatMessage {
                    text: Some("Hello".to_string()),
                    parts: None,
                }),
                response: Some(serde_json::json!({
                    "value": [{"value": "Hi there!"}]
                })),
                variable_data: None,
                request_id: None,
                response_id: None,
                model_id: Some("gpt-4".to_string()),
                agent: None,
                result: None,
                followups: None,
                is_canceled: None,
                content_references: None,
                code_citations: None,
                response_markdown_info: None,
                source_session: None,
            }],
        };

        let md = session_to_markdown(&session);
        assert!(md.contains("# Test Session"));
        assert!(md.contains("Hello"));
        assert!(md.contains("Hi there!"));
    }

    #[test]
    fn test_generic_session_conversion() {
        let session = ChatSession {
            version: 3,
            session_id: Some("test-123".to_string()),
            creation_date: 1700000000000,
            last_message_date: 1700000000000,
            is_imported: false,
            initial_location: "panel".to_string(),
            custom_title: Some("Test".to_string()),
            requester_username: None,
            requester_avatar_icon_uri: None,
            responder_username: Some("Copilot".to_string()),
            responder_avatar_icon_uri: None,
            requests: vec![],
        };

        let generic: GenericSession = session.clone().into();
        assert_eq!(generic.id, "test-123");
        assert_eq!(generic.title, Some("Test".to_string()));

        let back: ChatSession = generic.into();
        assert_eq!(back.session_id, Some("test-123".to_string()));
    }
}
