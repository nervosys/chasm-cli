// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Session Branching and Merging Module
//!
//! Provides Git-like branching capabilities for chat sessions:
//! - Create branches from any point in a conversation
//! - Merge branches with conflict detection and resolution
//! - Fork sessions to create parallel conversation paths
//! - Track branch history and relationships

use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// =============================================================================
// Core Types
// =============================================================================

/// A branch represents a divergent path in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionBranch {
    /// Unique branch identifier
    pub id: String,
    /// Human-readable branch name
    pub name: String,
    /// Parent session ID
    pub session_id: String,
    /// Parent branch ID (None for main branch)
    pub parent_branch_id: Option<String>,
    /// Message ID where branch diverges from parent
    pub fork_point_message_id: String,
    /// Branch creation time
    pub created_at: DateTime<Utc>,
    /// Last updated time
    pub updated_at: DateTime<Utc>,
    /// Branch description
    pub description: Option<String>,
    /// Whether this is the default/main branch
    pub is_default: bool,
    /// Whether branch is archived
    pub is_archived: bool,
    /// Branch metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A message in a branched conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchedMessage {
    /// Message ID
    pub id: String,
    /// Branch this message belongs to
    pub branch_id: String,
    /// Parent message ID (for threading)
    pub parent_message_id: Option<String>,
    /// Message role (user/assistant/system)
    pub role: String,
    /// Message content
    pub content: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Message metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Fork request to create a new branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkRequest {
    /// Session to fork from
    pub session_id: String,
    /// Message ID to fork from (branch point)
    pub fork_from_message_id: String,
    /// Name for the new branch
    pub branch_name: String,
    /// Optional description
    pub description: Option<String>,
}

/// Merge request to combine branches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeRequest {
    /// Source branch to merge from
    pub source_branch_id: String,
    /// Target branch to merge into
    pub target_branch_id: String,
    /// Merge strategy
    pub strategy: MergeStrategy,
    /// Conflict resolution preferences
    pub conflict_resolution: Option<ConflictResolution>,
}

/// Merge strategies
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    /// Append source messages after target (default)
    #[default]
    Append,
    /// Interleave by timestamp
    Interleave,
    /// Keep only target, discard source
    KeepTarget,
    /// Keep only source, discard target
    KeepSource,
    /// Manual merge with conflict markers
    Manual,
}

/// Conflict resolution preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    /// How to handle conflicting messages at same timestamp
    pub timestamp_conflict: TimestampConflictStrategy,
    /// How to handle duplicate content
    pub duplicate_content: DuplicateStrategy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimestampConflictStrategy {
    #[default]
    KeepBoth,
    PreferSource,
    PreferTarget,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateStrategy {
    #[default]
    KeepFirst,
    KeepLast,
    KeepBoth,
    Remove,
}

/// Result of a merge operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    /// Whether merge was successful
    pub success: bool,
    /// Resulting branch ID
    pub result_branch_id: String,
    /// Number of messages merged
    pub messages_merged: usize,
    /// Conflicts encountered
    pub conflicts: Vec<MergeConflict>,
    /// Merge commit message
    pub merge_message: String,
}

/// A merge conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeConflict {
    /// Conflict ID
    pub id: String,
    /// Conflict type
    pub conflict_type: ConflictType,
    /// Source message
    pub source_message: Option<BranchedMessage>,
    /// Target message
    pub target_message: Option<BranchedMessage>,
    /// Resolution status
    pub resolved: bool,
    /// Chosen resolution
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    /// Same timestamp, different content
    TimestampCollision,
    /// Identical content in both branches
    DuplicateContent,
    /// Divergent conversation paths
    DivergentPath,
    /// Missing parent message
    OrphanedMessage,
}

/// Branch comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchComparison {
    /// Base branch
    pub base_branch_id: String,
    /// Compare branch
    pub compare_branch_id: String,
    /// Common ancestor message ID
    pub common_ancestor_id: Option<String>,
    /// Messages only in base
    pub base_only: Vec<BranchedMessage>,
    /// Messages only in compare
    pub compare_only: Vec<BranchedMessage>,
    /// Messages in both (potentially modified)
    pub common: Vec<BranchedMessage>,
    /// Ahead/behind counts
    pub ahead: usize,
    pub behind: usize,
}

// =============================================================================
// Branching Service
// =============================================================================

/// Service for managing session branches
pub struct BranchingService {
    // In production, this would use the database
    branches: std::sync::RwLock<HashMap<String, SessionBranch>>,
    messages: std::sync::RwLock<HashMap<String, Vec<BranchedMessage>>>,
}

impl BranchingService {
    pub fn new() -> Self {
        Self {
            branches: std::sync::RwLock::new(HashMap::new()),
            messages: std::sync::RwLock::new(HashMap::new()),
        }
    }

    // =========================================================================
    // Branch Operations
    // =========================================================================

    /// Create a new branch (fork) from an existing session/branch
    pub async fn create_branch(&self, request: ForkRequest) -> Result<SessionBranch, String> {
        let branch = SessionBranch {
            id: Uuid::new_v4().to_string(),
            name: request.branch_name,
            session_id: request.session_id.clone(),
            parent_branch_id: None, // Set if forking from another branch
            fork_point_message_id: request.fork_from_message_id.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            description: request.description,
            is_default: false,
            is_archived: false,
            metadata: HashMap::new(),
        };

        // Copy messages up to fork point to new branch
        let branch_id = branch.id.clone();
        self.copy_messages_to_branch(
            &request.session_id,
            &branch_id,
            &request.fork_from_message_id,
        )?;

        let mut branches = self.branches.write().map_err(|e| e.to_string())?;
        branches.insert(branch.id.clone(), branch.clone());

        Ok(branch)
    }

    /// List all branches for a session
    pub async fn list_branches(&self, session_id: &str) -> Result<Vec<SessionBranch>, String> {
        let branches = self.branches.read().map_err(|e| e.to_string())?;
        let result: Vec<SessionBranch> = branches
            .values()
            .filter(|b| b.session_id == session_id)
            .cloned()
            .collect();
        Ok(result)
    }

    /// Get a specific branch
    pub async fn get_branch(&self, branch_id: &str) -> Result<SessionBranch, String> {
        let branches = self.branches.read().map_err(|e| e.to_string())?;
        branches
            .get(branch_id)
            .cloned()
            .ok_or_else(|| format!("Branch not found: {}", branch_id))
    }

    /// Update branch metadata
    pub async fn update_branch(
        &self,
        branch_id: &str,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<SessionBranch, String> {
        let mut branches = self.branches.write().map_err(|e| e.to_string())?;
        let branch = branches
            .get_mut(branch_id)
            .ok_or_else(|| format!("Branch not found: {}", branch_id))?;

        if let Some(n) = name {
            branch.name = n;
        }
        if let Some(d) = description {
            branch.description = Some(d);
        }
        branch.updated_at = Utc::now();

        Ok(branch.clone())
    }

    /// Delete a branch
    pub async fn delete_branch(&self, branch_id: &str) -> Result<(), String> {
        let mut branches = self.branches.write().map_err(|e| e.to_string())?;
        let branch = branches
            .get(branch_id)
            .ok_or_else(|| format!("Branch not found: {}", branch_id))?;

        if branch.is_default {
            return Err("Cannot delete the default branch".to_string());
        }

        branches.remove(branch_id);

        // Also remove messages for this branch
        let mut messages = self.messages.write().map_err(|e| e.to_string())?;
        messages.remove(branch_id);

        Ok(())
    }

    /// Archive a branch
    pub async fn archive_branch(&self, branch_id: &str) -> Result<SessionBranch, String> {
        let mut branches = self.branches.write().map_err(|e| e.to_string())?;
        let branch = branches
            .get_mut(branch_id)
            .ok_or_else(|| format!("Branch not found: {}", branch_id))?;

        branch.is_archived = true;
        branch.updated_at = Utc::now();

        Ok(branch.clone())
    }

    // =========================================================================
    // Merge Operations
    // =========================================================================

    /// Merge two branches
    pub async fn merge_branches(&self, request: MergeRequest) -> Result<MergeResult, String> {
        let source_messages = self.get_branch_messages(&request.source_branch_id).await?;
        let target_messages = self.get_branch_messages(&request.target_branch_id).await?;

        let mut conflicts = Vec::new();
        let mut merged_messages = Vec::new();

        match request.strategy {
            MergeStrategy::Append => {
                // Simple append: add all target messages, then source messages
                merged_messages.extend(target_messages.clone());
                for msg in source_messages {
                    // Check for duplicates
                    if !target_messages
                        .iter()
                        .any(|t| t.content == msg.content && t.role == msg.role)
                    {
                        merged_messages.push(msg);
                    }
                }
            }
            MergeStrategy::Interleave => {
                // Merge by timestamp
                let mut all_messages: Vec<_> = target_messages
                    .into_iter()
                    .chain(source_messages.into_iter())
                    .collect();
                all_messages.sort_by_key(|m| m.timestamp);

                // Detect timestamp conflicts
                for i in 1..all_messages.len() {
                    if all_messages[i].timestamp == all_messages[i - 1].timestamp
                        && all_messages[i].branch_id != all_messages[i - 1].branch_id
                    {
                        conflicts.push(MergeConflict {
                            id: Uuid::new_v4().to_string(),
                            conflict_type: ConflictType::TimestampCollision,
                            source_message: Some(all_messages[i].clone()),
                            target_message: Some(all_messages[i - 1].clone()),
                            resolved: false,
                            resolution: None,
                        });
                    }
                }
                merged_messages = all_messages;
            }
            MergeStrategy::KeepTarget => {
                merged_messages = target_messages;
            }
            MergeStrategy::KeepSource => {
                merged_messages = source_messages;
            }
            MergeStrategy::Manual => {
                // Return all messages with conflict markers for manual resolution
                merged_messages.extend(target_messages);
                for msg in source_messages {
                    conflicts.push(MergeConflict {
                        id: Uuid::new_v4().to_string(),
                        conflict_type: ConflictType::DivergentPath,
                        source_message: Some(msg),
                        target_message: None,
                        resolved: false,
                        resolution: None,
                    });
                }
            }
        }

        // Store merged messages in target branch
        {
            let mut messages = self.messages.write().map_err(|e| e.to_string())?;
            messages.insert(request.target_branch_id.clone(), merged_messages.clone());
        }

        Ok(MergeResult {
            success: conflicts.is_empty(),
            result_branch_id: request.target_branch_id,
            messages_merged: merged_messages.len(),
            conflicts,
            merge_message: format!(
                "Merged branch {} using {:?} strategy",
                request.source_branch_id, request.strategy
            ),
        })
    }

    /// Compare two branches
    pub async fn compare_branches(
        &self,
        base_branch_id: &str,
        compare_branch_id: &str,
    ) -> Result<BranchComparison, String> {
        let base_messages = self.get_branch_messages(base_branch_id).await?;
        let compare_messages = self.get_branch_messages(compare_branch_id).await?;

        let base_ids: std::collections::HashSet<_> =
            base_messages.iter().map(|m| m.id.clone()).collect();
        let compare_ids: std::collections::HashSet<_> =
            compare_messages.iter().map(|m| m.id.clone()).collect();

        let base_only: Vec<_> = base_messages
            .iter()
            .filter(|m| !compare_ids.contains(&m.id))
            .cloned()
            .collect();

        let compare_only: Vec<_> = compare_messages
            .iter()
            .filter(|m| !base_ids.contains(&m.id))
            .cloned()
            .collect();

        let common: Vec<_> = base_messages
            .iter()
            .filter(|m| compare_ids.contains(&m.id))
            .cloned()
            .collect();

        Ok(BranchComparison {
            base_branch_id: base_branch_id.to_string(),
            compare_branch_id: compare_branch_id.to_string(),
            common_ancestor_id: common.first().map(|m| m.id.clone()),
            ahead: compare_only.len(),
            behind: base_only.len(),
            base_only,
            compare_only,
            common,
        })
    }

    // =========================================================================
    // Message Operations
    // =========================================================================

    /// Get messages for a branch
    pub async fn get_branch_messages(
        &self,
        branch_id: &str,
    ) -> Result<Vec<BranchedMessage>, String> {
        let messages = self.messages.read().map_err(|e| e.to_string())?;
        Ok(messages.get(branch_id).cloned().unwrap_or_default())
    }

    /// Add a message to a branch
    pub async fn add_message(
        &self,
        branch_id: &str,
        role: &str,
        content: &str,
    ) -> Result<BranchedMessage, String> {
        let message = BranchedMessage {
            id: Uuid::new_v4().to_string(),
            branch_id: branch_id.to_string(),
            parent_message_id: None,
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        let mut messages = self.messages.write().map_err(|e| e.to_string())?;
        messages
            .entry(branch_id.to_string())
            .or_default()
            .push(message.clone());

        Ok(message)
    }

    /// Copy messages up to a point to a new branch
    fn copy_messages_to_branch(
        &self,
        source_session_id: &str,
        target_branch_id: &str,
        up_to_message_id: &str,
    ) -> Result<(), String> {
        // In a real implementation, this would query the database
        // For now, we just create an empty message list for the new branch
        let mut messages = self.messages.write().map_err(|e| e.to_string())?;
        messages.insert(target_branch_id.to_string(), Vec::new());
        Ok(())
    }
}

impl Default for BranchingService {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// HTTP Handlers
// =============================================================================

/// POST /api/branches - Create a new branch (fork)
pub async fn create_branch(
    service: web::Data<BranchingService>,
    request: web::Json<ForkRequest>,
) -> HttpResponse {
    match service.create_branch(request.into_inner()).await {
        Ok(branch) => HttpResponse::Created().json(branch),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/branches?session_id={id} - List branches for a session
pub async fn list_branches(
    service: web::Data<BranchingService>,
    query: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    let session_id = match query.get("session_id") {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": "session_id required" }))
        }
    };

    match service.list_branches(session_id).await {
        Ok(branches) => HttpResponse::Ok().json(branches),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/branches/{id} - Get a specific branch
pub async fn get_branch(
    service: web::Data<BranchingService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.get_branch(&path.into_inner()).await {
        Ok(branch) => HttpResponse::Ok().json(branch),
        Err(e) => HttpResponse::NotFound().json(serde_json::json!({ "error": e })),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateBranchRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// PATCH /api/branches/{id} - Update a branch
pub async fn update_branch(
    service: web::Data<BranchingService>,
    path: web::Path<String>,
    body: web::Json<UpdateBranchRequest>,
) -> HttpResponse {
    let request = body.into_inner();
    match service
        .update_branch(&path.into_inner(), request.name, request.description)
        .await
    {
        Ok(branch) => HttpResponse::Ok().json(branch),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// DELETE /api/branches/{id} - Delete a branch
pub async fn delete_branch(
    service: web::Data<BranchingService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.delete_branch(&path.into_inner()).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// POST /api/branches/{id}/archive - Archive a branch
pub async fn archive_branch(
    service: web::Data<BranchingService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.archive_branch(&path.into_inner()).await {
        Ok(branch) => HttpResponse::Ok().json(branch),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// POST /api/branches/merge - Merge branches
pub async fn merge_branches(
    service: web::Data<BranchingService>,
    request: web::Json<MergeRequest>,
) -> HttpResponse {
    match service.merge_branches(request.into_inner()).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/branches/{id}/compare/{other_id} - Compare two branches
pub async fn compare_branches(
    service: web::Data<BranchingService>,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (base_id, compare_id) = path.into_inner();
    match service.compare_branches(&base_id, &compare_id).await {
        Ok(comparison) => HttpResponse::Ok().json(comparison),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/branches/{id}/messages - Get messages for a branch
pub async fn get_branch_messages(
    service: web::Data<BranchingService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.get_branch_messages(&path.into_inner()).await {
        Ok(messages) => HttpResponse::Ok().json(messages),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

#[derive(Debug, Deserialize)]
pub struct AddMessageRequest {
    pub role: String,
    pub content: String,
}

/// POST /api/branches/{id}/messages - Add a message to a branch
pub async fn add_message(
    service: web::Data<BranchingService>,
    path: web::Path<String>,
    body: web::Json<AddMessageRequest>,
) -> HttpResponse {
    let request = body.into_inner();
    match service
        .add_message(&path.into_inner(), &request.role, &request.content)
        .await
    {
        Ok(message) => HttpResponse::Created().json(message),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// Configure branching routes
pub fn configure_branching_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/branches")
            .route("", web::post().to(create_branch))
            .route("", web::get().to(list_branches))
            .route("/{id}", web::get().to(get_branch))
            .route("/{id}", web::patch().to(update_branch))
            .route("/{id}", web::delete().to(delete_branch))
            .route("/{id}/archive", web::post().to(archive_branch))
            .route("/merge", web::post().to(merge_branches))
            .route("/{id}/compare/{other_id}", web::get().to(compare_branches))
            .route("/{id}/messages", web::get().to(get_branch_messages))
            .route("/{id}/messages", web::post().to(add_message)),
    );
}
