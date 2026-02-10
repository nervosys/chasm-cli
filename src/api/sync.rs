// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Sync Handler for real-time data synchronization
//!
//! This module provides HTTP/SSE-based synchronization between
//! csm-rust (backend), csm-web, and csm-app clients.
//!
//! Uses Server-Sent Events (SSE) for real-time push updates instead of
//! WebSockets for better compatibility with various deployment scenarios.

use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

// =============================================================================
// Sync Message Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncEntityType {
    Workspace,
    Session,
    Message,
    Agent,
    Swarm,
    Provider,
    Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncOperation {
    Create,
    Update,
    Delete,
    Sync,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub entity_type: SyncEntityType,
    pub operation: SyncOperation,
    pub entity_id: String,
    pub data: Option<serde_json::Value>,
    pub timestamp: i64,
    pub client_id: String,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSnapshot {
    pub workspaces: Vec<serde_json::Value>,
    pub sessions: Vec<serde_json::Value>,
    pub agents: Vec<serde_json::Value>,
    pub swarms: Vec<serde_json::Value>,
    pub providers: Vec<serde_json::Value>,
    pub timestamp: i64,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDelta {
    pub created: Vec<SyncEvent>,
    pub updated: Vec<SyncEvent>,
    pub deleted: Vec<SyncEvent>,
    pub timestamp: i64,
    pub from_version: u64,
    pub to_version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Welcome { version: u64 },
    SyncEvent { event: SyncEvent },
    Ack { version: u64 },
}

// =============================================================================
// Sync State
// =============================================================================

/// Global sync state shared across all connections
pub struct SyncState {
    /// Current version counter
    pub version: u64,
    /// Event history for delta sync
    pub events: Vec<SyncEvent>,
    /// Maximum events to keep in history
    pub max_history: usize,
    /// Broadcast channel for events
    pub broadcast_tx: broadcast::Sender<ServerMessage>,
}

impl SyncState {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        Self {
            version: 0,
            events: Vec::new(),
            max_history: 10000,
            broadcast_tx,
        }
    }

    /// Increment version and return new version
    pub fn next_version(&mut self) -> u64 {
        self.version += 1;
        self.version
    }

    /// Add event to history and broadcast
    pub fn add_event(&mut self, mut event: SyncEvent) -> u64 {
        let version = self.next_version();
        event.version = version;
        event.timestamp = chrono::Utc::now().timestamp_millis();

        self.events.push(event.clone());

        // Trim history if needed
        if self.events.len() > self.max_history {
            let trim_count = self.events.len() - self.max_history;
            self.events.drain(0..trim_count);
        }

        // Broadcast to all clients
        let _ = self.broadcast_tx.send(ServerMessage::SyncEvent { event });

        version
    }

    /// Get events since a version
    pub fn get_delta(&self, from_version: u64) -> SyncDelta {
        let events: Vec<_> = self
            .events
            .iter()
            .filter(|e| e.version > from_version)
            .cloned()
            .collect();

        let mut created = Vec::new();
        let mut updated = Vec::new();
        let mut deleted = Vec::new();

        for event in events {
            match event.operation {
                SyncOperation::Create => created.push(event),
                SyncOperation::Update | SyncOperation::Sync => updated.push(event),
                SyncOperation::Delete => deleted.push(event),
            }
        }

        SyncDelta {
            created,
            updated,
            deleted,
            timestamp: chrono::Utc::now().timestamp_millis(),
            from_version,
            to_version: self.version,
        }
    }

    /// Get broadcast receiver
    pub fn subscribe(&self) -> broadcast::Receiver<ServerMessage> {
        self.broadcast_tx.subscribe()
    }
}

impl Default for SyncState {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedSyncState = Arc<RwLock<SyncState>>;

/// Create shared sync state
pub fn create_sync_state() -> SharedSyncState {
    Arc::new(RwLock::new(SyncState::new()))
}

// =============================================================================
// HTTP/REST Sync Endpoints
// =============================================================================

/// Query parameters for delta request
#[derive(Debug, Deserialize)]
pub struct DeltaQuery {
    pub from: Option<u64>,
}

/// Get current sync version
pub async fn get_sync_version(sync_state: web::Data<SharedSyncState>) -> HttpResponse {
    let state = sync_state.read().unwrap();
    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": {
            "version": state.version,
            "eventCount": state.events.len(),
        }
    }))
}

/// Get sync delta since a version
pub async fn get_sync_delta(
    sync_state: web::Data<SharedSyncState>,
    query: web::Query<DeltaQuery>,
) -> HttpResponse {
    let from_version = query.from.unwrap_or(0);

    let state = sync_state.read().unwrap();
    let delta = state.get_delta(from_version);

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": delta,
    }))
}

/// Post a sync event
pub async fn post_sync_event(
    sync_state: web::Data<SharedSyncState>,
    body: web::Json<SyncEvent>,
) -> HttpResponse {
    let mut state = sync_state.write().unwrap();
    let version = state.add_event(body.into_inner());

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": {
            "version": version,
        }
    }))
}

/// Batch sync events request
#[derive(Debug, Deserialize)]
pub struct BatchSyncRequest {
    pub events: Vec<SyncEvent>,
}

/// Post multiple sync events
pub async fn post_sync_batch(
    sync_state: web::Data<SharedSyncState>,
    body: web::Json<BatchSyncRequest>,
) -> HttpResponse {
    let mut state = sync_state.write().unwrap();
    let mut last_version = 0;

    for event in body.into_inner().events {
        last_version = state.add_event(event);
    }

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": {
            "version": last_version,
        }
    }))
}

/// Get full snapshot
pub async fn get_sync_snapshot(
    sync_state: web::Data<SharedSyncState>,
    app_state: web::Data<crate::api::state::AppState>,
) -> HttpResponse {
    let db = app_state.db.lock().unwrap();
    let sync = sync_state.read().unwrap();

    // Get workspaces from database
    let workspaces = db
        .list_workspaces()
        .unwrap_or_default()
        .into_iter()
        .map(|w| serde_json::to_value(w).unwrap_or_default())
        .collect();

    // Get sessions (3 args: workspace_id, provider, limit)
    let sessions = db
        .list_sessions(None, None, 1000)
        .unwrap_or_default()
        .into_iter()
        .map(|s| serde_json::to_value(s).unwrap_or_default())
        .collect();

    // Query agents directly from database
    let agents: Vec<serde_json::Value> = query_agents_from_db(&db.conn).unwrap_or_default();

    // Query swarms directly from database
    let swarms: Vec<serde_json::Value> = query_swarms_from_db(&db.conn).unwrap_or_default();

    // Providers are hardcoded, return empty for snapshot
    // (clients should call /api/providers for the full list)
    let providers: Vec<serde_json::Value> = Vec::new();

    let snapshot = SyncSnapshot {
        workspaces,
        sessions,
        agents,
        swarms,
        providers,
        timestamp: chrono::Utc::now().timestamp_millis(),
        version: sync.version,
    };

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": snapshot,
    }))
}

/// Query agents directly from database
fn query_agents_from_db(
    conn: &rusqlite::Connection,
) -> Result<Vec<serde_json::Value>, rusqlite::Error> {
    // Ensure table exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agents (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            instruction TEXT NOT NULL,
            role TEXT,
            model TEXT,
            provider TEXT,
            temperature REAL DEFAULT 0.7,
            max_tokens INTEGER,
            tools TEXT,
            sub_agents TEXT,
            is_active INTEGER DEFAULT 1,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            metadata TEXT
        )",
        [],
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, name, description, instruction, role, model, provider, 
                temperature, max_tokens, tools, sub_agents, is_active, 
                created_at, updated_at, metadata 
         FROM agents ORDER BY updated_at DESC",
    )?;

    let agents: Vec<serde_json::Value> = stmt
        .query_map([], |row| {
            let tools_str: String = row.get::<_, Option<String>>(9)?.unwrap_or_default();
            let tools: Vec<String> = serde_json::from_str(&tools_str).unwrap_or_default();
            let sub_agents_str: String = row.get::<_, Option<String>>(10)?.unwrap_or_default();
            let sub_agents: Vec<String> = serde_json::from_str(&sub_agents_str).unwrap_or_default();
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "description": row.get::<_, Option<String>>(2)?,
                "instruction": row.get::<_, String>(3)?,
                "role": row.get::<_, Option<String>>(4)?,
                "model": row.get::<_, Option<String>>(5)?,
                "provider": row.get::<_, Option<String>>(6)?,
                "temperature": row.get::<_, f64>(7)?,
                "maxTokens": row.get::<_, Option<i32>>(8)?,
                "tools": tools,
                "subAgents": sub_agents,
                "isActive": row.get::<_, i32>(11)? == 1,
                "createdAt": row.get::<_, i64>(12)?,
                "updatedAt": row.get::<_, i64>(13)?,
                "metadata": row.get::<_, Option<String>>(14)?,
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(agents)
}

/// Query swarms directly from database  
fn query_swarms_from_db(
    conn: &rusqlite::Connection,
) -> Result<Vec<serde_json::Value>, rusqlite::Error> {
    // Ensure table exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS swarms (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            agents TEXT NOT NULL,
            orchestrator TEXT,
            is_active INTEGER DEFAULT 1,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            metadata TEXT
        )",
        [],
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, name, description, agents, orchestrator, is_active, 
                created_at, updated_at, metadata 
         FROM swarms ORDER BY updated_at DESC",
    )?;

    let swarms: Vec<serde_json::Value> = stmt
        .query_map([], |row| {
            let agents_str: String = row.get::<_, String>(3)?;
            let agents: Vec<String> = serde_json::from_str(&agents_str).unwrap_or_default();
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "description": row.get::<_, Option<String>>(2)?,
                "agents": agents,
                "orchestrator": row.get::<_, Option<String>>(4)?,
                "isActive": row.get::<_, i32>(5)? == 1,
                "createdAt": row.get::<_, i64>(6)?,
                "updatedAt": row.get::<_, i64>(7)?,
                "metadata": row.get::<_, Option<String>>(8)?,
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(swarms)
}

/// Subscribe to sync events via Server-Sent Events (SSE)
pub async fn sync_sse(sync_state: web::Data<SharedSyncState>) -> HttpResponse {
    let state = sync_state.read().unwrap();
    let mut rx = state.subscribe();
    let current_version = state.version;
    drop(state);

    let stream = async_stream::stream! {
        // Send initial welcome with version
        let welcome = ServerMessage::Welcome { version: current_version };
        if let Ok(json) = serde_json::to_string(&welcome) {
            yield Ok::<_, std::io::Error>(web::Bytes::from(format!("data: {}\n\n", json)));
        }

        // Stream events
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if let Ok(json) = serde_json::to_string(&msg) {
                        yield Ok(web::Bytes::from(format!("data: {}\n\n", json)));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    // Client missed some messages, they should request a delta
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .streaming(stream)
}

// =============================================================================
// Route Configuration
// =============================================================================

/// Configure sync routes
pub fn configure_sync_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/sync")
            .route("/version", web::get().to(get_sync_version))
            .route("/delta", web::get().to(get_sync_delta))
            .route("/event", web::post().to(post_sync_event))
            .route("/batch", web::post().to(post_sync_batch))
            .route("/snapshot", web::get().to(get_sync_snapshot))
            .route("/subscribe", web::get().to(sync_sse)),
    );
}
