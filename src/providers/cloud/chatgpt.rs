// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! ChatGPT (OpenAI) cloud provider
//!
//! Fetches conversation history from ChatGPT web interface.
//!
//! ## Authentication
//!
//! Requires either:
//! - API key via `OPENAI_API_KEY` environment variable (for API access)
//! - Session token for web interface access (retrieved from browser cookies)
//!
//! Note: The official API doesn't provide conversation history access.
//! Web scraping requires a session token from browser cookies.

use super::common::{
    build_http_client, CloudConversation, CloudMessage, CloudProvider, FetchOptions,
    HttpClientConfig,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};

const CHATGPT_API_BASE: &str = "https://chatgpt.com/backend-api";

/// Custom deserializer that handles both Unix timestamp (f64) and ISO8601 string
fn deserialize_timestamp<'de, D>(deserializer: D) -> std::result::Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum TimestampFormat {
        Float(f64),
        String(String),
    }

    match TimestampFormat::deserialize(deserializer)? {
        TimestampFormat::Float(f) => Ok(f),
        TimestampFormat::String(s) => {
            // Try to parse as ISO8601
            if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
                Ok(dt.timestamp() as f64)
            } else if let Ok(dt) = s.parse::<DateTime<Utc>>() {
                Ok(dt.timestamp() as f64)
            } else {
                Err(D::Error::custom(format!("Invalid timestamp format: {}", s)))
            }
        }
    }
}

/// Custom deserializer that handles optional timestamps in both formats
fn deserialize_optional_timestamp<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum TimestampFormat {
        Float(f64),
        String(String),
        Null,
    }

    match Option::<TimestampFormat>::deserialize(deserializer)? {
        None => Ok(None),
        Some(TimestampFormat::Null) => Ok(None),
        Some(TimestampFormat::Float(f)) => Ok(Some(f)),
        Some(TimestampFormat::String(s)) => {
            if s.is_empty() {
                return Ok(None);
            }
            // Try to parse as ISO8601
            if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
                Ok(Some(dt.timestamp() as f64))
            } else if let Ok(dt) = s.parse::<DateTime<Utc>>() {
                Ok(Some(dt.timestamp() as f64))
            } else {
                Err(D::Error::custom(format!("Invalid timestamp format: {}", s)))
            }
        }
    }
}

/// ChatGPT provider for fetching conversation history
pub struct ChatGPTProvider {
    api_key: Option<String>,
    session_token: Option<String>,
    access_token: Option<String>,
    client: Option<reqwest::blocking::Client>,
}

impl ChatGPTProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            session_token: None,
            access_token: None,
            client: None,
        }
    }

    /// Create provider with session token from browser cookies
    pub fn with_session_token(session_token: String) -> Self {
        Self {
            api_key: None,
            session_token: Some(session_token),
            access_token: None,
            client: None,
        }
    }

    fn ensure_client(&mut self) -> Result<&reqwest::blocking::Client> {
        if self.client.is_none() {
            let config = HttpClientConfig {
                user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
                ..Default::default()
            };
            self.client = Some(build_http_client(&config)?);
        }
        Ok(self.client.as_ref().unwrap())
    }

    /// Exchange session token for access token
    fn get_access_token(&mut self) -> Result<String> {
        if let Some(ref token) = self.access_token {
            return Ok(token.clone());
        }

        let session_token = self
            .session_token
            .clone()
            .ok_or_else(|| anyhow!("No session token available"))?;

        let client = self.ensure_client()?;

        // Call the session endpoint to get access token
        let response = client
            .get("https://chatgpt.com/api/auth/session")
            .header(
                "Cookie",
                format!("__Secure-next-auth.session-token={}", session_token),
            )
            .header("Accept", "application/json")
            .send()
            .map_err(|e| anyhow!("Failed to get access token: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!(
                "Session endpoint returned {}: {}. Authentication may have expired.",
                status,
                body
            ));
        }

        let session_data: serde_json::Value = response
            .json()
            .map_err(|e| anyhow!("Failed to parse session response: {}", e))?;

        let access_token = session_data
            .get("accessToken")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                anyhow!("No access token in session response - authentication may have expired")
            })?
            .to_string();

        self.access_token = Some(access_token.clone());
        Ok(access_token)
    }

    /// Build authorization header
    fn get_auth_header(&mut self) -> Result<String> {
        if let Some(ref token) = self.access_token {
            return Ok(format!("Bearer {}", token));
        }
        if self.session_token.is_some() {
            let token = self.get_access_token()?;
            return Ok(format!("Bearer {}", token));
        }
        if let Some(ref key) = self.api_key {
            return Ok(format!("Bearer {}", key));
        }
        Err(anyhow!("No authentication credentials available"))
    }
}

#[derive(Debug, Deserialize)]
struct ConversationListResponse {
    items: Vec<ConversationItem>,
    #[serde(default)]
    limit: i32,
    #[serde(default)]
    offset: i32,
    #[serde(default)]
    total: i32,
    #[serde(default)]
    has_missing_conversations: bool,
}

#[derive(Debug, Deserialize)]
struct ConversationItem {
    id: String,
    title: Option<String>,
    #[serde(deserialize_with = "deserialize_timestamp")]
    create_time: f64,
    #[serde(default, deserialize_with = "deserialize_optional_timestamp")]
    update_time: Option<f64>,
    #[serde(default)]
    is_archived: bool,
}

#[derive(Debug, Deserialize)]
struct ConversationDetailResponse {
    title: Option<String>,
    #[serde(deserialize_with = "deserialize_timestamp")]
    create_time: f64,
    #[serde(default, deserialize_with = "deserialize_optional_timestamp")]
    update_time: Option<f64>,
    mapping: std::collections::HashMap<String, MessageNode>,
    #[serde(default)]
    current_node: Option<String>,
    #[serde(default)]
    conversation_id: Option<String>,
    #[serde(default)]
    model: Option<ModelInfo>,
}

#[derive(Debug, Deserialize)]
struct MessageNode {
    id: String,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    children: Vec<String>,
    message: Option<MessageContent>,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    id: String,
    author: AuthorInfo,
    #[serde(default, deserialize_with = "deserialize_optional_timestamp")]
    create_time: Option<f64>,
    content: ContentParts,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct AuthorInfo {
    role: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ContentParts {
    content_type: String,
    #[serde(default)]
    parts: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModelInfo {
    slug: Option<String>,
    max_tokens: Option<i32>,
    title: Option<String>,
}

impl CloudProvider for ChatGPTProvider {
    fn name(&self) -> &'static str {
        "ChatGPT"
    }

    fn api_base_url(&self) -> &str {
        CHATGPT_API_BASE
    }

    fn is_authenticated(&self) -> bool {
        self.api_key.is_some() || self.session_token.is_some() || self.access_token.is_some()
    }

    fn set_credentials(&mut self, api_key: Option<String>, session_token: Option<String>) {
        self.api_key = api_key;
        self.session_token = session_token;
        self.access_token = None; // Clear cached access token when credentials change
    }

    fn list_conversations(&self, options: &FetchOptions) -> Result<Vec<CloudConversation>> {
        // We need mutable self to get access token, so use interior mutability pattern
        // For now, create a new instance - this is a workaround for the trait signature
        let mut provider = ChatGPTProvider {
            api_key: self.api_key.clone(),
            session_token: self.session_token.clone(),
            access_token: self.access_token.clone(),
            client: None,
        };

        if !provider.is_authenticated() {
            return Err(anyhow!(
                "ChatGPT requires authentication. Provide a session token from browser cookies.\n\
                Run 'chasm harvest scan --web' to check browser authentication status."
            ));
        }

        // Try to get access token and list conversations
        let auth_header = provider.get_auth_header()?;
        let client = provider.ensure_client()?;

        let limit = options.limit.unwrap_or(50).min(100);
        let url = format!(
            "{}/conversations?offset=0&limit={}&order=updated",
            CHATGPT_API_BASE, limit
        );

        let response = client
            .get(&url)
            .header("Authorization", &auth_header)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send()
            .map_err(|e| anyhow!("Failed to fetch conversations: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!(
                "ChatGPT API returned {}: {}. Session may have expired - log in to chatgpt.com in your browser.",
                status,
                body
            ));
        }

        let list_response: ConversationListResponse = response
            .json()
            .map_err(|e| anyhow!("Failed to parse conversation list: {}", e))?;

        // Debug: Found {} conversations (total: {})

        let mut conversations = Vec::new();
        for item in list_response.items {
            // Skip archived if not requested
            if item.is_archived && !options.include_archived {
                continue;
            }

            // Apply date filters
            let created = timestamp_to_datetime(item.create_time);
            if let Some(after) = options.after {
                if created < after {
                    continue;
                }
            }
            if let Some(before) = options.before {
                if created > before {
                    continue;
                }
            }

            conversations.push(CloudConversation {
                id: item.id,
                title: item.title,
                created_at: created,
                updated_at: item.update_time.map(timestamp_to_datetime),
                model: None,
                messages: Vec::new(), // Will be populated by fetch_conversation
                metadata: None,
            });
        }

        Ok(conversations)
    }

    fn fetch_conversation(&self, id: &str) -> Result<CloudConversation> {
        let mut provider = ChatGPTProvider {
            api_key: self.api_key.clone(),
            session_token: self.session_token.clone(),
            access_token: self.access_token.clone(),
            client: None,
        };

        if !provider.is_authenticated() {
            return Err(anyhow!("ChatGPT requires authentication"));
        }

        let auth_header = provider.get_auth_header()?;
        let client = provider.ensure_client()?;

        let url = format!("{}/conversation/{}", CHATGPT_API_BASE, id);

        let response = client
            .get(&url)
            .header("Authorization", &auth_header)
            .header("Accept", "application/json")
            .send()
            .map_err(|e| anyhow!("Failed to fetch conversation {}: {}", id, e))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(anyhow!(
                "Failed to fetch conversation {}: HTTP {}",
                id,
                status
            ));
        }

        let detail: ConversationDetailResponse = response
            .json()
            .map_err(|e| anyhow!("Failed to parse conversation {}: {}", id, e))?;

        // Extract messages from the mapping tree
        // Build a map of node IDs to their messages
        let mut message_order: Vec<(String, CloudMessage)> = Vec::new();

        for (node_id, node) in &detail.mapping {
            if let Some(ref msg_content) = node.message {
                let role = &msg_content.author.role;

                // Skip system messages and tool messages
                if role == "system" || role == "tool" {
                    continue;
                }

                let content = msg_content
                    .content
                    .parts
                    .as_ref()
                    .map(|parts| {
                        parts
                            .iter()
                            .filter_map(|p| p.as_str().map(String::from))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .or_else(|| msg_content.content.text.clone())
                    .unwrap_or_default();

                if content.is_empty() {
                    continue;
                }

                let cloud_message = CloudMessage {
                    id: Some(msg_content.id.clone()),
                    role: role.clone(),
                    content,
                    timestamp: msg_content.create_time.map(timestamp_to_datetime),
                    model: detail.model.as_ref().and_then(|m| m.slug.clone()),
                };

                message_order.push((node_id.clone(), cloud_message));
            }
        }

        // Sort messages by timestamp if available
        message_order.sort_by(|a, b| {
            let ts_a = a.1.timestamp.unwrap_or(DateTime::<Utc>::MIN_UTC);
            let ts_b = b.1.timestamp.unwrap_or(DateTime::<Utc>::MIN_UTC);
            ts_a.cmp(&ts_b)
        });

        let messages: Vec<CloudMessage> = message_order.into_iter().map(|(_, msg)| msg).collect();

        Ok(CloudConversation {
            id: id.to_string(),
            title: detail.title,
            created_at: timestamp_to_datetime(detail.create_time),
            updated_at: detail.update_time.map(timestamp_to_datetime),
            model: detail.model.and_then(|m| m.slug),
            messages,
            metadata: None,
        })
    }

    fn api_key_env_var(&self) -> &'static str {
        "OPENAI_API_KEY"
    }
}

/// Parse a ChatGPT export file (JSON format from "Export data" feature)
pub fn parse_chatgpt_export(json_data: &str) -> Result<Vec<CloudConversation>> {
    let conversations: Vec<ChatGPTExportConversation> = serde_json::from_str(json_data)?;

    Ok(conversations
        .into_iter()
        .map(|conv| CloudConversation {
            id: conv.id,
            title: conv.title,
            created_at: timestamp_to_datetime(conv.create_time),
            updated_at: conv.update_time.map(timestamp_to_datetime),
            model: None,
            messages: conv
                .mapping
                .into_iter()
                .filter_map(|(_, node)| {
                    node.message.map(|msg| {
                        let content = msg
                            .content
                            .parts
                            .map(|parts| {
                                parts
                                    .into_iter()
                                    .filter_map(|p| p.as_str().map(String::from))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            })
                            .or(msg.content.text)
                            .unwrap_or_default();

                        CloudMessage {
                            id: Some(msg.id),
                            role: msg.author.role,
                            content,
                            timestamp: msg.create_time.map(timestamp_to_datetime),
                            model: None,
                        }
                    })
                })
                .filter(|m| !m.content.is_empty() && m.role != "system")
                .collect(),
            metadata: None,
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct ChatGPTExportConversation {
    id: String,
    title: Option<String>,
    create_time: f64,
    update_time: Option<f64>,
    mapping: std::collections::HashMap<String, ChatGPTExportNode>,
}

#[derive(Debug, Deserialize)]
struct ChatGPTExportNode {
    message: Option<ChatGPTExportMessage>,
}

#[derive(Debug, Deserialize)]
struct ChatGPTExportMessage {
    id: String,
    author: ChatGPTExportAuthor,
    create_time: Option<f64>,
    content: ChatGPTExportContent,
}

#[derive(Debug, Deserialize)]
struct ChatGPTExportAuthor {
    role: String,
}

#[derive(Debug, Deserialize)]
struct ChatGPTExportContent {
    #[serde(default)]
    parts: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    text: Option<String>,
}

fn timestamp_to_datetime(ts: f64) -> DateTime<Utc> {
    use chrono::TimeZone;
    Utc.timestamp_opt(ts as i64, ((ts.fract()) * 1_000_000_000.0) as u32)
        .single()
        .unwrap_or_else(Utc::now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chatgpt_provider_new() {
        let provider = ChatGPTProvider::new(Some("test-key".to_string()));
        assert_eq!(provider.name(), "ChatGPT");
        assert!(provider.is_authenticated());
    }

    #[test]
    fn test_chatgpt_provider_unauthenticated() {
        let provider = ChatGPTProvider::new(None);
        assert!(!provider.is_authenticated());
    }

    #[test]
    fn test_timestamp_to_datetime() {
        let ts = 1700000000.123;
        let dt = timestamp_to_datetime(ts);
        assert_eq!(dt.timestamp(), 1700000000);
    }
}
