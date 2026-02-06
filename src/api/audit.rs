// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Audit Logging Module
//!
//! Provides comprehensive audit logging for enterprise compliance requirements.
//! Tracks all user actions, API calls, and system events with full context.

use actix_web::{dev::ServiceRequest, web, HttpMessage, HttpRequest, HttpResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// use crate::mcp::db::Database;
// TODO: Database abstraction for Q1 2027
pub type Database = std::sync::Arc<dyn DatabaseOps + Send + Sync>;

#[allow(dead_code)]
#[async_trait::async_trait]
pub trait DatabaseOps {
    async fn create(&self, table: &str, data: serde_json::Value) -> Result<serde_json::Value, String>;
    async fn get_by_id(&self, table: &str, id: &str) -> Result<Option<serde_json::Value>, String>;
    async fn query(&self, table: &str, filter: serde_json::Value) -> Result<Vec<serde_json::Value>, String>;
    async fn count(&self, table: &str, filter: serde_json::Value) -> Result<i64, String>;
    async fn update(&self, table: &str, id: &str, data: serde_json::Value) -> Result<(), String>;
    async fn delete(&self, table: &str, id: &str) -> Result<(), String>;
}

// =============================================================================
// Audit Event Types
// =============================================================================

/// Categories of auditable events
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    /// Authentication events (login, logout, token refresh)
    Authentication,
    /// Authorization events (permission checks, access denied)
    Authorization,
    /// User management (create, update, delete users)
    UserManagement,
    /// Session operations (harvest, view, export, delete)
    SessionManagement,
    /// Workspace operations
    WorkspaceManagement,
    /// Team/organization operations
    TeamManagement,
    /// Configuration changes
    Configuration,
    /// Data export/import operations
    DataTransfer,
    /// Administrative actions
    Administration,
    /// System events (startup, shutdown, errors)
    System,
    /// API access events
    ApiAccess,
    /// SSO/SAML events
    Sso,
}

impl AuditCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Authentication => "authentication",
            Self::Authorization => "authorization",
            Self::UserManagement => "user_management",
            Self::SessionManagement => "session_management",
            Self::WorkspaceManagement => "workspace_management",
            Self::TeamManagement => "team_management",
            Self::Configuration => "configuration",
            Self::DataTransfer => "data_transfer",
            Self::Administration => "administration",
            Self::System => "system",
            Self::ApiAccess => "api_access",
            Self::Sso => "sso",
        }
    }
}

/// Specific audit event actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    // Authentication
    Login,
    LoginFailed,
    Logout,
    TokenRefresh,
    PasswordChange,
    PasswordReset,
    MfaEnabled,
    MfaDisabled,

    // Authorization
    AccessGranted,
    AccessDenied,
    PermissionChanged,

    // User Management
    UserCreated,
    UserUpdated,
    UserDeleted,
    UserSuspended,
    UserActivated,
    UserInvited,
    InviteAccepted,
    SubscriptionChanged,

    // Session Management
    SessionHarvested,
    SessionViewed,
    SessionExported,
    SessionDeleted,
    SessionArchived,
    SessionShared,
    SessionUnshared,
    BulkExport,
    BulkDelete,

    // Workspace Management
    WorkspaceCreated,
    WorkspaceUpdated,
    WorkspaceDeleted,
    WorkspaceLinked,
    WorkspaceUnlinked,

    // Team Management
    TeamCreated,
    TeamUpdated,
    TeamDeleted,
    MemberAdded,
    MemberRemoved,
    RoleChanged,

    // Configuration
    SettingsUpdated,
    ProviderConfigured,
    ProviderDisabled,
    IdpConfigured,
    IdpUpdated,
    IdpDeleted,

    // Data Transfer
    DataExported,
    DataImported,
    BackupCreated,
    BackupRestored,

    // Administration
    AdminActionPerformed,
    SystemConfigChanged,
    RetentionPolicyApplied,
    DataPurged,

    // System
    ServiceStarted,
    ServiceStopped,
    ErrorOccurred,
    MaintenanceStarted,
    MaintenanceCompleted,

    // SSO
    SsoLoginInitiated,
    SsoLoginCompleted,
    SsoLoginFailed,
    SloInitiated,
    SloCompleted,

    // Generic
    Created,
    Updated,
    Deleted,
    Viewed,
    Listed,
    Searched,
}

impl AuditAction {
    pub fn as_str(&self) -> &str {
        // Convert enum variant to snake_case string
        let name = format!("{:?}", self);
        // This is a simplified conversion - production would use serde
        &name.to_lowercase()
    }
}

/// Outcome of an audited action
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    #[default]
    Success,
    Failure,
    Partial,
    Denied,
    Error,
}

// =============================================================================
// Audit Event Model
// =============================================================================

/// Complete audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event identifier
    pub id: String,
    /// Event timestamp (UTC)
    pub timestamp: DateTime<Utc>,
    /// Event category
    pub category: AuditCategory,
    /// Specific action performed
    pub action: AuditAction,
    /// Outcome of the action
    pub outcome: AuditOutcome,
    /// User who performed the action (if authenticated)
    pub actor: Option<AuditActor>,
    /// Resource that was affected
    pub resource: Option<AuditResource>,
    /// Request context
    pub request: Option<AuditRequest>,
    /// Additional event details
    pub details: HashMap<String, serde_json::Value>,
    /// Human-readable description
    pub description: String,
    /// Error message (if outcome is failure/error)
    pub error: Option<String>,
    /// Organization/tenant context
    pub organization_id: Option<String>,
    /// Correlation ID for tracing related events
    pub correlation_id: Option<String>,
    /// Tags for filtering
    pub tags: Vec<String>,
}

/// Information about the actor (user) who triggered the event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditActor {
    /// User ID
    pub user_id: String,
    /// User email
    pub email: String,
    /// Display name
    pub display_name: Option<String>,
    /// Subscription tier
    pub tier: Option<String>,
    /// Auth method used (password, sso, api_key, etc.)
    pub auth_method: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
}

/// Information about the affected resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResource {
    /// Resource type (session, workspace, user, etc.)
    pub resource_type: String,
    /// Resource ID
    pub resource_id: String,
    /// Resource name (if applicable)
    pub name: Option<String>,
    /// Parent resource (e.g., workspace for a session)
    pub parent: Option<Box<AuditResource>>,
}

/// HTTP request context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRequest {
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Query string (sanitized)
    pub query: Option<String>,
    /// Client IP address
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Request ID for tracing
    pub request_id: String,
    /// Response status code
    pub status_code: Option<u16>,
    /// Response time in milliseconds
    pub duration_ms: Option<u64>,
}

// =============================================================================
// Audit Event Builder
// =============================================================================

/// Builder for constructing audit events
#[derive(Default)]
pub struct AuditEventBuilder {
    category: Option<AuditCategory>,
    action: Option<AuditAction>,
    outcome: AuditOutcome,
    actor: Option<AuditActor>,
    resource: Option<AuditResource>,
    request: Option<AuditRequest>,
    details: HashMap<String, serde_json::Value>,
    description: Option<String>,
    error: Option<String>,
    organization_id: Option<String>,
    correlation_id: Option<String>,
    tags: Vec<String>,
}

impl AuditEventBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn category(mut self, category: AuditCategory) -> Self {
        self.category = Some(category);
        self
    }

    pub fn action(mut self, action: AuditAction) -> Self {
        self.action = Some(action);
        self
    }

    pub fn outcome(mut self, outcome: AuditOutcome) -> Self {
        self.outcome = outcome;
        self
    }

    pub fn success(mut self) -> Self {
        self.outcome = AuditOutcome::Success;
        self
    }

    pub fn failure(mut self, error: impl Into<String>) -> Self {
        self.outcome = AuditOutcome::Failure;
        self.error = Some(error.into());
        self
    }

    pub fn denied(mut self) -> Self {
        self.outcome = AuditOutcome::Denied;
        self
    }

    pub fn actor(mut self, actor: AuditActor) -> Self {
        self.actor = Some(actor);
        self
    }

    pub fn actor_from_user(mut self, user_id: &str, email: &str) -> Self {
        self.actor = Some(AuditActor {
            user_id: user_id.to_string(),
            email: email.to_string(),
            display_name: None,
            tier: None,
            auth_method: None,
            session_id: None,
        });
        self
    }

    pub fn resource(mut self, resource_type: &str, resource_id: &str) -> Self {
        self.resource = Some(AuditResource {
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            name: None,
            parent: None,
        });
        self
    }

    pub fn resource_with_name(
        mut self,
        resource_type: &str,
        resource_id: &str,
        name: &str,
    ) -> Self {
        self.resource = Some(AuditResource {
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            name: Some(name.to_string()),
            parent: None,
        });
        self
    }

    pub fn request(mut self, request: AuditRequest) -> Self {
        self.request = Some(request);
        self
    }

    pub fn request_from_http(mut self, req: &HttpRequest) -> Self {
        let connection_info = req.connection_info();
        self.request = Some(AuditRequest {
            method: req.method().to_string(),
            path: req.path().to_string(),
            query: req
                .query_string()
                .is_empty()
                .then(|| None)
                .unwrap_or(Some(req.query_string().to_string())),
            ip_address: connection_info.realip_remote_addr().map(|s| s.to_string()),
            user_agent: req
                .headers()
                .get("user-agent")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string()),
            request_id: Uuid::new_v4().to_string(),
            status_code: None,
            duration_ms: None,
        });
        self
    }

    pub fn detail<V: Serialize>(mut self, key: &str, value: V) -> Self {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.details.insert(key.to_string(), json_value);
        }
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn organization(mut self, organization_id: &str) -> Self {
        self.organization_id = Some(organization_id.to_string());
        self
    }

    pub fn correlation(mut self, correlation_id: &str) -> Self {
        self.correlation_id = Some(correlation_id.to_string());
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags.extend(tags);
        self
    }

    pub fn build(self) -> Result<AuditEvent, String> {
        let category = self.category.ok_or("Category is required")?;
        let action = self.action.ok_or("Action is required")?;

        let description = self
            .description
            .unwrap_or_else(|| format!("{:?} - {:?}", category, action));

        Ok(AuditEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            category,
            action,
            outcome: self.outcome,
            actor: self.actor,
            resource: self.resource,
            request: self.request,
            details: self.details,
            description,
            error: self.error,
            organization_id: self.organization_id,
            correlation_id: self.correlation_id,
            tags: self.tags,
        })
    }
}

// =============================================================================
// Audit Service
// =============================================================================

/// Audit logging service
pub struct AuditService {
    db: Database,
    /// In-memory buffer for batch writes
    buffer: Arc<RwLock<Vec<AuditEvent>>>,
    /// Buffer flush threshold
    buffer_size: usize,
    /// Whether audit logging is enabled
    enabled: bool,
    /// Categories to log (empty = all)
    categories_filter: Vec<AuditCategory>,
    /// Minimum severity to log
    log_failures_only: bool,
}

impl AuditService {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            buffer: Arc::new(RwLock::new(Vec::new())),
            buffer_size: 100,
            enabled: true,
            categories_filter: vec![],
            log_failures_only: false,
        }
    }

    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn categories(mut self, categories: Vec<AuditCategory>) -> Self {
        self.categories_filter = categories;
        self
    }

    pub fn failures_only(mut self) -> Self {
        self.log_failures_only = true;
        self
    }

    /// Log an audit event
    pub async fn log(&self, event: AuditEvent) {
        if !self.enabled {
            return;
        }

        // Apply category filter
        if !self.categories_filter.is_empty() && !self.categories_filter.contains(&event.category) {
            return;
        }

        // Apply outcome filter
        if self.log_failures_only && event.outcome == AuditOutcome::Success {
            return;
        }

        // Add to buffer
        let mut buffer = self.buffer.write().await;
        buffer.push(event);

        // Flush if buffer is full
        if buffer.len() >= self.buffer_size {
            let events: Vec<_> = buffer.drain(..).collect();
            drop(buffer); // Release lock before DB write

            if let Err(e) = self.flush_events(events).await {
                eprintln!("Failed to flush audit events: {}", e);
            }
        }
    }

    /// Log an event using the builder pattern
    pub async fn log_builder(&self, builder: AuditEventBuilder) {
        match builder.build() {
            Ok(event) => self.log(event).await,
            Err(e) => eprintln!("Failed to build audit event: {}", e),
        }
    }

    /// Flush buffered events to database
    pub async fn flush(&self) {
        let mut buffer = self.buffer.write().await;
        if buffer.is_empty() {
            return;
        }

        let events: Vec<_> = buffer.drain(..).collect();
        drop(buffer);

        if let Err(e) = self.flush_events(events).await {
            eprintln!("Failed to flush audit events: {}", e);
        }
    }

    async fn flush_events(&self, events: Vec<AuditEvent>) -> Result<(), String> {
        self.db
            .insert_audit_events(&events)
            .map_err(|e| format!("Database error: {}", e))
    }

    /// Query audit events
    pub async fn query(&self, query: AuditQuery) -> Result<AuditQueryResult, String> {
        self.db
            .query_audit_events(&query)
            .map_err(|e| format!("Database error: {}", e))
    }

    /// Get audit event by ID
    pub async fn get_event(&self, event_id: &str) -> Result<Option<AuditEvent>, String> {
        self.db
            .get_audit_event(event_id)
            .map_err(|e| format!("Database error: {}", e))
    }

    /// Get events for a specific resource
    pub async fn get_resource_history(
        &self,
        resource_type: &str,
        resource_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<AuditEvent>, String> {
        self.db
            .get_audit_events_for_resource(resource_type, resource_id, limit.unwrap_or(100))
            .map_err(|e| format!("Database error: {}", e))
    }

    /// Get events for a specific user
    pub async fn get_user_activity(
        &self,
        user_id: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<Vec<AuditEvent>, String> {
        self.db
            .get_audit_events_for_user(user_id, from, to, limit.unwrap_or(100))
            .map_err(|e| format!("Database error: {}", e))
    }

    /// Export audit events (for compliance)
    pub async fn export(&self, query: AuditQuery, format: ExportFormat) -> Result<Vec<u8>, String> {
        let result = self.query(query).await?;

        match format {
            ExportFormat::Json => serde_json::to_vec_pretty(&result.events)
                .map_err(|e| format!("JSON serialization error: {}", e)),
            ExportFormat::Csv => self.events_to_csv(&result.events),
            ExportFormat::JsonLines => {
                let mut output = Vec::new();
                for event in &result.events {
                    let line = serde_json::to_vec(event)
                        .map_err(|e| format!("JSON serialization error: {}", e))?;
                    output.extend(line);
                    output.push(b'\n');
                }
                Ok(output)
            }
        }
    }

    fn events_to_csv(&self, events: &[AuditEvent]) -> Result<Vec<u8>, String> {
        let mut output = String::new();

        // Header
        output.push_str("id,timestamp,category,action,outcome,actor_id,actor_email,resource_type,resource_id,description,error\n");

        // Rows
        for event in events {
            let actor_id = event
                .actor
                .as_ref()
                .map(|a| &a.user_id)
                .unwrap_or(&String::new());
            let actor_email = event
                .actor
                .as_ref()
                .map(|a| &a.email)
                .unwrap_or(&String::new());
            let resource_type = event
                .resource
                .as_ref()
                .map(|r| &r.resource_type)
                .unwrap_or(&String::new());
            let resource_id = event
                .resource
                .as_ref()
                .map(|r| &r.resource_id)
                .unwrap_or(&String::new());
            let error = event.error.as_ref().unwrap_or(&String::new());

            output.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{}\n",
                event.id,
                event.timestamp.to_rfc3339(),
                event.category.as_str(),
                format!("{:?}", event.action),
                format!("{:?}", event.outcome),
                csv_escape(actor_id),
                csv_escape(actor_email),
                resource_type,
                resource_id,
                csv_escape(&event.description),
                csv_escape(error),
            ));
        }

        Ok(output.into_bytes())
    }
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

// =============================================================================
// Query Types
// =============================================================================

/// Query parameters for audit log search
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuditQuery {
    /// Filter by category
    pub category: Option<AuditCategory>,
    /// Filter by action
    pub action: Option<AuditAction>,
    /// Filter by outcome
    pub outcome: Option<AuditOutcome>,
    /// Filter by actor user ID
    pub actor_id: Option<String>,
    /// Filter by actor email
    pub actor_email: Option<String>,
    /// Filter by resource type
    pub resource_type: Option<String>,
    /// Filter by resource ID
    pub resource_id: Option<String>,
    /// Filter by organization
    pub organization_id: Option<String>,
    /// Filter by correlation ID
    pub correlation_id: Option<String>,
    /// Search in description
    pub search: Option<String>,
    /// Start timestamp (inclusive)
    pub from: Option<DateTime<Utc>>,
    /// End timestamp (exclusive)
    pub to: Option<DateTime<Utc>>,
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    /// Pagination offset
    pub offset: Option<usize>,
    /// Page size (max 1000)
    pub limit: Option<usize>,
    /// Sort order (asc/desc)
    pub sort_order: Option<String>,
}

/// Result of an audit query
#[derive(Debug, Clone, Serialize)]
pub struct AuditQueryResult {
    pub events: Vec<AuditEvent>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}

/// Export format for audit logs
#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    #[default]
    Json,
    Csv,
    JsonLines,
}

// =============================================================================
// HTTP Handlers
// =============================================================================

/// GET /api/audit - Query audit logs
pub async fn query_audit_logs(
    audit_service: web::Data<AuditService>,
    query: web::Query<AuditQuery>,
) -> HttpResponse {
    match audit_service.query(query.into_inner()).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/audit/{event_id} - Get single audit event
pub async fn get_audit_event(
    audit_service: web::Data<AuditService>,
    path: web::Path<String>,
) -> HttpResponse {
    let event_id = path.into_inner();
    match audit_service.get_event(&event_id).await {
        Ok(Some(event)) => HttpResponse::Ok().json(event),
        Ok(None) => {
            HttpResponse::NotFound().json(serde_json::json!({ "error": "Event not found" }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/audit/resource/{type}/{id} - Get resource history
pub async fn get_resource_audit_history(
    audit_service: web::Data<AuditService>,
    path: web::Path<(String, String)>,
    query: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    let (resource_type, resource_id) = path.into_inner();
    let limit = query.get("limit").and_then(|s| s.parse().ok());

    match audit_service
        .get_resource_history(&resource_type, &resource_id, limit)
        .await
    {
        Ok(events) => HttpResponse::Ok().json(events),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/audit/user/{user_id} - Get user activity
pub async fn get_user_audit_history(
    audit_service: web::Data<AuditService>,
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    let user_id = path.into_inner();
    let from = query
        .get("from")
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));
    let to = query
        .get("to")
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));
    let limit = query.get("limit").and_then(|s| s.parse().ok());

    match audit_service
        .get_user_activity(&user_id, from, to, limit)
        .await
    {
        Ok(events) => HttpResponse::Ok().json(events),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    #[serde(flatten)]
    pub query: AuditQuery,
    pub format: Option<ExportFormat>,
}

/// POST /api/audit/export - Export audit logs
pub async fn export_audit_logs(
    audit_service: web::Data<AuditService>,
    request: web::Json<ExportQuery>,
) -> HttpResponse {
    let format = request.format.unwrap_or_default();
    let content_type = match format {
        ExportFormat::Json => "application/json",
        ExportFormat::Csv => "text/csv",
        ExportFormat::JsonLines => "application/x-ndjson",
    };

    match audit_service
        .export(request.into_inner().query, format)
        .await
    {
        Ok(data) => HttpResponse::Ok()
            .content_type(content_type)
            .append_header((
                "Content-Disposition",
                "attachment; filename=\"audit-log.export\"",
            ))
            .body(data),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// Configure audit routes
pub fn configure_audit_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/audit")
            .route("", web::get().to(query_audit_logs))
            .route("/{event_id}", web::get().to(get_audit_event))
            .route(
                "/resource/{type}/{id}",
                web::get().to(get_resource_audit_history),
            )
            .route("/user/{user_id}", web::get().to(get_user_audit_history))
            .route("/export", web::post().to(export_audit_logs)),
    );
}

// =============================================================================
// Convenience Macros
// =============================================================================

/// Macro for quick audit logging
#[macro_export]
macro_rules! audit {
    ($service:expr, $category:expr, $action:expr, $description:expr) => {
        $service.log_builder(
            $crate::api::audit::AuditEventBuilder::new()
                .category($category)
                .action($action)
                .description($description)
                .success()
        ).await
    };
    ($service:expr, $category:expr, $action:expr, $description:expr, $($key:expr => $value:expr),*) => {
        $service.log_builder(
            $crate::api::audit::AuditEventBuilder::new()
                .category($category)
                .action($action)
                .description($description)
                .success()
                $(.detail($key, $value))*
        ).await
    };
}
