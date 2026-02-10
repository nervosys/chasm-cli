// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Google Gemini cloud provider
//!
//! Fetches conversation history from Google Gemini (formerly Bard).
//!
//! ## Authentication
//!
//! Requires either:
//! - API key via `GOOGLE_API_KEY` or `GEMINI_API_KEY` environment variable
//! - Session token for web interface access

use super::common::{
    build_http_client, CloudConversation, CloudMessage, CloudProvider, FetchOptions,
    HttpClientConfig,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;

const GEMINI_API: &str = "https://generativelanguage.googleapis.com/v1";
const GEMINI_WEB_API: &str = "https://gemini.google.com/_/BardChatUi";

/// Google Gemini provider for fetching conversation history
pub struct GeminiProvider {
    api_key: Option<String>,
    session_token: Option<String>,
    client: Option<reqwest::blocking::Client>,
}

impl GeminiProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            session_token: None,
            client: None,
        }
    }

    fn ensure_client(&mut self) -> Result<&reqwest::blocking::Client> {
        if self.client.is_none() {
            let config = HttpClientConfig::default();
            self.client = Some(build_http_client(&config)?);
        }
        Ok(self.client.as_ref().unwrap())
    }
}

#[derive(Debug, Deserialize)]
struct GeminiConversation {
    #[serde(rename = "conversationId")]
    id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(rename = "createTime")]
    created_at: String,
    #[serde(rename = "updateTime")]
    updated_at: Option<String>,
    #[serde(default)]
    messages: Vec<GeminiMessage>,
}

#[derive(Debug, Deserialize)]
struct GeminiMessage {
    #[serde(default)]
    id: Option<String>,
    content: GeminiContent,
    role: String, // "user" or "model"
    #[serde(rename = "createTime")]
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    #[serde(default)]
    text: Option<String>,
}

impl CloudProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        "Gemini"
    }

    fn api_base_url(&self) -> &str {
        GEMINI_API
    }

    fn is_authenticated(&self) -> bool {
        self.api_key.is_some() || self.session_token.is_some()
    }

    fn set_credentials(&mut self, api_key: Option<String>, session_token: Option<String>) {
        self.api_key = api_key;
        self.session_token = session_token;
    }

    fn list_conversations(&self, _options: &FetchOptions) -> Result<Vec<CloudConversation>> {
        if !self.is_authenticated() {
            return Err(anyhow!(
                "Gemini requires authentication. Set GOOGLE_API_KEY or GEMINI_API_KEY, or provide a session token."
            ));
        }

        eprintln!("Note: Gemini conversation history requires web session authentication.");
        eprintln!("The Gemini API is stateless and doesn't store conversation history.");

        Ok(vec![])
    }

    fn fetch_conversation(&self, _id: &str) -> Result<CloudConversation> {
        if !self.is_authenticated() {
            return Err(anyhow!("Gemini requires authentication"));
        }

        Err(anyhow!(
            "Fetching Gemini conversations requires web session authentication."
        ))
    }

    fn api_key_env_var(&self) -> &'static str {
        "GOOGLE_API_KEY"
    }

    fn load_api_key_from_env(&self) -> Option<String> {
        std::env::var("GOOGLE_API_KEY")
            .or_else(|_| std::env::var("GEMINI_API_KEY"))
            .ok()
    }
}

/// Parse Gemini/Bard export data (Google Takeout format)
pub fn parse_gemini_export(json_data: &str) -> Result<Vec<CloudConversation>> {
    // Google Takeout exports Bard/Gemini data in a specific format
    let conversations: Vec<GeminiExportConversation> = serde_json::from_str(json_data)?;

    Ok(conversations
        .into_iter()
        .map(|conv| CloudConversation {
            id: conv.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            title: conv.title,
            created_at: conv
                .created_at
                .and_then(|s| parse_iso_timestamp(&s).ok())
                .unwrap_or_else(Utc::now),
            updated_at: conv.updated_at.and_then(|s| parse_iso_timestamp(&s).ok()),
            model: Some("gemini".to_string()),
            messages: conv
                .messages
                .into_iter()
                .map(|msg| {
                    let content = msg
                        .content
                        .parts
                        .iter()
                        .filter_map(|p| p.text.clone())
                        .collect::<Vec<_>>()
                        .join("\n");

                    CloudMessage {
                        id: msg.id,
                        role: if msg.role == "model" {
                            "assistant".to_string()
                        } else {
                            msg.role
                        },
                        content,
                        timestamp: msg.created_at.and_then(|s| parse_iso_timestamp(&s).ok()),
                        model: Some("gemini".to_string()),
                    }
                })
                .collect(),
            metadata: None,
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct GeminiExportConversation {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(rename = "createTime", default)]
    created_at: Option<String>,
    #[serde(rename = "updateTime", default)]
    updated_at: Option<String>,
    #[serde(default)]
    messages: Vec<GeminiExportMessage>,
}

#[derive(Debug, Deserialize)]
struct GeminiExportMessage {
    #[serde(default)]
    id: Option<String>,
    content: GeminiExportContent,
    role: String,
    #[serde(rename = "createTime", default)]
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiExportContent {
    #[serde(default)]
    parts: Vec<GeminiExportPart>,
}

#[derive(Debug, Deserialize)]
struct GeminiExportPart {
    #[serde(default)]
    text: Option<String>,
}

fn parse_iso_timestamp(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| anyhow!("Failed to parse timestamp: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_provider_new() {
        let provider = GeminiProvider::new(Some("test-key".to_string()));
        assert_eq!(provider.name(), "Gemini");
        assert!(provider.is_authenticated());
    }

    #[test]
    fn test_gemini_provider_unauthenticated() {
        let provider = GeminiProvider::new(None);
        assert!(!provider.is_authenticated());
    }

    #[test]
    fn test_api_key_env_var() {
        let provider = GeminiProvider::new(None);
        assert_eq!(provider.api_key_env_var(), "GOOGLE_API_KEY");
    }
}
