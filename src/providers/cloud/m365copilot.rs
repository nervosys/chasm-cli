// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Microsoft 365 Copilot cloud provider
//!
//! Fetches conversation history from Microsoft 365 Copilot using the Microsoft Graph API.
//!
//! ## Authentication
//!
//! Requires an Azure AD/Entra ID access token with the `AiEnterpriseInteraction.Read.All`
//! permission. This is typically obtained through:
//! - Azure AD application registration with admin consent
//! - MSAL (Microsoft Authentication Library) token acquisition
//!
//! ## API Documentation
//!
//! - [AI Interaction History API](https://learn.microsoft.com/en-us/microsoft-365-copilot/extensibility/api/ai-services/interaction-export/resources/aiinteractionhistory)
//! - [Get All Enterprise Interactions](https://learn.microsoft.com/en-us/microsoft-365-copilot/extensibility/api/ai-services/interaction-export/aiinteractionhistory-getallenterpriseinteractions)
//!
//! ## App Classes
//!
//! Microsoft 365 Copilot interactions are categorized by app class:
//! - `IPM.SkypeTeams.Message.Copilot.BizChat` - Microsoft 365 Copilot Chat (formerly Bing Chat Enterprise)
//! - `IPM.SkypeTeams.Message.Copilot.Teams` - Copilot in Teams
//! - `IPM.SkypeTeams.Message.Copilot.Word` - Copilot in Word
//! - `IPM.SkypeTeams.Message.Copilot.Excel` - Copilot in Excel
//! - `IPM.SkypeTeams.Message.Copilot.PowerPoint` - Copilot in PowerPoint
//! - `IPM.SkypeTeams.Message.Copilot.Outlook` - Copilot in Outlook
//! - `IPM.SkypeTeams.Message.Copilot.Loop` - Copilot in Loop
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use csm::providers::cloud::m365copilot::M365CopilotProvider;
//! use csm::providers::cloud::CloudProvider;
//!
//! // Create provider with Azure AD access token
//! let mut provider = M365CopilotProvider::new(Some(access_token));
//! provider.set_user_id("user-uuid".to_string());
//!
//! // List conversations
//! let options = FetchOptions::default();
//! let conversations = provider.list_conversations(&options)?;
//! ```

use super::common::{
    build_http_client, CloudConversation, CloudMessage, CloudProvider, FetchOptions,
    HttpClientConfig,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;

const GRAPH_API_BASE: &str = "https://graph.microsoft.com/v1.0";

/// Microsoft 365 Copilot provider for fetching AI interaction history
pub struct M365CopilotProvider {
    /// Azure AD access token (Bearer token)
    access_token: Option<String>,
    /// User ID (GUID) for querying interactions
    user_id: Option<String>,
    /// Optional app class filter (e.g., "IPM.SkypeTeams.Message.Copilot.BizChat")
    app_class_filter: Option<String>,
    /// HTTP client
    client: Option<reqwest::blocking::Client>,
}

impl M365CopilotProvider {
    /// Create a new M365 Copilot provider
    ///
    /// # Arguments
    /// * `access_token` - Azure AD access token with `AiEnterpriseInteraction.Read.All` permission
    pub fn new(access_token: Option<String>) -> Self {
        Self {
            access_token,
            user_id: None,
            app_class_filter: None,
            client: None,
        }
    }

    /// Set the user ID for querying interactions
    pub fn set_user_id(&mut self, user_id: String) {
        self.user_id = Some(user_id);
    }

    /// Set an app class filter to narrow down results
    ///
    /// Common app classes:
    /// - `IPM.SkypeTeams.Message.Copilot.BizChat` - Microsoft 365 Copilot Chat
    /// - `IPM.SkypeTeams.Message.Copilot.Teams` - Copilot in Teams
    /// - `IPM.SkypeTeams.Message.Copilot.Word` - Copilot in Word
    pub fn set_app_class_filter(&mut self, app_class: String) {
        self.app_class_filter = Some(app_class);
    }

    fn ensure_client(&mut self) -> Result<&reqwest::blocking::Client> {
        if self.client.is_none() {
            let config = HttpClientConfig::default();
            self.client = Some(build_http_client(&config)?);
        }
        Ok(self.client.as_ref().unwrap())
    }

    /// Build the API URL for fetching interactions
    fn build_interactions_url(&self) -> Result<String> {
        let user_id = self
            .user_id
            .as_ref()
            .ok_or_else(|| anyhow!("User ID is required. Call set_user_id() first."))?;

        let mut url = format!(
            "{}/copilot/users/{}/interactionHistory/getAllEnterpriseInteractions",
            GRAPH_API_BASE, user_id
        );

        // Add app class filter if specified
        if let Some(ref app_class) = self.app_class_filter {
            url.push_str(&format!("?$filter=appClass eq '{}'", app_class));
        }

        Ok(url)
    }
}

/// Microsoft Graph API response wrapper
#[derive(Debug, Deserialize)]
struct GraphResponse<T> {
    value: Vec<T>,
    #[serde(rename = "@odata.nextLink")]
    next_link: Option<String>,
}

/// AI Interaction from Microsoft Graph API
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AiInteraction {
    /// Unique identifier
    id: String,
    /// Session ID (conversation thread)
    session_id: Option<String>,
    /// Request ID that links prompts to responses
    request_id: Option<String>,
    /// App class (e.g., IPM.SkypeTeams.Message.Copilot.BizChat)
    app_class: Option<String>,
    /// Type: userPrompt or aiResponse
    interaction_type: Option<String>,
    /// Conversation type (e.g., bizchat, appchat)
    conversation_type: Option<String>,
    /// Creation timestamp
    created_date_time: Option<String>,
    /// Locale
    locale: Option<String>,
    /// Message body
    body: Option<AiInteractionBody>,
    /// Source identity
    from: Option<AiInteractionFrom>,
    /// Attachments
    #[serde(default)]
    attachments: Vec<AiInteractionAttachment>,
    /// Links in the response
    #[serde(default)]
    links: Vec<AiInteractionLink>,
    /// Contexts
    #[serde(default)]
    contexts: Vec<AiInteractionContext>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiInteractionBody {
    content_type: Option<String>,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AiInteractionFrom {
    user: Option<AiInteractionUser>,
    application: Option<AiInteractionApplication>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiInteractionUser {
    id: Option<String>,
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiInteractionApplication {
    id: Option<String>,
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiInteractionAttachment {
    attachment_id: Option<String>,
    content_type: Option<String>,
    content_url: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiInteractionLink {
    link_url: Option<String>,
    display_name: Option<String>,
    link_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiInteractionContext {
    context_reference: Option<String>,
    display_name: Option<String>,
    context_type: Option<String>,
}

impl CloudProvider for M365CopilotProvider {
    fn name(&self) -> &'static str {
        "Microsoft 365 Copilot"
    }

    fn api_base_url(&self) -> &str {
        GRAPH_API_BASE
    }

    fn is_authenticated(&self) -> bool {
        self.access_token.is_some()
    }

    fn set_credentials(&mut self, api_key: Option<String>, _session_token: Option<String>) {
        // For M365 Copilot, the "api_key" is actually an Azure AD access token
        self.access_token = api_key;
    }

    fn list_conversations(&self, _options: &FetchOptions) -> Result<Vec<CloudConversation>> {
        if !self.is_authenticated() {
            return Err(anyhow!(
                "Microsoft 365 Copilot requires authentication. Provide an Azure AD access token \
                 with AiEnterpriseInteraction.Read.All permission."
            ));
        }

        if self.user_id.is_none() {
            return Err(anyhow!(
                "User ID is required. Call set_user_id() with the user's Azure AD object ID."
            ));
        }

        // Note: This is a placeholder since we need mutable self for ensure_client
        // The actual implementation would need to be adjusted for the trait bounds
        eprintln!("Note: Microsoft 365 Copilot requires:");
        eprintln!(
            "  1. Azure AD app registration with AiEnterpriseInteraction.Read.All permission"
        );
        eprintln!("  2. Admin consent for the permission");
        eprintln!("  3. A valid access token");
        eprintln!("  4. The target user's Azure AD object ID");

        // Return empty for now - real implementation would make the API call
        Ok(vec![])
    }

    fn fetch_conversation(&self, id: &str) -> Result<CloudConversation> {
        if !self.is_authenticated() {
            return Err(anyhow!("Microsoft 365 Copilot requires authentication"));
        }

        // The M365 Copilot API doesn't have a direct "get single conversation" endpoint
        // Conversations are identified by session_id and must be filtered from the full list
        Err(anyhow!(
            "Microsoft 365 Copilot doesn't support fetching individual conversations by ID. \
             Use list_conversations() and filter by session_id: {}",
            id
        ))
    }

    fn api_key_env_var(&self) -> &'static str {
        "M365_COPILOT_ACCESS_TOKEN"
    }
}

/// Group AI interactions by session ID into conversations
pub(crate) fn group_interactions_into_conversations(
    interactions: Vec<AiInteraction>,
) -> Vec<CloudConversation> {
    // Group by session_id
    let mut sessions: HashMap<String, Vec<AiInteraction>> = HashMap::new();

    for interaction in interactions {
        let session_id = interaction
            .session_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        sessions.entry(session_id).or_default().push(interaction);
    }

    // Convert each session to a CloudConversation
    sessions
        .into_iter()
        .map(|(session_id, mut interactions)| {
            // Sort by created_date_time
            interactions.sort_by(|a, b| {
                let a_time = a.created_date_time.as_deref().unwrap_or("");
                let b_time = b.created_date_time.as_deref().unwrap_or("");
                a_time.cmp(b_time)
            });

            // Determine app class and conversation type from first interaction
            let app_class = interactions
                .first()
                .and_then(|i| i.app_class.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            let conversation_type = interactions
                .first()
                .and_then(|i| i.conversation_type.clone());

            // Parse timestamps
            let created_at = interactions
                .first()
                .and_then(|i| i.created_date_time.as_ref())
                .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            let updated_at = interactions
                .last()
                .and_then(|i| i.created_date_time.as_ref())
                .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                .map(|dt| dt.with_timezone(&Utc));

            // Convert interactions to messages
            let messages: Vec<CloudMessage> = interactions
                .into_iter()
                .filter_map(|interaction| {
                    let content = interaction.body.as_ref()?.content.clone()?;

                    let role = match interaction.interaction_type.as_deref() {
                        Some("userPrompt") => "user",
                        Some("aiResponse") => "assistant",
                        _ => "unknown",
                    };

                    let timestamp = interaction
                        .created_date_time
                        .as_ref()
                        .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                        .map(|dt| dt.with_timezone(&Utc));

                    let model = interaction
                        .from
                        .as_ref()
                        .and_then(|f| f.application.as_ref())
                        .and_then(|app| app.display_name.clone());

                    Some(CloudMessage {
                        id: Some(interaction.id),
                        role: role.to_string(),
                        content,
                        timestamp,
                        model,
                    })
                })
                .collect();

            // Generate title from app class
            let title = Some(format!(
                "{} - {}",
                get_friendly_app_name(&app_class),
                created_at.format("%Y-%m-%d %H:%M")
            ));

            CloudConversation {
                id: session_id,
                title,
                created_at,
                updated_at,
                model: Some(app_class),
                messages,
                metadata: conversation_type.map(|ct| serde_json::json!({ "conversationType": ct })),
            }
        })
        .collect()
}

/// Get a friendly name for the M365 Copilot app class
pub fn get_friendly_app_name(app_class: &str) -> &'static str {
    match app_class {
        "IPM.SkypeTeams.Message.Copilot.BizChat" => "Microsoft 365 Copilot Chat",
        "IPM.SkypeTeams.Message.Copilot.Teams" => "Copilot in Teams",
        "IPM.SkypeTeams.Message.Copilot.Word" => "Copilot in Word",
        "IPM.SkypeTeams.Message.Copilot.Excel" => "Copilot in Excel",
        "IPM.SkypeTeams.Message.Copilot.PowerPoint" => "Copilot in PowerPoint",
        "IPM.SkypeTeams.Message.Copilot.Outlook" => "Copilot in Outlook",
        "IPM.SkypeTeams.Message.Copilot.Loop" => "Copilot in Loop",
        "IPM.SkypeTeams.Message.Copilot.OneNote" => "Copilot in OneNote",
        "IPM.SkypeTeams.Message.Copilot.Whiteboard" => "Copilot in Whiteboard",
        _ => "Microsoft 365 Copilot",
    }
}

/// Parse Microsoft 365 Copilot export data (JSON format from Graph API)
pub fn parse_m365_copilot_export(json_data: &str) -> Result<Vec<CloudConversation>> {
    // Try to parse as a Graph API response
    if let Ok(response) = serde_json::from_str::<GraphResponse<AiInteraction>>(json_data) {
        return Ok(group_interactions_into_conversations(response.value));
    }

    // Try to parse as a direct array of interactions
    if let Ok(interactions) = serde_json::from_str::<Vec<AiInteraction>>(json_data) {
        return Ok(group_interactions_into_conversations(interactions));
    }

    Err(anyhow!(
        "Failed to parse Microsoft 365 Copilot export data. \
         Expected Graph API response format or array of aiInteraction objects."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_m365_copilot_provider_new() {
        let provider = M365CopilotProvider::new(Some("test-token".to_string()));
        assert_eq!(provider.name(), "Microsoft 365 Copilot");
        assert!(provider.is_authenticated());
    }

    #[test]
    fn test_m365_copilot_provider_unauthenticated() {
        let provider = M365CopilotProvider::new(None);
        assert!(!provider.is_authenticated());
    }

    #[test]
    fn test_api_key_env_var() {
        let provider = M365CopilotProvider::new(None);
        assert_eq!(provider.api_key_env_var(), "M365_COPILOT_ACCESS_TOKEN");
    }

    #[test]
    fn test_set_user_id() {
        let mut provider = M365CopilotProvider::new(Some("token".to_string()));
        provider.set_user_id("user-123".to_string());
        assert_eq!(provider.user_id, Some("user-123".to_string()));
    }

    #[test]
    fn test_set_app_class_filter() {
        let mut provider = M365CopilotProvider::new(Some("token".to_string()));
        provider.set_app_class_filter("IPM.SkypeTeams.Message.Copilot.BizChat".to_string());
        assert_eq!(
            provider.app_class_filter,
            Some("IPM.SkypeTeams.Message.Copilot.BizChat".to_string())
        );
    }

    #[test]
    fn test_build_interactions_url_no_filter() {
        let mut provider = M365CopilotProvider::new(Some("token".to_string()));
        provider.set_user_id("test-user-id".to_string());

        let url = provider.build_interactions_url().unwrap();
        assert_eq!(
            url,
            "https://graph.microsoft.com/v1.0/copilot/users/test-user-id/interactionHistory/getAllEnterpriseInteractions"
        );
    }

    #[test]
    fn test_build_interactions_url_with_filter() {
        let mut provider = M365CopilotProvider::new(Some("token".to_string()));
        provider.set_user_id("test-user-id".to_string());
        provider.set_app_class_filter("IPM.SkypeTeams.Message.Copilot.BizChat".to_string());

        let url = provider.build_interactions_url().unwrap();
        assert!(url.contains("$filter=appClass eq 'IPM.SkypeTeams.Message.Copilot.BizChat'"));
    }

    #[test]
    fn test_build_interactions_url_no_user_id() {
        let provider = M365CopilotProvider::new(Some("token".to_string()));
        let result = provider.build_interactions_url();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_friendly_app_name() {
        assert_eq!(
            get_friendly_app_name("IPM.SkypeTeams.Message.Copilot.BizChat"),
            "Microsoft 365 Copilot Chat"
        );
        assert_eq!(
            get_friendly_app_name("IPM.SkypeTeams.Message.Copilot.Teams"),
            "Copilot in Teams"
        );
        assert_eq!(
            get_friendly_app_name("IPM.SkypeTeams.Message.Copilot.Word"),
            "Copilot in Word"
        );
        assert_eq!(
            get_friendly_app_name("IPM.SkypeTeams.Message.Copilot.Excel"),
            "Copilot in Excel"
        );
        assert_eq!(get_friendly_app_name("unknown"), "Microsoft 365 Copilot");
    }

    #[test]
    fn test_parse_m365_copilot_export_empty_array() {
        let json = "[]";
        let result = parse_m365_copilot_export(json).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_m365_copilot_export_graph_response() {
        let json = r#"{
            "value": [
                {
                    "id": "1731701801008",
                    "sessionId": "session-123",
                    "requestId": "req-123",
                    "appClass": "IPM.SkypeTeams.Message.Copilot.BizChat",
                    "interactionType": "userPrompt",
                    "conversationType": "bizchat",
                    "createdDateTime": "2024-11-15T20:16:41.008Z",
                    "locale": "en-us",
                    "body": {
                        "contentType": "text",
                        "content": "What should be on my radar from emails last week?"
                    },
                    "from": {
                        "user": {
                            "id": "user-123",
                            "displayName": "Test User"
                        }
                    },
                    "attachments": [],
                    "links": [],
                    "contexts": []
                },
                {
                    "id": "1731701801009",
                    "sessionId": "session-123",
                    "requestId": "req-123",
                    "appClass": "IPM.SkypeTeams.Message.Copilot.BizChat",
                    "interactionType": "aiResponse",
                    "conversationType": "bizchat",
                    "createdDateTime": "2024-11-15T20:16:42.008Z",
                    "locale": "en-us",
                    "body": {
                        "contentType": "text",
                        "content": "Based on your emails from last week, here are the key items..."
                    },
                    "from": {
                        "application": {
                            "id": "copilot-app",
                            "displayName": "Microsoft 365 Chat"
                        }
                    },
                    "attachments": [],
                    "links": [],
                    "contexts": []
                }
            ]
        }"#;

        let result = parse_m365_copilot_export(json).unwrap();
        assert_eq!(result.len(), 1); // One conversation (grouped by session_id)

        let conv = &result[0];
        assert_eq!(conv.id, "session-123");
        assert_eq!(conv.messages.len(), 2);
        assert_eq!(conv.messages[0].role, "user");
        assert_eq!(conv.messages[1].role, "assistant");
    }

    #[test]
    fn test_parse_m365_copilot_export_multiple_sessions() {
        let json = r#"[
            {
                "id": "1",
                "sessionId": "session-a",
                "interactionType": "userPrompt",
                "appClass": "IPM.SkypeTeams.Message.Copilot.Word",
                "createdDateTime": "2024-11-15T10:00:00Z",
                "body": { "content": "Draft an email" }
            },
            {
                "id": "2",
                "sessionId": "session-b",
                "interactionType": "userPrompt",
                "appClass": "IPM.SkypeTeams.Message.Copilot.Excel",
                "createdDateTime": "2024-11-15T11:00:00Z",
                "body": { "content": "Create a formula" }
            }
        ]"#;

        let result = parse_m365_copilot_export(json).unwrap();
        assert_eq!(result.len(), 2); // Two separate conversations
    }

    #[test]
    fn test_group_interactions_preserves_order() {
        let interactions = vec![
            AiInteraction {
                id: "3".to_string(),
                session_id: Some("session-1".to_string()),
                request_id: None,
                app_class: Some("IPM.SkypeTeams.Message.Copilot.BizChat".to_string()),
                interaction_type: Some("aiResponse".to_string()),
                conversation_type: None,
                created_date_time: Some("2024-11-15T10:00:02Z".to_string()),
                locale: None,
                body: Some(AiInteractionBody {
                    content_type: Some("text".to_string()),
                    content: Some("Response 1".to_string()),
                }),
                from: None,
                attachments: vec![],
                links: vec![],
                contexts: vec![],
            },
            AiInteraction {
                id: "1".to_string(),
                session_id: Some("session-1".to_string()),
                request_id: None,
                app_class: Some("IPM.SkypeTeams.Message.Copilot.BizChat".to_string()),
                interaction_type: Some("userPrompt".to_string()),
                conversation_type: None,
                created_date_time: Some("2024-11-15T10:00:00Z".to_string()),
                locale: None,
                body: Some(AiInteractionBody {
                    content_type: Some("text".to_string()),
                    content: Some("Question 1".to_string()),
                }),
                from: None,
                attachments: vec![],
                links: vec![],
                contexts: vec![],
            },
        ];

        let result = group_interactions_into_conversations(interactions);
        assert_eq!(result.len(), 1);

        let conv = &result[0];
        assert_eq!(conv.messages.len(), 2);
        // Should be sorted by timestamp
        assert_eq!(conv.messages[0].content, "Question 1");
        assert_eq!(conv.messages[1].content, "Response 1");
    }
}
