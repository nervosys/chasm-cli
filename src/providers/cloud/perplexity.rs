// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Perplexity AI cloud provider
//!
//! Fetches conversation history from Perplexity web interface.
//!
//! ## Authentication
//!
//! Requires either:
//! - API key via `PERPLEXITY_API_KEY` environment variable
//! - Session token for web interface access

use super::common::{
    build_http_client, CloudConversation, CloudMessage, CloudProvider, FetchOptions,
    HttpClientConfig,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;

const PERPLEXITY_API: &str = "https://api.perplexity.ai";
const PERPLEXITY_WEB_API: &str = "https://www.perplexity.ai/api";

/// Perplexity AI provider for fetching conversation history
pub struct PerplexityProvider {
    api_key: Option<String>,
    session_token: Option<String>,
    client: Option<reqwest::blocking::Client>,
}

impl PerplexityProvider {
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
struct PerplexityThread {
    uuid: String,
    #[serde(default)]
    title: Option<String>,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    messages: Vec<PerplexityMessage>,
}

#[derive(Debug, Deserialize)]
struct PerplexityMessage {
    uuid: String,
    text: String,
    role: String, // "user" or "assistant"
    created_at: String,
    #[serde(default)]
    sources: Vec<PerplexitySource>,
}

#[derive(Debug, Deserialize)]
struct PerplexitySource {
    url: String,
    title: Option<String>,
}

impl CloudProvider for PerplexityProvider {
    fn name(&self) -> &'static str {
        "Perplexity"
    }

    fn api_base_url(&self) -> &str {
        PERPLEXITY_WEB_API
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
                "Perplexity requires authentication. Set PERPLEXITY_API_KEY or provide a session token."
            ));
        }

        eprintln!("Note: Perplexity conversation history requires web session authentication.");
        eprintln!("The Perplexity API is stateless and doesn't store conversation history.");

        Ok(vec![])
    }

    fn fetch_conversation(&self, _id: &str) -> Result<CloudConversation> {
        if !self.is_authenticated() {
            return Err(anyhow!("Perplexity requires authentication"));
        }

        Err(anyhow!(
            "Fetching Perplexity conversations requires web session authentication."
        ))
    }

    fn api_key_env_var(&self) -> &'static str {
        "PERPLEXITY_API_KEY"
    }
}

/// Parse Perplexity export data
pub fn parse_perplexity_export(json_data: &str) -> Result<Vec<CloudConversation>> {
    let threads: Vec<PerplexityThread> = serde_json::from_str(json_data)?;

    Ok(threads
        .into_iter()
        .map(|thread| {
            CloudConversation {
                id: thread.uuid,
                title: thread.title,
                created_at: parse_iso_timestamp(&thread.created_at).unwrap_or_else(|_| Utc::now()),
                updated_at: Some(
                    parse_iso_timestamp(&thread.updated_at).unwrap_or_else(|_| Utc::now()),
                ),
                model: Some("perplexity".to_string()),
                messages: thread
                    .messages
                    .into_iter()
                    .map(|msg| {
                        // Append sources to assistant messages
                        let content = if !msg.sources.is_empty() && msg.role == "assistant" {
                            let sources_text = msg
                                .sources
                                .iter()
                                .enumerate()
                                .map(|(i, s)| format!("[{}] {}", i + 1, s.url))
                                .collect::<Vec<_>>()
                                .join("\n");
                            format!("{}\n\nSources:\n{}", msg.text, sources_text)
                        } else {
                            msg.text
                        };

                        CloudMessage {
                            id: Some(msg.uuid),
                            role: msg.role,
                            content,
                            timestamp: parse_iso_timestamp(&msg.created_at).ok(),
                            model: None,
                        }
                    })
                    .collect(),
                metadata: None,
            }
        })
        .collect())
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
    fn test_perplexity_provider_new() {
        let provider = PerplexityProvider::new(Some("test-key".to_string()));
        assert_eq!(provider.name(), "Perplexity");
        assert!(provider.is_authenticated());
    }

    #[test]
    fn test_perplexity_provider_unauthenticated() {
        let provider = PerplexityProvider::new(None);
        assert!(!provider.is_authenticated());
    }

    #[test]
    fn test_api_key_env_var() {
        let provider = PerplexityProvider::new(None);
        assert_eq!(provider.api_key_env_var(), "PERPLEXITY_API_KEY");
    }
}
