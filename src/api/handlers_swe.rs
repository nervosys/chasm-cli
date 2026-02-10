// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! SWE (Software Engineering) Mode Handlers
//!
//! Provides API endpoints for the SWE mode with persistent project memory,
//! user-defined rules, and context injection for AI assistants.

#![allow(dead_code, unused_variables)]

use actix_web::{web, HttpResponse, Responder};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

use super::state::AppState;

// =============================================================================
// Response Types
// =============================================================================

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn success(data: T) -> HttpResponse {
        HttpResponse::Ok().json(Self {
            success: true,
            data: Some(data),
            error: None,
        })
    }

    fn error(message: &str) -> HttpResponse {
        HttpResponse::InternalServerError().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(message.to_string()),
        })
    }

    fn not_found(message: &str) -> HttpResponse {
        HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(message.to_string()),
        })
    }

    fn bad_request(message: &str) -> HttpResponse {
        HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(message.to_string()),
        })
    }
}

// =============================================================================
// Data Models
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SweProject {
    pub id: String,
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub git_remote: Option<String>,
    pub git_branch: Option<String>,
    pub language: Option<String>,
    pub framework: Option<String>,
    pub last_opened: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub memory_count: i64,
    pub rule_count: i64,
    pub session_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SweMemory {
    pub id: String,
    pub project_id: String,
    pub key: String,
    pub value: String,
    pub category: String,
    pub importance: String,
    pub source: Option<String>,
    pub source_message_id: Option<String>,
    pub expires_at: Option<i64>,
    pub access_count: i64,
    pub last_accessed: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SweRule {
    pub id: String,
    pub project_id: String,
    pub rule: String,
    pub description: Option<String>,
    pub category: String,
    pub priority: i32,
    pub enabled: bool,
    pub scope: Option<String>,      // JSON string
    pub conditions: Option<String>, // JSON string
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SweSession {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub model: Option<String>,
    pub provider: String,
    pub message_count: i64,
    pub token_count: Option<i64>,
    pub working_directory: Option<String>,
    pub git_branch: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub archived: bool,
}

// =============================================================================
// Request Types
// =============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub path: String,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMemoryRequest {
    pub key: String,
    pub value: String,
    pub category: Option<String>,
    pub importance: Option<String>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemoryRequest {
    pub value: Option<String>,
    pub category: Option<String>,
    pub importance: Option<String>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRuleRequest {
    pub rule: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub priority: Option<i32>,
    pub scope: Option<serde_json::Value>,
    pub conditions: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRuleRequest {
    pub rule: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
    pub scope: Option<serde_json::Value>,
    pub conditions: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteToolRequest {
    pub tool: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ProjectQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct MemoryQuery {
    pub category: Option<String>,
    pub importance: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct RuleQuery {
    pub category: Option<String>,
    pub enabled: Option<bool>,
}

// =============================================================================
// Database Initialization
// =============================================================================

pub fn init_swe_tables(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    // Projects table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS swe_projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            description TEXT,
            git_remote TEXT,
            git_branch TEXT,
            language TEXT,
            framework TEXT,
            last_opened INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            metadata TEXT
        )",
        [],
    )?;

    // Memory table (key-value store per project)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS swe_memory (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            key TEXT NOT NULL,
            value TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'context',
            importance TEXT NOT NULL DEFAULT 'medium',
            source TEXT,
            source_message_id TEXT,
            expires_at INTEGER,
            access_count INTEGER NOT NULL DEFAULT 0,
            last_accessed INTEGER,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            metadata TEXT,
            FOREIGN KEY (project_id) REFERENCES swe_projects(id) ON DELETE CASCADE,
            UNIQUE(project_id, key)
        )",
        [],
    )?;

    // Rules table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS swe_rules (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            rule TEXT NOT NULL,
            description TEXT,
            category TEXT NOT NULL DEFAULT 'custom',
            priority INTEGER NOT NULL DEFAULT 50,
            enabled INTEGER NOT NULL DEFAULT 1,
            scope TEXT,
            conditions TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            metadata TEXT,
            FOREIGN KEY (project_id) REFERENCES swe_projects(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Sessions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS swe_sessions (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            title TEXT NOT NULL,
            model TEXT,
            provider TEXT NOT NULL,
            message_count INTEGER NOT NULL DEFAULT 0,
            token_count INTEGER,
            working_directory TEXT,
            git_branch TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            archived INTEGER NOT NULL DEFAULT 0,
            metadata TEXT,
            FOREIGN KEY (project_id) REFERENCES swe_projects(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Messages table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS swe_messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            model TEXT,
            token_count INTEGER,
            tool_calls TEXT,
            tool_results TEXT,
            context_snapshot TEXT,
            created_at INTEGER NOT NULL,
            metadata TEXT,
            FOREIGN KEY (session_id) REFERENCES swe_sessions(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_swe_memory_project ON swe_memory(project_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_swe_memory_category ON swe_memory(category)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_swe_rules_project ON swe_rules(project_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_swe_sessions_project ON swe_sessions(project_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_swe_messages_session ON swe_messages(session_id)",
        [],
    )?;

    Ok(())
}

// =============================================================================
// Project Endpoints
// =============================================================================

/// List all SWE projects
pub async fn list_projects(
    state: web::Data<AppState>,
    query: web::Query<ProjectQuery>,
) -> impl Responder {
    let db = state.db.lock().unwrap();

    // Initialize tables if needed
    if let Err(e) = init_swe_tables(&db.conn) {
        return ApiResponse::<()>::error(&format!("Failed to init tables: {}", e));
    }

    let limit = query.limit.unwrap_or(50);

    let mut stmt = match db.conn.prepare(
        "SELECT p.id, p.name, p.path, p.description, p.git_remote, p.git_branch,
                p.language, p.framework, p.last_opened, p.created_at, p.updated_at,
                (SELECT COUNT(*) FROM swe_memory WHERE project_id = p.id) as memory_count,
                (SELECT COUNT(*) FROM swe_rules WHERE project_id = p.id) as rule_count,
                (SELECT COUNT(*) FROM swe_sessions WHERE project_id = p.id) as session_count
         FROM swe_projects p
         ORDER BY p.last_opened DESC
         LIMIT ?",
    ) {
        Ok(stmt) => stmt,
        Err(e) => return ApiResponse::<()>::error(&format!("Query error: {}", e)),
    };

    let projects: Vec<SweProject> = stmt
        .query_map([limit], |row| {
            Ok(SweProject {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                description: row.get(3)?,
                git_remote: row.get(4)?,
                git_branch: row.get(5)?,
                language: row.get(6)?,
                framework: row.get(7)?,
                last_opened: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                memory_count: row.get(11)?,
                rule_count: row.get(12)?,
                session_count: row.get(13)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    ApiResponse::success(projects)
}

/// Get a single project by ID
pub async fn get_project(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let project_id = path.into_inner();
    let db = state.db.lock().unwrap();

    let project: Option<SweProject> = db
        .conn
        .query_row(
            "SELECT p.id, p.name, p.path, p.description, p.git_remote, p.git_branch,
                    p.language, p.framework, p.last_opened, p.created_at, p.updated_at,
                    (SELECT COUNT(*) FROM swe_memory WHERE project_id = p.id) as memory_count,
                    (SELECT COUNT(*) FROM swe_rules WHERE project_id = p.id) as rule_count,
                    (SELECT COUNT(*) FROM swe_sessions WHERE project_id = p.id) as session_count
             FROM swe_projects p WHERE p.id = ?",
            [&project_id],
            |row| {
                Ok(SweProject {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    description: row.get(3)?,
                    git_remote: row.get(4)?,
                    git_branch: row.get(5)?,
                    language: row.get(6)?,
                    framework: row.get(7)?,
                    last_opened: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    memory_count: row.get(11)?,
                    rule_count: row.get(12)?,
                    session_count: row.get(13)?,
                })
            },
        )
        .optional()
        .unwrap_or(None);

    match project {
        Some(p) => ApiResponse::success(p),
        None => ApiResponse::<()>::not_found("Project not found"),
    }
}

/// Create a new SWE project
pub async fn create_project(
    state: web::Data<AppState>,
    body: web::Json<CreateProjectRequest>,
) -> impl Responder {
    let db = state.db.lock().unwrap();

    // Initialize tables if needed
    if let Err(e) = init_swe_tables(&db.conn) {
        return ApiResponse::<()>::error(&format!("Failed to init tables: {}", e));
    }

    let path = PathBuf::from(&body.path);
    if !path.exists() {
        return ApiResponse::<()>::bad_request("Project path does not exist");
    }

    let id = uuid::Uuid::new_v4().to_string();
    let name = body.name.clone().unwrap_or_else(|| {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string()
    });
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    // Try to detect git info
    let git_branch = get_git_branch(&body.path);
    let git_remote = get_git_remote(&body.path);

    // Try to detect language/framework from common files
    let (language, framework) = detect_project_type(&path);

    match db.conn.execute(
        "INSERT INTO swe_projects (id, name, path, description, git_remote, git_branch, 
                                   language, framework, last_opened, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, ?9)",
        params![
            &id,
            &name,
            &body.path,
            &body.description,
            &git_remote,
            &git_branch,
            &language,
            &framework,
            now,
        ],
    ) {
        Ok(_) => {
            let project = SweProject {
                id,
                name,
                path: body.path.clone(),
                description: body.description.clone(),
                git_remote,
                git_branch,
                language,
                framework,
                last_opened: now,
                created_at: now,
                updated_at: now,
                memory_count: 0,
                rule_count: 0,
                session_count: 0,
            };
            ApiResponse::success(project)
        }
        Err(e) => {
            if e.to_string().contains("UNIQUE constraint failed") {
                ApiResponse::<()>::bad_request("Project with this path already exists")
            } else {
                ApiResponse::<()>::error(&format!("Failed to create project: {}", e))
            }
        }
    }
}

/// Delete a project
pub async fn delete_project(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let project_id = path.into_inner();
    let db = state.db.lock().unwrap();

    match db
        .conn
        .execute("DELETE FROM swe_projects WHERE id = ?", [&project_id])
    {
        Ok(rows) if rows > 0 => ApiResponse::success(serde_json::json!({"deleted": true})),
        Ok(_) => ApiResponse::<()>::not_found("Project not found"),
        Err(e) => ApiResponse::<()>::error(&format!("Failed to delete project: {}", e)),
    }
}

/// Open a project (update last_opened timestamp)
pub async fn open_project(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let project_id = path.into_inner();
    let db = state.db.lock().unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    match db.conn.execute(
        "UPDATE swe_projects SET last_opened = ?, updated_at = ? WHERE id = ?",
        params![now, now, &project_id],
    ) {
        Ok(rows) if rows > 0 => ApiResponse::success(serde_json::json!({"opened": true})),
        Ok(_) => ApiResponse::<()>::not_found("Project not found"),
        Err(e) => ApiResponse::<()>::error(&format!("Failed to update project: {}", e)),
    }
}

// =============================================================================
// Memory Endpoints
// =============================================================================

/// List memory entries for a project
pub async fn list_memory(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<MemoryQuery>,
) -> impl Responder {
    let project_id = path.into_inner();
    let db = state.db.lock().unwrap();

    let limit = query.limit.unwrap_or(100);
    let mut sql = String::from(
        "SELECT id, project_id, key, value, category, importance, source, source_message_id,
                expires_at, access_count, last_accessed, created_at, updated_at
         FROM swe_memory WHERE project_id = ?",
    );

    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(project_id.clone())];

    if let Some(ref category) = query.category {
        sql.push_str(" AND category = ?");
        params_vec.push(Box::new(category.clone()));
    }

    if let Some(ref importance) = query.importance {
        sql.push_str(" AND importance = ?");
        params_vec.push(Box::new(importance.clone()));
    }

    if let Some(ref search) = query.search {
        sql.push_str(" AND (key LIKE ? OR value LIKE ?)");
        let pattern = format!("%{}%", search);
        params_vec.push(Box::new(pattern.clone()));
        params_vec.push(Box::new(pattern));
    }

    sql.push_str(" ORDER BY importance DESC, updated_at DESC LIMIT ?");
    params_vec.push(Box::new(limit as i64));

    let params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let mut stmt = match db.conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => return ApiResponse::<()>::error(&format!("Query error: {}", e)),
    };

    let memories: Vec<SweMemory> = stmt
        .query_map(params.as_slice(), |row| {
            Ok(SweMemory {
                id: row.get(0)?,
                project_id: row.get(1)?,
                key: row.get(2)?,
                value: row.get(3)?,
                category: row.get(4)?,
                importance: row.get(5)?,
                source: row.get(6)?,
                source_message_id: row.get(7)?,
                expires_at: row.get(8)?,
                access_count: row.get(9)?,
                last_accessed: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })
        .unwrap_or_else(|_| panic!())
        .filter_map(|r| r.ok())
        .collect();

    ApiResponse::success(memories)
}

/// Get a single memory entry
pub async fn get_memory(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (project_id, memory_id) = path.into_inner();
    let db = state.db.lock().unwrap();

    // Update access count and last_accessed
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let _ = db.conn.execute(
        "UPDATE swe_memory SET access_count = access_count + 1, last_accessed = ? 
         WHERE id = ? AND project_id = ?",
        params![now, &memory_id, &project_id],
    );

    let memory: Option<SweMemory> = db
        .conn
        .query_row(
            "SELECT id, project_id, key, value, category, importance, source, source_message_id,
                    expires_at, access_count, last_accessed, created_at, updated_at
             FROM swe_memory WHERE id = ? AND project_id = ?",
            params![&memory_id, &project_id],
            |row| {
                Ok(SweMemory {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    key: row.get(2)?,
                    value: row.get(3)?,
                    category: row.get(4)?,
                    importance: row.get(5)?,
                    source: row.get(6)?,
                    source_message_id: row.get(7)?,
                    expires_at: row.get(8)?,
                    access_count: row.get(9)?,
                    last_accessed: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            },
        )
        .optional()
        .unwrap_or(None);

    match memory {
        Some(m) => ApiResponse::success(m),
        None => ApiResponse::<()>::not_found("Memory entry not found"),
    }
}

/// Create a memory entry
pub async fn create_memory(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<CreateMemoryRequest>,
) -> impl Responder {
    let project_id = path.into_inner();
    let db = state.db.lock().unwrap();

    let id = uuid::Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let category = body
        .category
        .clone()
        .unwrap_or_else(|| "context".to_string());
    let importance = body
        .importance
        .clone()
        .unwrap_or_else(|| "medium".to_string());

    match db.conn.execute(
        "INSERT INTO swe_memory (id, project_id, key, value, category, importance, source, 
                                 expires_at, access_count, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'user', ?7, 0, ?8, ?8)",
        params![
            &id,
            &project_id,
            &body.key,
            &body.value,
            &category,
            &importance,
            body.expires_at,
            now,
        ],
    ) {
        Ok(_) => {
            let memory = SweMemory {
                id,
                project_id,
                key: body.key.clone(),
                value: body.value.clone(),
                category,
                importance,
                source: Some("user".to_string()),
                source_message_id: None,
                expires_at: body.expires_at,
                access_count: 0,
                last_accessed: None,
                created_at: now,
                updated_at: now,
            };
            ApiResponse::success(memory)
        }
        Err(e) => {
            if e.to_string().contains("UNIQUE constraint failed") {
                ApiResponse::<()>::bad_request("Memory with this key already exists")
            } else {
                ApiResponse::<()>::error(&format!("Failed to create memory: {}", e))
            }
        }
    }
}

/// Update a memory entry
pub async fn update_memory(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    body: web::Json<UpdateMemoryRequest>,
) -> impl Responder {
    let (project_id, memory_id) = path.into_inner();
    let db = state.db.lock().unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    // Build dynamic update
    let mut updates = vec!["updated_at = ?".to_string()];
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now)];

    if let Some(ref value) = body.value {
        updates.push("value = ?".to_string());
        params_vec.push(Box::new(value.clone()));
    }
    if let Some(ref category) = body.category {
        updates.push("category = ?".to_string());
        params_vec.push(Box::new(category.clone()));
    }
    if let Some(ref importance) = body.importance {
        updates.push("importance = ?".to_string());
        params_vec.push(Box::new(importance.clone()));
    }
    if body.expires_at.is_some() {
        updates.push("expires_at = ?".to_string());
        params_vec.push(Box::new(body.expires_at));
    }

    params_vec.push(Box::new(memory_id.clone()));
    params_vec.push(Box::new(project_id.clone()));

    let sql = format!(
        "UPDATE swe_memory SET {} WHERE id = ? AND project_id = ?",
        updates.join(", ")
    );

    let params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    match db.conn.execute(&sql, params.as_slice()) {
        Ok(rows) if rows > 0 => ApiResponse::success(serde_json::json!({"updated": true})),
        Ok(_) => ApiResponse::<()>::not_found("Memory entry not found"),
        Err(e) => ApiResponse::<()>::error(&format!("Failed to update memory: {}", e)),
    }
}

/// Delete a memory entry
pub async fn delete_memory(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (project_id, memory_id) = path.into_inner();
    let db = state.db.lock().unwrap();

    match db.conn.execute(
        "DELETE FROM swe_memory WHERE id = ? AND project_id = ?",
        params![&memory_id, &project_id],
    ) {
        Ok(rows) if rows > 0 => ApiResponse::success(serde_json::json!({"deleted": true})),
        Ok(_) => ApiResponse::<()>::not_found("Memory entry not found"),
        Err(e) => ApiResponse::<()>::error(&format!("Failed to delete memory: {}", e)),
    }
}

// =============================================================================
// Rules Endpoints
// =============================================================================

/// List rules for a project
pub async fn list_rules(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<RuleQuery>,
) -> impl Responder {
    let project_id = path.into_inner();
    let db = state.db.lock().unwrap();

    let mut sql = String::from(
        "SELECT id, project_id, rule, description, category, priority, enabled, scope, conditions,
                created_at, updated_at
         FROM swe_rules WHERE project_id = ?",
    );

    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(project_id.clone())];

    if let Some(ref category) = query.category {
        sql.push_str(" AND category = ?");
        params_vec.push(Box::new(category.clone()));
    }

    if let Some(enabled) = query.enabled {
        sql.push_str(" AND enabled = ?");
        params_vec.push(Box::new(enabled as i32));
    }

    sql.push_str(" ORDER BY priority ASC, created_at ASC");

    let params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let mut stmt = match db.conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => return ApiResponse::<()>::error(&format!("Query error: {}", e)),
    };

    let rules: Vec<SweRule> = stmt
        .query_map(params.as_slice(), |row| {
            Ok(SweRule {
                id: row.get(0)?,
                project_id: row.get(1)?,
                rule: row.get(2)?,
                description: row.get(3)?,
                category: row.get(4)?,
                priority: row.get(5)?,
                enabled: row.get::<_, i32>(6)? != 0,
                scope: row.get(7)?,
                conditions: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    ApiResponse::success(rules)
}

/// Create a rule
pub async fn create_rule(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<CreateRuleRequest>,
) -> impl Responder {
    let project_id = path.into_inner();
    let db = state.db.lock().unwrap();

    let id = uuid::Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let category = body
        .category
        .clone()
        .unwrap_or_else(|| "custom".to_string());
    let priority = body.priority.unwrap_or(50);
    let scope = body.scope.as_ref().map(|s| s.to_string());
    let conditions = body.conditions.as_ref().map(|c| c.to_string());

    match db.conn.execute(
        "INSERT INTO swe_rules (id, project_id, rule, description, category, priority, enabled,
                                scope, conditions, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, ?9, ?9)",
        params![
            &id,
            &project_id,
            &body.rule,
            &body.description,
            &category,
            priority,
            &scope,
            &conditions,
            now,
        ],
    ) {
        Ok(_) => {
            let rule = SweRule {
                id,
                project_id,
                rule: body.rule.clone(),
                description: body.description.clone(),
                category,
                priority,
                enabled: true,
                scope,
                conditions,
                created_at: now,
                updated_at: now,
            };
            ApiResponse::success(rule)
        }
        Err(e) => ApiResponse::<()>::error(&format!("Failed to create rule: {}", e)),
    }
}

/// Update a rule
pub async fn update_rule(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    body: web::Json<UpdateRuleRequest>,
) -> impl Responder {
    let (project_id, rule_id) = path.into_inner();
    let db = state.db.lock().unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let mut updates = vec!["updated_at = ?".to_string()];
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now)];

    if let Some(ref rule) = body.rule {
        updates.push("rule = ?".to_string());
        params_vec.push(Box::new(rule.clone()));
    }
    if let Some(ref description) = body.description {
        updates.push("description = ?".to_string());
        params_vec.push(Box::new(description.clone()));
    }
    if let Some(ref category) = body.category {
        updates.push("category = ?".to_string());
        params_vec.push(Box::new(category.clone()));
    }
    if let Some(priority) = body.priority {
        updates.push("priority = ?".to_string());
        params_vec.push(Box::new(priority));
    }
    if let Some(enabled) = body.enabled {
        updates.push("enabled = ?".to_string());
        params_vec.push(Box::new(enabled as i32));
    }
    if let Some(ref scope) = body.scope {
        updates.push("scope = ?".to_string());
        params_vec.push(Box::new(scope.to_string()));
    }
    if let Some(ref conditions) = body.conditions {
        updates.push("conditions = ?".to_string());
        params_vec.push(Box::new(conditions.to_string()));
    }

    params_vec.push(Box::new(rule_id.clone()));
    params_vec.push(Box::new(project_id.clone()));

    let sql = format!(
        "UPDATE swe_rules SET {} WHERE id = ? AND project_id = ?",
        updates.join(", ")
    );

    let params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    match db.conn.execute(&sql, params.as_slice()) {
        Ok(rows) if rows > 0 => ApiResponse::success(serde_json::json!({"updated": true})),
        Ok(_) => ApiResponse::<()>::not_found("Rule not found"),
        Err(e) => ApiResponse::<()>::error(&format!("Failed to update rule: {}", e)),
    }
}

/// Delete a rule
pub async fn delete_rule(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (project_id, rule_id) = path.into_inner();
    let db = state.db.lock().unwrap();

    match db.conn.execute(
        "DELETE FROM swe_rules WHERE id = ? AND project_id = ?",
        params![&rule_id, &project_id],
    ) {
        Ok(rows) if rows > 0 => ApiResponse::success(serde_json::json!({"deleted": true})),
        Ok(_) => ApiResponse::<()>::not_found("Rule not found"),
        Err(e) => ApiResponse::<()>::error(&format!("Failed to delete rule: {}", e)),
    }
}

// =============================================================================
// Context Injection Endpoint
// =============================================================================

/// Get the context to inject into model prompts
/// This returns all enabled rules and relevant memory for the project
pub async fn get_context(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let project_id = path.into_inner();
    let db = state.db.lock().unwrap();

    // Get all enabled rules, ordered by priority
    let mut rules_stmt = match db.conn.prepare(
        "SELECT rule, category, priority FROM swe_rules 
         WHERE project_id = ? AND enabled = 1 
         ORDER BY priority ASC",
    ) {
        Ok(s) => s,
        Err(e) => return ApiResponse::<()>::error(&format!("Query error: {}", e)),
    };

    let rules: Vec<serde_json::Value> = rules_stmt
        .query_map([&project_id], |row| {
            Ok(serde_json::json!({
                "rule": row.get::<_, String>(0)?,
                "category": row.get::<_, String>(1)?,
                "priority": row.get::<_, i32>(2)?,
            }))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // Get important memory entries (critical and high importance, not expired)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let mut memory_stmt = match db.conn.prepare(
        "SELECT key, value, category, importance FROM swe_memory 
         WHERE project_id = ? AND importance IN ('critical', 'high')
         AND (expires_at IS NULL OR expires_at > ?)
         ORDER BY 
            CASE importance WHEN 'critical' THEN 0 WHEN 'high' THEN 1 ELSE 2 END,
            updated_at DESC
         LIMIT 50",
    ) {
        Ok(s) => s,
        Err(e) => return ApiResponse::<()>::error(&format!("Query error: {}", e)),
    };

    let memory: Vec<serde_json::Value> = memory_stmt
        .query_map(params![&project_id, now], |row| {
            Ok(serde_json::json!({
                "key": row.get::<_, String>(0)?,
                "value": row.get::<_, String>(1)?,
                "category": row.get::<_, String>(2)?,
                "importance": row.get::<_, String>(3)?,
            }))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // Get project info
    let project_info: Option<(String, String, Option<String>, Option<String>)> = db
        .conn
        .query_row(
            "SELECT name, path, language, framework FROM swe_projects WHERE id = ?",
            [&project_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .unwrap_or(None);

    // Build system prompt addition
    let mut system_parts = vec![];

    if let Some((name, path, language, framework)) = project_info {
        system_parts.push(format!(
            "You are working on the project '{}' located at '{}'.",
            name, path
        ));
        if let Some(lang) = language {
            system_parts.push(format!("Primary language: {}", lang));
        }
        if let Some(fw) = framework {
            system_parts.push(format!("Framework: {}", fw));
        }
    }

    if !rules.is_empty() {
        system_parts.push("\n## Project Rules (MUST FOLLOW):".to_string());
        for rule in &rules {
            let rule_text = rule["rule"].as_str().unwrap_or("");
            let category = rule["category"].as_str().unwrap_or("custom");
            system_parts.push(format!("- [{}] {}", category.to_uppercase(), rule_text));
        }
    }

    if !memory.is_empty() {
        system_parts.push("\n## Project Context (Important Information):".to_string());
        for mem in &memory {
            let key = mem["key"].as_str().unwrap_or("");
            let value = mem["value"].as_str().unwrap_or("");
            let importance = mem["importance"].as_str().unwrap_or("medium");
            system_parts.push(format!(
                "- **{}** [{}]: {}",
                key,
                importance.to_uppercase(),
                value
            ));
        }
    }

    ApiResponse::success(serde_json::json!({
        "systemPromptAddition": system_parts.join("\n"),
        "rules": rules,
        "memory": memory,
        "ruleCount": rules.len(),
        "memoryCount": memory.len(),
    }))
}

// =============================================================================
// Tool Execution Endpoints
// =============================================================================

/// Execute a tool (file operations, terminal commands, etc.)
pub async fn execute_tool(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<ExecuteToolRequest>,
) -> impl Responder {
    let project_id = path.into_inner();
    let db = state.db.lock().unwrap();

    // Get project path
    let project_path: Option<String> = db
        .conn
        .query_row(
            "SELECT path FROM swe_projects WHERE id = ?",
            [&project_id],
            |row| row.get(0),
        )
        .optional()
        .unwrap_or(None);

    let project_path = match project_path {
        Some(p) => p,
        None => return ApiResponse::<()>::not_found("Project not found"),
    };

    let base_path = PathBuf::from(&project_path);

    match body.tool.as_str() {
        "read_file" => {
            let file_path = body.input["path"].as_str().unwrap_or("");
            let full_path = resolve_path(&base_path, file_path);

            match std::fs::read_to_string(&full_path) {
                Ok(content) => ApiResponse::success(serde_json::json!({
                    "success": true,
                    "content": content,
                    "path": full_path.to_string_lossy(),
                })),
                Err(e) => ApiResponse::success(serde_json::json!({
                    "success": false,
                    "error": e.to_string(),
                })),
            }
        }
        "write_file" => {
            let file_path = body.input["path"].as_str().unwrap_or("");
            let content = body.input["content"].as_str().unwrap_or("");
            let full_path = resolve_path(&base_path, file_path);

            // Create parent directories if needed
            if let Some(parent) = full_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            match std::fs::write(&full_path, content) {
                Ok(_) => ApiResponse::success(serde_json::json!({
                    "success": true,
                    "path": full_path.to_string_lossy(),
                })),
                Err(e) => ApiResponse::success(serde_json::json!({
                    "success": false,
                    "error": e.to_string(),
                })),
            }
        }
        "list_directory" => {
            let dir_path = body.input["path"].as_str().unwrap_or(".");
            let full_path = resolve_path(&base_path, dir_path);

            match std::fs::read_dir(&full_path) {
                Ok(entries) => {
                    let files: Vec<serde_json::Value> = entries
                        .filter_map(|e| e.ok())
                        .map(|entry| {
                            let name = entry.file_name().to_string_lossy().to_string();
                            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                            serde_json::json!({
                                "name": name,
                                "type": if is_dir { "directory" } else { "file" },
                                "size": size,
                            })
                        })
                        .collect();
                    ApiResponse::success(serde_json::json!({
                        "success": true,
                        "files": files,
                        "path": full_path.to_string_lossy(),
                    }))
                }
                Err(e) => ApiResponse::success(serde_json::json!({
                    "success": false,
                    "error": e.to_string(),
                })),
            }
        }
        "run_command" => {
            let command = body.input["command"].as_str().unwrap_or("");
            let working_dir = body.input["workingDirectory"]
                .as_str()
                .map(|d| resolve_path(&base_path, d))
                .unwrap_or_else(|| base_path.clone());

            // Execute command
            #[cfg(target_os = "windows")]
            let output = Command::new("cmd")
                .args(["/C", command])
                .current_dir(&working_dir)
                .output();

            #[cfg(not(target_os = "windows"))]
            let output = Command::new("sh")
                .args(["-c", command])
                .current_dir(&working_dir)
                .output();

            match output {
                Ok(out) => ApiResponse::success(serde_json::json!({
                    "success": out.status.success(),
                    "exitCode": out.status.code().unwrap_or(-1),
                    "stdout": String::from_utf8_lossy(&out.stdout),
                    "stderr": String::from_utf8_lossy(&out.stderr),
                    "command": command,
                    "workingDirectory": working_dir.to_string_lossy(),
                })),
                Err(e) => ApiResponse::success(serde_json::json!({
                    "success": false,
                    "error": e.to_string(),
                })),
            }
        }
        "search_files" => {
            let pattern = body.input["pattern"].as_str().unwrap_or("*");
            let search_dir = body.input["directory"]
                .as_str()
                .map(|d| resolve_path(&base_path, d))
                .unwrap_or_else(|| base_path.clone());

            let mut results = vec![];
            if let Ok(entries) =
                glob::glob(&format!("{}/{}", search_dir.to_string_lossy(), pattern))
            {
                for entry in entries
                    .filter_map(|e: Result<std::path::PathBuf, glob::GlobError>| e.ok())
                    .take(100)
                {
                    results.push(serde_json::json!({
                        "path": entry.to_string_lossy(),
                        "relativePath": entry.strip_prefix(&base_path)
                            .map(|p: &std::path::Path| p.to_string_lossy().to_string())
                            .unwrap_or_else(|_| entry.to_string_lossy().to_string()),
                    }));
                }
            }

            ApiResponse::success(serde_json::json!({
                "success": true,
                "results": results,
                "count": results.len(),
            }))
        }
        "git_status" => {
            let output = Command::new("git")
                .args(["status", "--porcelain", "-b"])
                .current_dir(&base_path)
                .output();

            match output {
                Ok(out) => {
                    let status = String::from_utf8_lossy(&out.stdout).to_string();
                    ApiResponse::success(serde_json::json!({
                        "success": out.status.success(),
                        "status": status,
                    }))
                }
                Err(e) => ApiResponse::success(serde_json::json!({
                    "success": false,
                    "error": e.to_string(),
                })),
            }
        }
        "git_diff" => {
            let staged = body.input["staged"].as_bool().unwrap_or(false);
            let mut args = vec!["diff"];
            if staged {
                args.push("--staged");
            }

            let output = Command::new("git")
                .args(&args)
                .current_dir(&base_path)
                .output();

            match output {
                Ok(out) => {
                    let diff = String::from_utf8_lossy(&out.stdout).to_string();
                    ApiResponse::success(serde_json::json!({
                        "success": out.status.success(),
                        "diff": diff,
                    }))
                }
                Err(e) => ApiResponse::success(serde_json::json!({
                    "success": false,
                    "error": e.to_string(),
                })),
            }
        }
        _ => ApiResponse::<()>::bad_request(&format!("Unknown tool: {}", body.tool)),
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn get_git_branch(path: &str) -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn get_git_remote(path: &str) -> Option<String> {
    Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn detect_project_type(path: &std::path::Path) -> (Option<String>, Option<String>) {
    let mut language = None;
    let mut framework = None;

    // Check for common project files
    if path.join("Cargo.toml").exists() {
        language = Some("rust".to_string());
    } else if path.join("package.json").exists() {
        language = Some("typescript".to_string());

        // Check for specific frameworks
        if path.join("next.config.js").exists() || path.join("next.config.mjs").exists() {
            framework = Some("next.js".to_string());
        } else if path.join("vite.config.ts").exists() || path.join("vite.config.js").exists() {
            framework = Some("vite".to_string());
        } else if path.join("angular.json").exists() {
            framework = Some("angular".to_string());
        }
    } else if path.join("requirements.txt").exists() || path.join("pyproject.toml").exists() {
        language = Some("python".to_string());

        if path.join("manage.py").exists() {
            framework = Some("django".to_string());
        }
    } else if path.join("go.mod").exists() {
        language = Some("go".to_string());
    } else if path.join("pom.xml").exists() || path.join("build.gradle").exists() {
        language = Some("java".to_string());
    } else if path.join("*.csproj").exists() || path.join("*.sln").exists() {
        language = Some("csharp".to_string());
    }

    (language, framework)
}

fn resolve_path(base: &std::path::Path, relative: &str) -> PathBuf {
    let path = PathBuf::from(relative);
    if path.is_absolute() {
        path
    } else {
        base.join(relative)
    }
}
