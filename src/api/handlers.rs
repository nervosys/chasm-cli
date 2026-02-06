// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! API request and response handlers

use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::state::AppState;
use crate::database::{self, Checkpoint, Message, Session, ShareLink, Workspace};
use crate::workspace::discover_workspaces as discover_vscode_workspaces;

// =============================================================================
// Response Types
// =============================================================================

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<ApiError>,
}

#[derive(Debug, Serialize)]
struct ApiError {
    code: String,
    message: String,
}

impl<T: Serialize> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(code: &str, message: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
            }),
        }
    }
}

#[derive(Debug, Serialize)]
struct PaginatedResponse<T> {
    items: Vec<T>,
    total: usize,
    limit: usize,
    offset: usize,
    has_more: bool,
}

// =============================================================================
// Query Parameters
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct WorkspaceQuery {
    provider: Option<String>,
    search: Option<String>,
    sort_by: Option<String>,
    sort_order: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SessionQuery {
    workspace_id: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    archived: Option<bool>,
    date_from: Option<i64>,
    date_to: Option<i64>,
    search: Option<String>,
    sort_by: Option<String>,
    sort_order: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
    include: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    limit: Option<usize>,
    before: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    q: String,
    types: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct TimelineQuery {
    days: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct PathQuery {
    path: String,
}

// =============================================================================
// Request Bodies
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceRequest {
    name: String,
    path: Option<String>,
    provider: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkspaceRequest {
    name: Option<String>,
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LinkWorkspaceRequest {
    project_path: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    workspace_id: Option<String>,
    title: String,
    provider: String,
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSessionRequest {
    title: Option<String>,
    model: Option<String>,
    archived: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ArchiveSessionRequest {
    archived: bool,
}

#[derive(Debug, Deserialize)]
pub struct ForkSessionRequest {
    from_message_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MergeSessionsRequest {
    session_ids: Vec<String>,
    title: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateMessageRequest {
    role: String,
    content: String,
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMessageRequest {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCheckpointRequest {
    name: String,
    description: Option<String>,
    message_id: Option<String>,
    git_commit: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateShareLinkRequest {
    provider: String,
    expires_in: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionRequest {
    provider: String,
    model: String,
    messages: Vec<ChatCompletionMessage>,
    temperature: Option<f32>,
    max_tokens: Option<i32>,
    stream: Option<bool>,
    session_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatCompletionMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    #[serde(rename = "type")]
    import_type: String,
    uri: String,
    provider: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    format: String,
    include_metadata: Option<bool>,
    session_ids: Option<Vec<String>>,
    workspace_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HarvestRequest {
    providers: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    direction: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    theme: Option<String>,
    syntax_theme: Option<String>,
    font_size: Option<i32>,
    default_provider: Option<String>,
    default_model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddAccountRequest {
    provider: String,
    credentials: serde_json::Value,
}

// =============================================================================
// Provider Types (simplified for API)
// =============================================================================

#[derive(Debug, Serialize)]
pub struct Provider {
    id: String,
    name: String,
    #[serde(rename = "type")]
    provider_type: String,
    icon: String,
    color: String,
    endpoint: Option<String>,
    models: Vec<String>,
    status: String,
}

#[derive(Debug, Serialize)]
pub struct ProviderHealth {
    provider_id: String,
    status: String,
    latency: Option<i64>,
    last_checked: i64,
    error: Option<String>,
    version: Option<String>,
    models: Vec<String>,
}

// =============================================================================
// Handler Implementations
// =============================================================================

// Health & System

pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime": 0 // Would track actual uptime in production
    })))
}

pub async fn system_info(state: web::Data<AppState>) -> impl Responder {
    let db = state.db.read().await;

    match db.get_statistics() {
        Ok(stats) => HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "platform": std::env::consts::OS,
            "databaseSize": 0, // Would get actual size
            "sessionCount": stats.session_count,
            "providerCount": 0 // Would count configured providers
        }))),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn vacuum_database(state: web::Data<AppState>) -> impl Responder {
    let db = state.db.read().await;

    match db.connection().execute_batch("VACUUM") {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "before": 0,
            "after": 0
        }))),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("VACUUM_ERROR", &e.to_string())),
    }
}

pub async fn clear_cache() -> impl Responder {
    // Cache clearing logic would go here
    HttpResponse::Ok().json(ApiResponse::success(()))
}

// Workspaces

pub async fn list_workspaces(
    state: web::Data<AppState>,
    query: web::Query<WorkspaceQuery>,
) -> impl Responder {
    let db = state.db.read().await;

    match db.list_workspaces() {
        Ok(workspaces) => {
            let limit = query.limit.unwrap_or(50);
            let offset = query.offset.unwrap_or(0);
            let total = workspaces.len();
            let items: Vec<_> = workspaces.into_iter().skip(offset).take(limit).collect();
            let has_more = offset + items.len() < total;

            HttpResponse::Ok().json(ApiResponse::success(PaginatedResponse {
                items,
                total,
                limit,
                offset,
                has_more,
            }))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn get_workspace(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = state.db.read().await;
    let id = path.into_inner();

    match db.get_workspace(&id) {
        Ok(Some(workspace)) => HttpResponse::Ok().json(ApiResponse::success(workspace)),
        Ok(None) => HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("NOT_FOUND", "Workspace not found")),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn get_workspace_by_path(
    state: web::Data<AppState>,
    query: web::Query<PathQuery>,
) -> impl Responder {
    let db = state.db.read().await;

    match db.list_workspaces() {
        Ok(workspaces) => {
            let workspace = workspaces
                .into_iter()
                .find(|w| w.path.as_ref() == Some(&query.path));

            match workspace {
                Some(w) => HttpResponse::Ok().json(ApiResponse::success(w)),
                None => HttpResponse::NotFound()
                    .json(ApiResponse::<()>::error("NOT_FOUND", "Workspace not found")),
            }
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn create_workspace(
    state: web::Data<AppState>,
    body: web::Json<CreateWorkspaceRequest>,
) -> impl Responder {
    let db = state.db.read().await;
    let now = Utc::now().timestamp();

    let workspace = Workspace {
        id: Uuid::new_v4().to_string(),
        name: body.name.clone(),
        path: body.path.clone(),
        provider: body.provider.clone(),
        provider_workspace_id: None,
        created_at: now,
        updated_at: now,
        metadata: None,
    };

    match db.upsert_workspace(&workspace) {
        Ok(_) => HttpResponse::Created().json(ApiResponse::success(workspace)),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn update_workspace(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<UpdateWorkspaceRequest>,
) -> impl Responder {
    let db = state.db.read().await;
    let id = path.into_inner();

    match db.get_workspace(&id) {
        Ok(Some(mut workspace)) => {
            if let Some(name) = &body.name {
                workspace.name = name.clone();
            }
            if let Some(path) = &body.path {
                workspace.path = Some(path.clone());
            }
            workspace.updated_at = Utc::now().timestamp();

            match db.upsert_workspace(&workspace) {
                Ok(_) => HttpResponse::Ok().json(ApiResponse::success(workspace)),
                Err(e) => HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
            }
        }
        Ok(None) => HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("NOT_FOUND", "Workspace not found")),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn delete_workspace(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let db = state.db.read().await;
    let id = path.into_inner();

    match db
        .connection()
        .execute("DELETE FROM workspaces WHERE id = ?", [&id])
    {
        Ok(0) => HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("NOT_FOUND", "Workspace not found")),
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(())),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn discover_workspaces(state: web::Data<AppState>) -> impl Responder {
    match discover_vscode_workspaces() {
        Ok(vscode_workspaces) => {
            let db = state.db.read().await;
            let now = Utc::now().timestamp();
            let mut imported = Vec::new();

            for ws in vscode_workspaces {
                let workspace = Workspace {
                    id: ws.hash.clone(),
                    name: ws
                        .project_path
                        .as_ref()
                        .and_then(|p| std::path::Path::new(p).file_name())
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| ws.hash.clone()),
                    path: ws.project_path.clone(),
                    provider: "vscode".to_string(),
                    provider_workspace_id: Some(ws.hash.clone()),
                    created_at: now,
                    updated_at: now,
                    metadata: None,
                };

                if db.upsert_workspace(&workspace).is_ok() {
                    imported.push(workspace);
                }
            }

            HttpResponse::Ok().json(ApiResponse::success(imported))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DISCOVERY_ERROR", &e.to_string())),
    }
}

pub async fn refresh_workspace(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let db = state.db.read().await;
    let id = path.into_inner();

    match db.get_workspace(&id) {
        Ok(Some(mut workspace)) => {
            workspace.updated_at = Utc::now().timestamp();
            match db.upsert_workspace(&workspace) {
                Ok(_) => HttpResponse::Ok().json(ApiResponse::success(workspace)),
                Err(e) => HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
            }
        }
        Ok(None) => HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("NOT_FOUND", "Workspace not found")),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn link_workspace(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<LinkWorkspaceRequest>,
) -> impl Responder {
    let db = state.db.read().await;
    let id = path.into_inner();

    match db.get_workspace(&id) {
        Ok(Some(mut workspace)) => {
            workspace.path = Some(body.project_path.clone());
            workspace.updated_at = Utc::now().timestamp();
            match db.upsert_workspace(&workspace) {
                Ok(_) => HttpResponse::Ok().json(ApiResponse::success(workspace)),
                Err(e) => HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
            }
        }
        Ok(None) => HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("NOT_FOUND", "Workspace not found")),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn get_workspace_git(path: web::Path<String>) -> impl Responder {
    let _id = path.into_inner();
    // Git info would be extracted from workspace path
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "path": "",
        "branch": "main",
        "remote": null,
        "uncommittedChanges": 0,
        "ahead": 0,
        "behind": 0
    })))
}

// Sessions

pub async fn list_sessions(
    state: web::Data<AppState>,
    query: web::Query<SessionQuery>,
) -> impl Responder {
    let db = state.db.read().await;

    match db.list_sessions(query.workspace_id.as_deref()) {
        Ok(sessions) => {
            let limit = query.limit.unwrap_or(50);
            let offset = query.offset.unwrap_or(0);
            let total = sessions.len();
            let items: Vec<_> = sessions.into_iter().skip(offset).take(limit).collect();
            let has_more = offset + items.len() < total;

            HttpResponse::Ok().json(ApiResponse::success(PaginatedResponse {
                items,
                total,
                limit,
                offset,
                has_more,
            }))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn get_session(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<SessionQuery>,
) -> impl Responder {
    let db = state.db.read().await;
    let id = path.into_inner();

    match db.get_session(&id) {
        Ok(Some(session)) => {
            if query.include.as_deref() == Some("messages") {
                match db.list_messages(&id) {
                    Ok(messages) => {
                        let mut response = serde_json::to_value(&session).unwrap();
                        response["messages"] = serde_json::to_value(&messages).unwrap();
                        HttpResponse::Ok().json(ApiResponse::success(response))
                    }
                    Err(e) => HttpResponse::InternalServerError()
                        .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
                }
            } else {
                HttpResponse::Ok().json(ApiResponse::success(session))
            }
        }
        Ok(None) => HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("NOT_FOUND", "Session not found")),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn create_session(
    state: web::Data<AppState>,
    body: web::Json<CreateSessionRequest>,
) -> impl Responder {
    let db = state.db.read().await;
    let now = Utc::now().timestamp();

    let session = Session {
        id: Uuid::new_v4().to_string(),
        workspace_id: body.workspace_id.clone(),
        provider: body.provider.clone(),
        provider_session_id: None,
        title: body.title.clone(),
        model: body.model.clone(),
        message_count: 0,
        token_count: None,
        created_at: now,
        updated_at: now,
        archived: false,
        metadata: None,
    };

    match db.upsert_session(&session) {
        Ok(_) => HttpResponse::Created().json(ApiResponse::success(session)),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn update_session(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<UpdateSessionRequest>,
) -> impl Responder {
    let db = state.db.read().await;
    let id = path.into_inner();

    match db.get_session(&id) {
        Ok(Some(mut session)) => {
            if let Some(title) = &body.title {
                session.title = title.clone();
            }
            if let Some(model) = &body.model {
                session.model = Some(model.clone());
            }
            if let Some(archived) = body.archived {
                session.archived = archived;
            }
            session.updated_at = Utc::now().timestamp();

            match db.upsert_session(&session) {
                Ok(_) => HttpResponse::Ok().json(ApiResponse::success(session)),
                Err(e) => HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
            }
        }
        Ok(None) => HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("NOT_FOUND", "Session not found")),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn delete_session(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = state.db.read().await;
    let id = path.into_inner();

    match db
        .connection()
        .execute("DELETE FROM sessions WHERE id = ?", [&id])
    {
        Ok(0) => HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("NOT_FOUND", "Session not found")),
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(())),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn archive_session(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<ArchiveSessionRequest>,
) -> impl Responder {
    let db = state.db.read().await;
    let id = path.into_inner();

    match db.get_session(&id) {
        Ok(Some(mut session)) => {
            session.archived = body.archived;
            session.updated_at = Utc::now().timestamp();

            match db.upsert_session(&session) {
                Ok(_) => HttpResponse::Ok().json(ApiResponse::success(session)),
                Err(e) => HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
            }
        }
        Ok(None) => HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("NOT_FOUND", "Session not found")),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn fork_session(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<ForkSessionRequest>,
) -> impl Responder {
    let db = state.db.read().await;
    let id = path.into_inner();

    match db.get_session(&id) {
        Ok(Some(original)) => {
            let now = Utc::now().timestamp();
            let forked = Session {
                id: Uuid::new_v4().to_string(),
                workspace_id: original.workspace_id.clone(),
                provider: original.provider.clone(),
                provider_session_id: None,
                title: format!("{} (fork)", original.title),
                model: original.model.clone(),
                message_count: 0,
                token_count: None,
                created_at: now,
                updated_at: now,
                archived: false,
                metadata: None,
            };

            // Copy messages up to from_message_id if specified
            if let Ok(messages) = db.list_messages(&id) {
                let messages_to_copy: Vec<_> = if let Some(ref msg_id) = body.from_message_id {
                    messages
                        .into_iter()
                        .take_while(|m| &m.id != msg_id)
                        .collect()
                } else {
                    messages
                };

                for msg in messages_to_copy {
                    let new_msg = Message {
                        id: Uuid::new_v4().to_string(),
                        session_id: forked.id.clone(),
                        role: msg.role,
                        content: msg.content,
                        model: msg.model,
                        token_count: msg.token_count,
                        created_at: msg.created_at,
                        parent_id: None,
                        metadata: msg.metadata,
                    };
                    let _ = db.upsert_message(&new_msg);
                }
            }

            match db.upsert_session(&forked) {
                Ok(_) => HttpResponse::Created().json(ApiResponse::success(forked)),
                Err(e) => HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
            }
        }
        Ok(None) => HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("NOT_FOUND", "Session not found")),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn merge_sessions(
    state: web::Data<AppState>,
    body: web::Json<MergeSessionsRequest>,
) -> impl Responder {
    let db = state.db.read().await;
    let now = Utc::now().timestamp();

    let merged = Session {
        id: Uuid::new_v4().to_string(),
        workspace_id: None,
        provider: "merged".to_string(),
        provider_session_id: None,
        title: body.title.clone(),
        model: None,
        message_count: 0,
        token_count: None,
        created_at: now,
        updated_at: now,
        archived: false,
        metadata: Some(
            serde_json::json!({
                "merged_from": body.session_ids
            })
            .to_string(),
        ),
    };

    // Collect and merge messages from all sessions
    let mut all_messages: Vec<Message> = Vec::new();
    for session_id in &body.session_ids {
        if let Ok(messages) = db.list_messages(session_id) {
            all_messages.extend(messages);
        }
    }

    // Sort by timestamp
    all_messages.sort_by_key(|m| m.created_at);

    // Insert merged session
    if let Err(e) = db.upsert_session(&merged) {
        return HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string()));
    }

    // Insert messages with new IDs
    for msg in all_messages {
        let new_msg = Message {
            id: Uuid::new_v4().to_string(),
            session_id: merged.id.clone(),
            ..msg
        };
        let _ = db.upsert_message(&new_msg);
    }

    HttpResponse::Created().json(ApiResponse::success(merged))
}

pub async fn export_session(path: web::Path<String>) -> impl Responder {
    let _id = path.into_inner();
    // Export would create a JSON/Markdown blob
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Export not yet implemented",
    ))
}

// Messages

pub async fn list_messages(
    state: web::Data<AppState>,
    path: web::Path<String>,
    _query: web::Query<MessageQuery>,
) -> impl Responder {
    let db = state.db.read().await;
    let session_id = path.into_inner();

    match db.list_messages(&session_id) {
        Ok(messages) => HttpResponse::Ok().json(ApiResponse::success(messages)),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn get_message(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let db = state.db.read().await;
    let (_session_id, message_id) = path.into_inner();

    match db.get_message(&message_id) {
        Ok(Some(message)) => HttpResponse::Ok().json(ApiResponse::success(message)),
        Ok(None) => HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("NOT_FOUND", "Message not found")),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn create_message(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<CreateMessageRequest>,
) -> impl Responder {
    let db = state.db.read().await;
    let session_id = path.into_inner();
    let now = Utc::now().timestamp();

    let message = Message {
        id: Uuid::new_v4().to_string(),
        session_id,
        role: body.role.clone(),
        content: body.content.clone(),
        model: body.model.clone(),
        token_count: None,
        created_at: now,
        parent_id: None,
        metadata: None,
    };

    match db.upsert_message(&message) {
        Ok(_) => HttpResponse::Created().json(ApiResponse::success(message)),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn update_message(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    body: web::Json<UpdateMessageRequest>,
) -> impl Responder {
    let db = state.db.read().await;
    let (_session_id, message_id) = path.into_inner();

    match db.get_message(&message_id) {
        Ok(Some(mut message)) => {
            if let Some(content) = &body.content {
                message.content = content.clone();
            }

            match db.upsert_message(&message) {
                Ok(_) => HttpResponse::Ok().json(ApiResponse::success(message)),
                Err(e) => HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
            }
        }
        Ok(None) => HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("NOT_FOUND", "Message not found")),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn delete_message(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let db = state.db.read().await;
    let (_session_id, message_id) = path.into_inner();

    match db
        .connection()
        .execute("DELETE FROM messages WHERE id = ?", [&message_id])
    {
        Ok(0) => HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("NOT_FOUND", "Message not found")),
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(())),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn regenerate_message(path: web::Path<(String, String)>) -> impl Responder {
    let (_session_id, _message_id) = path.into_inner();
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Message regeneration not yet implemented",
    ))
}

// Checkpoints

pub async fn list_checkpoints(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let db = state.db.read().await;
    let session_id = path.into_inner();

    match db.list_checkpoints(&session_id) {
        Ok(checkpoints) => HttpResponse::Ok().json(ApiResponse::success(checkpoints)),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn create_checkpoint(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<CreateCheckpointRequest>,
) -> impl Responder {
    let db = state.db.read().await;
    let session_id = path.into_inner();
    let now = Utc::now().timestamp();

    // Get session to create snapshot
    let session = match db.get_session(&session_id) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ApiResponse::<()>::error("NOT_FOUND", "Session not found"));
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string()));
        }
    };

    let checkpoint = Checkpoint {
        id: Uuid::new_v4().to_string(),
        session_id,
        name: body.name.clone(),
        description: body.description.clone(),
        message_count: session.message_count,
        session_snapshot: serde_json::to_string(&session).unwrap_or_default(),
        created_at: now,
        git_commit: body.git_commit.clone(),
    };

    match db.upsert_checkpoint(&checkpoint) {
        Ok(_) => HttpResponse::Created().json(ApiResponse::success(checkpoint)),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

// Share Links

pub async fn list_share_links(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let db = state.db.read().await;
    let session_id = path.into_inner();

    match db.list_share_links(&session_id) {
        Ok(links) => HttpResponse::Ok().json(ApiResponse::success(links)),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn create_share_link(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<CreateShareLinkRequest>,
) -> impl Responder {
    let db = state.db.read().await;
    let session_id = path.into_inner();
    let now = Utc::now().timestamp();

    let provider = match body.provider.as_str() {
        "github_gist" => database::ShareLinkProvider::GithubGist,
        "pastebin" => database::ShareLinkProvider::Pastebin,
        "hastebin" => database::ShareLinkProvider::Hastebin,
        _ => database::ShareLinkProvider::Custom,
    };

    let link = ShareLink {
        id: Uuid::new_v4().to_string(),
        session_id,
        provider,
        url: format!("https://example.com/share/{}", Uuid::new_v4()),
        expires_at: body.expires_in.map(|secs| now + secs),
        created_at: now,
        imported: false,
    };

    match db.upsert_share_link(&link) {
        Ok(_) => HttpResponse::Created().json(ApiResponse::success(link)),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn list_session_commits(path: web::Path<String>) -> impl Responder {
    let _session_id = path.into_inner();
    // Would fetch git commits linked to session
    HttpResponse::Ok().json(ApiResponse::success(Vec::<serde_json::Value>::new()))
}

// Providers (stub implementations)

pub async fn list_providers() -> impl Responder {
    let providers = vec![
        // ===========================================
        // Cloud Providers
        // ===========================================
        Provider {
            id: "copilot".to_string(),
            name: "GitHub Copilot".to_string(),
            provider_type: "cloud".to_string(),
            icon: "🤖".to_string(),
            color: "#000000".to_string(),
            endpoint: None,
            models: vec![
                "gpt-4.1".to_string(),
                "gpt-4.1-mini".to_string(),
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "o1".to_string(),
                "o1-mini".to_string(),
                "o1-preview".to_string(),
                "o3".to_string(),
                "o3-mini".to_string(),
                "o4-mini".to_string(),
                "claude-sonnet-4".to_string(),
                "claude-3.5-sonnet".to_string(),
                "gemini-2.0-flash".to_string(),
                "gemini-2.5-pro".to_string(),
            ],
            status: "connected".to_string(),
        },
        Provider {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            provider_type: "cloud".to_string(),
            icon: "🧠".to_string(),
            color: "#10a37f".to_string(),
            endpoint: Some("https://api.openai.com/v1".to_string()),
            models: vec![
                "gpt-4.1".to_string(),
                "gpt-4.1-mini".to_string(),
                "gpt-4.1-nano".to_string(),
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "gpt-4o-audio-preview".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-4".to_string(),
                "gpt-3.5-turbo".to_string(),
                "o1".to_string(),
                "o1-mini".to_string(),
                "o1-preview".to_string(),
                "o3".to_string(),
                "o3-mini".to_string(),
                "o4-mini".to_string(),
                "chatgpt-4o-latest".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            provider_type: "cloud".to_string(),
            icon: "🎭".to_string(),
            color: "#d4a574".to_string(),
            endpoint: Some("https://api.anthropic.com/v1".to_string()),
            models: vec![
                "claude-opus-4".to_string(),
                "claude-sonnet-4".to_string(),
                "claude-3.5-sonnet".to_string(),
                "claude-3.5-haiku".to_string(),
                "claude-3-opus".to_string(),
                "claude-3-sonnet".to_string(),
                "claude-3-haiku".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "google".to_string(),
            name: "Google AI".to_string(),
            provider_type: "cloud".to_string(),
            icon: "✨".to_string(),
            color: "#4285f4".to_string(),
            endpoint: Some("https://generativelanguage.googleapis.com/v1beta".to_string()),
            models: vec![
                "gemini-2.5-pro".to_string(),
                "gemini-2.5-flash".to_string(),
                "gemini-2.0-flash".to_string(),
                "gemini-2.0-flash-thinking".to_string(),
                "gemini-1.5-pro".to_string(),
                "gemini-1.5-flash".to_string(),
                "gemini-1.5-flash-8b".to_string(),
                "gemini-pro".to_string(),
                "gemini-pro-vision".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "azure-openai".to_string(),
            name: "Azure OpenAI".to_string(),
            provider_type: "cloud".to_string(),
            icon: "☁️".to_string(),
            color: "#0078d4".to_string(),
            endpoint: None,
            models: vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-4".to_string(),
                "gpt-35-turbo".to_string(),
                "o1".to_string(),
                "o1-mini".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "ai-foundry".to_string(),
            name: "Azure AI Foundry".to_string(),
            provider_type: "cloud".to_string(),
            icon: "🏭".to_string(),
            color: "#0078d4".to_string(),
            endpoint: None,
            models: vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "o1".to_string(),
                "o1-mini".to_string(),
                "Phi-4".to_string(),
                "Phi-3.5-MoE-instruct".to_string(),
                "Phi-3.5-mini-instruct".to_string(),
                "Phi-3.5-vision-instruct".to_string(),
                "Llama-3.3-70B-Instruct".to_string(),
                "Llama-3.2-90B-Vision-Instruct".to_string(),
                "Llama-3.1-405B-Instruct".to_string(),
                "Mistral-large-2411".to_string(),
                "Mistral-small".to_string(),
                "Codestral-2501".to_string(),
                "DeepSeek-R1".to_string(),
                "DeepSeek-V3".to_string(),
                "Cohere-command-r-plus".to_string(),
                "JAIS-30b-chat".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "github-models".to_string(),
            name: "GitHub Models".to_string(),
            provider_type: "cloud".to_string(),
            icon: "🐙".to_string(),
            color: "#24292e".to_string(),
            endpoint: Some("https://models.inference.ai.azure.com".to_string()),
            models: vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "o1".to_string(),
                "o1-mini".to_string(),
                "o1-preview".to_string(),
                "Phi-4".to_string(),
                "Phi-3.5-MoE-instruct".to_string(),
                "Llama-3.3-70B-Instruct".to_string(),
                "Llama-3.2-90B-Vision-Instruct".to_string(),
                "Meta-Llama-3.1-405B-Instruct".to_string(),
                "Mistral-large-2411".to_string(),
                "Mistral-small".to_string(),
                "Codestral-2501".to_string(),
                "DeepSeek-R1".to_string(),
                "Cohere-command-r-plus".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "deepseek".to_string(),
            name: "DeepSeek".to_string(),
            provider_type: "cloud".to_string(),
            icon: "🔍".to_string(),
            color: "#4d6bfe".to_string(),
            endpoint: Some("https://api.deepseek.com/v1".to_string()),
            models: vec![
                "deepseek-chat".to_string(),
                "deepseek-reasoner".to_string(),
                "deepseek-coder".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "xai".to_string(),
            name: "xAI".to_string(),
            provider_type: "cloud".to_string(),
            icon: "🚀".to_string(),
            color: "#000000".to_string(),
            endpoint: Some("https://api.x.ai/v1".to_string()),
            models: vec![
                "grok-3".to_string(),
                "grok-3-fast".to_string(),
                "grok-2".to_string(),
                "grok-2-mini".to_string(),
                "grok-2-vision".to_string(),
                "grok-beta".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "mistral".to_string(),
            name: "Mistral AI".to_string(),
            provider_type: "cloud".to_string(),
            icon: "🌬️".to_string(),
            color: "#ff7000".to_string(),
            endpoint: Some("https://api.mistral.ai/v1".to_string()),
            models: vec![
                "mistral-large-latest".to_string(),
                "mistral-large-2411".to_string(),
                "mistral-medium-latest".to_string(),
                "mistral-small-latest".to_string(),
                "mistral-small-2501".to_string(),
                "codestral-latest".to_string(),
                "codestral-2501".to_string(),
                "ministral-3b-latest".to_string(),
                "ministral-8b-latest".to_string(),
                "pixtral-large-latest".to_string(),
                "pixtral-12b".to_string(),
                "open-mistral-nemo".to_string(),
                "open-codestral-mamba".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "cohere".to_string(),
            name: "Cohere".to_string(),
            provider_type: "cloud".to_string(),
            icon: "🔗".to_string(),
            color: "#39594d".to_string(),
            endpoint: Some("https://api.cohere.ai/v1".to_string()),
            models: vec![
                "command-r-plus".to_string(),
                "command-r".to_string(),
                "command".to_string(),
                "command-light".to_string(),
                "command-nightly".to_string(),
                "aya-expanse-32b".to_string(),
                "aya-expanse-8b".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "perplexity".to_string(),
            name: "Perplexity".to_string(),
            provider_type: "cloud".to_string(),
            icon: "🔮".to_string(),
            color: "#20808d".to_string(),
            endpoint: Some("https://api.perplexity.ai".to_string()),
            models: vec![
                "sonar-pro".to_string(),
                "sonar".to_string(),
                "sonar-deep-research".to_string(),
                "sonar-reasoning-pro".to_string(),
                "sonar-reasoning".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "groq".to_string(),
            name: "Groq".to_string(),
            provider_type: "cloud".to_string(),
            icon: "⚡".to_string(),
            color: "#f55036".to_string(),
            endpoint: Some("https://api.groq.com/openai/v1".to_string()),
            models: vec![
                "llama-3.3-70b-versatile".to_string(),
                "llama-3.1-70b-versatile".to_string(),
                "llama-3.1-8b-instant".to_string(),
                "llama3-groq-70b-8192-tool-use-preview".to_string(),
                "llama3-groq-8b-8192-tool-use-preview".to_string(),
                "mixtral-8x7b-32768".to_string(),
                "gemma2-9b-it".to_string(),
                "deepseek-r1-distill-llama-70b".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "together".to_string(),
            name: "Together AI".to_string(),
            provider_type: "cloud".to_string(),
            icon: "🤝".to_string(),
            color: "#0f6fff".to_string(),
            endpoint: Some("https://api.together.xyz/v1".to_string()),
            models: vec![
                "meta-llama/Llama-3.3-70B-Instruct-Turbo".to_string(),
                "meta-llama/Meta-Llama-3.1-405B-Instruct-Turbo".to_string(),
                "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo".to_string(),
                "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo".to_string(),
                "Qwen/Qwen2.5-72B-Instruct-Turbo".to_string(),
                "Qwen/QwQ-32B-Preview".to_string(),
                "deepseek-ai/DeepSeek-R1".to_string(),
                "deepseek-ai/DeepSeek-V3".to_string(),
                "mistralai/Mixtral-8x22B-Instruct-v0.1".to_string(),
                "databricks/dbrx-instruct".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "fireworks".to_string(),
            name: "Fireworks AI".to_string(),
            provider_type: "cloud".to_string(),
            icon: "🎆".to_string(),
            color: "#ff6b35".to_string(),
            endpoint: Some("https://api.fireworks.ai/inference/v1".to_string()),
            models: vec![
                "accounts/fireworks/models/llama-v3p3-70b-instruct".to_string(),
                "accounts/fireworks/models/llama-v3p1-405b-instruct".to_string(),
                "accounts/fireworks/models/qwen2p5-72b-instruct".to_string(),
                "accounts/fireworks/models/mixtral-8x22b-instruct".to_string(),
                "accounts/fireworks/models/deepseek-r1".to_string(),
                "accounts/fireworks/models/deepseek-v3".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "replicate".to_string(),
            name: "Replicate".to_string(),
            provider_type: "cloud".to_string(),
            icon: "🔄".to_string(),
            color: "#000000".to_string(),
            endpoint: Some("https://api.replicate.com/v1".to_string()),
            models: vec![
                "meta/llama-3.3-70b-instruct".to_string(),
                "meta/meta-llama-3.1-405b-instruct".to_string(),
                "mistralai/mixtral-8x7b-instruct-v0.1".to_string(),
                "anthropic/claude-3.5-sonnet".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "openrouter".to_string(),
            name: "OpenRouter".to_string(),
            provider_type: "cloud".to_string(),
            icon: "🛤️".to_string(),
            color: "#6467f2".to_string(),
            endpoint: Some("https://openrouter.ai/api/v1".to_string()),
            models: vec![
                "openai/gpt-4o".to_string(),
                "openai/o1".to_string(),
                "anthropic/claude-sonnet-4".to_string(),
                "anthropic/claude-3.5-sonnet".to_string(),
                "google/gemini-2.0-flash".to_string(),
                "google/gemini-2.5-pro".to_string(),
                "meta-llama/llama-3.3-70b-instruct".to_string(),
                "deepseek/deepseek-r1".to_string(),
                "deepseek/deepseek-chat".to_string(),
                "mistralai/mistral-large-2411".to_string(),
                "qwen/qwq-32b-preview".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "aws-bedrock".to_string(),
            name: "AWS Bedrock".to_string(),
            provider_type: "cloud".to_string(),
            icon: "🪨".to_string(),
            color: "#ff9900".to_string(),
            endpoint: None,
            models: vec![
                "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
                "anthropic.claude-3-5-haiku-20241022-v1:0".to_string(),
                "anthropic.claude-3-opus-20240229-v1:0".to_string(),
                "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
                "meta.llama3-3-70b-instruct-v1:0".to_string(),
                "meta.llama3-1-405b-instruct-v1:0".to_string(),
                "mistral.mistral-large-2411-v1:0".to_string(),
                "amazon.nova-pro-v1:0".to_string(),
                "amazon.nova-lite-v1:0".to_string(),
                "amazon.nova-micro-v1:0".to_string(),
                "amazon.titan-text-premier-v1:0".to_string(),
                "cohere.command-r-plus-v1:0".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "ai21".to_string(),
            name: "AI21 Labs".to_string(),
            provider_type: "cloud".to_string(),
            icon: "🧪".to_string(),
            color: "#ec4899".to_string(),
            endpoint: Some("https://api.ai21.com/studio/v1".to_string()),
            models: vec![
                "jamba-1.5-large".to_string(),
                "jamba-1.5-mini".to_string(),
                "jamba-instruct".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "cursor".to_string(),
            name: "Cursor".to_string(),
            provider_type: "cloud".to_string(),
            icon: "📝".to_string(),
            color: "#000000".to_string(),
            endpoint: None,
            models: vec![
                "cursor-small".to_string(),
                "cursor-large".to_string(),
                "gpt-4".to_string(),
                "gpt-4o".to_string(),
                "claude-3.5-sonnet".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "m365-copilot".to_string(),
            name: "Microsoft 365 Copilot".to_string(),
            provider_type: "cloud".to_string(),
            icon: "📊".to_string(),
            color: "#0078d4".to_string(),
            endpoint: None,
            models: vec![
                "copilot-chat".to_string(),
                "copilot-word".to_string(),
                "copilot-excel".to_string(),
                "copilot-powerpoint".to_string(),
                "copilot-outlook".to_string(),
                "copilot-teams".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        // ===========================================
        // Local Providers
        // ===========================================
        Provider {
            id: "ollama".to_string(),
            name: "Ollama".to_string(),
            provider_type: "local".to_string(),
            icon: "🦙".to_string(),
            color: "#ffffff".to_string(),
            endpoint: Some("http://localhost:11434".to_string()),
            models: vec![
                "llama3.3:70b".to_string(),
                "llama3.3:latest".to_string(),
                "llama3.2:latest".to_string(),
                "llama3.1:405b".to_string(),
                "llama3.1:70b".to_string(),
                "llama3.1:latest".to_string(),
                "qwen2.5-coder:32b".to_string(),
                "qwen2.5-coder:14b".to_string(),
                "qwen2.5-coder:7b".to_string(),
                "qwen2.5:72b".to_string(),
                "qwen2.5:32b".to_string(),
                "qwen2.5:14b".to_string(),
                "qwen2.5:7b".to_string(),
                "qwq:32b".to_string(),
                "deepseek-r1:70b".to_string(),
                "deepseek-r1:32b".to_string(),
                "deepseek-r1:14b".to_string(),
                "deepseek-r1:8b".to_string(),
                "deepseek-r1:1.5b".to_string(),
                "deepseek-coder-v2:latest".to_string(),
                "codellama:70b".to_string(),
                "codellama:34b".to_string(),
                "codellama:13b".to_string(),
                "codellama:7b".to_string(),
                "mistral:latest".to_string(),
                "mistral-nemo:latest".to_string(),
                "mixtral:8x7b".to_string(),
                "mixtral:8x22b".to_string(),
                "phi4:latest".to_string(),
                "phi3.5:latest".to_string(),
                "phi3:latest".to_string(),
                "gemma2:27b".to_string(),
                "gemma2:9b".to_string(),
                "gemma2:2b".to_string(),
                "command-r:latest".to_string(),
                "command-r-plus:latest".to_string(),
                "yi:34b".to_string(),
                "yi-coder:9b".to_string(),
                "starcoder2:15b".to_string(),
                "starcoder2:7b".to_string(),
                "starcoder2:3b".to_string(),
                "nomic-embed-text:latest".to_string(),
                "mxbai-embed-large:latest".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "lm-studio".to_string(),
            name: "LM Studio".to_string(),
            provider_type: "local".to_string(),
            icon: "🎬".to_string(),
            color: "#1a1a2e".to_string(),
            endpoint: Some("http://localhost:1234/v1".to_string()),
            models: vec!["loaded-model".to_string()],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "localai".to_string(),
            name: "LocalAI".to_string(),
            provider_type: "local".to_string(),
            icon: "🏠".to_string(),
            color: "#00d4aa".to_string(),
            endpoint: Some("http://localhost:8080/v1".to_string()),
            models: vec![
                "gpt4all-j".to_string(),
                "ggml-gpt4all-j".to_string(),
                "wizardlm-13b-v1.2".to_string(),
                "llama-2-7b-chat".to_string(),
                "codellama-7b-instruct".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "llamafile".to_string(),
            name: "llamafile".to_string(),
            provider_type: "local".to_string(),
            icon: "📁".to_string(),
            color: "#fbbf24".to_string(),
            endpoint: Some("http://localhost:8080/v1".to_string()),
            models: vec!["loaded-model".to_string()],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "jan".to_string(),
            name: "Jan".to_string(),
            provider_type: "local".to_string(),
            icon: "💬".to_string(),
            color: "#1d4ed8".to_string(),
            endpoint: Some("http://localhost:1337/v1".to_string()),
            models: vec!["loaded-model".to_string()],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "gpt4all".to_string(),
            name: "GPT4All".to_string(),
            provider_type: "local".to_string(),
            icon: "🌐".to_string(),
            color: "#4ade80".to_string(),
            endpoint: Some("http://localhost:4891/v1".to_string()),
            models: vec![
                "gpt4all-falcon-newbpe-q4_0".to_string(),
                "gpt4all-mistral-7b-instruct-v0.2".to_string(),
                "orca-2-7b".to_string(),
                "nous-hermes-llama2-13b".to_string(),
                "wizardlm-13b-v1.2".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "text-gen-webui".to_string(),
            name: "Text Generation WebUI".to_string(),
            provider_type: "local".to_string(),
            icon: "🖥️".to_string(),
            color: "#a855f7".to_string(),
            endpoint: Some("http://localhost:5000/v1".to_string()),
            models: vec!["loaded-model".to_string()],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "vllm".to_string(),
            name: "vLLM".to_string(),
            provider_type: "local".to_string(),
            icon: "⚙️".to_string(),
            color: "#06b6d4".to_string(),
            endpoint: Some("http://localhost:8000/v1".to_string()),
            models: vec![
                "meta-llama/Llama-3.3-70B-Instruct".to_string(),
                "meta-llama/Llama-3.1-8B-Instruct".to_string(),
                "mistralai/Mistral-7B-Instruct-v0.3".to_string(),
                "Qwen/Qwen2.5-72B-Instruct".to_string(),
                "deepseek-ai/DeepSeek-V3".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "mlx".to_string(),
            name: "MLX (Apple Silicon)".to_string(),
            provider_type: "local".to_string(),
            icon: "🍎".to_string(),
            color: "#a3a3a3".to_string(),
            endpoint: Some("http://localhost:8080/v1".to_string()),
            models: vec![
                "mlx-community/Llama-3.3-70B-Instruct-4bit".to_string(),
                "mlx-community/Qwen2.5-Coder-32B-Instruct-4bit".to_string(),
                "mlx-community/Mistral-7B-Instruct-v0.3-4bit".to_string(),
            ],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "koboldcpp".to_string(),
            name: "KoboldCpp".to_string(),
            provider_type: "local".to_string(),
            icon: "🐉".to_string(),
            color: "#dc2626".to_string(),
            endpoint: Some("http://localhost:5001/v1".to_string()),
            models: vec!["loaded-model".to_string()],
            status: "disconnected".to_string(),
        },
        Provider {
            id: "tabby".to_string(),
            name: "Tabby".to_string(),
            provider_type: "local".to_string(),
            icon: "🐱".to_string(),
            color: "#f59e0b".to_string(),
            endpoint: Some("http://localhost:8080".to_string()),
            models: vec![
                "StarCoder-1B".to_string(),
                "StarCoder-3B".to_string(),
                "StarCoder-7B".to_string(),
                "CodeLlama-7B".to_string(),
                "CodeLlama-13B".to_string(),
                "DeepSeek-Coder-1.3B".to_string(),
                "DeepSeek-Coder-6.7B".to_string(),
            ],
            status: "disconnected".to_string(),
        },
    ];
    HttpResponse::Ok().json(ApiResponse::success(providers))
}

pub async fn get_provider(path: web::Path<String>) -> impl Responder {
    let id = path.into_inner();
    HttpResponse::Ok().json(ApiResponse::success(Provider {
        id: id.clone(),
        name: id,
        provider_type: "local".to_string(),
        icon: "robot".to_string(),
        color: "#000000".to_string(),
        endpoint: None,
        models: vec![],
        status: "unknown".to_string(),
    }))
}

pub async fn create_provider() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Provider creation not yet implemented",
    ))
}

pub async fn update_provider() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Provider update not yet implemented",
    ))
}

pub async fn delete_provider() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Provider deletion not yet implemented",
    ))
}

pub async fn provider_health_check() -> impl Responder {
    let health = vec![ProviderHealth {
        provider_id: "copilot".to_string(),
        status: "connected".to_string(),
        latency: Some(150),
        last_checked: Utc::now().timestamp(),
        error: None,
        version: Some("1.0.0".to_string()),
        models: vec!["gpt-4o".to_string()],
    }];
    HttpResponse::Ok().json(ApiResponse::success(health))
}

pub async fn check_provider_health(path: web::Path<String>) -> impl Responder {
    let provider_id = path.into_inner();
    HttpResponse::Ok().json(ApiResponse::success(ProviderHealth {
        provider_id,
        status: "unknown".to_string(),
        latency: None,
        last_checked: Utc::now().timestamp(),
        error: None,
        version: None,
        models: vec![],
    }))
}

pub async fn list_provider_models(path: web::Path<String>) -> impl Responder {
    let _provider_id = path.into_inner();
    HttpResponse::Ok().json(ApiResponse::success(vec![
        "gpt-4o".to_string(),
        "gpt-4o-mini".to_string(),
    ]))
}

pub async fn test_provider(path: web::Path<String>) -> impl Responder {
    let _provider_id = path.into_inner();
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "success": true,
        "latency": 150
    })))
}

// Agents (stub implementations)

pub async fn list_agents() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::success(Vec::<serde_json::Value>::new()))
}

pub async fn get_agent(path: web::Path<String>) -> impl Responder {
    let _id = path.into_inner();
    HttpResponse::NotFound().json(ApiResponse::<()>::error("NOT_FOUND", "Agent not found"))
}

pub async fn create_agent() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Agent creation not yet implemented",
    ))
}

pub async fn update_agent() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Agent update not yet implemented",
    ))
}

pub async fn delete_agent() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Agent deletion not yet implemented",
    ))
}

pub async fn clone_agent() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Agent cloning not yet implemented",
    ))
}

// Swarms (stub implementations)

pub async fn list_swarms() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::success(Vec::<serde_json::Value>::new()))
}

pub async fn get_swarm(path: web::Path<String>) -> impl Responder {
    let _id = path.into_inner();
    HttpResponse::NotFound().json(ApiResponse::<()>::error("NOT_FOUND", "Swarm not found"))
}

pub async fn create_swarm() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Swarm creation not yet implemented",
    ))
}

pub async fn update_swarm() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Swarm update not yet implemented",
    ))
}

pub async fn delete_swarm() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Swarm deletion not yet implemented",
    ))
}

pub async fn start_swarm() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Swarm execution not yet implemented",
    ))
}

pub async fn pause_swarm() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Swarm pause not yet implemented",
    ))
}

pub async fn resume_swarm() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Swarm resume not yet implemented",
    ))
}

pub async fn stop_swarm() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Swarm stop not yet implemented",
    ))
}

// Chat Completions

pub async fn chat_completions(body: web::Json<ChatCompletionRequest>) -> impl Responder {
    // This would forward to the actual provider
    let _request = body.into_inner();
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Chat completions not yet implemented",
    ))
}

// Search

pub async fn search_all(query: web::Query<SearchQuery>) -> impl Responder {
    let _q = &query.q;
    HttpResponse::Ok().json(ApiResponse::success(Vec::<serde_json::Value>::new()))
}

pub async fn search_sessions(
    state: web::Data<AppState>,
    query: web::Query<SearchQuery>,
) -> impl Responder {
    let db = state.db.read().await;
    let search_term = &query.q;

    match db.list_sessions(None) {
        Ok(sessions) => {
            let filtered: Vec<_> = sessions
                .into_iter()
                .filter(|s| s.title.to_lowercase().contains(&search_term.to_lowercase()))
                .collect();

            HttpResponse::Ok().json(ApiResponse::success(PaginatedResponse {
                total: filtered.len(),
                limit: query.limit.unwrap_or(50),
                offset: 0,
                has_more: false,
                items: filtered,
            }))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn search_messages(query: web::Query<SearchQuery>) -> impl Responder {
    let _q = &query.q;
    HttpResponse::Ok().json(ApiResponse::success(Vec::<serde_json::Value>::new()))
}

pub async fn semantic_search(query: web::Query<SearchQuery>) -> impl Responder {
    let _q = &query.q;
    HttpResponse::Ok().json(ApiResponse::success(Vec::<serde_json::Value>::new()))
}

// Statistics

pub async fn stats_overview(state: web::Data<AppState>) -> impl Responder {
    let db = state.db.read().await;

    match db.get_statistics() {
        Ok(stats) => HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "totalSessions": stats.session_count,
            "totalMessages": stats.message_count,
            "totalWorkspaces": stats.workspace_count,
            "totalProviders": 2,
            "sessionsThisWeek": 0,
            "messagesThisWeek": 0,
            "sessionsByProvider": [],
            "messagesByDay": [],
            "topWorkspaces": []
        }))),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("DATABASE_ERROR", &e.to_string())),
    }
}

pub async fn stats_workspace(path: web::Path<String>) -> impl Responder {
    let _id = path.into_inner();
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "totalSessions": 0,
        "totalMessages": 0,
        "totalWorkspaces": 1,
        "totalProviders": 0,
        "sessionsThisWeek": 0,
        "messagesThisWeek": 0,
        "sessionsByProvider": [],
        "messagesByDay": [],
        "topWorkspaces": []
    })))
}

pub async fn stats_providers() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({})))
}

pub async fn stats_timeline(query: web::Query<TimelineQuery>) -> impl Responder {
    let _days = query.days.unwrap_or(30);
    HttpResponse::Ok().json(ApiResponse::success(Vec::<serde_json::Value>::new()))
}

// Import/Export

pub async fn import_sessions(_body: web::Json<ImportRequest>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Import not yet implemented",
    ))
}

pub async fn export_sessions(_body: web::Json<ExportRequest>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Export not yet implemented",
    ))
}

pub async fn harvest_sessions(_body: web::Json<HarvestRequest>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Harvest not yet implemented",
    ))
}

pub async fn sync_sessions(_body: web::Json<SyncRequest>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Sync not yet implemented",
    ))
}

// Settings

pub async fn get_settings() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "theme": "dark",
        "syntaxTheme": "monokai",
        "fontSize": 14,
        "showTimestamps": true,
        "soundEnabled": false,
        "streamResponses": true,
        "defaultProvider": null,
        "defaultModel": null,
        "autoSave": true,
        "harvestPath": null
    })))
}

pub async fn update_settings(_body: web::Json<UpdateSettingsRequest>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Settings update not yet implemented",
    ))
}

pub async fn list_accounts() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::success(Vec::<serde_json::Value>::new()))
}

pub async fn add_account(_body: web::Json<AddAccountRequest>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Account add not yet implemented",
    ))
}

pub async fn remove_account(path: web::Path<String>) -> impl Responder {
    let _id = path.into_inner();
    HttpResponse::Ok().json(ApiResponse::<()>::error(
        "NOT_IMPLEMENTED",
        "Account removal not yet implemented",
    ))
}
