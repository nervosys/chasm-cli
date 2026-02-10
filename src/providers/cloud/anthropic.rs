// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Anthropic (Claude) cloud provider
//!
//! Fetches conversation history from Claude web interface.
//!
//! ## Authentication
//!
//! Requires either:
//! - API key via `ANTHROPIC_API_KEY` environment variable
//! - Session token for web interface access
//!
//! Note: The official Anthropic API is stateless and doesn't store conversations.
//! Web conversation history requires session authentication.

use super::common::{
    build_http_client, CloudConversation, CloudMessage, CloudProvider, FetchOptions,
    HttpClientConfig,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;

const ANTHROPIC_WEB_API: &str = "https://claude.ai/api";

/// Anthropic Claude provider for fetching conversation history
pub struct AnthropicProvider {
    api_key: Option<String>,
    session_token: Option<String>,
    organization_id: Option<String>,
    client: Option<reqwest::blocking::Client>,
}

impl AnthropicProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            session_token: None,
            organization_id: None,
            client: None,
        }
    }

    /// Create provider with session token from browser cookies
    pub fn with_session_token(session_token: String) -> Self {
        Self {
            api_key: None,
            session_token: Some(session_token),
            organization_id: None,
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

    /// Get organization ID from the bootstrap endpoint
    fn get_organization_id(&mut self) -> Result<String> {
        if let Some(ref org_id) = self.organization_id {
            return Ok(org_id.clone());
        }

        let session_token = self
            .session_token
            .clone()
            .ok_or_else(|| anyhow!("No session token available"))?;

        let client = self.ensure_client()?;

        // Get organization info from bootstrap
        let response = client
            .get("https://claude.ai/api/bootstrap")
            .header("Cookie", format!("sessionKey={}", session_token))
            .header("Accept", "application/json")
            .send()
            .map_err(|e| anyhow!("Failed to get organization info: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Bootstrap endpoint returned {}: authentication may have expired",
                response.status()
            ));
        }

        let bootstrap: serde_json::Value = response
            .json()
            .map_err(|e| anyhow!("Failed to parse bootstrap response: {}", e))?;

        // Try to get organization UUID from various paths
        let org_id = bootstrap
            .get("account")
            .and_then(|a| a.get("memberships"))
            .and_then(|m| m.as_array())
            .and_then(|arr| arr.first())
            .and_then(|m| m.get("organization"))
            .and_then(|o| o.get("uuid"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Could not find organization ID in bootstrap response"))?
            .to_string();

        self.organization_id = Some(org_id.clone());
        Ok(org_id)
    }
}

#[derive(Debug, Deserialize)]
struct ClaudeConversationList {
    conversations: Vec<ClaudeConversationSummary>,
}

#[derive(Debug, Deserialize)]
struct ClaudeConversationSummary {
    uuid: String,
    name: Option<String>,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeConversationDetail {
    uuid: String,
    name: Option<String>,
    created_at: String,
    updated_at: String,
    chat_messages: Vec<ClaudeMessage>,
    #[serde(default)]
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeMessage {
    uuid: String,
    text: String,
    sender: String, // "human" or "assistant"
    created_at: String,
    #[serde(default)]
    attachments: Vec<serde_json::Value>,
}

impl CloudProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "Claude"
    }

    fn api_base_url(&self) -> &str {
        ANTHROPIC_WEB_API
    }

    fn is_authenticated(&self) -> bool {
        self.api_key.is_some() || self.session_token.is_some()
    }

    fn set_credentials(&mut self, api_key: Option<String>, session_token: Option<String>) {
        self.api_key = api_key;
        self.session_token = session_token;
        self.organization_id = None; // Clear cached org ID
    }

    fn list_conversations(&self, options: &FetchOptions) -> Result<Vec<CloudConversation>> {
        let mut provider = AnthropicProvider {
            api_key: self.api_key.clone(),
            session_token: self.session_token.clone(),
            organization_id: self.organization_id.clone(),
            client: None,
        };

        if !provider.is_authenticated() {
            return Err(anyhow!(
                "Claude requires authentication. Provide a session token from browser cookies.\n\
                Run 'chasm harvest scan --web' to check browser authentication status."
            ));
        }

        if provider.session_token.is_none() {
            return Err(anyhow!(
                "Claude conversation history requires web session authentication.\n\
                The Anthropic API is stateless and doesn't store conversation history."
            ));
        }

        let session_token = provider.session_token.clone().unwrap();
        let org_id = provider.get_organization_id()?;
        let client = provider.ensure_client()?;

        let url = format!(
            "{}/organizations/{}/chat_conversations",
            ANTHROPIC_WEB_API, org_id
        );

        let response = client
            .get(&url)
            .header("Cookie", format!("sessionKey={}", session_token))
            .header("Accept", "application/json")
            .send()
            .map_err(|e| anyhow!("Failed to fetch conversations: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(anyhow!(
                "Claude API returned {}: session may have expired - log in to claude.ai in your browser.",
                status
            ));
        }

        let conversations: Vec<ClaudeConversationSummary> = response
            .json()
            .map_err(|e| anyhow!("Failed to parse conversation list: {}", e))?;

        let mut result = Vec::new();
        let limit = options.limit.unwrap_or(usize::MAX);

        for conv in conversations.into_iter().take(limit) {
            let created = parse_iso_timestamp(&conv.created_at)?;
            let updated = parse_iso_timestamp(&conv.updated_at).ok();

            // Apply date filters
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

            result.push(CloudConversation {
                id: conv.uuid,
                title: conv.name,
                created_at: created,
                updated_at: updated,
                model: conv.model,
                messages: Vec::new(),
                metadata: None,
            });
        }

        Ok(result)
    }

    fn fetch_conversation(&self, id: &str) -> Result<CloudConversation> {
        let mut provider = AnthropicProvider {
            api_key: self.api_key.clone(),
            session_token: self.session_token.clone(),
            organization_id: self.organization_id.clone(),
            client: None,
        };

        if provider.session_token.is_none() {
            return Err(anyhow!(
                "Claude requires session token for conversation details"
            ));
        }

        let session_token = provider.session_token.clone().unwrap();
        let org_id = provider.get_organization_id()?;
        let client = provider.ensure_client()?;

        let url = format!(
            "{}/organizations/{}/chat_conversations/{}",
            ANTHROPIC_WEB_API, org_id, id
        );

        let response = client
            .get(&url)
            .header("Cookie", format!("sessionKey={}", session_token))
            .header("Accept", "application/json")
            .send()
            .map_err(|e| anyhow!("Failed to fetch conversation {}: {}", id, e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch conversation {}: HTTP {}",
                id,
                response.status()
            ));
        }

        let detail: ClaudeConversationDetail = response
            .json()
            .map_err(|e| anyhow!("Failed to parse conversation {}: {}", id, e))?;

        let messages: Vec<CloudMessage> = detail
            .chat_messages
            .into_iter()
            .map(|msg| CloudMessage {
                id: Some(msg.uuid),
                role: if msg.sender == "human" {
                    "user".to_string()
                } else {
                    "assistant".to_string()
                },
                content: msg.text,
                timestamp: parse_iso_timestamp(&msg.created_at).ok(),
                model: detail.model.clone(),
            })
            .collect();

        Ok(CloudConversation {
            id: detail.uuid,
            title: detail.name,
            created_at: parse_iso_timestamp(&detail.created_at)?,
            updated_at: parse_iso_timestamp(&detail.updated_at).ok(),
            model: detail.model,
            messages,
            metadata: None,
        })
    }

    fn api_key_env_var(&self) -> &'static str {
        "ANTHROPIC_API_KEY"
    }
}

/// Parse a Claude export file (if available)
pub fn parse_claude_export(json_data: &str) -> Result<Vec<CloudConversation>> {
    // Claude doesn't have an official export format yet
    // This is a placeholder for when/if they add one
    let conversations: Vec<ClaudeExportConversation> = serde_json::from_str(json_data)?;

    Ok(conversations
        .into_iter()
        .map(|conv| CloudConversation {
            id: conv.uuid,
            title: conv.name,
            created_at: parse_iso_timestamp(&conv.created_at).unwrap_or_else(|_| Utc::now()),
            updated_at: Some(parse_iso_timestamp(&conv.updated_at).unwrap_or_else(|_| Utc::now())),
            model: conv.model,
            messages: conv
                .messages
                .into_iter()
                .map(|msg| CloudMessage {
                    id: Some(msg.uuid),
                    role: if msg.sender == "human" {
                        "user".to_string()
                    } else {
                        "assistant".to_string()
                    },
                    content: msg.text,
                    timestamp: parse_iso_timestamp(&msg.created_at).ok(),
                    model: None,
                })
                .collect(),
            metadata: None,
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct ClaudeExportConversation {
    uuid: String,
    name: Option<String>,
    created_at: String,
    updated_at: String,
    messages: Vec<ClaudeExportMessage>,
    #[serde(default)]
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeExportMessage {
    uuid: String,
    text: String,
    sender: String,
    created_at: String,
}

fn parse_iso_timestamp(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| anyhow!("Failed to parse timestamp: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_anthropic_provider_new() {
        let provider = AnthropicProvider::new(Some("test-key".to_string()));
        assert_eq!(provider.name(), "Claude");
        assert!(provider.is_authenticated());
    }

    #[test]
    fn test_anthropic_provider_unauthenticated() {
        let provider = AnthropicProvider::new(None);
        assert!(!provider.is_authenticated());
    }

    #[test]
    fn test_parse_iso_timestamp() {
        let ts = "2024-01-15T10:30:00Z";
        let dt = parse_iso_timestamp(ts).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }
}
