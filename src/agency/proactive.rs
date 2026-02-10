// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Proactive Agent System
//!
//! Agents that monitor, detect problems, and solve them with user permission.
//!
//! ## Household Agent
//! - Smart home monitoring
//! - Bill and expense tracking
//! - Maintenance scheduling
//! - Grocery and supply management
//!
//! ## Business Agent
//! - Calendar optimization
//! - Email triage and follow-ups
//! - Meeting preparation
//! - Project health monitoring

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Permission System
// =============================================================================

/// Permission level for proactive actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum PermissionLevel {
    /// Agent can only send notifications, no actions taken
    NotifyOnly,
    /// Auto-approve low-risk actions (notifications, reading data)
    #[default]
    LowRisk,
    /// Auto-approve medium-risk (scheduling, drafts)
    MediumRisk,
    /// High autonomy - approve most except financial/external comms
    HighAutonomy,
}

impl PermissionLevel {
    /// Actions that can be auto-approved at this level
    pub fn auto_approve_actions(&self) -> Vec<&'static str> {
        match self {
            PermissionLevel::NotifyOnly => vec![],
            PermissionLevel::LowRisk => vec![
                "send_notification",
                "calendar_read",
                "email_read",
                "web_search",
                "package_track",
                "weather_check",
                "energy_read",
            ],
            PermissionLevel::MediumRisk => vec![
                "send_notification",
                "calendar_read",
                "calendar_create",
                "email_read",
                "email_draft",
                "document_create",
                "web_search",
                "package_track",
                "weather_check",
                "energy_read",
                "reminder_create",
                "task_create",
            ],
            PermissionLevel::HighAutonomy => vec!["*"], // All except blocklist
        }
    }

    /// Actions that always require approval regardless of level
    pub fn always_require_approval() -> Vec<&'static str> {
        vec![
            "bill_pay",
            "email_send",
            "slack_send",
            "teams_send",
            "discord_send",
            "sms_send",
            "expense_submit",
            "purchase",
            "transfer_money",
            "delete_file",
            "share_external",
            "change_password",
        ]
    }

    /// Check if an action can be auto-approved
    pub fn can_auto_approve(&self, action: &str) -> bool {
        // Check blocklist first
        if Self::always_require_approval().contains(&action) {
            return false;
        }

        let allowed = self.auto_approve_actions();
        allowed.contains(&"*") || allowed.contains(&action)
    }
}

// =============================================================================
// Problem Detection
// =============================================================================

/// Severity level of a detected problem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProblemSeverity {
    /// Informational, no action required
    Info,
    /// Warning, action recommended
    Warning,
    /// Urgent, action needed soon
    Urgent,
    /// Critical, immediate action required
    Critical,
}

/// Status of a detected problem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProblemStatus {
    /// Newly detected
    #[default]
    New,
    /// User acknowledged
    Acknowledged,
    /// Being addressed
    InProgress,
    /// Problem resolved
    Resolved,
    /// User dismissed
    Dismissed,
}

/// Category of problem (household or business)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProblemCategory {
    // Household
    BillDue {
        amount: f64,
        due_date: DateTime<Utc>,
        vendor: String,
    },
    MaintenanceNeeded {
        item: String,
        urgency: String,
    },
    SupplyLow {
        item: String,
        current_quantity: u32,
        reorder_threshold: u32,
    },
    EnergyAnomaly {
        device: String,
        expected_kwh: f64,
        actual_kwh: f64,
    },
    DeviceOffline {
        device_id: String,
        device_name: String,
        last_seen: DateTime<Utc>,
    },
    SecurityAlert {
        alert_type: String,
        location: String,
    },
    PackageDelayed {
        tracking_id: String,
        carrier: String,
        expected_date: String,
    },
    AppointmentReminder {
        title: String,
        time: DateTime<Utc>,
        location: Option<String>,
    },
    WeatherAlert {
        alert_type: String,
        severity: String,
    },
    SubscriptionRenewal {
        service: String,
        amount: f64,
        renewal_date: DateTime<Utc>,
    },

    // Business
    CalendarConflict {
        event1: String,
        event2: String,
        overlap_minutes: u32,
    },
    DeadlineApproaching {
        project: String,
        deadline: DateTime<Utc>,
        days_remaining: u32,
    },
    EmailUrgent {
        from: String,
        subject: String,
        received_at: DateTime<Utc>,
    },
    MeetingPrepNeeded {
        meeting: String,
        time: DateTime<Utc>,
        prep_items: Vec<String>,
    },
    FollowUpDue {
        context: String,
        person: String,
        due_date: DateTime<Utc>,
    },
    ExpensePending {
        amount: f64,
        category: String,
        days_pending: u32,
    },
    ProjectAtRisk {
        project: String,
        risk_factors: Vec<String>,
    },
    CompetitorNews {
        competitor: String,
        headline: String,
    },
    TeamBlocker {
        team_member: String,
        blocker: String,
    },
    ReportDue {
        report_name: String,
        due_date: DateTime<Utc>,
    },

    /// Custom problem type
    Custom {
        category: String,
        details: HashMap<String, serde_json::Value>,
    },
}

/// A detected problem requiring attention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedProblem {
    /// Unique problem ID
    pub id: String,
    /// Agent that detected the problem
    pub agent_id: String,
    /// Problem category with details
    pub category: ProblemCategory,
    /// Human-readable title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Problem severity
    pub severity: ProblemSeverity,
    /// When the problem was detected
    pub detected_at: DateTime<Utc>,
    /// Source of detection (integration, scan, etc.)
    pub source: String,
    /// Suggested actions to resolve
    pub suggested_actions: Vec<ProactiveAction>,
    /// Current status
    pub status: ProblemStatus,
    /// When the problem was resolved
    pub resolved_at: Option<DateTime<Utc>>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

// =============================================================================
// Proactive Actions
// =============================================================================

/// Risk level of an action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionRisk {
    Low,
    Medium,
    High,
}

/// Status of a proactive action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    /// Waiting for user approval
    #[default]
    Pending,
    /// User approved the action
    Approved,
    /// User rejected the action
    Rejected,
    /// Action was executed
    Executed,
    /// Action was cancelled
    Cancelled,
    /// Action failed during execution
    Failed,
}

/// A proactive action proposed by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveAction {
    /// Unique action ID
    pub id: String,
    /// Agent proposing the action
    pub agent_id: String,
    /// Related problem ID (if any)
    pub problem_id: Option<String>,
    /// Type of action (e.g., "send_email", "schedule_maintenance")
    pub action_type: String,
    /// Human-readable description
    pub description: String,
    /// Why the agent recommends this action
    pub reasoning: String,
    /// Estimated impact/benefit
    pub estimated_impact: Option<String>,
    /// Risk level
    pub risk_level: ActionRisk,
    /// Current status
    pub status: ActionStatus,
    /// Whether it was auto-approved based on permission level
    pub auto_approved: bool,
    /// When the action was approved
    pub approved_at: Option<DateTime<Utc>>,
    /// Who approved (user ID or "auto")
    pub approved_by: Option<String>,
    /// When the action was executed
    pub executed_at: Option<DateTime<Utc>>,
    /// Result of execution
    pub result: Option<String>,
    /// Error if execution failed
    pub error: Option<String>,
    /// Action parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// When the action was created
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// Proactive Agent Configuration
// =============================================================================

/// Configuration for a proactive agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveAgentConfig {
    /// Agent ID
    pub agent_id: String,
    /// Whether the agent is enabled
    pub enabled: bool,
    /// Permission level for auto-approving actions
    pub permission_level: PermissionLevel,
    /// Integrations to monitor
    pub monitored_integrations: Vec<String>,
    /// Problem categories to watch for
    pub watch_categories: Vec<String>,
    /// How often to scan (in seconds)
    pub scan_interval_secs: u64,
    /// Quiet hours - don't notify during these times
    pub quiet_hours: Option<QuietHours>,
    /// Maximum pending actions before requiring review
    pub max_pending_actions: u32,
    /// Custom rules for auto-approval
    pub custom_rules: Vec<ApprovalRule>,
}

impl Default for ProactiveAgentConfig {
    fn default() -> Self {
        Self {
            agent_id: String::new(),
            enabled: true,
            permission_level: PermissionLevel::LowRisk,
            monitored_integrations: Vec::new(),
            watch_categories: Vec::new(),
            scan_interval_secs: 300, // 5 minutes
            quiet_hours: None,
            max_pending_actions: 10,
            custom_rules: Vec::new(),
        }
    }
}

/// Quiet hours configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuietHours {
    /// Start time (e.g., "22:00")
    pub start: String,
    /// End time (e.g., "07:00")
    pub end: String,
    /// Days to apply (0=Sunday, 6=Saturday)
    pub days: Vec<u8>,
    /// Whether to still allow critical alerts
    pub allow_critical: bool,
}

/// Custom approval rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRule {
    /// Rule name
    pub name: String,
    /// Action type to match
    pub action_type: String,
    /// Conditions that must be met
    pub conditions: Vec<RuleCondition>,
    /// Whether to auto-approve if conditions match
    pub auto_approve: bool,
}

/// Condition for an approval rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    /// Field to check
    pub field: String,
    /// Operator
    pub operator: ConditionOperator,
    /// Value to compare
    pub value: serde_json::Value,
}

/// Condition operators
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    Contains,
    StartsWith,
    EndsWith,
    Matches,
}

// =============================================================================
// Household Agent Specifics
// =============================================================================

/// Household agent preset configuration
pub fn household_agent_config() -> ProactiveAgentConfig {
    ProactiveAgentConfig {
        agent_id: "household".to_string(),
        enabled: true,
        permission_level: PermissionLevel::LowRisk,
        monitored_integrations: vec![
            "home_assistant".to_string(),
            "google_calendar".to_string(),
            "gmail".to_string(),
            "todoist".to_string(),
            "plaid".to_string(),
            "amazon".to_string(),
            "instacart".to_string(),
            "hue".to_string(),
            "nest".to_string(),
        ],
        watch_categories: vec![
            "bill_due".to_string(),
            "maintenance_needed".to_string(),
            "supply_low".to_string(),
            "energy_anomaly".to_string(),
            "device_offline".to_string(),
            "security_alert".to_string(),
            "package_delayed".to_string(),
            "appointment_reminder".to_string(),
            "weather_alert".to_string(),
            "subscription_renewal".to_string(),
        ],
        scan_interval_secs: 300,
        quiet_hours: Some(QuietHours {
            start: "22:00".to_string(),
            end: "07:00".to_string(),
            days: vec![0, 1, 2, 3, 4, 5, 6],
            allow_critical: true,
        }),
        max_pending_actions: 10,
        custom_rules: vec![ApprovalRule {
            name: "auto_remind_low_supplies".to_string(),
            action_type: "send_notification".to_string(),
            conditions: vec![RuleCondition {
                field: "category".to_string(),
                operator: ConditionOperator::Equals,
                value: serde_json::json!("supply_low"),
            }],
            auto_approve: true,
        }],
    }
}

// =============================================================================
// Business Agent Specifics
// =============================================================================

/// Business agent preset configuration
pub fn business_agent_config() -> ProactiveAgentConfig {
    ProactiveAgentConfig {
        agent_id: "business".to_string(),
        enabled: true,
        permission_level: PermissionLevel::MediumRisk,
        monitored_integrations: vec![
            "google_calendar".to_string(),
            "outlook".to_string(),
            "gmail".to_string(),
            "slack".to_string(),
            "teams".to_string(),
            "notion".to_string(),
            "linear".to_string(),
            "github".to_string(),
        ],
        watch_categories: vec![
            "calendar_conflict".to_string(),
            "deadline_approaching".to_string(),
            "email_urgent".to_string(),
            "meeting_prep_needed".to_string(),
            "follow_up_due".to_string(),
            "expense_pending".to_string(),
            "project_at_risk".to_string(),
            "competitor_news".to_string(),
            "team_blocker".to_string(),
            "report_due".to_string(),
        ],
        scan_interval_secs: 300,
        quiet_hours: Some(QuietHours {
            start: "20:00".to_string(),
            end: "08:00".to_string(),
            days: vec![0, 6], // Weekends only
            allow_critical: true,
        }),
        max_pending_actions: 15,
        custom_rules: vec![
            ApprovalRule {
                name: "auto_prep_meeting_docs".to_string(),
                action_type: "document_create".to_string(),
                conditions: vec![RuleCondition {
                    field: "category".to_string(),
                    operator: ConditionOperator::Equals,
                    value: serde_json::json!("meeting_prep_needed"),
                }],
                auto_approve: true,
            },
            ApprovalRule {
                name: "auto_flag_deadline".to_string(),
                action_type: "send_notification".to_string(),
                conditions: vec![RuleCondition {
                    field: "days_remaining".to_string(),
                    operator: ConditionOperator::LessThan,
                    value: serde_json::json!(3),
                }],
                auto_approve: true,
            },
        ],
    }
}

// =============================================================================
// Agent Traits
// =============================================================================

/// Trait for proactive monitoring agents
#[async_trait::async_trait]
pub trait ProactiveMonitor: Send + Sync {
    /// Scan integrations for problems
    async fn scan(&self) -> Result<Vec<DetectedProblem>, Box<dyn std::error::Error + Send + Sync>>;

    /// Propose actions for a detected problem
    async fn propose_actions(
        &self,
        problem: &DetectedProblem,
    ) -> Result<Vec<ProactiveAction>, Box<dyn std::error::Error + Send + Sync>>;

    /// Execute an approved action
    async fn execute_action(
        &self,
        action: &ProactiveAction,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Check if an action can be auto-approved
    fn can_auto_approve(&self, action: &ProactiveAction) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_levels() {
        assert!(!PermissionLevel::NotifyOnly.can_auto_approve("send_notification"));
        assert!(PermissionLevel::LowRisk.can_auto_approve("send_notification"));
        assert!(PermissionLevel::LowRisk.can_auto_approve("calendar_read"));
        assert!(!PermissionLevel::LowRisk.can_auto_approve("bill_pay"));
        assert!(PermissionLevel::MediumRisk.can_auto_approve("calendar_create"));
        assert!(!PermissionLevel::HighAutonomy.can_auto_approve("email_send")); // Always requires approval
    }

    #[test]
    fn test_household_config() {
        let config = household_agent_config();
        assert_eq!(config.agent_id, "household");
        assert!(config
            .monitored_integrations
            .contains(&"home_assistant".to_string()));
    }

    #[test]
    fn test_business_config() {
        let config = business_agent_config();
        assert_eq!(config.agent_id, "business");
        assert!(config
            .watch_categories
            .contains(&"calendar_conflict".to_string()));
    }
}
