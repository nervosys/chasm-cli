// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Webhook integration module
//!
//! Provides webhook dispatch for session and system events.

use actix_web::{web, HttpResponse, Responder};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Webhook event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    /// Session created
    SessionCreated,
    /// Session updated
    SessionUpdated,
    /// Session deleted
    SessionDeleted,
    /// Session archived
    SessionArchived,
    /// Session merged
    SessionMerged,
    /// Session exported
    SessionExported,
    /// Workspace created
    WorkspaceCreated,
    /// Workspace deleted
    WorkspaceDeleted,
    /// Harvest completed
    HarvestCompleted,
    /// Sync completed
    SyncCompleted,
    /// Provider health changed
    ProviderHealthChanged,
}

impl std::fmt::Display for WebhookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebhookEvent::SessionCreated => write!(f, "session.created"),
            WebhookEvent::SessionUpdated => write!(f, "session.updated"),
            WebhookEvent::SessionDeleted => write!(f, "session.deleted"),
            WebhookEvent::SessionArchived => write!(f, "session.archived"),
            WebhookEvent::SessionMerged => write!(f, "session.merged"),
            WebhookEvent::SessionExported => write!(f, "session.exported"),
            WebhookEvent::WorkspaceCreated => write!(f, "workspace.created"),
            WebhookEvent::WorkspaceDeleted => write!(f, "workspace.deleted"),
            WebhookEvent::HarvestCompleted => write!(f, "harvest.completed"),
            WebhookEvent::SyncCompleted => write!(f, "sync.completed"),
            WebhookEvent::ProviderHealthChanged => write!(f, "provider.health_changed"),
        }
    }
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Unique ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Target URL
    pub url: String,
    /// Secret for HMAC signature
    #[serde(skip_serializing)]
    pub secret: Option<String>,
    /// Events to subscribe to
    pub events: Vec<WebhookEvent>,
    /// Whether webhook is enabled
    pub enabled: bool,
    /// Custom headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Retry configuration
    #[serde(default)]
    pub retry_count: u8,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last triggered timestamp
    pub last_triggered: Option<DateTime<Utc>>,
    /// Failure count
    #[serde(default)]
    pub failure_count: u32,
}

/// Webhook payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    /// Event ID
    pub id: String,
    /// Event type
    pub event: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Event data
    pub data: serde_json::Value,
    /// Webhook ID
    pub webhook_id: String,
}

/// Webhook delivery result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    /// Delivery ID
    pub id: String,
    /// Webhook ID
    pub webhook_id: String,
    /// Event type
    pub event: String,
    /// Response status code
    pub status_code: Option<u16>,
    /// Whether delivery succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Delivery timestamp
    pub delivered_at: DateTime<Utc>,
    /// Response time in ms
    pub response_time_ms: u64,
}

/// Webhook state (in-memory for now)
pub struct WebhookState {
    /// Registered webhooks
    webhooks: RwLock<HashMap<String, WebhookConfig>>,
    /// Recent deliveries (circular buffer)
    deliveries: RwLock<Vec<WebhookDelivery>>,
    /// HTTP client
    client: reqwest::Client,
}

impl WebhookState {
    /// Create new webhook state
    pub fn new() -> Self {
        Self {
            webhooks: RwLock::new(HashMap::new()),
            deliveries: RwLock::new(Vec::new()),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Register a webhook
    pub fn register(&self, config: WebhookConfig) {
        let mut webhooks = self.webhooks.write().unwrap();
        webhooks.insert(config.id.clone(), config);
    }

    /// Remove a webhook
    pub fn remove(&self, id: &str) -> bool {
        let mut webhooks = self.webhooks.write().unwrap();
        webhooks.remove(id).is_some()
    }

    /// Get a webhook by ID
    pub fn get(&self, id: &str) -> Option<WebhookConfig> {
        let webhooks = self.webhooks.read().unwrap();
        webhooks.get(id).cloned()
    }

    /// List all webhooks
    pub fn list(&self) -> Vec<WebhookConfig> {
        let webhooks = self.webhooks.read().unwrap();
        webhooks.values().cloned().collect()
    }

    /// Get webhooks subscribed to an event
    pub fn get_for_event(&self, event: &WebhookEvent) -> Vec<WebhookConfig> {
        let webhooks = self.webhooks.read().unwrap();
        webhooks
            .values()
            .filter(|w| w.enabled && w.events.contains(event))
            .cloned()
            .collect()
    }

    /// Dispatch an event to all subscribed webhooks
    pub async fn dispatch(&self, event: WebhookEvent, data: serde_json::Value) {
        let webhooks = self.get_for_event(&event);

        for webhook in webhooks {
            let payload = WebhookPayload {
                id: Uuid::new_v4().to_string(),
                event: event.to_string(),
                timestamp: Utc::now(),
                data: data.clone(),
                webhook_id: webhook.id.clone(),
            };

            let delivery = self.deliver(&webhook, &payload).await;

            // Store delivery result
            let mut deliveries = self.deliveries.write().unwrap();
            deliveries.push(delivery);

            // Keep only last 1000 deliveries
            if deliveries.len() > 1000 {
                let drain_count = deliveries.len() - 1000;
                deliveries.drain(0..drain_count);
            }
        }
    }

    /// Deliver a webhook
    async fn deliver(&self, webhook: &WebhookConfig, payload: &WebhookPayload) -> WebhookDelivery {
        let start = std::time::Instant::now();
        let delivery_id = Uuid::new_v4().to_string();

        let mut request = self
            .client
            .post(&webhook.url)
            .header("Content-Type", "application/json")
            .header("X-Chasm-Webhook-Id", &webhook.id)
            .header("X-Chasm-Event", &payload.event)
            .header("X-Chasm-Delivery-Id", &delivery_id);

        // Add signature if secret is configured
        if let Some(ref secret) = webhook.secret {
            if let Ok(body) = serde_json::to_string(payload) {
                let signature = compute_signature(secret, &body);
                request = request.header("X-Chasm-Signature", format!("sha256={}", signature));
            }
        }

        // Add custom headers
        for (key, value) in &webhook.headers {
            request = request.header(key, value);
        }

        let result = request.json(payload).send().await;

        let elapsed = start.elapsed();

        match result {
            Ok(response) => {
                let status = response.status().as_u16();
                WebhookDelivery {
                    id: delivery_id,
                    webhook_id: webhook.id.clone(),
                    event: payload.event.clone(),
                    status_code: Some(status),
                    success: response.status().is_success(),
                    error: if response.status().is_success() {
                        None
                    } else {
                        Some(format!("HTTP {}", status))
                    },
                    delivered_at: Utc::now(),
                    response_time_ms: elapsed.as_millis() as u64,
                }
            }
            Err(e) => WebhookDelivery {
                id: delivery_id,
                webhook_id: webhook.id.clone(),
                event: payload.event.clone(),
                status_code: None,
                success: false,
                error: Some(e.to_string()),
                delivered_at: Utc::now(),
                response_time_ms: elapsed.as_millis() as u64,
            },
        }
    }

    /// Get recent deliveries
    pub fn get_deliveries(&self, limit: usize) -> Vec<WebhookDelivery> {
        let deliveries = self.deliveries.read().unwrap();
        deliveries.iter().rev().take(limit).cloned().collect()
    }

    /// Get deliveries for a specific webhook
    pub fn get_webhook_deliveries(&self, webhook_id: &str, limit: usize) -> Vec<WebhookDelivery> {
        let deliveries = self.deliveries.read().unwrap();
        deliveries
            .iter()
            .rev()
            .filter(|d| d.webhook_id == webhook_id)
            .take(limit)
            .cloned()
            .collect()
    }
}

impl Default for WebhookState {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute HMAC-SHA256 signature
fn compute_signature(secret: &str, body: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(body.as_bytes());

    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

/// Hex encoding helper
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

// ============================================================================
// API Handlers
// ============================================================================

/// Request to create a webhook
#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    pub events: Vec<WebhookEvent>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default = "default_retry_count")]
    pub retry_count: u8,
}

fn default_retry_count() -> u8 {
    3
}

/// Request to update a webhook
#[derive(Debug, Deserialize)]
pub struct UpdateWebhookRequest {
    pub name: Option<String>,
    pub url: Option<String>,
    pub secret: Option<String>,
    pub events: Option<Vec<WebhookEvent>>,
    pub enabled: Option<bool>,
    pub headers: Option<HashMap<String, String>>,
    pub retry_count: Option<u8>,
}

/// Test webhook request
#[derive(Debug, Deserialize)]
pub struct TestWebhookRequest {
    pub event: Option<WebhookEvent>,
}

/// List webhooks
pub async fn list_webhooks(state: web::Data<Arc<WebhookState>>) -> impl Responder {
    let webhooks = state.list();
    HttpResponse::Ok().json(webhooks)
}

/// Create webhook
pub async fn create_webhook(
    state: web::Data<Arc<WebhookState>>,
    body: web::Json<CreateWebhookRequest>,
) -> impl Responder {
    let config = WebhookConfig {
        id: Uuid::new_v4().to_string(),
        name: body.name.clone(),
        url: body.url.clone(),
        secret: body.secret.clone(),
        events: body.events.clone(),
        enabled: true,
        headers: body.headers.clone(),
        retry_count: body.retry_count,
        created_at: Utc::now(),
        last_triggered: None,
        failure_count: 0,
    };

    state.register(config.clone());
    HttpResponse::Created().json(config)
}

/// Get webhook by ID
pub async fn get_webhook(
    state: web::Data<Arc<WebhookState>>,
    path: web::Path<String>,
) -> impl Responder {
    let id = path.into_inner();
    match state.get(&id) {
        Some(webhook) => HttpResponse::Ok().json(webhook),
        None => HttpResponse::NotFound().json(serde_json::json!({
            "error": "not_found",
            "message": "Webhook not found"
        })),
    }
}

/// Update webhook
pub async fn update_webhook(
    state: web::Data<Arc<WebhookState>>,
    path: web::Path<String>,
    body: web::Json<UpdateWebhookRequest>,
) -> impl Responder {
    let id = path.into_inner();

    if let Some(mut webhook) = state.get(&id) {
        if let Some(name) = &body.name {
            webhook.name = name.clone();
        }
        if let Some(url) = &body.url {
            webhook.url = url.clone();
        }
        if let Some(secret) = &body.secret {
            webhook.secret = Some(secret.clone());
        }
        if let Some(events) = &body.events {
            webhook.events = events.clone();
        }
        if let Some(enabled) = body.enabled {
            webhook.enabled = enabled;
        }
        if let Some(headers) = &body.headers {
            webhook.headers = headers.clone();
        }
        if let Some(retry_count) = body.retry_count {
            webhook.retry_count = retry_count;
        }

        state.register(webhook.clone());
        HttpResponse::Ok().json(webhook)
    } else {
        HttpResponse::NotFound().json(serde_json::json!({
            "error": "not_found",
            "message": "Webhook not found"
        }))
    }
}

/// Delete webhook
pub async fn delete_webhook(
    state: web::Data<Arc<WebhookState>>,
    path: web::Path<String>,
) -> impl Responder {
    let id = path.into_inner();
    if state.remove(&id) {
        HttpResponse::NoContent().finish()
    } else {
        HttpResponse::NotFound().json(serde_json::json!({
            "error": "not_found",
            "message": "Webhook not found"
        }))
    }
}

/// Test webhook
pub async fn test_webhook(
    state: web::Data<Arc<WebhookState>>,
    path: web::Path<String>,
    body: web::Json<TestWebhookRequest>,
) -> impl Responder {
    let id = path.into_inner();

    if let Some(webhook) = state.get(&id) {
        let event = body.event.clone().unwrap_or(WebhookEvent::SessionCreated);
        let test_data = serde_json::json!({
            "test": true,
            "message": "This is a test webhook delivery from Chasm"
        });

        let payload = WebhookPayload {
            id: Uuid::new_v4().to_string(),
            event: event.to_string(),
            timestamp: Utc::now(),
            data: test_data,
            webhook_id: webhook.id.clone(),
        };

        let delivery = state.deliver(&webhook, &payload).await;
        HttpResponse::Ok().json(delivery)
    } else {
        HttpResponse::NotFound().json(serde_json::json!({
            "error": "not_found",
            "message": "Webhook not found"
        }))
    }
}

/// Get webhook deliveries
pub async fn get_deliveries(
    state: web::Data<Arc<WebhookState>>,
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let id = path.into_inner();
    let limit: usize = query
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);

    let deliveries = state.get_webhook_deliveries(&id, limit);
    HttpResponse::Ok().json(deliveries)
}

/// Get all recent deliveries
pub async fn get_all_deliveries(
    state: web::Data<Arc<WebhookState>>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let limit: usize = query
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);

    let deliveries = state.get_deliveries(limit);
    HttpResponse::Ok().json(deliveries)
}

/// Configure webhook routes
pub fn configure_webhook_routes(cfg: &mut web::ServiceConfig, state: web::Data<Arc<WebhookState>>) {
    cfg.app_data(state).service(
        web::scope("/webhooks")
            .route("", web::get().to(list_webhooks))
            .route("", web::post().to(create_webhook))
            .route("/deliveries", web::get().to(get_all_deliveries))
            .route("/{id}", web::get().to(get_webhook))
            .route("/{id}", web::put().to(update_webhook))
            .route("/{id}", web::delete().to(delete_webhook))
            .route("/{id}/test", web::post().to(test_webhook))
            .route("/{id}/deliveries", web::get().to(get_deliveries)),
    );
}

/// Create webhook state
pub fn create_webhook_state() -> Arc<WebhookState> {
    Arc::new(WebhookState::new())
}
