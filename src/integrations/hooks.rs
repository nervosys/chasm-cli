// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Hook System
//!
//! Hooks allow agents to respond to events and automate workflows.
//! They connect triggers (events) to actions (integrations).

#![allow(dead_code)]

use super::{Capability, IntegrationResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A hook connects a trigger to one or more actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    /// Unique hook identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this hook does
    pub description: Option<String>,
    /// Whether the hook is enabled
    pub enabled: bool,
    /// Trigger that activates this hook
    pub trigger: HookTrigger,
    /// Conditions that must be met
    pub conditions: Vec<HookCondition>,
    /// Actions to execute when triggered
    pub actions: Vec<HookAction>,
    /// Hook configuration
    pub config: HookConfig,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last execution timestamp
    pub last_run: Option<DateTime<Utc>>,
    /// Run count
    pub run_count: u64,
}

/// Trigger types that can activate a hook
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookTrigger {
    // =========================================================================
    // Time-based Triggers
    // =========================================================================
    /// Cron schedule (e.g., "0 9 * * *" for 9 AM daily)
    Schedule {
        cron: String,
        timezone: Option<String>,
    },
    /// Interval (e.g., every 30 minutes)
    Interval { seconds: u64 },
    /// Specific datetime
    DateTime { at: DateTime<Utc> },
    /// Recurring at specific times
    Daily { times: Vec<String> }, // ["09:00", "17:00"]

    // =========================================================================
    // Event-based Triggers
    // =========================================================================
    /// Webhook received
    Webhook {
        path: String,
        method: Option<String>,
    },
    /// File system change
    FileChange {
        path: String,
        events: Vec<FileEvent>,
        recursive: bool,
    },
    /// Email received
    EmailReceived {
        account: String,
        filters: Option<EmailFilters>,
    },
    /// Calendar event
    CalendarEvent {
        calendar_id: String,
        event_type: CalendarEventType,
        minutes_before: Option<i32>,
    },
    /// Message received (Slack, Discord, etc.)
    MessageReceived {
        platform: String,
        channel: Option<String>,
        from: Option<String>,
        contains: Option<String>,
    },
    /// Git event
    GitEvent {
        repo: String,
        events: Vec<GitEventType>,
    },
    /// System event
    SystemEvent { event: SystemEventType },
    /// Smart home device event
    DeviceEvent {
        device_id: String,
        state_change: Option<String>,
    },
    /// Location-based trigger
    Location {
        latitude: f64,
        longitude: f64,
        radius_meters: u32,
        on_enter: bool,
        on_exit: bool,
    },
    /// Manual trigger (user-initiated)
    Manual,
    /// Chain trigger (activated by another hook)
    Chain { hook_id: String },
    /// Agent request
    AgentRequest { agent_name: Option<String> },
}

/// File system events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileEvent {
    Created,
    Modified,
    Deleted,
    Renamed,
    Any,
}

/// Email filters for trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailFilters {
    pub from: Option<String>,
    pub subject_contains: Option<String>,
    pub has_attachment: Option<bool>,
    pub labels: Option<Vec<String>>,
}

/// Calendar event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarEventType {
    Created,
    Updated,
    Deleted,
    Starting,
    Ended,
    Reminder,
}

/// Git event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitEventType {
    Push,
    Pull,
    Commit,
    Branch,
    Tag,
    PullRequest,
    Issue,
    Release,
}

/// System event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemEventType {
    Startup,
    Shutdown,
    Sleep,
    Wake,
    NetworkChange,
    BatteryLow,
    StorageLow,
    AppLaunched { app: String },
    AppClosed { app: String },
    ClipboardChange,
    ScreenLock,
    ScreenUnlock,
}

/// Condition for hook execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookCondition {
    /// Time window
    TimeWindow {
        start: String,
        end: String,
        days: Option<Vec<String>>,
    },
    /// Expression evaluation
    Expression { expr: String },
    /// Previous action result
    PreviousResult { action_index: usize, success: bool },
    /// Environment variable
    EnvVar {
        name: String,
        value: Option<String>,
        exists: Option<bool>,
    },
    /// System state
    SystemState {
        state: String,
        value: serde_json::Value,
    },
    /// Rate limit
    RateLimit { max_runs: u32, period_seconds: u64 },
    /// Cooldown period
    Cooldown { seconds: u64 },
}

/// Action to execute when hook triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookAction {
    // =========================================================================
    // Productivity Actions
    // =========================================================================
    /// Create calendar event
    CreateCalendarEvent {
        calendar_id: String,
        title: String,
        start: String,
        end: String,
        description: Option<String>,
        attendees: Option<Vec<String>>,
        location: Option<String>,
    },
    /// Send email
    SendEmail {
        account: String,
        to: Vec<String>,
        subject: String,
        body: String,
        attachments: Option<Vec<String>>,
    },
    /// Create note
    CreateNote {
        app: String, // "obsidian", "notion", "apple_notes"
        title: String,
        content: String,
        folder: Option<String>,
        tags: Option<Vec<String>>,
    },
    /// Create task
    CreateTask {
        app: String, // "todoist", "things", "reminders"
        title: String,
        description: Option<String>,
        due_date: Option<String>,
        priority: Option<u8>,
        project: Option<String>,
    },

    // =========================================================================
    // Communication Actions
    // =========================================================================
    /// Send Slack message
    SlackMessage {
        channel: String,
        text: String,
        thread_ts: Option<String>,
    },
    /// Send Discord message
    DiscordMessage { channel_id: String, content: String },
    /// Send Teams message
    TeamsMessage { channel: String, content: String },
    /// Send Telegram message
    TelegramMessage { chat_id: String, text: String },
    /// Send SMS
    SendSms { to: String, message: String },
    /// Send notification
    Notification {
        title: String,
        body: String,
        sound: Option<String>,
        actions: Option<Vec<String>>,
    },

    // =========================================================================
    // Browser/Automation Actions
    // =========================================================================
    /// Open URL
    OpenUrl {
        url: String,
        browser: Option<String>,
    },
    /// Run browser automation
    BrowserAutomation { script: String, headless: bool },
    /// Scrape webpage
    ScrapeWebpage {
        url: String,
        selectors: HashMap<String, String>,
    },
    /// Fill form
    FillForm {
        url: String,
        fields: HashMap<String, String>,
        submit: bool,
    },

    // =========================================================================
    // Development Actions
    // =========================================================================
    /// Run shell command
    RunCommand {
        command: String,
        args: Vec<String>,
        cwd: Option<String>,
        env: Option<HashMap<String, String>>,
    },
    /// Git operation
    GitOperation {
        repo: String,
        operation: String, // "commit", "push", "pull", "branch"
        args: HashMap<String, String>,
    },
    /// Create GitHub issue
    CreateGitHubIssue {
        repo: String,
        title: String,
        body: String,
        labels: Option<Vec<String>>,
    },
    /// Run Docker command
    DockerCommand {
        command: String,
        container: Option<String>,
        args: Vec<String>,
    },

    // =========================================================================
    // Smart Home Actions
    // =========================================================================
    /// Control device
    ControlDevice {
        device_id: String,
        action: String,
        parameters: Option<HashMap<String, serde_json::Value>>,
    },
    /// Set scene
    SetScene { scene_id: String },
    /// Home Assistant service call
    HomeAssistantService {
        domain: String,
        service: String,
        data: Option<serde_json::Value>,
    },

    // =========================================================================
    // File Actions
    // =========================================================================
    /// Copy file
    CopyFile { source: String, destination: String },
    /// Move file
    MoveFile { source: String, destination: String },
    /// Create file
    CreateFile { path: String, content: String },
    /// Delete file
    DeleteFile { path: String },
    /// Sync folder
    SyncFolder {
        source: String,
        destination: String,
        delete_extra: bool,
    },

    // =========================================================================
    // Data Actions
    // =========================================================================
    /// HTTP request
    HttpRequest {
        method: String,
        url: String,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
    },
    /// Store data
    StoreData {
        key: String,
        value: serde_json::Value,
        ttl_seconds: Option<u64>,
    },
    /// Query database
    QueryDatabase {
        connection: String,
        query: String,
        params: Option<Vec<serde_json::Value>>,
    },

    // =========================================================================
    // AI/Agent Actions
    // =========================================================================
    /// Run agent
    RunAgent {
        agent_name: String,
        input: String,
        context: Option<HashMap<String, serde_json::Value>>,
    },
    /// Summarize content
    Summarize {
        content: String,
        max_length: Option<u32>,
    },
    /// Classify content
    Classify {
        content: String,
        categories: Vec<String>,
    },
    /// Extract data
    ExtractData {
        content: String,
        schema: serde_json::Value,
    },

    // =========================================================================
    // Meta Actions
    // =========================================================================
    /// Chain to another hook
    ChainHook {
        hook_id: String,
        data: Option<serde_json::Value>,
    },
    /// Conditional branch
    Conditional {
        condition: String,
        if_true: Box<HookAction>,
        if_false: Option<Box<HookAction>>,
    },
    /// Parallel execution
    Parallel { actions: Vec<HookAction> },
    /// Delay
    Delay { seconds: u64 },
    /// Log message
    Log { level: String, message: String },
}

/// Hook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// Maximum retries on failure
    pub max_retries: u32,
    /// Retry delay in seconds
    pub retry_delay_seconds: u64,
    /// Timeout for each action
    pub timeout_seconds: u64,
    /// Whether to continue on action failure
    pub continue_on_error: bool,
    /// Whether to run actions in parallel
    pub parallel: bool,
    /// Tags for organization
    pub tags: Vec<String>,
    /// Priority (higher = more important)
    pub priority: u8,
}

impl Default for HookConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_seconds: 5,
            timeout_seconds: 300,
            continue_on_error: false,
            parallel: false,
            tags: vec![],
            priority: 50,
        }
    }
}

/// Result of hook execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    pub hook_id: String,
    pub success: bool,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub action_results: Vec<ActionResult>,
    pub error: Option<String>,
}

/// Result of a single action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action_index: usize,
    pub action_type: String,
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Hook builder for ergonomic construction
pub struct HookBuilder {
    hook: Hook,
}

impl HookBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            hook: Hook {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.into(),
                description: None,
                enabled: true,
                trigger: HookTrigger::Manual,
                conditions: vec![],
                actions: vec![],
                config: HookConfig::default(),
                created_at: Utc::now(),
                last_run: None,
                run_count: 0,
            },
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.hook.description = Some(desc.into());
        self
    }

    pub fn trigger(mut self, trigger: HookTrigger) -> Self {
        self.hook.trigger = trigger;
        self
    }

    pub fn condition(mut self, condition: HookCondition) -> Self {
        self.hook.conditions.push(condition);
        self
    }

    pub fn action(mut self, action: HookAction) -> Self {
        self.hook.actions.push(action);
        self
    }

    pub fn actions(mut self, actions: Vec<HookAction>) -> Self {
        self.hook.actions = actions;
        self
    }

    pub fn config(mut self, config: HookConfig) -> Self {
        self.hook.config = config;
        self
    }

    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.hook.config.tags = tags;
        self
    }

    pub fn priority(mut self, priority: u8) -> Self {
        self.hook.config.priority = priority;
        self
    }

    pub fn build(self) -> Hook {
        self.hook
    }
}

// ============================================================================
// Preset Hooks - Common automations
// ============================================================================

pub mod presets {
    use super::*;

    /// Morning briefing hook
    pub fn morning_briefing() -> Hook {
        HookBuilder::new("Morning Briefing")
            .description("Daily morning summary of calendar, emails, and tasks")
            .trigger(HookTrigger::Daily {
                times: vec!["07:30".to_string()],
            })
            .action(HookAction::RunAgent {
                agent_name: "assistant".to_string(),
                input:
                    "Give me a morning briefing: today's calendar, important emails, and top tasks"
                        .to_string(),
                context: None,
            })
            .action(HookAction::Notification {
                title: "Good Morning!".to_string(),
                body: "Your daily briefing is ready".to_string(),
                sound: Some("default".to_string()),
                actions: None,
            })
            .tags(vec!["daily".to_string(), "productivity".to_string()])
            .build()
    }

    /// Meeting prep hook
    pub fn meeting_prep() -> Hook {
        HookBuilder::new("Meeting Prep")
            .description("Prepare summary 10 minutes before meetings")
            .trigger(HookTrigger::CalendarEvent {
                calendar_id: "primary".to_string(),
                event_type: CalendarEventType::Starting,
                minutes_before: Some(10),
            })
            .action(HookAction::RunAgent {
                agent_name: "researcher".to_string(),
                input: "Research the attendees and prepare talking points for this meeting"
                    .to_string(),
                context: None,
            })
            .action(HookAction::Notification {
                title: "Meeting in 10 minutes".to_string(),
                body: "Prep notes ready".to_string(),
                sound: None,
                actions: None,
            })
            .build()
    }

    /// Auto-respond to emails
    pub fn email_auto_respond() -> Hook {
        HookBuilder::new("Email Auto-Respond")
            .description("Draft responses to emails matching criteria")
            .trigger(HookTrigger::EmailReceived {
                account: "primary".to_string(),
                filters: Some(EmailFilters {
                    from: None,
                    subject_contains: None,
                    has_attachment: None,
                    labels: Some(vec!["needs-response".to_string()]),
                }),
            })
            .action(HookAction::RunAgent {
                agent_name: "writer".to_string(),
                input: "Draft a professional response to this email".to_string(),
                context: None,
            })
            .build()
    }

    /// Code review reminder
    pub fn code_review_reminder() -> Hook {
        HookBuilder::new("Code Review Reminder")
            .description("Remind about pending code reviews")
            .trigger(HookTrigger::Schedule {
                cron: "0 10,15 * * 1-5".to_string(),
                timezone: None,
            })
            .action(HookAction::HttpRequest {
                method: "GET".to_string(),
                url: "https://api.github.com/user/repos".to_string(),
                headers: None,
                body: None,
            })
            .action(HookAction::Notification {
                title: "Code Reviews".to_string(),
                body: "You have pending reviews".to_string(),
                sound: None,
                actions: Some(vec!["Open GitHub".to_string()]),
            })
            .build()
    }

    /// Smart home goodnight routine
    pub fn goodnight_routine() -> Hook {
        HookBuilder::new("Goodnight Routine")
            .description("Turn off lights and set thermostat at bedtime")
            .trigger(HookTrigger::Daily {
                times: vec!["23:00".to_string()],
            })
            .condition(HookCondition::TimeWindow {
                start: "22:00".to_string(),
                end: "01:00".to_string(),
                days: None,
            })
            .action(HookAction::SetScene {
                scene_id: "goodnight".to_string(),
            })
            .action(HookAction::ControlDevice {
                device_id: "thermostat".to_string(),
                action: "set_temperature".to_string(),
                parameters: Some(
                    [("temperature".to_string(), serde_json::json!(68))]
                        .into_iter()
                        .collect(),
                ),
            })
            .build()
    }

    /// Backup important files
    pub fn daily_backup() -> Hook {
        HookBuilder::new("Daily Backup")
            .description("Backup important directories daily")
            .trigger(HookTrigger::Schedule {
                cron: "0 2 * * *".to_string(),
                timezone: None,
            })
            .action(HookAction::SyncFolder {
                source: "~/Documents".to_string(),
                destination: "~/Backups/Documents".to_string(),
                delete_extra: false,
            })
            .action(HookAction::SyncFolder {
                source: "~/Projects".to_string(),
                destination: "~/Backups/Projects".to_string(),
                delete_extra: false,
            })
            .action(HookAction::Log {
                level: "info".to_string(),
                message: "Daily backup completed".to_string(),
            })
            .build()
    }

    /// Focus mode
    pub fn focus_mode() -> Hook {
        HookBuilder::new("Focus Mode")
            .description("Block distractions and notify status")
            .trigger(HookTrigger::Manual)
            .action(HookAction::SlackMessage {
                channel: "#status".to_string(),
                text: "[Focus] In focus mode - will respond later".to_string(),
                thread_ts: None,
            })
            .action(HookAction::RunCommand {
                command: "osascript".to_string(),
                args: vec![
                    "-e".to_string(),
                    "tell application \"System Events\" to set do not disturb to true".to_string(),
                ],
                cwd: None,
                env: None,
            })
            .build()
    }

    /// Expense tracking
    pub fn expense_tracker() -> Hook {
        HookBuilder::new("Expense Tracker")
            .description("Log expenses from receipts")
            .trigger(HookTrigger::EmailReceived {
                account: "primary".to_string(),
                filters: Some(EmailFilters {
                    from: None,
                    subject_contains: Some("receipt".to_string()),
                    has_attachment: Some(true),
                    labels: None,
                }),
            })
            .action(HookAction::ExtractData {
                content: "${email.body}".to_string(),
                schema: serde_json::json!({
                    "vendor": "string",
                    "amount": "number",
                    "date": "date",
                    "category": "string"
                }),
            })
            .build()
    }

    /// Get all preset hooks
    pub fn all() -> Vec<Hook> {
        vec![
            morning_briefing(),
            meeting_prep(),
            email_auto_respond(),
            code_review_reminder(),
            goodnight_routine(),
            daily_backup(),
            focus_mode(),
            expense_tracker(),
        ]
    }
}
