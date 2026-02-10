// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Automation Engine Module
//!
//! Provides workflow automation capabilities:
//! - Rule-based triggers and actions
//! - Scheduled tasks
//! - Event-driven workflows
//! - Conditional logic
//! - Action chaining

use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, Duration, NaiveTime, Timelike, Utc, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// =============================================================================
// Workflow Definition
// =============================================================================

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Workflow ID
    pub id: String,
    /// Workflow name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Whether workflow is enabled
    pub enabled: bool,
    /// Trigger conditions
    pub triggers: Vec<Trigger>,
    /// Conditions that must be met
    pub conditions: Vec<Condition>,
    /// Actions to execute
    pub actions: Vec<Action>,
    /// Error handling strategy
    pub on_error: ErrorStrategy,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last modified timestamp
    pub updated_at: DateTime<Utc>,
    /// Last executed timestamp
    pub last_run: Option<DateTime<Utc>>,
    /// Execution count
    pub run_count: u64,
}

// =============================================================================
// Triggers
// =============================================================================

/// Trigger that starts a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Trigger {
    /// Event-based trigger
    Event {
        /// Event type to listen for
        event_type: String,
        /// Optional filter conditions
        filter: Option<HashMap<String, serde_json::Value>>,
    },
    /// Schedule-based trigger
    Schedule {
        /// Cron expression
        cron: String,
        /// Timezone
        timezone: Option<String>,
    },
    /// Interval-based trigger
    Interval {
        /// Interval in seconds
        seconds: u64,
    },
    /// Time of day trigger
    TimeOfDay {
        /// Time to trigger
        time: NaiveTime,
        /// Days of week (None = every day)
        days: Option<Vec<Weekday>>,
    },
    /// Manual trigger
    Manual,
    /// Webhook trigger
    Webhook {
        /// Webhook path
        path: String,
        /// HTTP methods
        methods: Vec<String>,
    },
    /// File change trigger
    FileChange {
        /// Path pattern
        pattern: String,
        /// Change types
        events: Vec<FileChangeEvent>,
    },
}

/// File change event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeEvent {
    Created,
    Modified,
    Deleted,
    Renamed,
}

// =============================================================================
// Conditions
// =============================================================================

/// Condition that must be met for workflow to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// All sub-conditions must be true
    And { conditions: Vec<Condition> },
    /// Any sub-condition must be true
    Or { conditions: Vec<Condition> },
    /// Negate a condition
    Not { condition: Box<Condition> },
    /// Compare a value
    Compare {
        /// Left operand (supports variable interpolation)
        left: String,
        /// Comparison operator
        operator: CompareOp,
        /// Right operand
        right: serde_json::Value,
    },
    /// Check if value exists
    Exists {
        /// Path to check
        path: String,
    },
    /// Check if value matches pattern
    Matches {
        /// Value to check
        value: String,
        /// Regex pattern
        pattern: String,
    },
    /// Time-based condition
    TimeWindow {
        /// Start time
        start: NaiveTime,
        /// End time
        end: NaiveTime,
        /// Days of week (None = every day)
        days: Option<Vec<Weekday>>,
    },
    /// Custom expression
    Expression {
        /// Expression string
        expr: String,
    },
}

/// Comparison operator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompareOp {
    Equals,
    NotEquals,
    GreaterThan,
    GreaterOrEqual,
    LessThan,
    LessOrEqual,
    Contains,
    StartsWith,
    EndsWith,
    In,
}

// =============================================================================
// Actions
// =============================================================================

/// Action to execute in a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Export sessions
    Export {
        /// Session filter
        filter: SessionFilter,
        /// Export format
        format: String,
        /// Output path
        output: String,
    },
    /// Archive sessions
    Archive {
        /// Session filter
        filter: SessionFilter,
        /// Archive location
        destination: String,
    },
    /// Delete sessions
    Delete {
        /// Session filter
        filter: SessionFilter,
    },
    /// Tag sessions
    Tag {
        /// Session filter
        filter: SessionFilter,
        /// Tags to add
        add_tags: Vec<String>,
        /// Tags to remove
        remove_tags: Vec<String>,
    },
    /// Sync sessions
    Sync {
        /// Provider to sync with
        provider: String,
        /// Direction
        direction: SyncDirection,
    },
    /// Run harvest
    Harvest {
        /// Provider to harvest from
        provider: Option<String>,
    },
    /// Execute plugin
    Plugin {
        /// Plugin ID
        plugin_id: String,
        /// Plugin action
        action: String,
        /// Action parameters
        params: HashMap<String, serde_json::Value>,
    },
    /// Send notification
    Notify {
        /// Notification channel
        channel: NotificationChannel,
        /// Message template
        message: String,
        /// Message title
        title: Option<String>,
    },
    /// Make HTTP request
    Http {
        /// Request URL
        url: String,
        /// HTTP method
        method: String,
        /// Request headers
        headers: HashMap<String, String>,
        /// Request body
        body: Option<String>,
    },
    /// Execute shell command
    Shell {
        /// Command to execute
        command: String,
        /// Working directory
        cwd: Option<String>,
        /// Environment variables
        env: HashMap<String, String>,
    },
    /// Set variable
    SetVariable {
        /// Variable name
        name: String,
        /// Variable value
        value: serde_json::Value,
    },
    /// Conditional action
    If {
        /// Condition
        condition: Condition,
        /// Actions if true
        then: Vec<Action>,
        /// Actions if false
        else_: Option<Vec<Action>>,
    },
    /// Loop action
    ForEach {
        /// Items to iterate
        items: String,
        /// Variable name for current item
        as_var: String,
        /// Actions to execute for each item
        actions: Vec<Action>,
    },
    /// Delay action
    Delay {
        /// Delay in seconds
        seconds: u64,
    },
    /// Log message
    Log {
        /// Log level
        level: LogLevel,
        /// Message
        message: String,
    },
}

/// Session filter for actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFilter {
    /// Provider filter
    pub provider: Option<String>,
    /// Age filter (older than N days)
    pub older_than_days: Option<u32>,
    /// Tags filter
    pub tags: Option<Vec<String>>,
    /// Query filter
    pub query: Option<String>,
}

/// Sync direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncDirection {
    Push,
    Pull,
    Both,
}

/// Notification channel
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotificationChannel {
    /// System notification
    System,
    /// Email
    Email { to: String },
    /// Slack webhook
    Slack { webhook_url: String },
    /// Discord webhook
    Discord { webhook_url: String },
    /// Generic webhook
    Webhook { url: String },
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

/// Error handling strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorStrategy {
    /// Stop workflow on error
    Stop,
    /// Continue to next action
    Continue,
    /// Retry N times
    Retry {
        max_attempts: u32,
        delay_seconds: u64,
    },
    /// Execute fallback actions
    Fallback { actions: Vec<Action> },
}

// =============================================================================
// Execution Context
// =============================================================================

/// Workflow execution context
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Workflow ID
    pub workflow_id: String,
    /// Run ID
    pub run_id: String,
    /// Trigger event
    pub trigger_event: Option<serde_json::Value>,
    /// Variables
    pub variables: HashMap<String, serde_json::Value>,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// Action results
    pub results: Vec<ActionResult>,
}

impl ExecutionContext {
    pub fn new(workflow_id: String, trigger_event: Option<serde_json::Value>) -> Self {
        Self {
            workflow_id,
            run_id: uuid::Uuid::new_v4().to_string(),
            trigger_event,
            variables: HashMap::new(),
            started_at: Utc::now(),
            results: Vec::new(),
        }
    }

    /// Get variable value
    pub fn get_var(&self, name: &str) -> Option<&serde_json::Value> {
        self.variables.get(name)
    }

    /// Set variable value
    pub fn set_var(&mut self, name: String, value: serde_json::Value) {
        self.variables.insert(name, value);
    }

    /// Interpolate variables in a string
    pub fn interpolate(&self, template: &str) -> String {
        let mut result = template.to_string();

        for (key, value) in &self.variables {
            let placeholder = format!("{{{{{}}}}}", key);
            let replacement = match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            result = result.replace(&placeholder, &replacement);
        }

        result
    }
}

/// Result of an action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Action index
    pub action_index: usize,
    /// Success status
    pub success: bool,
    /// Result data
    pub data: Option<serde_json::Value>,
    /// Error message
    pub error: Option<String>,
    /// Execution time (ms)
    pub duration_ms: u64,
    /// Timestamp
    pub executed_at: DateTime<Utc>,
}

// =============================================================================
// Automation Engine
// =============================================================================

/// Workflow run status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Workflow run record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    /// Run ID
    pub id: String,
    /// Workflow ID
    pub workflow_id: String,
    /// Status
    pub status: RunStatus,
    /// Trigger that started the run
    pub trigger: String,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// End time
    pub ended_at: Option<DateTime<Utc>>,
    /// Action results
    pub results: Vec<ActionResult>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Automation engine
pub struct AutomationEngine {
    /// Registered workflows
    workflows: Arc<RwLock<HashMap<String, Workflow>>>,
    /// Run history
    runs: Arc<RwLock<Vec<WorkflowRun>>>,
    /// Max history size
    max_history: usize,
}

impl AutomationEngine {
    pub fn new(max_history: usize) -> Self {
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
            runs: Arc::new(RwLock::new(Vec::new())),
            max_history,
        }
    }

    /// Register a workflow
    pub async fn register(&self, workflow: Workflow) -> Result<()> {
        self.validate_workflow(&workflow)?;
        self.workflows
            .write()
            .await
            .insert(workflow.id.clone(), workflow);
        Ok(())
    }

    /// Validate workflow definition
    fn validate_workflow(&self, workflow: &Workflow) -> Result<()> {
        if workflow.id.is_empty() {
            return Err(anyhow!("Workflow ID cannot be empty"));
        }
        if workflow.triggers.is_empty() {
            return Err(anyhow!("Workflow must have at least one trigger"));
        }
        if workflow.actions.is_empty() {
            return Err(anyhow!("Workflow must have at least one action"));
        }
        Ok(())
    }

    /// Unregister a workflow
    pub async fn unregister(&self, workflow_id: &str) -> Result<()> {
        self.workflows
            .write()
            .await
            .remove(workflow_id)
            .ok_or_else(|| anyhow!("Workflow not found: {}", workflow_id))?;
        Ok(())
    }

    /// Get workflow
    pub async fn get_workflow(&self, workflow_id: &str) -> Option<Workflow> {
        self.workflows.read().await.get(workflow_id).cloned()
    }

    /// List all workflows
    pub async fn list_workflows(&self) -> Vec<Workflow> {
        self.workflows.read().await.values().cloned().collect()
    }

    /// Trigger a workflow manually
    pub async fn trigger(
        &self,
        workflow_id: &str,
        event: Option<serde_json::Value>,
    ) -> Result<String> {
        let workflow = self
            .workflows
            .read()
            .await
            .get(workflow_id)
            .cloned()
            .ok_or_else(|| anyhow!("Workflow not found: {}", workflow_id))?;

        if !workflow.enabled {
            return Err(anyhow!("Workflow is disabled"));
        }

        self.execute_workflow(&workflow, event).await
    }

    /// Execute a workflow
    async fn execute_workflow(
        &self,
        workflow: &Workflow,
        event: Option<serde_json::Value>,
    ) -> Result<String> {
        let mut ctx = ExecutionContext::new(workflow.id.clone(), event);

        // Check conditions
        for condition in &workflow.conditions {
            if !self.evaluate_condition(condition, &ctx).await? {
                return Err(anyhow!("Workflow conditions not met"));
            }
        }

        let run = WorkflowRun {
            id: ctx.run_id.clone(),
            workflow_id: workflow.id.clone(),
            status: RunStatus::Running,
            trigger: "manual".to_string(),
            started_at: ctx.started_at,
            ended_at: None,
            results: Vec::new(),
            error: None,
        };

        self.record_run(run.clone()).await;

        // Execute actions
        let mut final_status = RunStatus::Completed;
        let mut final_error = None;

        for (i, action) in workflow.actions.iter().enumerate() {
            let start = std::time::Instant::now();

            match self.execute_action(action, &mut ctx).await {
                Ok(data) => {
                    ctx.results.push(ActionResult {
                        action_index: i,
                        success: true,
                        data,
                        error: None,
                        duration_ms: start.elapsed().as_millis() as u64,
                        executed_at: Utc::now(),
                    });
                }
                Err(e) => {
                    ctx.results.push(ActionResult {
                        action_index: i,
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                        duration_ms: start.elapsed().as_millis() as u64,
                        executed_at: Utc::now(),
                    });

                    match &workflow.on_error {
                        ErrorStrategy::Stop => {
                            final_status = RunStatus::Failed;
                            final_error = Some(e.to_string());
                            break;
                        }
                        ErrorStrategy::Continue => continue,
                        ErrorStrategy::Retry {
                            max_attempts,
                            delay_seconds,
                        } => {
                            // Simple retry logic
                            let mut retry_success = false;
                            for _ in 0..*max_attempts {
                                tokio::time::sleep(tokio::time::Duration::from_secs(
                                    *delay_seconds,
                                ))
                                .await;
                                if self.execute_action(action, &mut ctx).await.is_ok() {
                                    retry_success = true;
                                    break;
                                }
                            }
                            if !retry_success {
                                final_status = RunStatus::Failed;
                                final_error = Some(e.to_string());
                                break;
                            }
                        }
                        ErrorStrategy::Fallback { actions } => {
                            for fallback in actions {
                                self.execute_action(fallback, &mut ctx).await.ok();
                            }
                        }
                    }
                }
            }
        }

        // Update run record
        self.update_run(&ctx.run_id, final_status, final_error, ctx.results)
            .await;

        // Update workflow stats
        self.update_workflow_stats(&workflow.id).await;

        Ok(ctx.run_id)
    }

    /// Evaluate a condition (boxed for recursion)
    fn evaluate_condition<'a>(
        &'a self,
        condition: &'a Condition,
        ctx: &'a ExecutionContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool>> + Send + 'a>> {
        Box::pin(async move {
            match condition {
                Condition::And { conditions } => {
                    for c in conditions {
                        if !self.evaluate_condition(c, ctx).await? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                }
                Condition::Or { conditions } => {
                    for c in conditions {
                        if self.evaluate_condition(c, ctx).await? {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
                Condition::Not { condition } => {
                    Ok(!self.evaluate_condition(condition, ctx).await?)
                }
                Condition::Compare {
                    left,
                    operator,
                    right,
                } => {
                    let left_val = ctx.interpolate(left);
                    self.compare_values(&left_val, operator, right)
                }
                Condition::Exists { path } => Ok(ctx.variables.contains_key(path)),
                Condition::Matches { value, pattern } => {
                    let val = ctx.interpolate(value);
                    let re = regex::Regex::new(pattern)?;
                    Ok(re.is_match(&val))
                }
                Condition::TimeWindow { start, end, days } => {
                    let now = Utc::now();
                    let current_time = now.time();
                    let current_day = now.weekday();

                    // Check day of week
                    if let Some(valid_days) = days {
                        if !valid_days.contains(&current_day) {
                            return Ok(false);
                        }
                    }

                    // Check time window
                    if start <= end {
                        Ok(current_time >= *start && current_time <= *end)
                    } else {
                        // Window spans midnight
                        Ok(current_time >= *start || current_time <= *end)
                    }
                }
                Condition::Expression { expr: _ } => {
                    // Would need expression evaluator
                    Ok(true)
                }
            }
        })
    }

    fn compare_values(
        &self,
        left: &str,
        op: &CompareOp,
        right: &serde_json::Value,
    ) -> Result<bool> {
        match op {
            CompareOp::Equals => {
                if let serde_json::Value::String(s) = right {
                    Ok(left == s)
                } else {
                    Ok(left == &right.to_string())
                }
            }
            CompareOp::NotEquals => {
                if let serde_json::Value::String(s) = right {
                    Ok(left != s)
                } else {
                    Ok(left != &right.to_string())
                }
            }
            CompareOp::Contains => {
                if let serde_json::Value::String(s) = right {
                    Ok(left.contains(s.as_str()))
                } else {
                    Ok(false)
                }
            }
            CompareOp::StartsWith => {
                if let serde_json::Value::String(s) = right {
                    Ok(left.starts_with(s.as_str()))
                } else {
                    Ok(false)
                }
            }
            CompareOp::EndsWith => {
                if let serde_json::Value::String(s) = right {
                    Ok(left.ends_with(s.as_str()))
                } else {
                    Ok(false)
                }
            }
            CompareOp::In => {
                if let serde_json::Value::Array(arr) = right {
                    for item in arr {
                        if let serde_json::Value::String(s) = item {
                            if left == s {
                                return Ok(true);
                            }
                        }
                    }
                }
                Ok(false)
            }
            _ => {
                // Numeric comparisons
                let left_num: f64 = left.parse()?;
                let right_num = match right {
                    serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
                    _ => right.to_string().parse()?,
                };

                Ok(match op {
                    CompareOp::GreaterThan => left_num > right_num,
                    CompareOp::GreaterOrEqual => left_num >= right_num,
                    CompareOp::LessThan => left_num < right_num,
                    CompareOp::LessOrEqual => left_num <= right_num,
                    _ => false,
                })
            }
        }
    }

    /// Execute an action (boxed for recursion)
    fn execute_action<'a>(
        &'a self,
        action: &'a Action,
        ctx: &'a mut ExecutionContext,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Option<serde_json::Value>>> + Send + 'a>,
    > {
        Box::pin(async move {
            match action {
                Action::Export {
                    filter,
                    format,
                    output,
                } => {
                    // Would call actual export logic
                    log::info!(
                        "Exporting sessions: filter={:?}, format={}, output={}",
                        filter,
                        format,
                        output
                    );
                    Ok(Some(serde_json::json!({ "exported": true })))
                }
                Action::Archive {
                    filter,
                    destination,
                } => {
                    log::info!(
                        "Archiving sessions: filter={:?}, destination={}",
                        filter,
                        destination
                    );
                    Ok(Some(serde_json::json!({ "archived": true })))
                }
                Action::Delete { filter } => {
                    log::info!("Deleting sessions: filter={:?}", filter);
                    Ok(Some(serde_json::json!({ "deleted": true })))
                }
                Action::Tag {
                    filter,
                    add_tags,
                    remove_tags,
                } => {
                    log::info!(
                        "Tagging sessions: filter={:?}, add={:?}, remove={:?}",
                        filter,
                        add_tags,
                        remove_tags
                    );
                    Ok(Some(serde_json::json!({ "tagged": true })))
                }
                Action::Sync {
                    provider,
                    direction,
                } => {
                    log::info!(
                        "Syncing with provider {}: direction={:?}",
                        provider,
                        direction
                    );
                    Ok(Some(serde_json::json!({ "synced": true })))
                }
                Action::Harvest { provider } => {
                    log::info!("Harvesting from provider: {:?}", provider);
                    Ok(Some(serde_json::json!({ "harvested": true })))
                }
                Action::Plugin {
                    plugin_id,
                    action,
                    params,
                } => {
                    log::info!(
                        "Executing plugin {}: action={}, params={:?}",
                        plugin_id,
                        action,
                        params
                    );
                    Ok(Some(serde_json::json!({ "plugin_executed": true })))
                }
                Action::Notify {
                    channel,
                    message,
                    title,
                } => {
                    let msg = ctx.interpolate(message);
                    log::info!(
                        "Sending notification: channel={:?}, title={:?}, message={}",
                        channel,
                        title,
                        msg
                    );
                    Ok(Some(serde_json::json!({ "notified": true })))
                }
                Action::Http {
                    url,
                    method,
                    headers: _,
                    body: _,
                } => {
                    let url = ctx.interpolate(url);
                    log::info!("HTTP request: {} {}", method, url);
                    // Would make actual HTTP request
                    Ok(Some(serde_json::json!({ "status": 200 })))
                }
                Action::Shell { command, cwd, env: _ } => {
                    let cmd = ctx.interpolate(command);
                    log::info!("Executing shell: {} (cwd={:?})", cmd, cwd);
                    // Would execute actual command
                    Ok(Some(serde_json::json!({ "exit_code": 0 })))
                }
                Action::SetVariable { name, value } => {
                    ctx.set_var(name.clone(), value.clone());
                    Ok(None)
                }
                Action::If {
                    condition,
                    then,
                    else_,
                } => {
                    if self.evaluate_condition(condition, ctx).await? {
                        for action in then {
                            self.execute_action(action, ctx).await?;
                        }
                    } else if let Some(else_actions) = else_ {
                        for action in else_actions {
                            self.execute_action(action, ctx).await?;
                        }
                    }
                    Ok(None)
                }
                Action::ForEach {
                    items,
                    as_var,
                    actions,
                } => {
                    if let Some(arr) = ctx.get_var(items) {
                        if let serde_json::Value::Array(items_arr) = arr.clone() {
                            for item in items_arr {
                                ctx.set_var(as_var.clone(), item);
                                for action in actions {
                                    self.execute_action(action, ctx).await?;
                                }
                            }
                        }
                    }
                    Ok(None)
                }
                Action::Delay { seconds } => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(*seconds)).await;
                    Ok(None)
                }
                Action::Log { level, message } => {
                    let msg = ctx.interpolate(message);
                    match level {
                        LogLevel::Debug => log::debug!("{}", msg),
                        LogLevel::Info => log::info!("{}", msg),
                        LogLevel::Warning => log::warn!("{}", msg),
                        LogLevel::Error => log::error!("{}", msg),
                    }
                    Ok(None)
                }
            }
        })
    }

    async fn record_run(&self, run: WorkflowRun) {
        let mut runs = self.runs.write().await;
        runs.push(run);

        // Trim history
        if runs.len() > self.max_history {
            runs.remove(0);
        }
    }

    async fn update_run(
        &self,
        run_id: &str,
        status: RunStatus,
        error: Option<String>,
        results: Vec<ActionResult>,
    ) {
        let mut runs = self.runs.write().await;
        if let Some(run) = runs.iter_mut().find(|r| r.id == run_id) {
            run.status = status;
            run.ended_at = Some(Utc::now());
            run.error = error;
            run.results = results;
        }
    }

    async fn update_workflow_stats(&self, workflow_id: &str) {
        let mut workflows = self.workflows.write().await;
        if let Some(workflow) = workflows.get_mut(workflow_id) {
            workflow.last_run = Some(Utc::now());
            workflow.run_count += 1;
        }
    }

    /// Get run history
    pub async fn get_runs(&self, workflow_id: Option<&str>, limit: usize) -> Vec<WorkflowRun> {
        let runs = self.runs.read().await;
        runs.iter()
            .filter(|r| workflow_id.map(|id| r.workflow_id == id).unwrap_or(true))
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get specific run
    pub async fn get_run(&self, run_id: &str) -> Option<WorkflowRun> {
        self.runs
            .read()
            .await
            .iter()
            .find(|r| r.id == run_id)
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_automation_engine() {
        let engine = AutomationEngine::new(100);

        let workflow = Workflow {
            id: "test-workflow".to_string(),
            name: "Test Workflow".to_string(),
            description: None,
            enabled: true,
            triggers: vec![Trigger::Manual],
            conditions: vec![],
            actions: vec![Action::Log {
                level: LogLevel::Info,
                message: "Test action executed".to_string(),
            }],
            on_error: ErrorStrategy::Stop,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_run: None,
            run_count: 0,
        };

        engine.register(workflow).await.unwrap();
        let run_id = engine.trigger("test-workflow", None).await.unwrap();

        let run = engine.get_run(&run_id).await.unwrap();
        assert_eq!(run.status, RunStatus::Completed);
    }

    #[test]
    fn test_interpolation() {
        let mut ctx = ExecutionContext::new("test".to_string(), None);
        ctx.set_var("name".to_string(), serde_json::json!("World"));

        let result = ctx.interpolate("Hello, {{name}}!");
        assert_eq!(result, "Hello, World!");
    }
}
