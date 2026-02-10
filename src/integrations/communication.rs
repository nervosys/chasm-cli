// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Communication Integrations
//!
//! Slack, Discord, Teams, Telegram, SMS, Email

use super::IntegrationResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Common Types
// =============================================================================

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub channel_id: String,
    pub author: ChatUser,
    pub content: String,
    pub timestamp: String,
    pub thread_id: Option<String>,
    pub attachments: Vec<ChatAttachment>,
    pub reactions: Vec<Reaction>,
    pub edited: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatUser {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatAttachment {
    pub id: String,
    pub filename: String,
    pub url: String,
    pub mime_type: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub emoji: String,
    pub count: u32,
    pub users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub topic: Option<String>,
    pub is_private: bool,
    pub is_archived: bool,
    pub member_count: Option<u32>,
}

// =============================================================================
// Slack
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub workspace: String,
    pub bot_token: String,
    pub user_token: Option<String>,
}

/// Slack provider trait
#[async_trait::async_trait]
pub trait SlackProvider: Send + Sync {
    async fn send_message(
        &self,
        channel: &str,
        text: &str,
        thread_ts: Option<&str>,
    ) -> IntegrationResult;
    async fn list_channels(&self) -> IntegrationResult;
    async fn list_messages(&self, channel: &str, limit: u32) -> IntegrationResult;
    async fn upload_file(
        &self,
        channel: &str,
        file_path: &str,
        comment: Option<&str>,
    ) -> IntegrationResult;
    async fn set_status(&self, status_text: &str, status_emoji: &str) -> IntegrationResult;
    async fn search_messages(&self, query: &str) -> IntegrationResult;
    async fn add_reaction(&self, channel: &str, timestamp: &str, emoji: &str) -> IntegrationResult;
    async fn get_user(&self, user_id: &str) -> IntegrationResult;
}

// =============================================================================
// Discord
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub bot_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordGuild {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub owner_id: String,
    pub member_count: Option<u32>,
}

/// Discord provider trait
#[async_trait::async_trait]
pub trait DiscordProvider: Send + Sync {
    async fn send_message(&self, channel_id: &str, content: &str) -> IntegrationResult;
    async fn list_guilds(&self) -> IntegrationResult;
    async fn list_channels(&self, guild_id: &str) -> IntegrationResult;
    async fn list_messages(&self, channel_id: &str, limit: u32) -> IntegrationResult;
    async fn add_reaction(
        &self,
        channel_id: &str,
        message_id: &str,
        emoji: &str,
    ) -> IntegrationResult;
    async fn create_thread(
        &self,
        channel_id: &str,
        name: &str,
        message_id: Option<&str>,
    ) -> IntegrationResult;
}

// =============================================================================
// Microsoft Teams
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsConfig {
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsTeam {
    pub id: String,
    pub display_name: String,
    pub description: Option<String>,
}

/// Teams provider trait
#[async_trait::async_trait]
pub trait TeamsProvider: Send + Sync {
    async fn send_message(
        &self,
        team_id: &str,
        channel_id: &str,
        content: &str,
    ) -> IntegrationResult;
    async fn list_teams(&self) -> IntegrationResult;
    async fn list_channels(&self, team_id: &str) -> IntegrationResult;
    async fn create_meeting(
        &self,
        subject: &str,
        start: &str,
        end: &str,
        attendees: &[String],
    ) -> IntegrationResult;
    async fn get_presence(&self, user_id: &str) -> IntegrationResult;
    async fn set_presence(&self, availability: &str, activity: &str) -> IntegrationResult;
}

// =============================================================================
// Telegram
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
    pub chat_type: String,
    pub title: Option<String>,
    pub username: Option<String>,
}

/// Telegram provider trait
#[async_trait::async_trait]
pub trait TelegramProvider: Send + Sync {
    async fn send_message(&self, chat_id: &str, text: &str) -> IntegrationResult;
    async fn send_photo(
        &self,
        chat_id: &str,
        photo_path: &str,
        caption: Option<&str>,
    ) -> IntegrationResult;
    async fn send_document(
        &self,
        chat_id: &str,
        file_path: &str,
        caption: Option<&str>,
    ) -> IntegrationResult;
    async fn get_updates(&self, offset: Option<i64>) -> IntegrationResult;
    async fn set_webhook(&self, url: &str) -> IntegrationResult;
}

// =============================================================================
// SMS
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsConfig {
    pub provider: SmsProvider,
    pub account_sid: String,
    pub auth_token: String,
    pub from_number: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SmsProvider {
    Twilio,
    Vonage,
    MessageBird,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub body: String,
    pub status: SmsStatus,
    pub timestamp: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SmsStatus {
    Queued,
    Sent,
    Delivered,
    Failed,
}

/// SMS provider trait
#[async_trait::async_trait]
pub trait SmsProviderTrait: Send + Sync {
    async fn send_sms(&self, to: &str, body: &str) -> IntegrationResult;
    async fn get_messages(&self, limit: u32) -> IntegrationResult;
}

// =============================================================================
// Voice (Alexa, Google Assistant, Siri)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCommand {
    pub text: String,
    pub intent: Option<String>,
    pub slots: HashMap<String, String>,
    pub confidence: f32,
}

/// Voice assistant provider trait
#[async_trait::async_trait]
pub trait VoiceProvider: Send + Sync {
    async fn speak(&self, text: &str) -> IntegrationResult;
    async fn listen(&self) -> IntegrationResult;
    async fn process_command(&self, command: &VoiceCommand) -> IntegrationResult;
}

// =============================================================================
// Notifications
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub title: String,
    pub body: String,
    pub icon: Option<String>,
    pub sound: Option<String>,
    pub badge: Option<u32>,
    pub actions: Vec<NotificationAction>,
    pub data: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub id: String,
    pub title: String,
    pub destructive: bool,
}

/// Notifications provider trait
#[async_trait::async_trait]
pub trait NotificationsProvider: Send + Sync {
    async fn send(&self, notification: &Notification) -> IntegrationResult;
    async fn schedule(&self, notification: &Notification, at: &str) -> IntegrationResult;
    async fn cancel(&self, notification_id: &str) -> IntegrationResult;
}
