// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Data Retention Module
//!
//! Provides configurable data retention policies for enterprise compliance.
//! Supports automatic cleanup, archival, and data lifecycle management.

use actix_web::{web, HttpResponse};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time;
use uuid::Uuid;

use super::audit::{AuditAction, AuditCategory, AuditEventBuilder, AuditService};
use super::audit::Database;

// =============================================================================
// Retention Policy Configuration
// =============================================================================

/// Data retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Unique policy identifier
    pub id: String,
    /// Policy name
    pub name: String,
    /// Policy description
    pub description: Option<String>,
    /// Whether this policy is active
    pub enabled: bool,
    /// Organization this policy applies to (None = global)
    pub organization_id: Option<String>,
    /// Resource types this policy applies to
    pub resource_types: Vec<ResourceType>,
    /// Retention rules
    pub rules: Vec<RetentionRule>,
    /// Schedule for policy execution
    pub schedule: RetentionSchedule,
    /// Actions to take when data expires
    pub expiry_actions: Vec<ExpiryAction>,
    /// Created timestamp
    pub created_at: i64,
    /// Updated timestamp
    pub updated_at: i64,
    /// Last execution timestamp
    pub last_run_at: Option<i64>,
    /// Next scheduled execution
    pub next_run_at: Option<i64>,
}

/// Types of resources that can have retention policies
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    /// Chat sessions
    Session,
    /// Workspaces
    Workspace,
    /// Audit logs
    AuditLog,
    /// User accounts
    User,
    /// Exported files
    Export,
    /// Temporary files
    TempFile,
    /// Backups
    Backup,
    /// API logs
    ApiLog,
    /// Analytics data
    Analytics,
}

impl ResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::Workspace => "workspace",
            Self::AuditLog => "audit_log",
            Self::User => "user",
            Self::Export => "export",
            Self::TempFile => "temp_file",
            Self::Backup => "backup",
            Self::ApiLog => "api_log",
            Self::Analytics => "analytics",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "session" => Some(Self::Session),
            "workspace" => Some(Self::Workspace),
            "audit_log" => Some(Self::AuditLog),
            "user" => Some(Self::User),
            "export" => Some(Self::Export),
            "temp_file" => Some(Self::TempFile),
            "backup" => Some(Self::Backup),
            "api_log" => Some(Self::ApiLog),
            "analytics" => Some(Self::Analytics),
            _ => None,
        }
    }
}

/// Retention rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionRule {
    /// Rule name
    pub name: String,
    /// Condition for this rule
    pub condition: RetentionCondition,
    /// Retention period
    pub retention_period: RetentionPeriod,
    /// Priority (higher = evaluated first)
    pub priority: i32,
}

/// Conditions that determine when a rule applies
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RetentionCondition {
    /// Always applies
    Always,
    /// Based on data age
    Age {
        /// Field to check age against
        field: AgeField,
    },
    /// Based on tags
    HasTag { tag: String },
    /// Based on status
    Status { status: String },
    /// Based on metadata field
    MetadataMatch { field: String, value: String },
    /// Combined conditions (AND)
    And { conditions: Vec<RetentionCondition> },
    /// Combined conditions (OR)
    Or { conditions: Vec<RetentionCondition> },
    /// Negated condition
    Not { condition: Box<RetentionCondition> },
}

/// Fields that can be used for age-based conditions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgeField {
    CreatedAt,
    UpdatedAt,
    LastAccessedAt,
    ArchivedAt,
    DeletedAt,
}

/// Retention period configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RetentionPeriod {
    /// Keep forever (no expiry)
    Forever,
    /// Keep for specified duration
    Duration { days: u32 },
    /// Keep until specific date
    Until { date: DateTime<Utc> },
    /// Keep for compliance period then archive
    ComplianceArchive {
        /// Active retention period
        active_days: u32,
        /// Archive retention period
        archive_days: u32,
    },
}

impl RetentionPeriod {
    pub fn get_expiry_date(&self, reference_date: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Self::Forever => None,
            Self::Duration { days } => Some(reference_date + Duration::days(*days as i64)),
            Self::Until { date } => Some(*date),
            Self::ComplianceArchive { active_days, .. } => {
                Some(reference_date + Duration::days(*active_days as i64))
            }
        }
    }
}

/// Schedule for retention policy execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RetentionSchedule {
    /// Run on demand only
    Manual,
    /// Run at specific interval
    Interval { hours: u32 },
    /// Run daily at specific time (UTC)
    Daily { hour: u8, minute: u8 },
    /// Run weekly on specific day
    Weekly {
        day: u8, // 0 = Sunday, 6 = Saturday
        hour: u8,
        minute: u8,
    },
    /// Run monthly on specific day
    Monthly {
        day: u8, // 1-28
        hour: u8,
        minute: u8,
    },
}

/// Actions to take when data expires
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExpiryAction {
    /// Permanently delete
    Delete,
    /// Move to archive storage
    Archive,
    /// Mark as deleted (soft delete)
    SoftDelete,
    /// Anonymize data
    Anonymize,
    /// Export before deletion
    ExportBeforeDelete,
    /// Notify owner before expiry
    NotifyOwner,
}

// =============================================================================
// Execution Results
// =============================================================================

/// Result of a retention policy execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionExecutionResult {
    /// Execution ID
    pub execution_id: String,
    /// Policy that was executed
    pub policy_id: String,
    /// Start timestamp
    pub started_at: DateTime<Utc>,
    /// End timestamp
    pub completed_at: DateTime<Utc>,
    /// Whether execution succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Statistics per resource type
    pub stats: HashMap<ResourceType, RetentionStats>,
    /// Total items processed
    pub total_processed: usize,
    /// Total items affected (deleted/archived/etc.)
    pub total_affected: usize,
    /// Items that failed to process
    pub total_failed: usize,
    /// Detailed actions taken
    pub actions_taken: Vec<RetentionActionLog>,
}

/// Statistics for a single resource type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetentionStats {
    pub scanned: usize,
    pub expired: usize,
    pub deleted: usize,
    pub archived: usize,
    pub anonymized: usize,
    pub soft_deleted: usize,
    pub exported: usize,
    pub notified: usize,
    pub failed: usize,
    pub skipped: usize,
    pub bytes_freed: u64,
}

/// Log entry for a specific retention action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionActionLog {
    pub resource_type: ResourceType,
    pub resource_id: String,
    pub action: ExpiryAction,
    pub success: bool,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

// =============================================================================
// Retention Service
// =============================================================================

/// Data retention management service
pub struct RetentionService {
    db: Database,
    audit_service: Option<web::Data<AuditService>>,
}

impl RetentionService {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            audit_service: None,
        }
    }

    pub fn with_audit(mut self, audit_service: web::Data<AuditService>) -> Self {
        self.audit_service = Some(audit_service);
        self
    }

    /// Get all retention policies
    pub async fn list_policies(
        &self,
        organization_id: Option<&str>,
    ) -> Result<Vec<RetentionPolicy>, String> {
        self.db
            .list_retention_policies(organization_id)
            .map_err(|e| format!("Database error: {}", e))
    }

    /// Get a specific policy
    pub async fn get_policy(&self, policy_id: &str) -> Result<Option<RetentionPolicy>, String> {
        self.db
            .get_retention_policy(policy_id)
            .map_err(|e| format!("Database error: {}", e))
    }

    /// Create a new retention policy
    pub async fn create_policy(
        &self,
        request: CreatePolicyRequest,
    ) -> Result<RetentionPolicy, String> {
        let now = Utc::now().timestamp();

        let policy = RetentionPolicy {
            id: Uuid::new_v4().to_string(),
            name: request.name,
            description: request.description,
            enabled: request.enabled.unwrap_or(true),
            organization_id: request.organization_id,
            resource_types: request.resource_types,
            rules: request.rules,
            schedule: request.schedule,
            expiry_actions: request.expiry_actions,
            created_at: now,
            updated_at: now,
            last_run_at: None,
            next_run_at: self.calculate_next_run(&request.schedule, None),
        };

        self.db
            .create_retention_policy(&policy)
            .map_err(|e| format!("Failed to create policy: {}", e))?;

        // Audit log
        if let Some(audit) = &self.audit_service {
            audit
                .log_builder(
                    AuditEventBuilder::new()
                        .category(AuditCategory::Configuration)
                        .action(AuditAction::Created)
                        .resource("retention_policy", &policy.id)
                        .description(format!("Created retention policy: {}", policy.name))
                        .success(),
                )
                .await;
        }

        Ok(policy)
    }

    /// Update a retention policy
    pub async fn update_policy(
        &self,
        policy_id: &str,
        request: UpdatePolicyRequest,
    ) -> Result<RetentionPolicy, String> {
        let mut policy = self
            .get_policy(policy_id)
            .await?
            .ok_or("Policy not found")?;

        if let Some(name) = request.name {
            policy.name = name;
        }
        if let Some(description) = request.description {
            policy.description = Some(description);
        }
        if let Some(enabled) = request.enabled {
            policy.enabled = enabled;
        }
        if let Some(resource_types) = request.resource_types {
            policy.resource_types = resource_types;
        }
        if let Some(rules) = request.rules {
            policy.rules = rules;
        }
        if let Some(schedule) = request.schedule {
            policy.schedule = schedule.clone();
            policy.next_run_at = self.calculate_next_run(&schedule, policy.last_run_at);
        }
        if let Some(expiry_actions) = request.expiry_actions {
            policy.expiry_actions = expiry_actions;
        }

        policy.updated_at = Utc::now().timestamp();

        self.db
            .update_retention_policy(&policy)
            .map_err(|e| format!("Failed to update policy: {}", e))?;

        // Audit log
        if let Some(audit) = &self.audit_service {
            audit
                .log_builder(
                    AuditEventBuilder::new()
                        .category(AuditCategory::Configuration)
                        .action(AuditAction::Updated)
                        .resource("retention_policy", &policy.id)
                        .description(format!("Updated retention policy: {}", policy.name))
                        .success(),
                )
                .await;
        }

        Ok(policy)
    }

    /// Delete a retention policy
    pub async fn delete_policy(&self, policy_id: &str) -> Result<(), String> {
        let policy = self
            .get_policy(policy_id)
            .await?
            .ok_or("Policy not found")?;

        self.db
            .delete_retention_policy(policy_id)
            .map_err(|e| format!("Failed to delete policy: {}", e))?;

        // Audit log
        if let Some(audit) = &self.audit_service {
            audit
                .log_builder(
                    AuditEventBuilder::new()
                        .category(AuditCategory::Configuration)
                        .action(AuditAction::Deleted)
                        .resource("retention_policy", policy_id)
                        .description(format!("Deleted retention policy: {}", policy.name))
                        .success(),
                )
                .await;
        }

        Ok(())
    }

    /// Execute a retention policy immediately
    pub async fn execute_policy(
        &self,
        policy_id: &str,
    ) -> Result<RetentionExecutionResult, String> {
        let policy = self
            .get_policy(policy_id)
            .await?
            .ok_or("Policy not found")?;

        if !policy.enabled {
            return Err("Policy is disabled".to_string());
        }

        let execution_id = Uuid::new_v4().to_string();
        let started_at = Utc::now();
        let mut stats = HashMap::new();
        let mut actions_taken = Vec::new();
        let mut total_failed = 0;

        // Process each resource type
        for resource_type in &policy.resource_types {
            let type_stats = self
                .process_resource_type(&policy, *resource_type, &mut actions_taken)
                .await?;
            total_failed += type_stats.failed;
            stats.insert(*resource_type, type_stats);
        }

        let completed_at = Utc::now();
        let total_processed: usize = stats.values().map(|s| s.scanned).sum();
        let total_affected: usize = stats
            .values()
            .map(|s| s.deleted + s.archived + s.anonymized + s.soft_deleted)
            .sum();

        // Update policy execution time
        self.db
            .update_retention_policy_execution(
                policy_id,
                completed_at.timestamp(),
                self.calculate_next_run(&policy.schedule, Some(completed_at.timestamp())),
            )
            .map_err(|e| format!("Failed to update execution time: {}", e))?;

        // Audit log
        if let Some(audit) = &self.audit_service {
            audit
                .log_builder(
                    AuditEventBuilder::new()
                        .category(AuditCategory::Administration)
                        .action(AuditAction::RetentionPolicyApplied)
                        .resource("retention_policy", policy_id)
                        .description(format!(
                            "Executed retention policy '{}': {} items processed, {} affected",
                            policy.name, total_processed, total_affected
                        ))
                        .detail("total_processed", total_processed)
                        .detail("total_affected", total_affected)
                        .detail("total_failed", total_failed)
                        .success(),
                )
                .await;
        }

        Ok(RetentionExecutionResult {
            execution_id,
            policy_id: policy_id.to_string(),
            started_at,
            completed_at,
            success: total_failed == 0,
            error: None,
            stats,
            total_processed,
            total_affected,
            total_failed,
            actions_taken,
        })
    }

    /// Process expired data for a specific resource type
    async fn process_resource_type(
        &self,
        policy: &RetentionPolicy,
        resource_type: ResourceType,
        actions_taken: &mut Vec<RetentionActionLog>,
    ) -> Result<RetentionStats, String> {
        let mut stats = RetentionStats::default();

        // Get expired items based on rules
        let expired_items = self.get_expired_items(&policy, resource_type).await?;
        stats.scanned = expired_items.len();
        stats.expired = expired_items.len();

        // Process each expired item
        for item in expired_items {
            let result = self
                .process_expired_item(&policy, resource_type, &item, &mut stats)
                .await;

            let action_log = RetentionActionLog {
                resource_type,
                resource_id: item.id.clone(),
                action: policy
                    .expiry_actions
                    .first()
                    .copied()
                    .unwrap_or(ExpiryAction::SoftDelete),
                success: result.is_ok(),
                error: result.err(),
                timestamp: Utc::now(),
            };
            actions_taken.push(action_log);
        }

        Ok(stats)
    }

    /// Get expired items for a resource type
    async fn get_expired_items(
        &self,
        policy: &RetentionPolicy,
        resource_type: ResourceType,
    ) -> Result<Vec<ExpiredItem>, String> {
        // Find matching rule with highest priority
        let rule = policy
            .rules
            .iter()
            .filter(|r| self.condition_applies(resource_type, &r.condition))
            .max_by_key(|r| r.priority);

        let rule = match rule {
            Some(r) => r,
            None => return Ok(vec![]), // No applicable rule
        };

        // Calculate expiry date
        let now = Utc::now();
        let expiry_threshold = match &rule.retention_period {
            RetentionPeriod::Forever => return Ok(vec![]),
            RetentionPeriod::Duration { days } => now - Duration::days(*days as i64),
            RetentionPeriod::Until { date } => *date,
            RetentionPeriod::ComplianceArchive { active_days, .. } => {
                now - Duration::days(*active_days as i64)
            }
        };

        // Query database for expired items
        self.db
            .get_expired_items(resource_type, expiry_threshold)
            .map_err(|e| format!("Database error: {}", e))
    }

    fn condition_applies(
        &self,
        _resource_type: ResourceType,
        condition: &RetentionCondition,
    ) -> bool {
        match condition {
            RetentionCondition::Always => true,
            RetentionCondition::Age { .. } => true,
            RetentionCondition::And { conditions } => conditions
                .iter()
                .all(|c| self.condition_applies(_resource_type, c)),
            RetentionCondition::Or { conditions } => conditions
                .iter()
                .any(|c| self.condition_applies(_resource_type, c)),
            RetentionCondition::Not { condition } => {
                !self.condition_applies(_resource_type, condition)
            }
            _ => true, // Other conditions evaluated at item level
        }
    }

    /// Process a single expired item
    async fn process_expired_item(
        &self,
        policy: &RetentionPolicy,
        resource_type: ResourceType,
        item: &ExpiredItem,
        stats: &mut RetentionStats,
    ) -> Result<(), String> {
        for action in &policy.expiry_actions {
            match action {
                ExpiryAction::Delete => {
                    self.db
                        .delete_item(resource_type, &item.id)
                        .map_err(|e| format!("Delete failed: {}", e))?;
                    stats.deleted += 1;
                    stats.bytes_freed += item.size_bytes.unwrap_or(0);
                }
                ExpiryAction::Archive => {
                    self.db
                        .archive_item(resource_type, &item.id)
                        .map_err(|e| format!("Archive failed: {}", e))?;
                    stats.archived += 1;
                }
                ExpiryAction::SoftDelete => {
                    self.db
                        .soft_delete_item(resource_type, &item.id)
                        .map_err(|e| format!("Soft delete failed: {}", e))?;
                    stats.soft_deleted += 1;
                }
                ExpiryAction::Anonymize => {
                    self.db
                        .anonymize_item(resource_type, &item.id)
                        .map_err(|e| format!("Anonymize failed: {}", e))?;
                    stats.anonymized += 1;
                }
                ExpiryAction::ExportBeforeDelete => {
                    self.db
                        .export_item(resource_type, &item.id)
                        .map_err(|e| format!("Export failed: {}", e))?;
                    stats.exported += 1;
                    // Then delete
                    self.db
                        .delete_item(resource_type, &item.id)
                        .map_err(|e| format!("Delete after export failed: {}", e))?;
                    stats.deleted += 1;
                }
                ExpiryAction::NotifyOwner => {
                    // Send notification (would integrate with notification system)
                    stats.notified += 1;
                }
            }
        }

        Ok(())
    }

    /// Calculate next run time based on schedule
    fn calculate_next_run(
        &self,
        schedule: &RetentionSchedule,
        last_run: Option<i64>,
    ) -> Option<i64> {
        let now = Utc::now();
        let last = last_run.map(|ts| DateTime::from_timestamp(ts, 0).unwrap_or(now));

        match schedule {
            RetentionSchedule::Manual => None,
            RetentionSchedule::Interval { hours } => {
                let next = last.unwrap_or(now) + Duration::hours(*hours as i64);
                Some(next.timestamp())
            }
            RetentionSchedule::Daily { hour, minute } => {
                let mut next = now
                    .date_naive()
                    .and_hms_opt(*hour as u32, *minute as u32, 0)
                    .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
                    .unwrap_or(now);
                if next <= now {
                    next = next + Duration::days(1);
                }
                Some(next.timestamp())
            }
            RetentionSchedule::Weekly { day, hour, minute } => {
                let current_day = now.weekday().num_days_from_sunday() as u8;
                let days_until = ((*day as i64 - current_day as i64) + 7) % 7;
                let next = (now + Duration::days(days_until))
                    .date_naive()
                    .and_hms_opt(*hour as u32, *minute as u32, 0)
                    .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
                    .unwrap_or(now);
                Some(next.timestamp())
            }
            RetentionSchedule::Monthly { day, hour, minute } => {
                // Simplified: use same day next month
                let next = now
                    .date_naive()
                    .with_day(*day as u32)
                    .and_then(|d| d.and_hms_opt(*hour as u32, *minute as u32, 0))
                    .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
                    .unwrap_or(now);
                Some(next.timestamp())
            }
        }
    }

    /// Start background scheduler for automatic policy execution
    pub fn start_scheduler(self: std::sync::Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = time::interval(time::Duration::from_secs(60)); // Check every minute

            loop {
                interval.tick().await;

                if let Err(e) = self.run_due_policies().await {
                    eprintln!("Retention scheduler error: {}", e);
                }
            }
        });
    }

    /// Execute all policies that are due
    async fn run_due_policies(&self) -> Result<(), String> {
        let now = Utc::now().timestamp();
        let due_policies = self
            .db
            .get_due_retention_policies(now)
            .map_err(|e| format!("Database error: {}", e))?;

        for policy in due_policies {
            if let Err(e) = self.execute_policy(&policy.id).await {
                eprintln!("Failed to execute retention policy {}: {}", policy.id, e);
            }
        }

        Ok(())
    }
}

// =============================================================================
// Helper Types
// =============================================================================

/// Representation of an expired item
#[derive(Debug, Clone)]
pub struct ExpiredItem {
    pub id: String,
    pub created_at: i64,
    pub owner_id: Option<String>,
    pub size_bytes: Option<u64>,
}

// =============================================================================
// API Request/Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct CreatePolicyRequest {
    pub name: String,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub organization_id: Option<String>,
    pub resource_types: Vec<ResourceType>,
    pub rules: Vec<RetentionRule>,
    pub schedule: RetentionSchedule,
    pub expiry_actions: Vec<ExpiryAction>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePolicyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub resource_types: Option<Vec<ResourceType>>,
    pub rules: Option<Vec<RetentionRule>>,
    pub schedule: Option<RetentionSchedule>,
    pub expiry_actions: Option<Vec<ExpiryAction>>,
}

// =============================================================================
// HTTP Handlers
// =============================================================================

/// GET /api/retention/policies - List retention policies
pub async fn list_policies(
    retention_service: web::Data<RetentionService>,
    query: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    let org_id = query.get("organization_id").map(|s| s.as_str());
    match retention_service.list_policies(org_id).await {
        Ok(policies) => HttpResponse::Ok().json(policies),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/retention/policies/{policy_id} - Get policy
pub async fn get_policy(
    retention_service: web::Data<RetentionService>,
    path: web::Path<String>,
) -> HttpResponse {
    let policy_id = path.into_inner();
    match retention_service.get_policy(&policy_id).await {
        Ok(Some(policy)) => HttpResponse::Ok().json(policy),
        Ok(None) => {
            HttpResponse::NotFound().json(serde_json::json!({ "error": "Policy not found" }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// POST /api/retention/policies - Create policy
pub async fn create_policy(
    retention_service: web::Data<RetentionService>,
    request: web::Json<CreatePolicyRequest>,
) -> HttpResponse {
    match retention_service.create_policy(request.into_inner()).await {
        Ok(policy) => HttpResponse::Created().json(policy),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// PUT /api/retention/policies/{policy_id} - Update policy
pub async fn update_policy(
    retention_service: web::Data<RetentionService>,
    path: web::Path<String>,
    request: web::Json<UpdatePolicyRequest>,
) -> HttpResponse {
    let policy_id = path.into_inner();
    match retention_service
        .update_policy(&policy_id, request.into_inner())
        .await
    {
        Ok(policy) => HttpResponse::Ok().json(policy),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// DELETE /api/retention/policies/{policy_id} - Delete policy
pub async fn delete_policy(
    retention_service: web::Data<RetentionService>,
    path: web::Path<String>,
) -> HttpResponse {
    let policy_id = path.into_inner();
    match retention_service.delete_policy(&policy_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// POST /api/retention/policies/{policy_id}/execute - Execute policy immediately
pub async fn execute_policy(
    retention_service: web::Data<RetentionService>,
    path: web::Path<String>,
) -> HttpResponse {
    let policy_id = path.into_inner();
    match retention_service.execute_policy(&policy_id).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// Configure retention routes
pub fn configure_retention_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/retention")
            .route("/policies", web::get().to(list_policies))
            .route("/policies", web::post().to(create_policy))
            .route("/policies/{policy_id}", web::get().to(get_policy))
            .route("/policies/{policy_id}", web::put().to(update_policy))
            .route("/policies/{policy_id}", web::delete().to(delete_policy))
            .route(
                "/policies/{policy_id}/execute",
                web::post().to(execute_policy),
            ),
    );
}
