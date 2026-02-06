// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Data models for VS Code chat sessions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// VS Code workspace information
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Workspace hash (folder name in workspaceStorage)
    pub hash: String,
    /// Associated project path
    pub project_path: Option<String>,
    /// Full path to workspace directory
    pub workspace_path: std::path::PathBuf,
    /// Path to chatSessions directory
    pub chat_sessions_path: std::path::PathBuf,
    /// Number of chat session files
    pub chat_session_count: usize,
    /// Whether chatSessions directory exists
    pub has_chat_sessions: bool,
    /// Last modified timestamp
    #[allow(dead_code)]
    pub last_modified: Option<DateTime<Utc>>,
}

/// VS Code workspace.json structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceJson {
    pub folder: Option<String>,
}

/// VS Code Chat Session (version 3 format)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSession {
    /// Session format version
    #[serde(default = "default_version")]
    pub version: u32,

    /// Unique session identifier (may not be present in file, use filename)
    #[serde(default)]
    pub session_id: Option<String>,

    /// Creation timestamp (milliseconds)
    #[serde(default)]
    pub creation_date: i64,

    /// Last message timestamp (milliseconds)
    #[serde(default)]
    pub last_message_date: i64,

    /// Whether this session was imported
    #[serde(default)]
    pub is_imported: bool,

    /// Initial location (panel, terminal, notebook, editor)
    #[serde(default = "default_location")]
    pub initial_location: String,

    /// Custom title set by user
    #[serde(default)]
    pub custom_title: Option<String>,

    /// Requester username
    #[serde(default)]
    pub requester_username: Option<String>,

    /// Requester avatar URI
    #[serde(default)]
    pub requester_avatar_icon_uri: Option<serde_json::Value>,

    /// Responder username
    #[serde(default)]
    pub responder_username: Option<String>,

    /// Responder avatar URI
    #[serde(default)]
    pub responder_avatar_icon_uri: Option<serde_json::Value>,

    /// Chat requests/messages
    #[serde(default)]
    pub requests: Vec<ChatRequest>,
}

impl ChatSession {
    /// Collect all text content from the session (user messages and responses)
    pub fn collect_all_text(&self) -> String {
        self.requests
            .iter()
            .flat_map(|req| {
                let mut texts = Vec::new();
                if let Some(msg) = &req.message {
                    if let Some(text) = &msg.text {
                        texts.push(text.as_str());
                    }
                }
                if let Some(resp) = &req.response {
                    if let Some(result) = resp.get("result").and_then(|v| v.as_str()) {
                        texts.push(result);
                    }
                }
                texts
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Get user message texts
    pub fn user_messages(&self) -> Vec<&str> {
        self.requests
            .iter()
            .filter_map(|req| req.message.as_ref().and_then(|m| m.text.as_deref()))
            .collect()
    }

    /// Get assistant response texts
    pub fn assistant_responses(&self) -> Vec<String> {
        self.requests
            .iter()
            .filter_map(|req| {
                req.response.as_ref().and_then(|r| {
                    r.get("result")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
            })
            .collect()
    }
}

fn default_version() -> u32 {
    3
}

fn default_location() -> String {
    "panel".to_string()
}

/// A single chat request (message + response)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    /// Request timestamp (milliseconds)
    #[serde(default)]
    pub timestamp: Option<i64>,

    /// The user's message
    #[serde(default)]
    pub message: Option<ChatMessage>,

    /// The AI's response (complex structure - use Value for flexibility)
    #[serde(default)]
    pub response: Option<serde_json::Value>,

    /// Variable data (context, files, etc.)
    #[serde(default)]
    pub variable_data: Option<serde_json::Value>,

    /// Request ID
    #[serde(default)]
    pub request_id: Option<String>,

    /// Response ID
    #[serde(default)]
    pub response_id: Option<String>,

    /// Model ID
    #[serde(default)]
    pub model_id: Option<String>,

    /// Agent information
    #[serde(default)]
    pub agent: Option<serde_json::Value>,

    /// Result metadata
    #[serde(default)]
    pub result: Option<serde_json::Value>,

    /// Follow-up suggestions
    #[serde(default)]
    pub followups: Option<Vec<serde_json::Value>>,

    /// Whether canceled
    #[serde(default)]
    pub is_canceled: Option<bool>,

    /// Content references
    #[serde(default)]
    pub content_references: Option<Vec<serde_json::Value>>,

    /// Code citations
    #[serde(default)]
    pub code_citations: Option<Vec<serde_json::Value>>,

    /// Response markdown info
    #[serde(default)]
    pub response_markdown_info: Option<Vec<serde_json::Value>>,

    /// Source session for merged requests
    #[serde(rename = "_sourceSession", skip_serializing_if = "Option::is_none")]
    pub source_session: Option<String>,
}

/// User message in a chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    /// Message text
    #[serde(alias = "content")]
    pub text: Option<String>,

    /// Message parts (for complex messages)
    #[serde(default)]
    pub parts: Option<Vec<serde_json::Value>>,
}

impl ChatMessage {
    /// Get the text content of this message
    pub fn get_text(&self) -> String {
        self.text.clone().unwrap_or_default()
    }
}

/// AI response in a chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ChatResponse {
    /// Response text
    #[serde(alias = "content")]
    pub text: Option<String>,

    /// Response parts
    #[serde(default)]
    pub parts: Option<Vec<serde_json::Value>>,

    /// Result metadata
    #[serde(default)]
    pub result: Option<serde_json::Value>,
}

/// VS Code chat session index (stored in state.vscdb)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionIndex {
    /// Index version
    #[serde(default = "default_index_version")]
    pub version: u32,

    /// Session entries keyed by session ID
    #[serde(default)]
    pub entries: HashMap<String, ChatSessionIndexEntry>,
}

fn default_index_version() -> u32 {
    1
}

impl Default for ChatSessionIndex {
    fn default() -> Self {
        Self {
            version: 1,
            entries: HashMap::new(),
        }
    }
}

/// Entry in the chat session index
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSessionIndexEntry {
    /// Session ID
    pub session_id: String,

    /// Session title
    pub title: String,

    /// Last message timestamp (milliseconds)
    pub last_message_date: i64,

    /// Whether this session was imported
    #[serde(default)]
    pub is_imported: bool,

    /// Initial location (panel, terminal, etc.)
    #[serde(default = "default_location")]
    pub initial_location: String,

    /// Whether the session is empty
    #[serde(default)]
    pub is_empty: bool,
}

/// Session with its file path for internal processing
#[derive(Debug, Clone)]
pub struct SessionWithPath {
    pub path: std::path::PathBuf,
    pub session: ChatSession,
}

impl SessionWithPath {
    /// Get the session ID from the session data or from the filename
    #[allow(dead_code)]
    pub fn get_session_id(&self) -> String {
        self.session.session_id.clone().unwrap_or_else(|| {
            self.path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
        })
    }
}

impl ChatSession {
    /// Get the session ID (from field or will need to be set from filename)
    #[allow(dead_code)]
    pub fn get_session_id(&self) -> String {
        self.session_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// Get the title for this session (from custom_title or first message)
    pub fn title(&self) -> String {
        // First try custom_title
        if let Some(title) = &self.custom_title {
            if !title.is_empty() {
                return title.clone();
            }
        }

        // Try to extract from first message
        if let Some(first_req) = self.requests.first() {
            if let Some(msg) = &first_req.message {
                if let Some(text) = &msg.text {
                    // Truncate to first 50 chars
                    let title: String = text.chars().take(50).collect();
                    if !title.is_empty() {
                        if title.len() < text.len() {
                            return format!("{}...", title);
                        }
                        return title;
                    }
                }
            }
        }

        "Untitled".to_string()
    }

    /// Check if this session is empty
    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    /// Get the request count
    pub fn request_count(&self) -> usize {
        self.requests.len()
    }

    /// Get the timestamp range of requests
    pub fn timestamp_range(&self) -> Option<(i64, i64)> {
        if self.requests.is_empty() {
            return None;
        }

        let timestamps: Vec<i64> = self.requests.iter().filter_map(|r| r.timestamp).collect();

        if timestamps.is_empty() {
            return None;
        }

        let min = *timestamps.iter().min().unwrap();
        let max = *timestamps.iter().max().unwrap();
        Some((min, max))
    }
}
