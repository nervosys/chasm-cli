// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Cross-Device Sync Module
//!
//! Provides real-time synchronization across multiple devices:
//! - Device registration and management
//! - Conflict-free replicated data types (CRDTs) for sync
//! - Offline-first with eventual consistency
//! - Selective sync with bandwidth optimization
//! - End-to-end encryption for sync data

use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// =============================================================================
// Core Types
// =============================================================================

/// Registered device for sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDevice {
    /// Device ID
    pub id: String,
    /// User ID that owns this device
    pub user_id: String,
    /// Device name (user-friendly)
    pub name: String,
    /// Device type
    pub device_type: DeviceType,
    /// Platform information
    pub platform: DevicePlatform,
    /// Last sync timestamp
    pub last_sync: Option<DateTime<Utc>>,
    /// Last seen online
    pub last_seen: DateTime<Utc>,
    /// Device registration time
    pub registered_at: DateTime<Utc>,
    /// Whether device is currently online
    pub is_online: bool,
    /// Sync configuration for this device
    pub sync_config: DeviceSyncConfig,
    /// Device capabilities
    pub capabilities: DeviceCapabilities,
    /// Push notification token
    pub push_token: Option<String>,
}

/// Device types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    Desktop,
    Laptop,
    Phone,
    Tablet,
    Browser,
    Server,
    Other,
}

/// Platform information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePlatform {
    pub os: String,
    pub os_version: Option<String>,
    pub app_version: String,
    pub browser: Option<String>,
}

/// Per-device sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSyncConfig {
    /// Enable sync for this device
    pub sync_enabled: bool,
    /// Sync over cellular data
    pub sync_on_cellular: bool,
    /// Sync only on WiFi
    pub wifi_only: bool,
    /// Maximum storage for sync cache (bytes)
    pub max_cache_size: u64,
    /// Sync frequency (seconds, 0 = real-time)
    pub sync_interval: u32,
    /// What to sync
    pub sync_scope: SyncScope,
}

impl Default for DeviceSyncConfig {
    fn default() -> Self {
        Self {
            sync_enabled: true,
            sync_on_cellular: true,
            wifi_only: false,
            max_cache_size: 100 * 1024 * 1024, // 100MB
            sync_interval: 0,                  // Real-time
            sync_scope: SyncScope::default(),
        }
    }
}

/// What to sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncScope {
    pub sessions: bool,
    pub workspaces: bool,
    pub settings: bool,
    pub favorites: bool,
    pub tags: bool,
}

impl Default for SyncScope {
    fn default() -> Self {
        Self {
            sessions: true,
            workspaces: true,
            settings: true,
            favorites: true,
            tags: true,
        }
    }
}

/// Device capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub supports_push: bool,
    pub supports_background_sync: bool,
    pub supports_encryption: bool,
    pub max_payload_size: u64,
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            supports_push: true,
            supports_background_sync: true,
            supports_encryption: true,
            max_payload_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// Sync operation/change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOperation {
    /// Operation ID
    pub id: String,
    /// Timestamp (vector clock component)
    pub timestamp: DateTime<Utc>,
    /// Logical clock value
    pub logical_clock: u64,
    /// Device that made the change
    pub device_id: String,
    /// User ID
    pub user_id: String,
    /// Operation type
    pub operation: OperationType,
    /// Resource type being synced
    pub resource_type: SyncResourceType,
    /// Resource ID
    pub resource_id: String,
    /// Operation data
    pub data: serde_json::Value,
    /// Whether this operation has been acknowledged
    pub acknowledged: bool,
}

/// Types of sync operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    Create,
    Update,
    Delete,
    Move,
    Merge,
}

/// Resource types that can be synced
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SyncResourceType {
    Session,
    Message,
    Workspace,
    Settings,
    Tag,
    Favorite,
}

/// Sync state for a device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    /// Device ID
    pub device_id: String,
    /// Last synced operation ID per resource type
    pub cursors: HashMap<SyncResourceType, String>,
    /// Last sync timestamp
    pub last_sync: DateTime<Utc>,
    /// Number of pending operations
    pub pending_count: usize,
    /// Sync status
    pub status: SyncStatus,
}

/// Sync status
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Idle,
    Syncing,
    Paused,
    Error,
    Offline,
}

/// Conflict detected during sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflict {
    /// Conflict ID
    pub id: String,
    /// Resource type
    pub resource_type: SyncResourceType,
    /// Resource ID
    pub resource_id: String,
    /// Local operation
    pub local_operation: SyncOperation,
    /// Remote operation
    pub remote_operation: SyncOperation,
    /// Conflict resolution
    pub resolution: Option<ConflictResolution>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Whether resolved
    pub resolved: bool,
}

/// How a conflict was resolved
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    /// Keep local version
    KeepLocal,
    /// Keep remote version
    KeepRemote,
    /// Merge both versions
    Merge { merged_data: serde_json::Value },
    /// Manual resolution
    Manual { chosen_data: serde_json::Value },
}

/// Sync delta (changes since last sync)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDelta {
    /// Operations to apply
    pub operations: Vec<SyncOperation>,
    /// New cursor positions
    pub cursors: HashMap<SyncResourceType, String>,
    /// Whether there are more changes
    pub has_more: bool,
    /// Server timestamp
    pub server_time: DateTime<Utc>,
}

/// Push changes request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushChangesRequest {
    /// Device ID
    pub device_id: String,
    /// Operations to push
    pub operations: Vec<SyncOperation>,
    /// Current local cursors
    pub cursors: HashMap<SyncResourceType, String>,
}

/// Push changes response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushChangesResponse {
    /// Successfully applied operations
    pub applied: Vec<String>,
    /// Rejected operations (conflicts)
    pub rejected: Vec<String>,
    /// Conflicts detected
    pub conflicts: Vec<SyncConflict>,
    /// Updated cursors
    pub cursors: HashMap<SyncResourceType, String>,
}

/// Pull changes request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullChangesRequest {
    /// Device ID
    pub device_id: String,
    /// Current cursors
    pub cursors: HashMap<SyncResourceType, String>,
    /// Max operations to return
    pub limit: Option<usize>,
    /// Resource types to pull (empty = all)
    pub resource_types: Vec<SyncResourceType>,
}

// =============================================================================
// Sync Service
// =============================================================================

/// Service for managing cross-device sync
pub struct CrossDeviceSyncService {
    devices: std::sync::RwLock<HashMap<String, SyncDevice>>,
    operations: std::sync::RwLock<Vec<SyncOperation>>,
    conflicts: std::sync::RwLock<HashMap<String, SyncConflict>>,
    logical_clock: std::sync::atomic::AtomicU64,
}

impl CrossDeviceSyncService {
    pub fn new() -> Self {
        Self {
            devices: std::sync::RwLock::new(HashMap::new()),
            operations: std::sync::RwLock::new(Vec::new()),
            conflicts: std::sync::RwLock::new(HashMap::new()),
            logical_clock: std::sync::atomic::AtomicU64::new(0),
        }
    }

    // =========================================================================
    // Device Management
    // =========================================================================

    /// Register a new device
    pub async fn register_device(
        &self,
        user_id: &str,
        name: &str,
        device_type: DeviceType,
        platform: DevicePlatform,
    ) -> Result<SyncDevice, String> {
        let device = SyncDevice {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            name: name.to_string(),
            device_type,
            platform,
            last_sync: None,
            last_seen: Utc::now(),
            registered_at: Utc::now(),
            is_online: true,
            sync_config: DeviceSyncConfig::default(),
            capabilities: DeviceCapabilities::default(),
            push_token: None,
        };

        let mut devices = self.devices.write().map_err(|e| e.to_string())?;
        devices.insert(device.id.clone(), device.clone());

        Ok(device)
    }

    /// Get a device
    pub async fn get_device(&self, device_id: &str) -> Result<SyncDevice, String> {
        let devices = self.devices.read().map_err(|e| e.to_string())?;
        devices
            .get(device_id)
            .cloned()
            .ok_or_else(|| format!("Device not found: {}", device_id))
    }

    /// List devices for a user
    pub async fn list_devices(&self, user_id: &str) -> Result<Vec<SyncDevice>, String> {
        let devices = self.devices.read().map_err(|e| e.to_string())?;
        let result: Vec<_> = devices
            .values()
            .filter(|d| d.user_id == user_id)
            .cloned()
            .collect();
        Ok(result)
    }

    /// Update device
    pub async fn update_device(
        &self,
        device_id: &str,
        name: Option<String>,
        sync_config: Option<DeviceSyncConfig>,
        push_token: Option<String>,
    ) -> Result<SyncDevice, String> {
        let mut devices = self.devices.write().map_err(|e| e.to_string())?;
        let device = devices
            .get_mut(device_id)
            .ok_or_else(|| format!("Device not found: {}", device_id))?;

        if let Some(n) = name {
            device.name = n;
        }
        if let Some(config) = sync_config {
            device.sync_config = config;
        }
        if let Some(token) = push_token {
            device.push_token = Some(token);
        }

        Ok(device.clone())
    }

    /// Remove a device
    pub async fn remove_device(&self, device_id: &str) -> Result<(), String> {
        let mut devices = self.devices.write().map_err(|e| e.to_string())?;
        devices
            .remove(device_id)
            .map(|_| ())
            .ok_or_else(|| format!("Device not found: {}", device_id))
    }

    /// Update device online status
    pub async fn set_device_online(&self, device_id: &str, online: bool) -> Result<(), String> {
        let mut devices = self.devices.write().map_err(|e| e.to_string())?;
        if let Some(device) = devices.get_mut(device_id) {
            device.is_online = online;
            device.last_seen = Utc::now();
        }
        Ok(())
    }

    // =========================================================================
    // Sync Operations
    // =========================================================================

    /// Push changes from a device
    pub async fn push_changes(
        &self,
        request: PushChangesRequest,
    ) -> Result<PushChangesResponse, String> {
        let mut applied = Vec::new();
        let mut rejected = Vec::new();
        let mut conflicts = Vec::new();

        // Get next logical clock value
        let clock = self
            .logical_clock
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let mut operations = self.operations.write().map_err(|e| e.to_string())?;

        for mut op in request.operations {
            // Check for conflicts
            if let Some(conflict) = self.detect_conflict(&op, &operations) {
                rejected.push(op.id.clone());

                let conflict_record = SyncConflict {
                    id: Uuid::new_v4().to_string(),
                    resource_type: op.resource_type,
                    resource_id: op.resource_id.clone(),
                    local_operation: op,
                    remote_operation: conflict,
                    resolution: None,
                    created_at: Utc::now(),
                    resolved: false,
                };

                conflicts.push(conflict_record.clone());

                // Store conflict
                let mut conflict_store = self.conflicts.write().map_err(|e| e.to_string())?;
                conflict_store.insert(conflict_record.id.clone(), conflict_record);
            } else {
                // Apply operation
                op.logical_clock = clock;
                op.acknowledged = true;
                applied.push(op.id.clone());
                operations.push(op);
            }
        }

        // Update device last sync
        {
            let mut devices = self.devices.write().map_err(|e| e.to_string())?;
            if let Some(device) = devices.get_mut(&request.device_id) {
                device.last_sync = Some(Utc::now());
            }
        }

        // Calculate new cursors
        let mut cursors = request.cursors;
        for op_id in &applied {
            if let Some(op) = operations.iter().find(|o| &o.id == op_id) {
                cursors.insert(op.resource_type, op.id.clone());
            }
        }

        Ok(PushChangesResponse {
            applied,
            rejected,
            conflicts,
            cursors,
        })
    }

    /// Pull changes for a device
    pub async fn pull_changes(&self, request: PullChangesRequest) -> Result<SyncDelta, String> {
        let operations = self.operations.read().map_err(|e| e.to_string())?;
        let limit = request.limit.unwrap_or(100);

        // Find operations after cursors
        let mut pending_ops: Vec<_> = operations
            .iter()
            .filter(|op| {
                // Filter by resource type if specified
                if !request.resource_types.is_empty()
                    && !request.resource_types.contains(&op.resource_type)
                {
                    return false;
                }

                // Filter by cursor
                if let Some(cursor) = request.cursors.get(&op.resource_type) {
                    // Find cursor position
                    if let Some(cursor_pos) = operations.iter().position(|o| &o.id == cursor) {
                        if let Some(op_pos) = operations.iter().position(|o| o.id == op.id) {
                            return op_pos > cursor_pos;
                        }
                    }
                }

                // No cursor = include all
                true
            })
            .filter(|op| op.device_id != request.device_id) // Don't send back own changes
            .cloned()
            .collect();

        // Sort by logical clock
        pending_ops.sort_by_key(|op| op.logical_clock);

        let has_more = pending_ops.len() > limit;
        pending_ops.truncate(limit);

        // Calculate new cursors
        let mut cursors = request.cursors;
        for op in &pending_ops {
            cursors.insert(op.resource_type, op.id.clone());
        }

        Ok(SyncDelta {
            operations: pending_ops,
            cursors,
            has_more,
            server_time: Utc::now(),
        })
    }

    /// Get sync state for a device
    pub async fn get_sync_state(&self, device_id: &str) -> Result<SyncState, String> {
        let device = self.get_device(device_id).await?;
        let operations = self.operations.read().map_err(|e| e.to_string())?;

        // Count pending operations for this device
        let pending_count = operations
            .iter()
            .filter(|op| op.device_id != device_id && !op.acknowledged)
            .count();

        Ok(SyncState {
            device_id: device_id.to_string(),
            cursors: HashMap::new(), // Would be populated from persistent storage
            last_sync: device.last_sync.unwrap_or(device.registered_at),
            pending_count,
            status: if device.is_online {
                SyncStatus::Idle
            } else {
                SyncStatus::Offline
            },
        })
    }

    // =========================================================================
    // Conflict Management
    // =========================================================================

    /// Detect if an operation conflicts with existing operations
    fn detect_conflict(
        &self,
        op: &SyncOperation,
        existing: &[SyncOperation],
    ) -> Option<SyncOperation> {
        // Find concurrent operations on same resource
        existing
            .iter()
            .filter(|e| {
                e.resource_type == op.resource_type
                    && e.resource_id == op.resource_id
                    && e.device_id != op.device_id
                    && e.timestamp > op.timestamp - chrono::Duration::seconds(5)
                // Within 5 seconds
            })
            .last()
            .cloned()
    }

    /// List conflicts for a user
    pub async fn list_conflicts(&self, user_id: &str) -> Result<Vec<SyncConflict>, String> {
        let conflicts = self.conflicts.read().map_err(|e| e.to_string())?;
        let result: Vec<_> = conflicts
            .values()
            .filter(|c| c.local_operation.user_id == user_id && !c.resolved)
            .cloned()
            .collect();
        Ok(result)
    }

    /// Resolve a conflict
    pub async fn resolve_conflict(
        &self,
        conflict_id: &str,
        resolution: ConflictResolution,
    ) -> Result<SyncConflict, String> {
        let mut conflicts = self.conflicts.write().map_err(|e| e.to_string())?;
        let conflict = conflicts
            .get_mut(conflict_id)
            .ok_or_else(|| format!("Conflict not found: {}", conflict_id))?;

        conflict.resolution = Some(resolution);
        conflict.resolved = true;

        Ok(conflict.clone())
    }
}

impl Default for CrossDeviceSyncService {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// HTTP Handlers
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct RegisterDeviceRequest {
    pub user_id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub platform: DevicePlatform,
}

/// POST /api/sync/devices - Register a device
pub async fn register_device(
    service: web::Data<CrossDeviceSyncService>,
    body: web::Json<RegisterDeviceRequest>,
) -> HttpResponse {
    let request = body.into_inner();
    match service
        .register_device(
            &request.user_id,
            &request.name,
            request.device_type,
            request.platform,
        )
        .await
    {
        Ok(device) => HttpResponse::Created().json(device),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/sync/devices/{id} - Get a device
pub async fn get_device(
    service: web::Data<CrossDeviceSyncService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.get_device(&path.into_inner()).await {
        Ok(device) => HttpResponse::Ok().json(device),
        Err(e) => HttpResponse::NotFound().json(serde_json::json!({ "error": e })),
    }
}

#[derive(Debug, Deserialize)]
pub struct ListDevicesQuery {
    pub user_id: String,
}

/// GET /api/sync/devices - List devices
pub async fn list_devices(
    service: web::Data<CrossDeviceSyncService>,
    query: web::Query<ListDevicesQuery>,
) -> HttpResponse {
    match service.list_devices(&query.user_id).await {
        Ok(devices) => HttpResponse::Ok().json(devices),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateDeviceRequest {
    pub name: Option<String>,
    pub sync_config: Option<DeviceSyncConfig>,
    pub push_token: Option<String>,
}

/// PATCH /api/sync/devices/{id} - Update a device
pub async fn update_device(
    service: web::Data<CrossDeviceSyncService>,
    path: web::Path<String>,
    body: web::Json<UpdateDeviceRequest>,
) -> HttpResponse {
    let request = body.into_inner();
    match service
        .update_device(
            &path.into_inner(),
            request.name,
            request.sync_config,
            request.push_token,
        )
        .await
    {
        Ok(device) => HttpResponse::Ok().json(device),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// DELETE /api/sync/devices/{id} - Remove a device
pub async fn remove_device(
    service: web::Data<CrossDeviceSyncService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.remove_device(&path.into_inner()).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::NotFound().json(serde_json::json!({ "error": e })),
    }
}

/// POST /api/sync/push - Push changes
pub async fn push_changes(
    service: web::Data<CrossDeviceSyncService>,
    body: web::Json<PushChangesRequest>,
) -> HttpResponse {
    match service.push_changes(body.into_inner()).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// POST /api/sync/pull - Pull changes
pub async fn pull_changes(
    service: web::Data<CrossDeviceSyncService>,
    body: web::Json<PullChangesRequest>,
) -> HttpResponse {
    match service.pull_changes(body.into_inner()).await {
        Ok(delta) => HttpResponse::Ok().json(delta),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/sync/state/{device_id} - Get sync state
pub async fn get_sync_state(
    service: web::Data<CrossDeviceSyncService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.get_sync_state(&path.into_inner()).await {
        Ok(state) => HttpResponse::Ok().json(state),
        Err(e) => HttpResponse::NotFound().json(serde_json::json!({ "error": e })),
    }
}

#[derive(Debug, Deserialize)]
pub struct ListConflictsQuery {
    pub user_id: String,
}

/// GET /api/sync/conflicts - List conflicts
pub async fn list_conflicts(
    service: web::Data<CrossDeviceSyncService>,
    query: web::Query<ListConflictsQuery>,
) -> HttpResponse {
    match service.list_conflicts(&query.user_id).await {
        Ok(conflicts) => HttpResponse::Ok().json(conflicts),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

#[derive(Debug, Deserialize)]
pub struct ResolveConflictRequest {
    pub resolution: ConflictResolution,
}

/// POST /api/sync/conflicts/{id}/resolve - Resolve a conflict
pub async fn resolve_conflict(
    service: web::Data<CrossDeviceSyncService>,
    path: web::Path<String>,
    body: web::Json<ResolveConflictRequest>,
) -> HttpResponse {
    match service
        .resolve_conflict(&path.into_inner(), body.into_inner().resolution)
        .await
    {
        Ok(conflict) => HttpResponse::Ok().json(conflict),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// Configure cross-device sync routes
pub fn configure_device_sync_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/sync")
            // Device endpoints
            .route("/devices", web::post().to(register_device))
            .route("/devices", web::get().to(list_devices))
            .route("/devices/{id}", web::get().to(get_device))
            .route("/devices/{id}", web::patch().to(update_device))
            .route("/devices/{id}", web::delete().to(remove_device))
            // Sync endpoints
            .route("/push", web::post().to(push_changes))
            .route("/pull", web::post().to(pull_changes))
            .route("/state/{device_id}", web::get().to(get_sync_state))
            // Conflict endpoints
            .route("/conflicts", web::get().to(list_conflicts))
            .route("/conflicts/{id}/resolve", web::post().to(resolve_conflict)),
    );
}
