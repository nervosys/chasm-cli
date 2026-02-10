// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! DeepSeek cloud provider
//!
//! Fetches conversation history from DeepSeek web interface.
//!
//! ## Authentication
//!
//! Requires either:
//! - API key via `DEEPSEEK_API_KEY` environment variable
//! - Session token for web interface access

use super::common::{
    build_http_client, CloudConversation, CloudMessage, CloudProvider, FetchOptions,
    HttpClientConfig,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;

const DEEPSEEK_API: &str = "https://api.deepseek.com/v1";
const DEEPSEEK_WEB_API: &str = "https://chat.deepseek.com/api";

/// DeepSeek provider for fetching conversation history
pub struct DeepSeekProvider {
    api_key: Option<String>,
    session_token: Option<String>,
    client: Option<reqwest::blocking::Client>,
}

impl DeepSeekProvider {
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
struct DeepSeekConversation {
    id: String,
    #[serde(default)]
    title: Option<String>,
    created_at: i64,
    updated_at: i64,
    #[serde(default)]
    messages: Vec<DeepSeekMessage>,
    #[serde(default)]
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekMessage {
    id: String,
    content: String,
    role: String,
    created_at: i64,
}

impl CloudProvider for DeepSeekProvider {
    fn name(&self) -> &'static str {
        "DeepSeek"
    }

    fn api_base_url(&self) -> &str {
        DEEPSEEK_WEB_API
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
                "DeepSeek requires authentication. Set DEEPSEEK_API_KEY or provide a session token."
            ));
        }

        eprintln!("Note: DeepSeek conversation history requires web session authentication.");
        eprintln!("The DeepSeek API is stateless and doesn't store conversation history.");

        Ok(vec![])
    }

    fn fetch_conversation(&self, _id: &str) -> Result<CloudConversation> {
        if !self.is_authenticated() {
            return Err(anyhow!("DeepSeek requires authentication"));
        }

        Err(anyhow!(
            "Fetching DeepSeek conversations requires web session authentication."
        ))
    }

    fn api_key_env_var(&self) -> &'static str {
        "DEEPSEEK_API_KEY"
    }
}

/// Parse DeepSeek export data
pub fn parse_deepseek_export(json_data: &str) -> Result<Vec<CloudConversation>> {
    let conversations: Vec<DeepSeekConversation> = serde_json::from_str(json_data)?;

    Ok(conversations
        .into_iter()
        .map(|conv| CloudConversation {
            id: conv.id,
            title: conv.title,
            created_at: timestamp_millis_to_datetime(conv.created_at),
            updated_at: Some(timestamp_millis_to_datetime(conv.updated_at)),
            model: conv.model,
            messages: conv
                .messages
                .into_iter()
                .map(|msg| CloudMessage {
                    id: Some(msg.id),
                    role: msg.role,
                    content: msg.content,
                    timestamp: Some(timestamp_millis_to_datetime(msg.created_at)),
                    model: None,
                })
                .collect(),
            metadata: None,
        })
        .collect())
}

fn timestamp_millis_to_datetime(ts: i64) -> DateTime<Utc> {
    Utc.timestamp_millis_opt(ts)
        .single()
        .unwrap_or_else(Utc::now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deepseek_provider_new() {
        let provider = DeepSeekProvider::new(Some("test-key".to_string()));
        assert_eq!(provider.name(), "DeepSeek");
        assert!(provider.is_authenticated());
    }

    #[test]
    fn test_deepseek_provider_unauthenticated() {
        let provider = DeepSeekProvider::new(None);
        assert!(!provider.is_authenticated());
    }

    #[test]
    fn test_api_key_env_var() {
        let provider = DeepSeekProvider::new(None);
        assert_eq!(provider.api_key_env_var(), "DEEPSEEK_API_KEY");
    }

    #[test]
    fn test_timestamp_millis_to_datetime() {
        let ts = 1700000000000i64;
        let dt = timestamp_millis_to_datetime(ts);
        assert_eq!(dt.timestamp_millis(), ts);
    }
}
