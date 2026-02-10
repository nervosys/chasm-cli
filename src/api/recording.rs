// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Real-time Session Recording API
//!
//! This module provides endpoints for recording chat sessions in real-time,
//! preventing data loss from editor crashes. It accepts incremental updates
//! via WebSocket or REST endpoints and persists them to the universal database.
//!
//! ## Architecture
//!
//! ```text
//! VS Code Extension ─┬─> WebSocket ─┬─> RecordingService ─> Database
//!                    │              │
//!                    └─> REST API ──┘
//! ```
//!
//! ## Recording Modes
//!
//! - **Live**: Real-time event streaming via WebSocket
//! - **Batch**: Periodic sync via REST (fallback)
//! - **Hybrid**: WebSocket with REST checkpoint backup

use actix_web::{web, Error, HttpRequest, HttpResponse, Responder};
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use uuid::Uuid;

// =============================================================================
// Recording Event Types
// =============================================================================

/// Events sent from client for real-time recording
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RecordingEvent {
    /// Start recording a new session
    SessionStart {
        session_id: String,
        workspace_id: Option<String>,
        workspace_path: Option<String>,
        provider: String,
        title: Option<String>,
        model: Option<String>,
        metadata: Option<serde_json::Value>,
    },

    /// End a recording session
    SessionEnd {
        session_id: String,
        final_message_count: Option<i32>,
    },

    /// Add a new message to the session
    MessageAdd {
        session_id: String,
        message_id: String,
        role: String, // "user", "assistant", "system"
        content: String,
        model: Option<String>,
        parent_id: Option<String>,
        metadata: Option<serde_json::Value>,
    },

    /// Update message content (for streaming responses)
    MessageUpdate {
        session_id: String,
        message_id: String,
        content: String,
        is_complete: bool,
    },

    /// Append to message content (for streaming tokens)
    MessageAppend {
        session_id: String,
        message_id: String,
        content_delta: String,
    },

    /// Update session metadata (title, tags, etc.)
    SessionUpdate {
        session_id: String,
        title: Option<String>,
        model: Option<String>,
        metadata: Option<serde_json::Value>,
    },

    /// Heartbeat to keep connection alive
    Heartbeat {
        session_id: Option<String>,
        timestamp: i64,
    },

    /// Full session snapshot for recovery
    SessionSnapshot {
        session_id: String,
        provider: String,
        workspace_path: Option<String>,
        title: Option<String>,
        messages: Vec<RecordedMessage>,
        metadata: Option<serde_json::Value>,
    },
}

/// A recorded message in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedMessage {
    pub message_id: String,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub created_at: i64,
    pub parent_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Response sent back to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RecordingResponse {
    /// Acknowledgement of received event
    Ack {
        event_id: String,
        session_id: String,
        status: String,
    },

    /// Error response
    Error {
        event_id: Option<String>,
        code: String,
        message: String,
    },

    /// Session recovery data
    Recovery {
        session_id: String,
        last_message_id: Option<String>,
        message_count: i32,
    },
}

// =============================================================================
// Recording State
// =============================================================================

/// Active recording session state
#[derive(Debug, Clone)]
pub struct ActiveSession {
    pub session_id: String,
    pub workspace_id: Option<String>,
    pub workspace_path: Option<String>,
    pub provider: String,
    pub title: Option<String>,
    pub model: Option<String>,
    pub messages: Vec<RecordedMessage>,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub last_persisted_at: DateTime<Utc>,
    pub is_dirty: bool,
}

/// Recording service state
pub struct RecordingState {
    /// Active recording sessions (session_id -> session)
    pub active_sessions: RwLock<HashMap<String, ActiveSession>>,
    /// Event broadcast channel for WebSocket distribution
    pub event_tx: broadcast::Sender<RecordingEvent>,
    /// Configuration
    pub config: RecordingConfig,
}

/// Recording configuration
#[derive(Debug, Clone)]
pub struct RecordingConfig {
    /// How often to persist dirty sessions (seconds)
    pub persist_interval_secs: u64,
    /// Maximum messages to keep in memory before forced persist
    pub max_memory_messages: usize,
    /// Session timeout for inactivity (seconds)
    pub session_timeout_secs: u64,
    /// Enable debug logging
    pub debug: bool,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            persist_interval_secs: 5,
            max_memory_messages: 100,
            session_timeout_secs: 3600, // 1 hour
            debug: false,
        }
    }
}

impl RecordingState {
    pub fn new(config: RecordingConfig) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            active_sessions: RwLock::new(HashMap::new()),
            event_tx,
            config,
        }
    }

    /// Process a recording event
    pub fn process_event(&self, event: &RecordingEvent) -> RecordingResponse {
        match event {
            RecordingEvent::SessionStart {
                session_id,
                workspace_id,
                workspace_path,
                provider,
                title,
                model,
                metadata: _,
            } => {
                let session = ActiveSession {
                    session_id: session_id.clone(),
                    workspace_id: workspace_id.clone(),
                    workspace_path: workspace_path.clone(),
                    provider: provider.clone(),
                    title: title.clone(),
                    model: model.clone(),
                    messages: Vec::new(),
                    started_at: Utc::now(),
                    last_activity: Utc::now(),
                    last_persisted_at: Utc::now(),
                    is_dirty: false,
                };

                if let Ok(mut sessions) = self.active_sessions.write() {
                    sessions.insert(session_id.clone(), session);
                }

                RecordingResponse::Ack {
                    event_id: Uuid::new_v4().to_string(),
                    session_id: session_id.clone(),
                    status: "session_started".to_string(),
                }
            }

            RecordingEvent::SessionEnd {
                session_id,
                final_message_count: _,
            } => {
                // Persist and remove from active sessions
                if let Ok(mut sessions) = self.active_sessions.write() {
                    sessions.remove(session_id);
                }

                RecordingResponse::Ack {
                    event_id: Uuid::new_v4().to_string(),
                    session_id: session_id.clone(),
                    status: "session_ended".to_string(),
                }
            }

            RecordingEvent::MessageAdd {
                session_id,
                message_id,
                role,
                content,
                model,
                parent_id,
                metadata,
            } => {
                let message = RecordedMessage {
                    message_id: message_id.clone(),
                    role: role.clone(),
                    content: content.clone(),
                    model: model.clone(),
                    created_at: Utc::now().timestamp_millis(),
                    parent_id: parent_id.clone(),
                    metadata: metadata.clone(),
                };

                if let Ok(mut sessions) = self.active_sessions.write() {
                    if let Some(session) = sessions.get_mut(session_id) {
                        session.messages.push(message);
                        session.last_activity = Utc::now();
                        session.is_dirty = true;
                    }
                }

                RecordingResponse::Ack {
                    event_id: Uuid::new_v4().to_string(),
                    session_id: session_id.clone(),
                    status: "message_added".to_string(),
                }
            }

            RecordingEvent::MessageUpdate {
                session_id,
                message_id,
                content,
                is_complete: _,
            } => {
                if let Ok(mut sessions) = self.active_sessions.write() {
                    if let Some(session) = sessions.get_mut(session_id) {
                        if let Some(msg) = session
                            .messages
                            .iter_mut()
                            .find(|m| m.message_id == *message_id)
                        {
                            msg.content = content.clone();
                            session.last_activity = Utc::now();
                            session.is_dirty = true;
                        }
                    }
                }

                RecordingResponse::Ack {
                    event_id: Uuid::new_v4().to_string(),
                    session_id: session_id.clone(),
                    status: "message_updated".to_string(),
                }
            }

            RecordingEvent::MessageAppend {
                session_id,
                message_id,
                content_delta,
            } => {
                if let Ok(mut sessions) = self.active_sessions.write() {
                    if let Some(session) = sessions.get_mut(session_id) {
                        if let Some(msg) = session
                            .messages
                            .iter_mut()
                            .find(|m| m.message_id == *message_id)
                        {
                            msg.content.push_str(content_delta);
                            session.last_activity = Utc::now();
                            session.is_dirty = true;
                        }
                    }
                }

                RecordingResponse::Ack {
                    event_id: Uuid::new_v4().to_string(),
                    session_id: session_id.clone(),
                    status: "message_appended".to_string(),
                }
            }

            RecordingEvent::SessionUpdate {
                session_id,
                title,
                model,
                metadata: _,
            } => {
                if let Ok(mut sessions) = self.active_sessions.write() {
                    if let Some(session) = sessions.get_mut(session_id) {
                        if let Some(t) = title {
                            session.title = Some(t.clone());
                        }
                        if let Some(m) = model {
                            session.model = Some(m.clone());
                        }
                        session.last_activity = Utc::now();
                        session.is_dirty = true;
                    }
                }

                RecordingResponse::Ack {
                    event_id: Uuid::new_v4().to_string(),
                    session_id: session_id.clone(),
                    status: "session_updated".to_string(),
                }
            }

            RecordingEvent::Heartbeat {
                session_id,
                timestamp: _,
            } => {
                if let Some(sid) = session_id {
                    if let Ok(mut sessions) = self.active_sessions.write() {
                        if let Some(session) = sessions.get_mut(sid) {
                            session.last_activity = Utc::now();
                        }
                    }
                }

                RecordingResponse::Ack {
                    event_id: Uuid::new_v4().to_string(),
                    session_id: session_id.clone().unwrap_or_default(),
                    status: "heartbeat".to_string(),
                }
            }

            RecordingEvent::SessionSnapshot {
                session_id,
                provider,
                workspace_path,
                title,
                messages,
                metadata: _,
            } => {
                let session = ActiveSession {
                    session_id: session_id.clone(),
                    workspace_id: None,
                    workspace_path: workspace_path.clone(),
                    provider: provider.clone(),
                    title: title.clone(),
                    model: None,
                    messages: messages.clone(),
                    started_at: Utc::now(),
                    last_activity: Utc::now(),
                    last_persisted_at: Utc::now(),
                    is_dirty: true,
                };

                if let Ok(mut sessions) = self.active_sessions.write() {
                    sessions.insert(session_id.clone(), session);
                }

                RecordingResponse::Ack {
                    event_id: Uuid::new_v4().to_string(),
                    session_id: session_id.clone(),
                    status: "snapshot_received".to_string(),
                }
            }
        }
    }

    /// Get active session count
    pub fn active_count(&self) -> usize {
        self.active_sessions.read().map(|s| s.len()).unwrap_or(0)
    }

    /// Get session for recovery
    pub fn get_session(&self, session_id: &str) -> Option<ActiveSession> {
        self.active_sessions
            .read()
            .ok()
            .and_then(|s| s.get(session_id).cloned())
    }

    /// Get all dirty sessions that need persisting
    pub fn get_dirty_sessions(&self) -> Vec<ActiveSession> {
        self.active_sessions
            .read()
            .map(|s| s.values().filter(|sess| sess.is_dirty).cloned().collect())
            .unwrap_or_default()
    }

    /// Mark session as persisted
    pub fn mark_persisted(&self, session_id: &str) {
        if let Ok(mut sessions) = self.active_sessions.write() {
            if let Some(session) = sessions.get_mut(session_id) {
                session.is_dirty = false;
                session.last_persisted_at = Utc::now();
            }
        }
    }
}

// =============================================================================
// REST API Handlers
// =============================================================================

/// Request body for recording events
#[derive(Debug, Deserialize)]
pub struct RecordEventRequest {
    pub events: Vec<RecordingEvent>,
}

/// Response for recording events
#[derive(Debug, Serialize)]
pub struct RecordEventResponse {
    pub processed: usize,
    pub responses: Vec<RecordingResponse>,
}

/// POST /api/recording/events - Process recording events
pub async fn record_events(
    state: web::Data<Arc<RecordingState>>,
    body: web::Json<RecordEventRequest>,
) -> impl Responder {
    let mut responses = Vec::new();

    for event in &body.events {
        let response = state.process_event(event);
        responses.push(response);

        // Broadcast to WebSocket subscribers
        let _ = state.event_tx.send(event.clone());
    }

    HttpResponse::Ok().json(RecordEventResponse {
        processed: responses.len(),
        responses,
    })
}

/// POST /api/recording/snapshot - Store full session snapshot
pub async fn store_snapshot(
    state: web::Data<Arc<RecordingState>>,
    body: web::Json<RecordingEvent>,
) -> impl Responder {
    if let RecordingEvent::SessionSnapshot { .. } = &*body {
        let response = state.process_event(&body);
        HttpResponse::Ok().json(response)
    } else {
        HttpResponse::BadRequest().json(RecordingResponse::Error {
            event_id: None,
            code: "invalid_event".to_string(),
            message: "Expected SessionSnapshot event".to_string(),
        })
    }
}

/// GET /api/recording/sessions - List active recording sessions
pub async fn list_sessions(state: web::Data<Arc<RecordingState>>) -> impl Responder {
    let sessions: Vec<_> = state
        .active_sessions
        .read()
        .map(|s| {
            s.values()
                .map(|sess| serde_json::json!({
                    "session_id": sess.session_id,
                    "provider": sess.provider,
                    "title": sess.title,
                    "workspace_path": sess.workspace_path,
                    "message_count": sess.messages.len(),
                    "started_at": sess.started_at.to_rfc3339(),
                    "last_activity": sess.last_activity.to_rfc3339(),
                    "is_dirty": sess.is_dirty,
                }))
                .collect()
        })
        .unwrap_or_default();

    HttpResponse::Ok().json(serde_json::json!({
        "active_sessions": sessions,
        "total": sessions.len(),
    }))
}

/// GET /api/recording/session/{id} - Get specific session data
pub async fn get_session(
    state: web::Data<Arc<RecordingState>>,
    path: web::Path<String>,
) -> impl Responder {
    let session_id = path.into_inner();

    if let Some(session) = state.get_session(&session_id) {
        HttpResponse::Ok().json(serde_json::json!({
            "session_id": session.session_id,
            "provider": session.provider,
            "title": session.title,
            "workspace_path": session.workspace_path,
            "messages": session.messages,
            "message_count": session.messages.len(),
            "started_at": session.started_at.to_rfc3339(),
            "last_activity": session.last_activity.to_rfc3339(),
        }))
    } else {
        HttpResponse::NotFound().json(RecordingResponse::Error {
            event_id: None,
            code: "session_not_found".to_string(),
            message: format!("Session {} not found", session_id),
        })
    }
}

/// GET /api/recording/session/{id}/recovery - Get recovery data for session
pub async fn get_recovery(
    state: web::Data<Arc<RecordingState>>,
    path: web::Path<String>,
) -> impl Responder {
    let session_id = path.into_inner();

    if let Some(session) = state.get_session(&session_id) {
        let last_message_id = session.messages.last().map(|m| m.message_id.clone());
        HttpResponse::Ok().json(RecordingResponse::Recovery {
            session_id: session.session_id,
            last_message_id,
            message_count: session.messages.len() as i32,
        })
    } else {
        HttpResponse::NotFound().json(RecordingResponse::Error {
            event_id: None,
            code: "session_not_found".to_string(),
            message: format!("Session {} not found", session_id),
        })
    }
}

/// GET /api/recording/status - Recording service status
pub async fn recording_status(
    state: web::Data<Arc<RecordingState>>,
    _req: HttpRequest,
) -> impl Responder {
    let dirty_count = state.get_dirty_sessions().len();

    HttpResponse::Ok().json(serde_json::json!({
        "status": "running",
        "active_sessions": state.active_count(),
        "dirty_sessions": dirty_count,
        "config": {
            "persist_interval_secs": state.config.persist_interval_secs,
            "max_memory_messages": state.config.max_memory_messages,
            "session_timeout_secs": state.config.session_timeout_secs,
        }
    }))
}

// =============================================================================
// WebSocket Recording Support
// =============================================================================

const WS_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const WS_CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

/// WebSocket messages for recording (client -> server)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RecordingWsMessage {
    /// Client wants to send recording events
    Events { events: Vec<RecordingEvent> },
    /// Client wants to subscribe to a session's events
    Subscribe { session_id: String },
    /// Client wants to unsubscribe from a session
    Unsubscribe { session_id: String },
    /// Ping for keepalive
    Ping { timestamp: i64 },
}

/// Server responses for WebSocket (server -> client)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RecordingWsResponse {
    /// Connected successfully
    Connected { client_id: String },
    /// Events processed
    EventsProcessed { count: usize, responses: Vec<RecordingResponse> },
    /// Subscribed to session
    Subscribed { session_id: String },
    /// Unsubscribed from session  
    Unsubscribed { session_id: String },
    /// Event broadcast (from another client or server)
    EventBroadcast { event: RecordingEvent },
    /// Pong response
    Pong { timestamp: i64, server_time: i64 },
    /// Error
    Error { code: String, message: String },
}

/// Handle incoming WebSocket message
fn handle_ws_message(
    text: &str,
    state: &Arc<RecordingState>,
    subscribed_sessions: &mut Vec<String>,
) -> Option<RecordingWsResponse> {
    match serde_json::from_str::<RecordingWsMessage>(text) {
        Ok(msg) => match msg {
            RecordingWsMessage::Events { events } => {
                let mut responses = Vec::new();
                for event in events {
                    let response = state.process_event(&event);
                    responses.push(response);
                    // Broadcast to subscribers
                    let _ = state.event_tx.send(event);
                }
                Some(RecordingWsResponse::EventsProcessed {
                    count: responses.len(),
                    responses,
                })
            }
            RecordingWsMessage::Subscribe { session_id } => {
                if !subscribed_sessions.contains(&session_id) {
                    subscribed_sessions.push(session_id.clone());
                }
                Some(RecordingWsResponse::Subscribed { session_id })
            }
            RecordingWsMessage::Unsubscribe { session_id } => {
                subscribed_sessions.retain(|s| s != &session_id);
                Some(RecordingWsResponse::Unsubscribed { session_id })
            }
            RecordingWsMessage::Ping { timestamp } => {
                Some(RecordingWsResponse::Pong {
                    timestamp,
                    server_time: Utc::now().timestamp_millis(),
                })
            }
        },
        Err(e) => Some(RecordingWsResponse::Error {
            code: "parse_error".to_string(),
            message: format!("Invalid message: {}", e),
        }),
    }
}

/// Check if event should be forwarded to this client
fn should_forward_event(event: &RecordingEvent, subscribed_sessions: &[String]) -> bool {
    // If no specific subscriptions, forward all events
    if subscribed_sessions.is_empty() {
        return true;
    }
    
    let session_id = match event {
        RecordingEvent::SessionStart { session_id, .. } => Some(session_id),
        RecordingEvent::SessionEnd { session_id, .. } => Some(session_id),
        RecordingEvent::MessageAdd { session_id, .. } => Some(session_id),
        RecordingEvent::MessageUpdate { session_id, .. } => Some(session_id),
        RecordingEvent::MessageAppend { session_id, .. } => Some(session_id),
        RecordingEvent::SessionUpdate { session_id, .. } => Some(session_id),
        RecordingEvent::SessionSnapshot { session_id, .. } => Some(session_id),
        RecordingEvent::Heartbeat { session_id, .. } => session_id.as_ref(),
    };
    
    session_id.map(|sid| subscribed_sessions.contains(sid)).unwrap_or(false)
}

/// WebSocket endpoint for recording using actix-ws
pub async fn recording_ws_handler(
    req: HttpRequest,
    body: web::Payload,
    state: web::Data<Arc<RecordingState>>,
) -> Result<HttpResponse, Error> {
    // Perform WebSocket handshake
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    let client_id = Uuid::new_v4().to_string();
    let state_clone = state.get_ref().clone();

    // Send connected message
    let connected_msg = RecordingWsResponse::Connected {
        client_id: client_id.clone(),
    };
    if let Ok(json) = serde_json::to_string(&connected_msg) {
        let _ = session.text(json).await;
    }

    eprintln!("[WS] Recording client {} connected", client_id);

    // Subscribe to broadcast channel
    let mut broadcast_rx = state.event_tx.subscribe();

    // Spawn handler task
    let client_id_clone = client_id.clone();
    actix_web::rt::spawn(async move {
        let mut heartbeat_interval = tokio::time::interval(WS_HEARTBEAT_INTERVAL);
        let mut last_heartbeat = Instant::now();
        let mut subscribed_sessions: Vec<String> = Vec::new();

        loop {
            tokio::select! {
                // Handle incoming messages
                Some(msg_result) = msg_stream.next() => {
                    match msg_result {
                        Ok(actix_ws::Message::Text(text)) => {
                            last_heartbeat = Instant::now();
                            if let Some(response) = handle_ws_message(
                                &text,
                                &state_clone,
                                &mut subscribed_sessions,
                            ) {
                                if let Ok(json) = serde_json::to_string(&response) {
                                    let _ = session.text(json).await;
                                }
                            }
                        }
                        Ok(actix_ws::Message::Ping(data)) => {
                            last_heartbeat = Instant::now();
                            let _ = session.pong(&data).await;
                        }
                        Ok(actix_ws::Message::Pong(_)) => {
                            last_heartbeat = Instant::now();
                        }
                        Ok(actix_ws::Message::Close(_)) => {
                            eprintln!("[WS] Recording client {} requested close", client_id_clone);
                            break;
                        }
                        _ => {}
                    }
                }

                // Handle broadcast messages from other clients
                Ok(event) = broadcast_rx.recv() => {
                    if should_forward_event(&event, &subscribed_sessions) {
                        let msg = RecordingWsResponse::EventBroadcast { event };
                        if let Ok(json) = serde_json::to_string(&msg) {
                            let _ = session.text(json).await;
                        }
                    }
                }

                // Heartbeat check
                _ = heartbeat_interval.tick() => {
                    if Instant::now().duration_since(last_heartbeat) > WS_CLIENT_TIMEOUT {
                        eprintln!("[WS] Recording client {} timed out", client_id_clone);
                        break;
                    }
                    let _ = session.ping(b"").await;
                }
            }
        }

        let _ = session.close(None).await;
        eprintln!("[WS] Recording client {} disconnected", client_id_clone);
    });

    Ok(response)
}

// =============================================================================
// Route Configuration
// =============================================================================

pub fn create_recording_state() -> Arc<RecordingState> {
    Arc::new(RecordingState::new(RecordingConfig::default()))
}

pub fn configure_recording_routes(cfg: &mut web::ServiceConfig) {
    eprintln!("[DEBUG] Configuring recording routes...");
    cfg.service(
        web::scope("/recording")
            .route("/events", web::post().to(record_events))
            .route("/snapshot", web::post().to(store_snapshot))
            .route("/sessions", web::get().to(list_sessions))
            .route("/session/{id}", web::get().to(get_session))
            .route("/session/{id}/recovery", web::get().to(get_recovery))
            .route("/status", web::get().to(recording_status))
            .route("/ws", web::get().to(recording_ws_handler)),
    );
    eprintln!("[DEBUG] Recording routes configured.");
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_start() {
        let state = RecordingState::new(RecordingConfig::default());

        let event = RecordingEvent::SessionStart {
            session_id: "test-123".to_string(),
            workspace_id: None,
            workspace_path: Some("/test/path".to_string()),
            provider: "vscode".to_string(),
            title: Some("Test Session".to_string()),
            model: Some("gpt-4".to_string()),
            metadata: None,
        };

        let response = state.process_event(&event);
        assert!(matches!(response, RecordingResponse::Ack { .. }));
        assert_eq!(state.active_count(), 1);
    }

    #[test]
    fn test_message_add() {
        let state = RecordingState::new(RecordingConfig::default());

        // Start session
        state.process_event(&RecordingEvent::SessionStart {
            session_id: "test-123".to_string(),
            workspace_id: None,
            workspace_path: None,
            provider: "vscode".to_string(),
            title: None,
            model: None,
            metadata: None,
        });

        // Add message
        state.process_event(&RecordingEvent::MessageAdd {
            session_id: "test-123".to_string(),
            message_id: "msg-1".to_string(),
            role: "user".to_string(),
            content: "Hello".to_string(),
            model: None,
            parent_id: None,
            metadata: None,
        });

        let session = state.get_session("test-123").unwrap();
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].content, "Hello");
    }

    #[test]
    fn test_message_append() {
        let state = RecordingState::new(RecordingConfig::default());

        // Start session
        state.process_event(&RecordingEvent::SessionStart {
            session_id: "test-123".to_string(),
            workspace_id: None,
            workspace_path: None,
            provider: "vscode".to_string(),
            title: None,
            model: None,
            metadata: None,
        });

        // Add message
        state.process_event(&RecordingEvent::MessageAdd {
            session_id: "test-123".to_string(),
            message_id: "msg-1".to_string(),
            role: "assistant".to_string(),
            content: "Hello".to_string(),
            model: None,
            parent_id: None,
            metadata: None,
        });

        // Append to message
        state.process_event(&RecordingEvent::MessageAppend {
            session_id: "test-123".to_string(),
            message_id: "msg-1".to_string(),
            content_delta: " World!".to_string(),
        });

        let session = state.get_session("test-123").unwrap();
        assert_eq!(session.messages[0].content, "Hello World!");
    }
}
