// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Integration Registry
//!
//! Central registry for all available integrations.

#![allow(dead_code)]

use super::{AuthMethod, Capability, IntegrationResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Integration category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationCategory {
    // Productivity
    Calendar,
    Email,
    Notes,
    Tasks,
    Documents,

    // Communication
    Chat,
    VideoConference,
    Social,
    Messaging,

    // Development
    Git,
    Ide,
    Terminal,
    Ci,
    Containers,
    Cloud,

    // Browser & Web
    Browser,
    Automation,
    Scraping,

    // Smart Home & IoT
    SmartHome,
    Iot,
    Voice,

    // Media & Entertainment
    Music,
    Video,
    Podcasts,
    Reading,

    // Finance
    Banking,
    Crypto,
    Trading,
    Payments,

    // Health & Fitness
    Health,
    Fitness,
    Sleep,
    Nutrition,

    // Travel & Transport
    Maps,
    RideShare,
    Travel,

    // Shopping & Commerce
    Shopping,
    Grocery,
    Food,

    // Utilities
    Weather,
    Storage,
    Backup,
    Security,

    // System
    System,
    Clipboard,
    Notifications,
}

/// Integration status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationStatus {
    Available,
    Connected,
    Disconnected,
    NeedsAuth,
    Error,
    Disabled,
}

/// Integration definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Integration {
    /// Unique identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Category
    pub category: IntegrationCategory,
    /// Icon (emoji or icon name)
    pub icon: String,
    /// Brand color
    pub color: String,
    /// Website URL
    pub website: Option<String>,
    /// Status
    pub status: IntegrationStatus,
    /// Available capabilities
    pub capabilities: Vec<Capability>,
    /// Required authentication method
    pub auth_method: Option<AuthMethod>,
    /// Whether this integration is a native/built-in
    pub is_native: bool,
    /// Platform availability
    pub platforms: Vec<String>,
    /// Setup instructions
    pub setup_instructions: Option<String>,
}

/// Integration registry
pub struct IntegrationRegistry {
    integrations: HashMap<String, Integration>,
}

impl IntegrationRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            integrations: HashMap::new(),
        };
        registry.register_all();
        registry
    }

    fn register_all(&mut self) {
        // =====================================================================
        // Productivity - Calendar
        // =====================================================================
        self.register(Integration {
            id: "google_calendar".to_string(),
            name: "Google Calendar".to_string(),
            description: "Manage events, meetings, and schedules".to_string(),
            category: IntegrationCategory::Calendar,
            icon: "".to_string(),
            color: "#4285f4".to_string(),
            website: Some("https://calendar.google.com".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("list_events", "List Events", "Get upcoming calendar events"),
                capability("create_event", "Create Event", "Create a new calendar event"),
                capability("update_event", "Update Event", "Modify an existing event"),
                capability("delete_event", "Delete Event", "Remove a calendar event"),
                capability("find_free_time", "Find Free Time", "Find available time slots"),
            ],
            auth_method: Some(AuthMethod::OAuth2(super::OAuthConfig {
                client_id: String::new(),
                client_secret: None,
                auth_url: "https://accounts.google.com/o/oauth2/auth".to_string(),
                token_url: "https://oauth2.googleapis.com/token".to_string(),
                scopes: vec!["https://www.googleapis.com/auth/calendar".to_string()],
                redirect_uri: "http://localhost:8080/oauth/callback".to_string(),
            })),
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: Some("1. Create OAuth credentials in Google Cloud Console\n2. Add credentials to CSM settings".to_string()),
        });

        self.register(Integration {
            id: "outlook_calendar".to_string(),
            name: "Outlook Calendar".to_string(),
            description: "Microsoft 365 calendar integration".to_string(),
            category: IntegrationCategory::Calendar,
            icon: "".to_string(),
            color: "#0078d4".to_string(),
            website: Some("https://outlook.office.com/calendar".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("list_events", "List Events", "Get upcoming calendar events"),
                capability(
                    "create_event",
                    "Create Event",
                    "Create a new calendar event",
                ),
                capability(
                    "schedule_meeting",
                    "Schedule Meeting",
                    "Schedule a Teams meeting",
                ),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "apple_calendar".to_string(),
            name: "Apple Calendar".to_string(),
            description: "macOS/iOS native calendar".to_string(),
            category: IntegrationCategory::Calendar,
            icon: "".to_string(),
            color: "#ff3b30".to_string(),
            website: None,
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("list_events", "List Events", "Get upcoming calendar events"),
                capability(
                    "create_event",
                    "Create Event",
                    "Create a new calendar event",
                ),
            ],
            auth_method: None,
            is_native: true,
            platforms: vec!["macos".to_string(), "ios".to_string()],
            setup_instructions: None,
        });

        // =====================================================================
        // Productivity - Email
        // =====================================================================
        self.register(Integration {
            id: "gmail".to_string(),
            name: "Gmail".to_string(),
            description: "Read, send, and organize emails".to_string(),
            category: IntegrationCategory::Email,
            icon: "".to_string(),
            color: "#ea4335".to_string(),
            website: Some("https://mail.google.com".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("list_emails", "List Emails", "Get emails from inbox"),
                capability("send_email", "Send Email", "Send a new email"),
                capability("reply_email", "Reply to Email", "Reply to an email thread"),
                capability("label_email", "Label Email", "Apply labels to emails"),
                capability("search_emails", "Search Emails", "Search through emails"),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "outlook_mail".to_string(),
            name: "Outlook Mail".to_string(),
            description: "Microsoft 365 email".to_string(),
            category: IntegrationCategory::Email,
            icon: "".to_string(),
            color: "#0078d4".to_string(),
            website: Some("https://outlook.office.com".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("list_emails", "List Emails", "Get emails from inbox"),
                capability("send_email", "Send Email", "Send a new email"),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        // =====================================================================
        // Productivity - Notes
        // =====================================================================
        self.register(Integration {
            id: "notion".to_string(),
            name: "Notion".to_string(),
            description: "All-in-one workspace for notes, docs, and wikis".to_string(),
            category: IntegrationCategory::Notes,
            icon: "".to_string(),
            color: "#000000".to_string(),
            website: Some("https://notion.so".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("create_page", "Create Page", "Create a new Notion page"),
                capability("update_page", "Update Page", "Update page content"),
                capability("search", "Search", "Search Notion workspace"),
                capability(
                    "create_database",
                    "Create Database",
                    "Create a new database",
                ),
                capability("query_database", "Query Database", "Query database entries"),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "obsidian".to_string(),
            name: "Obsidian".to_string(),
            description: "Local-first knowledge base with markdown".to_string(),
            category: IntegrationCategory::Notes,
            icon: "".to_string(),
            color: "#7c3aed".to_string(),
            website: Some("https://obsidian.md".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("create_note", "Create Note", "Create a new markdown note"),
                capability("update_note", "Update Note", "Update note content"),
                capability("search", "Search", "Search vault"),
                capability("list_notes", "List Notes", "List notes in folder"),
                capability("get_backlinks", "Get Backlinks", "Get note backlinks"),
            ],
            auth_method: None,
            is_native: true,
            platforms: vec!["all".to_string()],
            setup_instructions: Some("Set vault path in CSM settings".to_string()),
        });

        self.register(Integration {
            id: "apple_notes".to_string(),
            name: "Apple Notes".to_string(),
            description: "Native Apple notes app".to_string(),
            category: IntegrationCategory::Notes,
            icon: "".to_string(),
            color: "#ffcc00".to_string(),
            website: None,
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("create_note", "Create Note", "Create a new note"),
                capability("list_notes", "List Notes", "List all notes"),
            ],
            auth_method: None,
            is_native: true,
            platforms: vec!["macos".to_string(), "ios".to_string()],
            setup_instructions: None,
        });

        // =====================================================================
        // Productivity - Tasks
        // =====================================================================
        self.register(Integration {
            id: "todoist".to_string(),
            name: "Todoist".to_string(),
            description: "Task management and to-do lists".to_string(),
            category: IntegrationCategory::Tasks,
            icon: "".to_string(),
            color: "#e44332".to_string(),
            website: Some("https://todoist.com".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("list_tasks", "List Tasks", "Get tasks from projects"),
                capability("create_task", "Create Task", "Create a new task"),
                capability("complete_task", "Complete Task", "Mark task as complete"),
                capability("list_projects", "List Projects", "Get all projects"),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "things".to_string(),
            name: "Things 3".to_string(),
            description: "Beautiful task manager for Apple devices".to_string(),
            category: IntegrationCategory::Tasks,
            icon: "".to_string(),
            color: "#4a90d9".to_string(),
            website: Some("https://culturedcode.com/things".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("create_task", "Create Task", "Create a new to-do"),
                capability("list_today", "Today", "Get today's tasks"),
            ],
            auth_method: None,
            is_native: true,
            platforms: vec!["macos".to_string(), "ios".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "apple_reminders".to_string(),
            name: "Apple Reminders".to_string(),
            description: "Native Apple reminders".to_string(),
            category: IntegrationCategory::Tasks,
            icon: "".to_string(),
            color: "#ff9500".to_string(),
            website: None,
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability(
                    "create_reminder",
                    "Create Reminder",
                    "Create a new reminder",
                ),
                capability("list_reminders", "List Reminders", "Get all reminders"),
            ],
            auth_method: None,
            is_native: true,
            platforms: vec!["macos".to_string(), "ios".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "linear".to_string(),
            name: "Linear".to_string(),
            description: "Issue tracking for modern software teams".to_string(),
            category: IntegrationCategory::Tasks,
            icon: "".to_string(),
            color: "#5e6ad2".to_string(),
            website: Some("https://linear.app".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("create_issue", "Create Issue", "Create a new issue"),
                capability("list_issues", "List Issues", "Get issues"),
                capability("update_issue", "Update Issue", "Update issue status"),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        // =====================================================================
        // Communication - Chat
        // =====================================================================
        self.register(Integration {
            id: "slack".to_string(),
            name: "Slack".to_string(),
            description: "Team communication and collaboration".to_string(),
            category: IntegrationCategory::Chat,
            icon: "".to_string(),
            color: "#4a154b".to_string(),
            website: Some("https://slack.com".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability(
                    "send_message",
                    "Send Message",
                    "Send a message to a channel",
                ),
                capability("list_channels", "List Channels", "Get all channels"),
                capability("set_status", "Set Status", "Update your status"),
                capability("search", "Search", "Search messages"),
                capability("upload_file", "Upload File", "Share a file"),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "discord".to_string(),
            name: "Discord".to_string(),
            description: "Voice, video, and text communication".to_string(),
            category: IntegrationCategory::Chat,
            icon: "".to_string(),
            color: "#5865f2".to_string(),
            website: Some("https://discord.com".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability(
                    "send_message",
                    "Send Message",
                    "Send a message to a channel",
                ),
                capability("list_servers", "List Servers", "Get all servers"),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "teams".to_string(),
            name: "Microsoft Teams".to_string(),
            description: "Microsoft 365 collaboration".to_string(),
            category: IntegrationCategory::Chat,
            icon: "".to_string(),
            color: "#6264a7".to_string(),
            website: Some("https://teams.microsoft.com".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability(
                    "send_message",
                    "Send Message",
                    "Send a message to a channel",
                ),
                capability(
                    "schedule_meeting",
                    "Schedule Meeting",
                    "Schedule a Teams meeting",
                ),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        // =====================================================================
        // Communication - Messaging
        // =====================================================================
        self.register(Integration {
            id: "telegram".to_string(),
            name: "Telegram".to_string(),
            description: "Secure messaging".to_string(),
            category: IntegrationCategory::Messaging,
            icon: "".to_string(),
            color: "#0088cc".to_string(),
            website: Some("https://telegram.org".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("send_message", "Send Message", "Send a message"),
                capability("send_photo", "Send Photo", "Send an image"),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "whatsapp".to_string(),
            name: "WhatsApp".to_string(),
            description: "Messaging and calls".to_string(),
            category: IntegrationCategory::Messaging,
            icon: "".to_string(),
            color: "#25d366".to_string(),
            website: Some("https://whatsapp.com".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![capability("send_message", "Send Message", "Send a message")],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "imessage".to_string(),
            name: "iMessage".to_string(),
            description: "Apple Messages".to_string(),
            category: IntegrationCategory::Messaging,
            icon: "".to_string(),
            color: "#34c759".to_string(),
            website: None,
            status: IntegrationStatus::Available,
            capabilities: vec![capability(
                "send_message",
                "Send Message",
                "Send an iMessage",
            )],
            auth_method: None,
            is_native: true,
            platforms: vec!["macos".to_string(), "ios".to_string()],
            setup_instructions: None,
        });

        // =====================================================================
        // Development - Git
        // =====================================================================
        self.register(Integration {
            id: "github".to_string(),
            name: "GitHub".to_string(),
            description: "Code hosting and collaboration".to_string(),
            category: IntegrationCategory::Git,
            icon: "".to_string(),
            color: "#24292e".to_string(),
            website: Some("https://github.com".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("list_repos", "List Repos", "Get repositories"),
                capability("create_issue", "Create Issue", "Create a new issue"),
                capability("create_pr", "Create PR", "Create a pull request"),
                capability("list_prs", "List PRs", "Get pull requests"),
                capability("merge_pr", "Merge PR", "Merge a pull request"),
                capability("create_branch", "Create Branch", "Create a new branch"),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "gitlab".to_string(),
            name: "GitLab".to_string(),
            description: "DevOps platform".to_string(),
            category: IntegrationCategory::Git,
            icon: "".to_string(),
            color: "#fc6d26".to_string(),
            website: Some("https://gitlab.com".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("list_projects", "List Projects", "Get projects"),
                capability("create_issue", "Create Issue", "Create a new issue"),
                capability("create_mr", "Create MR", "Create a merge request"),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        // =====================================================================
        // Browser & Automation
        // =====================================================================
        self.register(Integration {
            id: "chrome".to_string(),
            name: "Chrome".to_string(),
            description: "Browser control and automation".to_string(),
            category: IntegrationCategory::Browser,
            icon: "".to_string(),
            color: "#4285f4".to_string(),
            website: Some("https://chrome.google.com".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("open_url", "Open URL", "Open a URL in Chrome"),
                capability("list_tabs", "List Tabs", "Get open tabs"),
                capability("close_tab", "Close Tab", "Close a tab"),
                capability("get_bookmarks", "Get Bookmarks", "Get bookmarks"),
                capability("get_history", "Get History", "Get browsing history"),
            ],
            auth_method: None,
            is_native: true,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "arc".to_string(),
            name: "Arc Browser".to_string(),
            description: "Modern browser with spaces".to_string(),
            category: IntegrationCategory::Browser,
            icon: "".to_string(),
            color: "#5c5ce6".to_string(),
            website: Some("https://arc.net".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("open_url", "Open URL", "Open a URL in Arc"),
                capability("list_tabs", "List Tabs", "Get open tabs"),
                capability("list_spaces", "List Spaces", "Get Arc spaces"),
            ],
            auth_method: None,
            is_native: true,
            platforms: vec!["macos".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "playwright".to_string(),
            name: "Playwright".to_string(),
            description: "Browser automation framework".to_string(),
            category: IntegrationCategory::Automation,
            icon: "".to_string(),
            color: "#2ead33".to_string(),
            website: Some("https://playwright.dev".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("run_script", "Run Script", "Run automation script"),
                capability("screenshot", "Screenshot", "Take screenshot"),
                capability("scrape", "Scrape", "Extract page data"),
            ],
            auth_method: None,
            is_native: true,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        // =====================================================================
        // Smart Home
        // =====================================================================
        self.register(Integration {
            id: "home_assistant".to_string(),
            name: "Home Assistant".to_string(),
            description: "Open source home automation".to_string(),
            category: IntegrationCategory::SmartHome,
            icon: "".to_string(),
            color: "#41bdf5".to_string(),
            website: Some("https://home-assistant.io".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("list_devices", "List Devices", "Get all devices"),
                capability("control_device", "Control Device", "Control a device"),
                capability("get_state", "Get State", "Get device state"),
                capability("call_service", "Call Service", "Call a service"),
                capability("run_automation", "Run Automation", "Trigger an automation"),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: Some("1. Get long-lived access token from Home Assistant\n2. Add URL and token to CSM settings".to_string()),
        });

        self.register(Integration {
            id: "homekit".to_string(),
            name: "Apple HomeKit".to_string(),
            description: "Apple smart home control".to_string(),
            category: IntegrationCategory::SmartHome,
            icon: "".to_string(),
            color: "#ff9500".to_string(),
            website: None,
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability(
                    "list_accessories",
                    "List Accessories",
                    "Get all accessories",
                ),
                capability(
                    "control_accessory",
                    "Control Accessory",
                    "Control an accessory",
                ),
                capability("run_scene", "Run Scene", "Activate a scene"),
            ],
            auth_method: None,
            is_native: true,
            platforms: vec!["macos".to_string(), "ios".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "hue".to_string(),
            name: "Philips Hue".to_string(),
            description: "Smart lighting control".to_string(),
            category: IntegrationCategory::SmartHome,
            icon: "".to_string(),
            color: "#0065d3".to_string(),
            website: Some("https://meethue.com".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("list_lights", "List Lights", "Get all lights"),
                capability("control_light", "Control Light", "Control a light"),
                capability("set_scene", "Set Scene", "Activate a scene"),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        // =====================================================================
        // Media
        // =====================================================================
        self.register(Integration {
            id: "spotify".to_string(),
            name: "Spotify".to_string(),
            description: "Music streaming".to_string(),
            category: IntegrationCategory::Music,
            icon: "".to_string(),
            color: "#1db954".to_string(),
            website: Some("https://spotify.com".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("play", "Play", "Play music"),
                capability("pause", "Pause", "Pause playback"),
                capability("next", "Next", "Skip to next track"),
                capability("search", "Search", "Search for music"),
                capability("get_now_playing", "Now Playing", "Get current track"),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "youtube".to_string(),
            name: "YouTube".to_string(),
            description: "Video streaming".to_string(),
            category: IntegrationCategory::Video,
            icon: "".to_string(),
            color: "#ff0000".to_string(),
            website: Some("https://youtube.com".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("search", "Search", "Search for videos"),
                capability("get_video", "Get Video", "Get video info"),
                capability("list_playlists", "List Playlists", "Get playlists"),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        // =====================================================================
        // System
        // =====================================================================
        self.register(Integration {
            id: "shell".to_string(),
            name: "Shell".to_string(),
            description: "Run terminal commands".to_string(),
            category: IntegrationCategory::System,
            icon: "".to_string(),
            color: "#1e1e1e".to_string(),
            website: None,
            status: IntegrationStatus::Connected,
            capabilities: vec![
                capability("run_command", "Run Command", "Execute a shell command"),
                capability("run_script", "Run Script", "Execute a script file"),
            ],
            auth_method: None,
            is_native: true,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "clipboard".to_string(),
            name: "Clipboard".to_string(),
            description: "System clipboard access".to_string(),
            category: IntegrationCategory::Clipboard,
            icon: "".to_string(),
            color: "#6b7280".to_string(),
            website: None,
            status: IntegrationStatus::Connected,
            capabilities: vec![
                capability("get", "Get Clipboard", "Get clipboard contents"),
                capability("set", "Set Clipboard", "Set clipboard contents"),
                capability("history", "Clipboard History", "Get clipboard history"),
            ],
            auth_method: None,
            is_native: true,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        self.register(Integration {
            id: "notifications".to_string(),
            name: "Notifications".to_string(),
            description: "System notifications".to_string(),
            category: IntegrationCategory::Notifications,
            icon: "".to_string(),
            color: "#f97316".to_string(),
            website: None,
            status: IntegrationStatus::Connected,
            capabilities: vec![capability(
                "send",
                "Send Notification",
                "Send a system notification",
            )],
            auth_method: None,
            is_native: true,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        // =====================================================================
        // Finance
        // =====================================================================
        self.register(Integration {
            id: "plaid".to_string(),
            name: "Plaid".to_string(),
            description: "Bank account integration".to_string(),
            category: IntegrationCategory::Banking,
            icon: "".to_string(),
            color: "#00d632".to_string(),
            website: Some("https://plaid.com".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("get_accounts", "Get Accounts", "List connected accounts"),
                capability(
                    "get_transactions",
                    "Get Transactions",
                    "Get recent transactions",
                ),
                capability("get_balance", "Get Balance", "Get account balance"),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        // =====================================================================
        // Travel
        // =====================================================================
        self.register(Integration {
            id: "google_maps".to_string(),
            name: "Google Maps".to_string(),
            description: "Maps and directions".to_string(),
            category: IntegrationCategory::Maps,
            icon: "".to_string(),
            color: "#4285f4".to_string(),
            website: Some("https://maps.google.com".to_string()),
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability(
                    "directions",
                    "Get Directions",
                    "Get directions between locations",
                ),
                capability("search_places", "Search Places", "Search for places"),
                capability("get_eta", "Get ETA", "Get estimated travel time"),
            ],
            auth_method: None,
            is_native: false,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });

        // =====================================================================
        // Weather
        // =====================================================================
        self.register(Integration {
            id: "weather".to_string(),
            name: "Weather".to_string(),
            description: "Weather forecasts".to_string(),
            category: IntegrationCategory::Weather,
            icon: "".to_string(),
            color: "#00b4d8".to_string(),
            website: None,
            status: IntegrationStatus::Available,
            capabilities: vec![
                capability("current", "Current Weather", "Get current weather"),
                capability("forecast", "Forecast", "Get weather forecast"),
            ],
            auth_method: None,
            is_native: true,
            platforms: vec!["all".to_string()],
            setup_instructions: None,
        });
    }

    pub fn register(&mut self, integration: Integration) {
        self.integrations
            .insert(integration.id.clone(), integration);
    }

    pub fn get(&self, id: &str) -> Option<&Integration> {
        self.integrations.get(id)
    }

    pub fn list(&self) -> Vec<&Integration> {
        self.integrations.values().collect()
    }

    pub fn list_by_category(&self, category: IntegrationCategory) -> Vec<&Integration> {
        self.integrations
            .values()
            .filter(|i| i.category == category)
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<&Integration> {
        let query = query.to_lowercase();
        self.integrations
            .values()
            .filter(|i| {
                i.name.to_lowercase().contains(&query)
                    || i.description.to_lowercase().contains(&query)
                    || i.id.contains(&query)
            })
            .collect()
    }

    pub fn count(&self) -> usize {
        self.integrations.len()
    }

    pub fn categories(&self) -> Vec<(IntegrationCategory, usize)> {
        let mut counts: HashMap<IntegrationCategory, usize> = HashMap::new();
        for integration in self.integrations.values() {
            *counts.entry(integration.category).or_insert(0) += 1;
        }
        let mut result: Vec<_> = counts.into_iter().collect();
        result.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        result
    }
}

impl Default for IntegrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn capability(id: &str, name: &str, description: &str) -> Capability {
    Capability {
        id: id.to_string(),
        name: name.to_string(),
        description: description.to_string(),
        permissions: vec![],
        parameters: vec![],
        requires_confirmation: false,
    }
}
