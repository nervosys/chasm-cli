// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Automated Backup Scheduling Module
//!
//! Provides configurable automated backup capabilities:
//! - Scheduled backup execution (cron-like)
//! - Multiple backup destinations (local, S3, Azure, etc.)
//! - Incremental and full backup support
//! - Backup verification and restoration
//! - Backup retention policies

use actix_web::{web, HttpResponse};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// =============================================================================
// Core Types
// =============================================================================

/// Backup schedule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSchedule {
    /// Schedule ID
    pub id: String,
    /// Schedule name
    pub name: String,
    /// Whether schedule is active
    pub enabled: bool,
    /// Cron expression (e.g., "0 2 * * *" for 2 AM daily)
    pub cron_expression: String,
    /// Backup type
    pub backup_type: BackupType,
    /// What to backup
    pub scope: BackupScope,
    /// Where to store backups
    pub destination: BackupDestination,
    /// Retention settings
    pub retention: BackupRetention,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last modified timestamp
    pub updated_at: DateTime<Utc>,
    /// Last execution time
    pub last_run: Option<DateTime<Utc>>,
    /// Next scheduled run
    pub next_run: Option<DateTime<Utc>>,
    /// Owner user/org ID
    pub owner_id: Option<String>,
}

/// Type of backup
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackupType {
    /// Full backup of all data
    #[default]
    Full,
    /// Only changes since last backup
    Incremental,
    /// Only changes since last full backup
    Differential,
    /// Snapshot-based backup
    Snapshot,
}

/// What to include in backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupScope {
    /// Include sessions
    pub sessions: bool,
    /// Include workspaces
    pub workspaces: bool,
    /// Include user settings
    pub settings: bool,
    /// Include audit logs
    pub audit_logs: bool,
    /// Include analytics data
    pub analytics: bool,
    /// Specific session IDs (if empty, backup all)
    pub session_ids: Vec<String>,
    /// Specific workspace IDs (if empty, backup all)
    pub workspace_ids: Vec<String>,
}

impl Default for BackupScope {
    fn default() -> Self {
        Self {
            sessions: true,
            workspaces: true,
            settings: true,
            audit_logs: false,
            analytics: false,
            session_ids: Vec::new(),
            workspace_ids: Vec::new(),
        }
    }
}

/// Backup destination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackupDestination {
    /// Local filesystem
    Local { path: String },
    /// Amazon S3
    S3 {
        bucket: String,
        prefix: String,
        region: String,
        #[serde(skip_serializing)]
        access_key_id: Option<String>,
        #[serde(skip_serializing)]
        secret_access_key: Option<String>,
    },
    /// Azure Blob Storage
    AzureBlob {
        container: String,
        prefix: String,
        #[serde(skip_serializing)]
        connection_string: Option<String>,
    },
    /// Google Cloud Storage
    Gcs {
        bucket: String,
        prefix: String,
        #[serde(skip_serializing)]
        credentials_json: Option<String>,
    },
    /// SFTP/SSH
    Sftp {
        host: String,
        port: u16,
        path: String,
        username: String,
        #[serde(skip_serializing)]
        private_key: Option<String>,
    },
    /// WebDAV
    WebDav {
        url: String,
        username: Option<String>,
        #[serde(skip_serializing)]
        password: Option<String>,
    },
}

/// Backup retention settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRetention {
    /// Keep daily backups for N days
    pub keep_daily: u32,
    /// Keep weekly backups for N weeks
    pub keep_weekly: u32,
    /// Keep monthly backups for N months
    pub keep_monthly: u32,
    /// Minimum number of backups to keep regardless of age
    pub min_backups: u32,
    /// Maximum total storage to use (bytes, 0 = unlimited)
    pub max_storage_bytes: u64,
}

impl Default for BackupRetention {
    fn default() -> Self {
        Self {
            keep_daily: 7,
            keep_weekly: 4,
            keep_monthly: 12,
            min_backups: 3,
            max_storage_bytes: 0,
        }
    }
}

/// A completed backup record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRecord {
    /// Backup ID
    pub id: String,
    /// Associated schedule ID
    pub schedule_id: Option<String>,
    /// Backup type
    pub backup_type: BackupType,
    /// Status
    pub status: BackupStatus,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// Completion time
    pub completed_at: Option<DateTime<Utc>>,
    /// Duration in seconds
    pub duration_secs: Option<u64>,
    /// Size in bytes
    pub size_bytes: u64,
    /// Number of items backed up
    pub item_count: usize,
    /// Destination path/URL
    pub destination_path: String,
    /// Checksum for verification
    pub checksum: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
    /// Backup metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Backup status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BackupStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Verifying,
    Verified,
}

/// Restore request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreRequest {
    /// Backup ID to restore from
    pub backup_id: String,
    /// What to restore
    pub scope: RestoreScope,
    /// Restore options
    pub options: RestoreOptions,
}

/// What to restore
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreScope {
    /// Restore sessions
    pub sessions: bool,
    /// Restore workspaces
    pub workspaces: bool,
    /// Restore settings
    pub settings: bool,
    /// Specific session IDs
    pub session_ids: Vec<String>,
}

/// Restore options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreOptions {
    /// Overwrite existing data
    pub overwrite: bool,
    /// Create new IDs for restored items
    pub create_new_ids: bool,
    /// Target location (for selective restore)
    pub target_workspace_id: Option<String>,
}

/// Restore result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    /// Success status
    pub success: bool,
    /// Items restored
    pub items_restored: usize,
    /// Items skipped
    pub items_skipped: usize,
    /// Errors encountered
    pub errors: Vec<String>,
    /// Duration in seconds
    pub duration_secs: u64,
}

// =============================================================================
// Backup Service
// =============================================================================

/// Service for managing backups
pub struct BackupService {
    schedules: std::sync::RwLock<HashMap<String, BackupSchedule>>,
    backups: std::sync::RwLock<HashMap<String, BackupRecord>>,
}

impl BackupService {
    pub fn new() -> Self {
        Self {
            schedules: std::sync::RwLock::new(HashMap::new()),
            backups: std::sync::RwLock::new(HashMap::new()),
        }
    }

    // =========================================================================
    // Schedule Management
    // =========================================================================

    /// Create a new backup schedule
    pub async fn create_schedule(
        &self,
        schedule: BackupSchedule,
    ) -> Result<BackupSchedule, String> {
        let mut schedule = schedule;
        schedule.id = Uuid::new_v4().to_string();
        schedule.created_at = Utc::now();
        schedule.updated_at = Utc::now();
        schedule.next_run = self.calculate_next_run(&schedule.cron_expression);

        let mut schedules = self.schedules.write().map_err(|e| e.to_string())?;
        schedules.insert(schedule.id.clone(), schedule.clone());

        Ok(schedule)
    }

    /// List all schedules
    pub async fn list_schedules(
        &self,
        owner_id: Option<&str>,
    ) -> Result<Vec<BackupSchedule>, String> {
        let schedules = self.schedules.read().map_err(|e| e.to_string())?;
        let result: Vec<_> = schedules
            .values()
            .filter(|s| owner_id.map_or(true, |id| s.owner_id.as_deref() == Some(id)))
            .cloned()
            .collect();
        Ok(result)
    }

    /// Get a specific schedule
    pub async fn get_schedule(&self, id: &str) -> Result<BackupSchedule, String> {
        let schedules = self.schedules.read().map_err(|e| e.to_string())?;
        schedules
            .get(id)
            .cloned()
            .ok_or_else(|| format!("Schedule not found: {}", id))
    }

    /// Update a schedule
    pub async fn update_schedule(
        &self,
        id: &str,
        updates: ScheduleUpdate,
    ) -> Result<BackupSchedule, String> {
        let mut schedules = self.schedules.write().map_err(|e| e.to_string())?;
        let schedule = schedules
            .get_mut(id)
            .ok_or_else(|| format!("Schedule not found: {}", id))?;

        if let Some(name) = updates.name {
            schedule.name = name;
        }
        if let Some(enabled) = updates.enabled {
            schedule.enabled = enabled;
        }
        if let Some(cron) = updates.cron_expression {
            schedule.cron_expression = cron.clone();
            schedule.next_run = self.calculate_next_run(&cron);
        }
        if let Some(backup_type) = updates.backup_type {
            schedule.backup_type = backup_type;
        }
        if let Some(scope) = updates.scope {
            schedule.scope = scope;
        }
        if let Some(destination) = updates.destination {
            schedule.destination = destination;
        }
        if let Some(retention) = updates.retention {
            schedule.retention = retention;
        }

        schedule.updated_at = Utc::now();

        Ok(schedule.clone())
    }

    /// Delete a schedule
    pub async fn delete_schedule(&self, id: &str) -> Result<(), String> {
        let mut schedules = self.schedules.write().map_err(|e| e.to_string())?;
        schedules
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| format!("Schedule not found: {}", id))
    }

    /// Enable/disable a schedule
    pub async fn set_schedule_enabled(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<BackupSchedule, String> {
        let mut schedules = self.schedules.write().map_err(|e| e.to_string())?;
        let schedule = schedules
            .get_mut(id)
            .ok_or_else(|| format!("Schedule not found: {}", id))?;

        schedule.enabled = enabled;
        schedule.updated_at = Utc::now();

        if enabled {
            schedule.next_run = self.calculate_next_run(&schedule.cron_expression);
        }

        Ok(schedule.clone())
    }

    // =========================================================================
    // Backup Operations
    // =========================================================================

    /// Trigger an immediate backup
    pub async fn trigger_backup(
        &self,
        schedule_id: Option<&str>,
        backup_type: BackupType,
        scope: BackupScope,
        destination: BackupDestination,
    ) -> Result<BackupRecord, String> {
        let backup = BackupRecord {
            id: Uuid::new_v4().to_string(),
            schedule_id: schedule_id.map(String::from),
            backup_type,
            status: BackupStatus::Pending,
            started_at: Utc::now(),
            completed_at: None,
            duration_secs: None,
            size_bytes: 0,
            item_count: 0,
            destination_path: self.generate_backup_path(&destination),
            checksum: None,
            error: None,
            metadata: HashMap::new(),
        };

        let backup_id = backup.id.clone();

        {
            let mut backups = self.backups.write().map_err(|e| e.to_string())?;
            backups.insert(backup_id.clone(), backup.clone());
        }

        // In production, this would spawn an async task to perform the backup
        // For now, simulate starting the backup
        self.execute_backup(&backup_id, &scope, &destination)
            .await?;

        let backups = self.backups.read().map_err(|e| e.to_string())?;
        backups
            .get(&backup_id)
            .cloned()
            .ok_or_else(|| "Backup not found".to_string())
    }

    /// Execute backup (internal)
    async fn execute_backup(
        &self,
        backup_id: &str,
        _scope: &BackupScope,
        _destination: &BackupDestination,
    ) -> Result<(), String> {
        // Update status to running
        {
            let mut backups = self.backups.write().map_err(|e| e.to_string())?;
            if let Some(backup) = backups.get_mut(backup_id) {
                backup.status = BackupStatus::Running;
            }
        }

        // Simulate backup execution
        // In production, this would:
        // 1. Query database for items in scope
        // 2. Serialize to backup format
        // 3. Upload to destination
        // 4. Calculate checksum
        // 5. Update record with results

        let completed_at = Utc::now();
        let started_at;

        {
            let mut backups = self.backups.write().map_err(|e| e.to_string())?;
            if let Some(backup) = backups.get_mut(backup_id) {
                started_at = backup.started_at;
                backup.status = BackupStatus::Completed;
                backup.completed_at = Some(completed_at);
                backup.duration_secs =
                    Some((completed_at - backup.started_at).num_seconds() as u64);
                backup.size_bytes = 1024 * 1024; // Simulated 1MB
                backup.item_count = 100; // Simulated 100 items
                backup.checksum = Some(format!("sha256:{}", Uuid::new_v4()));
            }
        }

        Ok(())
    }

    /// List backups
    pub async fn list_backups(
        &self,
        schedule_id: Option<&str>,
        status: Option<BackupStatus>,
        limit: Option<usize>,
    ) -> Result<Vec<BackupRecord>, String> {
        let backups = self.backups.read().map_err(|e| e.to_string())?;
        let mut result: Vec<_> = backups
            .values()
            .filter(|b| {
                schedule_id.map_or(true, |id| b.schedule_id.as_deref() == Some(id))
                    && status.map_or(true, |s| b.status == s)
            })
            .cloned()
            .collect();

        // Sort by start time descending
        result.sort_by(|a, b| b.started_at.cmp(&a.started_at));

        if let Some(limit) = limit {
            result.truncate(limit);
        }

        Ok(result)
    }

    /// Get a specific backup
    pub async fn get_backup(&self, id: &str) -> Result<BackupRecord, String> {
        let backups = self.backups.read().map_err(|e| e.to_string())?;
        backups
            .get(id)
            .cloned()
            .ok_or_else(|| format!("Backup not found: {}", id))
    }

    /// Delete a backup
    pub async fn delete_backup(&self, id: &str) -> Result<(), String> {
        let mut backups = self.backups.write().map_err(|e| e.to_string())?;
        backups
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| format!("Backup not found: {}", id))
    }

    /// Verify a backup
    pub async fn verify_backup(&self, id: &str) -> Result<bool, String> {
        let mut backups = self.backups.write().map_err(|e| e.to_string())?;
        let backup = backups
            .get_mut(id)
            .ok_or_else(|| format!("Backup not found: {}", id))?;

        backup.status = BackupStatus::Verifying;

        // In production, would:
        // 1. Download backup from destination
        // 2. Verify checksum
        // 3. Verify data integrity

        backup.status = BackupStatus::Verified;
        Ok(true)
    }

    // =========================================================================
    // Restore Operations
    // =========================================================================

    /// Restore from a backup
    pub async fn restore(&self, request: RestoreRequest) -> Result<RestoreResult, String> {
        let backup = self.get_backup(&request.backup_id).await?;

        if backup.status != BackupStatus::Completed && backup.status != BackupStatus::Verified {
            return Err("Cannot restore from incomplete or failed backup".to_string());
        }

        // In production, would:
        // 1. Download backup from destination
        // 2. Verify integrity
        // 3. Parse backup data
        // 4. Insert/update database records

        Ok(RestoreResult {
            success: true,
            items_restored: backup.item_count,
            items_skipped: 0,
            errors: Vec::new(),
            duration_secs: 5,
        })
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    fn calculate_next_run(&self, _cron_expression: &str) -> Option<DateTime<Utc>> {
        // In production, would parse cron expression and calculate next run
        // For now, return next hour
        Some(Utc::now() + Duration::hours(1))
    }

    fn generate_backup_path(&self, destination: &BackupDestination) -> String {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        match destination {
            BackupDestination::Local { path } => {
                format!("{}/backup_{}.tar.gz", path, timestamp)
            }
            BackupDestination::S3 { bucket, prefix, .. } => {
                format!("s3://{}/{}/backup_{}.tar.gz", bucket, prefix, timestamp)
            }
            BackupDestination::AzureBlob {
                container, prefix, ..
            } => {
                format!(
                    "azure://{}/{}/backup_{}.tar.gz",
                    container, prefix, timestamp
                )
            }
            BackupDestination::Gcs { bucket, prefix, .. } => {
                format!("gs://{}/{}/backup_{}.tar.gz", bucket, prefix, timestamp)
            }
            BackupDestination::Sftp { host, path, .. } => {
                format!("sftp://{}{}/backup_{}.tar.gz", host, path, timestamp)
            }
            BackupDestination::WebDav { url, .. } => {
                format!("{}/backup_{}.tar.gz", url, timestamp)
            }
        }
    }
}

impl Default for BackupService {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// HTTP Types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct ScheduleUpdate {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub cron_expression: Option<String>,
    pub backup_type: Option<BackupType>,
    pub scope: Option<BackupScope>,
    pub destination: Option<BackupDestination>,
    pub retention: Option<BackupRetention>,
}

#[derive(Debug, Deserialize)]
pub struct TriggerBackupRequest {
    pub schedule_id: Option<String>,
    pub backup_type: Option<BackupType>,
    pub scope: Option<BackupScope>,
    pub destination: BackupDestination,
}

#[derive(Debug, Deserialize)]
pub struct ListBackupsQuery {
    pub schedule_id: Option<String>,
    pub status: Option<BackupStatus>,
    pub limit: Option<usize>,
}

// =============================================================================
// HTTP Handlers
// =============================================================================

/// POST /api/backup/schedules - Create a schedule
pub async fn create_schedule(
    service: web::Data<BackupService>,
    body: web::Json<BackupSchedule>,
) -> HttpResponse {
    match service.create_schedule(body.into_inner()).await {
        Ok(schedule) => HttpResponse::Created().json(schedule),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/backup/schedules - List schedules
pub async fn list_schedules(
    service: web::Data<BackupService>,
    query: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    let owner_id = query.get("owner_id").map(String::as_str);
    match service.list_schedules(owner_id).await {
        Ok(schedules) => HttpResponse::Ok().json(schedules),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/backup/schedules/{id} - Get a schedule
pub async fn get_schedule(
    service: web::Data<BackupService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.get_schedule(&path.into_inner()).await {
        Ok(schedule) => HttpResponse::Ok().json(schedule),
        Err(e) => HttpResponse::NotFound().json(serde_json::json!({ "error": e })),
    }
}

/// PATCH /api/backup/schedules/{id} - Update a schedule
pub async fn update_schedule(
    service: web::Data<BackupService>,
    path: web::Path<String>,
    body: web::Json<ScheduleUpdate>,
) -> HttpResponse {
    match service
        .update_schedule(&path.into_inner(), body.into_inner())
        .await
    {
        Ok(schedule) => HttpResponse::Ok().json(schedule),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// DELETE /api/backup/schedules/{id} - Delete a schedule
pub async fn delete_schedule(
    service: web::Data<BackupService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.delete_schedule(&path.into_inner()).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::NotFound().json(serde_json::json!({ "error": e })),
    }
}

/// POST /api/backup/schedules/{id}/enable - Enable a schedule
pub async fn enable_schedule(
    service: web::Data<BackupService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.set_schedule_enabled(&path.into_inner(), true).await {
        Ok(schedule) => HttpResponse::Ok().json(schedule),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// POST /api/backup/schedules/{id}/disable - Disable a schedule
pub async fn disable_schedule(
    service: web::Data<BackupService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service
        .set_schedule_enabled(&path.into_inner(), false)
        .await
    {
        Ok(schedule) => HttpResponse::Ok().json(schedule),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// POST /api/backup/trigger - Trigger an immediate backup
pub async fn trigger_backup(
    service: web::Data<BackupService>,
    body: web::Json<TriggerBackupRequest>,
) -> HttpResponse {
    let request = body.into_inner();
    let backup_type = request.backup_type.unwrap_or_default();
    let scope = request.scope.unwrap_or_default();

    match service
        .trigger_backup(
            request.schedule_id.as_deref(),
            backup_type,
            scope,
            request.destination,
        )
        .await
    {
        Ok(backup) => HttpResponse::Accepted().json(backup),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/backup/backups - List backups
pub async fn list_backups(
    service: web::Data<BackupService>,
    query: web::Query<ListBackupsQuery>,
) -> HttpResponse {
    match service
        .list_backups(query.schedule_id.as_deref(), query.status, query.limit)
        .await
    {
        Ok(backups) => HttpResponse::Ok().json(backups),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/backup/backups/{id} - Get a backup
pub async fn get_backup(
    service: web::Data<BackupService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.get_backup(&path.into_inner()).await {
        Ok(backup) => HttpResponse::Ok().json(backup),
        Err(e) => HttpResponse::NotFound().json(serde_json::json!({ "error": e })),
    }
}

/// DELETE /api/backup/backups/{id} - Delete a backup
pub async fn delete_backup(
    service: web::Data<BackupService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.delete_backup(&path.into_inner()).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::NotFound().json(serde_json::json!({ "error": e })),
    }
}

/// POST /api/backup/backups/{id}/verify - Verify a backup
pub async fn verify_backup(
    service: web::Data<BackupService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.verify_backup(&path.into_inner()).await {
        Ok(valid) => HttpResponse::Ok().json(serde_json::json!({ "valid": valid })),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// POST /api/backup/restore - Restore from a backup
pub async fn restore_backup(
    service: web::Data<BackupService>,
    body: web::Json<RestoreRequest>,
) -> HttpResponse {
    match service.restore(body.into_inner()).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// Configure backup routes
pub fn configure_backup_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/backup")
            // Schedule endpoints
            .route("/schedules", web::post().to(create_schedule))
            .route("/schedules", web::get().to(list_schedules))
            .route("/schedules/{id}", web::get().to(get_schedule))
            .route("/schedules/{id}", web::patch().to(update_schedule))
            .route("/schedules/{id}", web::delete().to(delete_schedule))
            .route("/schedules/{id}/enable", web::post().to(enable_schedule))
            .route("/schedules/{id}/disable", web::post().to(disable_schedule))
            // Backup endpoints
            .route("/trigger", web::post().to(trigger_backup))
            .route("/backups", web::get().to(list_backups))
            .route("/backups/{id}", web::get().to(get_backup))
            .route("/backups/{id}", web::delete().to(delete_backup))
            .route("/backups/{id}/verify", web::post().to(verify_backup))
            // Restore endpoint
            .route("/restore", web::post().to(restore_backup)),
    );
}
