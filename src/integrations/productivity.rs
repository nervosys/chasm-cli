// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Productivity Integrations
//!
//! Calendar, Email, Notes, Tasks, Documents

#![allow(dead_code)]

use super::{AuthMethod, IntegrationResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Calendar
// =============================================================================

/// Calendar event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub start: String,
    pub end: String,
    pub location: Option<String>,
    pub attendees: Vec<Attendee>,
    pub is_all_day: bool,
    pub recurrence: Option<String>,
    pub status: EventStatus,
    pub calendar_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attendee {
    pub email: String,
    pub name: Option<String>,
    pub response: AttendeeResponse,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttendeeResponse {
    Accepted,
    Declined,
    Tentative,
    NeedsAction,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

/// Calendar provider trait
#[async_trait::async_trait]
pub trait CalendarProvider: Send + Sync {
    async fn list_calendars(&self) -> IntegrationResult;
    async fn list_events(&self, calendar_id: &str, start: &str, end: &str) -> IntegrationResult;
    async fn create_event(&self, calendar_id: &str, event: &CalendarEvent) -> IntegrationResult;
    async fn update_event(&self, event_id: &str, event: &CalendarEvent) -> IntegrationResult;
    async fn delete_event(&self, event_id: &str) -> IntegrationResult;
    async fn find_free_time(
        &self,
        calendars: &[String],
        duration_minutes: u32,
        range_start: &str,
        range_end: &str,
    ) -> IntegrationResult;
}

// =============================================================================
// Email
// =============================================================================

/// Email message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMessage {
    pub id: String,
    pub thread_id: Option<String>,
    pub from: EmailAddress,
    pub to: Vec<EmailAddress>,
    pub cc: Vec<EmailAddress>,
    pub bcc: Vec<EmailAddress>,
    pub subject: String,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub attachments: Vec<Attachment>,
    pub labels: Vec<String>,
    pub is_read: bool,
    pub is_starred: bool,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAddress {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub filename: String,
    pub mime_type: String,
    pub size: u64,
}

/// Email draft
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailDraft {
    pub to: Vec<String>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub subject: String,
    pub body: String,
    pub is_html: bool,
    pub attachments: Option<Vec<String>>,
    pub reply_to: Option<String>,
}

/// Email provider trait
#[async_trait::async_trait]
pub trait EmailProvider: Send + Sync {
    async fn list_emails(&self, folder: &str, limit: u32) -> IntegrationResult;
    async fn get_email(&self, email_id: &str) -> IntegrationResult;
    async fn send_email(&self, draft: &EmailDraft) -> IntegrationResult;
    async fn reply_to_email(&self, email_id: &str, body: &str) -> IntegrationResult;
    async fn forward_email(&self, email_id: &str, to: &[String]) -> IntegrationResult;
    async fn move_email(&self, email_id: &str, folder: &str) -> IntegrationResult;
    async fn label_email(&self, email_id: &str, labels: &[String]) -> IntegrationResult;
    async fn search_emails(&self, query: &str, limit: u32) -> IntegrationResult;
    async fn mark_read(&self, email_id: &str, is_read: bool) -> IntegrationResult;
}

// =============================================================================
// Notes
// =============================================================================

/// Note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub folder: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub links: Vec<String>,
    pub backlinks: Vec<String>,
}

/// Notes provider trait
#[async_trait::async_trait]
pub trait NotesProvider: Send + Sync {
    async fn list_notes(&self, folder: Option<&str>) -> IntegrationResult;
    async fn get_note(&self, note_id: &str) -> IntegrationResult;
    async fn create_note(
        &self,
        title: &str,
        content: &str,
        folder: Option<&str>,
    ) -> IntegrationResult;
    async fn update_note(&self, note_id: &str, content: &str) -> IntegrationResult;
    async fn delete_note(&self, note_id: &str) -> IntegrationResult;
    async fn search_notes(&self, query: &str) -> IntegrationResult;
    async fn get_backlinks(&self, note_id: &str) -> IntegrationResult;
}

// =============================================================================
// Tasks
// =============================================================================

/// Task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<String>,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub project: Option<String>,
    pub labels: Vec<String>,
    pub subtasks: Vec<Task>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    None,
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

/// Tasks provider trait
#[async_trait::async_trait]
pub trait TasksProvider: Send + Sync {
    async fn list_tasks(&self, project: Option<&str>, include_completed: bool)
        -> IntegrationResult;
    async fn get_task(&self, task_id: &str) -> IntegrationResult;
    async fn create_task(&self, task: &Task) -> IntegrationResult;
    async fn update_task(&self, task_id: &str, task: &Task) -> IntegrationResult;
    async fn complete_task(&self, task_id: &str) -> IntegrationResult;
    async fn delete_task(&self, task_id: &str) -> IntegrationResult;
    async fn list_projects(&self) -> IntegrationResult;
}

// =============================================================================
// Documents
// =============================================================================

/// Document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: Option<String>,
    pub mime_type: String,
    pub folder: Option<String>,
    pub shared_with: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub web_url: Option<String>,
}

/// Documents provider trait
#[async_trait::async_trait]
pub trait DocumentsProvider: Send + Sync {
    async fn list_documents(&self, folder: Option<&str>) -> IntegrationResult;
    async fn get_document(&self, doc_id: &str) -> IntegrationResult;
    async fn create_document(&self, title: &str, content: &str) -> IntegrationResult;
    async fn update_document(&self, doc_id: &str, content: &str) -> IntegrationResult;
    async fn share_document(&self, doc_id: &str, emails: &[String]) -> IntegrationResult;
    async fn search_documents(&self, query: &str) -> IntegrationResult;
}
