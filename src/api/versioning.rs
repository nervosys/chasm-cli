// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Version Control Module for Conversations
//!
//! Provides Git-like version control capabilities for chat sessions:
//! - Commit messages with snapshots
//! - View history and diffs between versions
//! - Checkout previous versions
//! - Tag important conversation states
//! - Revert to previous states

use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// =============================================================================
// Core Types
// =============================================================================

/// A commit represents a snapshot of a conversation at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationCommit {
    /// Unique commit identifier (hash-like)
    pub id: String,
    /// Short hash for display
    pub short_id: String,
    /// Session this commit belongs to
    pub session_id: String,
    /// Branch this commit is on
    pub branch_id: Option<String>,
    /// Parent commit ID (None for initial commit)
    pub parent_id: Option<String>,
    /// Commit message
    pub message: String,
    /// Author information
    pub author: CommitAuthor,
    /// Commit timestamp
    pub created_at: DateTime<Utc>,
    /// Number of messages at this commit
    pub message_count: usize,
    /// Hash of conversation state
    pub state_hash: String,
    /// Tags associated with this commit
    pub tags: Vec<String>,
    /// Commit metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Author information for a commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitAuthor {
    pub user_id: Option<String>,
    pub name: String,
    pub email: Option<String>,
}

/// A tag marks a specific commit as important
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTag {
    /// Tag name (e.g., "v1.0", "milestone-1")
    pub name: String,
    /// Commit ID this tag points to
    pub commit_id: String,
    /// Session ID
    pub session_id: String,
    /// Tag message/annotation
    pub message: Option<String>,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Who created the tag
    pub created_by: Option<String>,
}

/// Snapshot of conversation state at a commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSnapshot {
    /// Commit this snapshot belongs to
    pub commit_id: String,
    /// Session ID
    pub session_id: String,
    /// Messages at this point
    pub messages: Vec<SnapshotMessage>,
    /// Conversation metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Message in a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Diff between two commits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDiff {
    /// Base commit
    pub base_commit_id: String,
    /// Compare commit
    pub compare_commit_id: String,
    /// Added messages
    pub added: Vec<DiffEntry>,
    /// Removed messages
    pub removed: Vec<DiffEntry>,
    /// Modified messages (if content changed)
    pub modified: Vec<DiffModification>,
    /// Summary statistics
    pub stats: DiffStats,
}

/// A diff entry for added/removed messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    pub message_id: String,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// A modification entry showing before/after
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffModification {
    pub message_id: String,
    pub before: DiffEntry,
    pub after: DiffEntry,
}

/// Diff statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub modifications: usize,
    pub total_changes: usize,
}

/// History entry for log display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub commit: ConversationCommit,
    pub tags: Vec<String>,
    pub is_head: bool,
}

// =============================================================================
// Version Control Service
// =============================================================================

/// Service for managing conversation version control
pub struct VersionControlService {
    commits: std::sync::RwLock<HashMap<String, ConversationCommit>>,
    snapshots: std::sync::RwLock<HashMap<String, ConversationSnapshot>>,
    tags: std::sync::RwLock<HashMap<String, ConversationTag>>,
    heads: std::sync::RwLock<HashMap<String, String>>, // session_id -> commit_id
}

impl VersionControlService {
    pub fn new() -> Self {
        Self {
            commits: std::sync::RwLock::new(HashMap::new()),
            snapshots: std::sync::RwLock::new(HashMap::new()),
            tags: std::sync::RwLock::new(HashMap::new()),
            heads: std::sync::RwLock::new(HashMap::new()),
        }
    }

    // =========================================================================
    // Commit Operations
    // =========================================================================

    /// Create a new commit (snapshot of current state)
    pub async fn commit(
        &self,
        session_id: &str,
        message: &str,
        author: CommitAuthor,
        messages: Vec<SnapshotMessage>,
    ) -> Result<ConversationCommit, String> {
        let commit_id = Uuid::new_v4().to_string();
        let short_id = commit_id[..8].to_string();

        // Get parent commit (current HEAD)
        let parent_id = {
            let heads = self.heads.read().map_err(|e| e.to_string())?;
            heads.get(session_id).cloned()
        };

        // Calculate state hash
        let state_hash = self.calculate_state_hash(&messages);

        let commit = ConversationCommit {
            id: commit_id.clone(),
            short_id,
            session_id: session_id.to_string(),
            branch_id: None,
            parent_id,
            message: message.to_string(),
            author,
            created_at: Utc::now(),
            message_count: messages.len(),
            state_hash,
            tags: Vec::new(),
            metadata: HashMap::new(),
        };

        // Store snapshot
        let snapshot = ConversationSnapshot {
            commit_id: commit_id.clone(),
            session_id: session_id.to_string(),
            messages,
            metadata: HashMap::new(),
        };

        {
            let mut commits = self.commits.write().map_err(|e| e.to_string())?;
            commits.insert(commit_id.clone(), commit.clone());
        }

        {
            let mut snapshots = self.snapshots.write().map_err(|e| e.to_string())?;
            snapshots.insert(commit_id.clone(), snapshot);
        }

        // Update HEAD
        {
            let mut heads = self.heads.write().map_err(|e| e.to_string())?;
            heads.insert(session_id.to_string(), commit_id);
        }

        Ok(commit)
    }

    /// Get a specific commit
    pub async fn get_commit(&self, commit_id: &str) -> Result<ConversationCommit, String> {
        let commits = self.commits.read().map_err(|e| e.to_string())?;
        commits
            .get(commit_id)
            .cloned()
            .ok_or_else(|| format!("Commit not found: {}", commit_id))
    }

    /// Get commit history for a session
    pub async fn get_history(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<HistoryEntry>, String> {
        let commits = self.commits.read().map_err(|e| e.to_string())?;
        let tags = self.tags.read().map_err(|e| e.to_string())?;
        let heads = self.heads.read().map_err(|e| e.to_string())?;

        let head_commit = heads.get(session_id);

        let mut history: Vec<_> = commits
            .values()
            .filter(|c| c.session_id == session_id)
            .map(|c| {
                let commit_tags: Vec<_> = tags
                    .values()
                    .filter(|t| t.commit_id == c.id)
                    .map(|t| t.name.clone())
                    .collect();

                HistoryEntry {
                    commit: c.clone(),
                    tags: commit_tags,
                    is_head: head_commit.map(|h| h == &c.id).unwrap_or(false),
                }
            })
            .collect();

        // Sort by timestamp descending
        history.sort_by(|a, b| b.commit.created_at.cmp(&a.commit.created_at));

        if let Some(limit) = limit {
            history.truncate(limit);
        }

        Ok(history)
    }

    /// Get snapshot for a commit
    pub async fn get_snapshot(&self, commit_id: &str) -> Result<ConversationSnapshot, String> {
        let snapshots = self.snapshots.read().map_err(|e| e.to_string())?;
        snapshots
            .get(commit_id)
            .cloned()
            .ok_or_else(|| format!("Snapshot not found: {}", commit_id))
    }

    // =========================================================================
    // Diff Operations
    // =========================================================================

    /// Get diff between two commits
    pub async fn diff(
        &self,
        base_commit_id: &str,
        compare_commit_id: &str,
    ) -> Result<CommitDiff, String> {
        let base_snapshot = self.get_snapshot(base_commit_id).await?;
        let compare_snapshot = self.get_snapshot(compare_commit_id).await?;

        let base_ids: std::collections::HashSet<_> = base_snapshot
            .messages
            .iter()
            .map(|m| m.id.clone())
            .collect();
        let compare_ids: std::collections::HashSet<_> = compare_snapshot
            .messages
            .iter()
            .map(|m| m.id.clone())
            .collect();

        // Find added (in compare but not in base)
        let added: Vec<_> = compare_snapshot
            .messages
            .iter()
            .filter(|m| !base_ids.contains(&m.id))
            .map(|m| DiffEntry {
                message_id: m.id.clone(),
                role: m.role.clone(),
                content: m.content.clone(),
                timestamp: m.timestamp,
            })
            .collect();

        // Find removed (in base but not in compare)
        let removed: Vec<_> = base_snapshot
            .messages
            .iter()
            .filter(|m| !compare_ids.contains(&m.id))
            .map(|m| DiffEntry {
                message_id: m.id.clone(),
                role: m.role.clone(),
                content: m.content.clone(),
                timestamp: m.timestamp,
            })
            .collect();

        // Find modified (in both but content differs)
        let mut modified = Vec::new();
        for base_msg in &base_snapshot.messages {
            if let Some(compare_msg) = compare_snapshot
                .messages
                .iter()
                .find(|m| m.id == base_msg.id)
            {
                if base_msg.content != compare_msg.content {
                    modified.push(DiffModification {
                        message_id: base_msg.id.clone(),
                        before: DiffEntry {
                            message_id: base_msg.id.clone(),
                            role: base_msg.role.clone(),
                            content: base_msg.content.clone(),
                            timestamp: base_msg.timestamp,
                        },
                        after: DiffEntry {
                            message_id: compare_msg.id.clone(),
                            role: compare_msg.role.clone(),
                            content: compare_msg.content.clone(),
                            timestamp: compare_msg.timestamp,
                        },
                    });
                }
            }
        }

        let stats = DiffStats {
            additions: added.len(),
            deletions: removed.len(),
            modifications: modified.len(),
            total_changes: added.len() + removed.len() + modified.len(),
        };

        Ok(CommitDiff {
            base_commit_id: base_commit_id.to_string(),
            compare_commit_id: compare_commit_id.to_string(),
            added,
            removed,
            modified,
            stats,
        })
    }

    // =========================================================================
    // Checkout/Revert Operations
    // =========================================================================

    /// Checkout a specific commit (returns the snapshot for that state)
    pub async fn checkout(&self, commit_id: &str) -> Result<ConversationSnapshot, String> {
        self.get_snapshot(commit_id).await
    }

    /// Revert to a previous commit (creates a new commit with that state)
    pub async fn revert(
        &self,
        session_id: &str,
        target_commit_id: &str,
        author: CommitAuthor,
    ) -> Result<ConversationCommit, String> {
        let target_snapshot = self.get_snapshot(target_commit_id).await?;
        let target_commit = self.get_commit(target_commit_id).await?;

        let revert_message = format!(
            "Revert to commit {} ({})",
            target_commit.short_id, target_commit.message
        );

        self.commit(
            session_id,
            &revert_message,
            author,
            target_snapshot.messages,
        )
        .await
    }

    // =========================================================================
    // Tag Operations
    // =========================================================================

    /// Create a tag for a commit
    pub async fn create_tag(
        &self,
        name: &str,
        commit_id: &str,
        message: Option<String>,
        created_by: Option<String>,
    ) -> Result<ConversationTag, String> {
        let commit = self.get_commit(commit_id).await?;

        let tag = ConversationTag {
            name: name.to_string(),
            commit_id: commit_id.to_string(),
            session_id: commit.session_id,
            message,
            created_at: Utc::now(),
            created_by,
        };

        let mut tags = self.tags.write().map_err(|e| e.to_string())?;

        if tags.contains_key(name) {
            return Err(format!("Tag '{}' already exists", name));
        }

        tags.insert(name.to_string(), tag.clone());

        Ok(tag)
    }

    /// List all tags for a session
    pub async fn list_tags(&self, session_id: &str) -> Result<Vec<ConversationTag>, String> {
        let tags = self.tags.read().map_err(|e| e.to_string())?;
        let result: Vec<_> = tags
            .values()
            .filter(|t| t.session_id == session_id)
            .cloned()
            .collect();
        Ok(result)
    }

    /// Delete a tag
    pub async fn delete_tag(&self, name: &str) -> Result<(), String> {
        let mut tags = self.tags.write().map_err(|e| e.to_string())?;
        tags.remove(name)
            .map(|_| ())
            .ok_or_else(|| format!("Tag not found: {}", name))
    }

    /// Get commit by tag name
    pub async fn get_by_tag(&self, tag_name: &str) -> Result<ConversationCommit, String> {
        let tags = self.tags.read().map_err(|e| e.to_string())?;
        let tag = tags
            .get(tag_name)
            .ok_or_else(|| format!("Tag not found: {}", tag_name))?;

        self.get_commit(&tag.commit_id).await
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    fn calculate_state_hash(&self, messages: &[SnapshotMessage]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for msg in messages {
            msg.id.hash(&mut hasher);
            msg.content.hash(&mut hasher);
        }
        format!("{:016x}", hasher.finish())
    }
}

impl Default for VersionControlService {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// HTTP Handlers
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct CommitRequest {
    pub session_id: String,
    pub message: String,
    pub author_name: String,
    pub author_email: Option<String>,
    pub messages: Vec<SnapshotMessage>,
}

/// POST /api/vcs/commit - Create a new commit
pub async fn create_commit(
    service: web::Data<VersionControlService>,
    body: web::Json<CommitRequest>,
) -> HttpResponse {
    let request = body.into_inner();
    let author = CommitAuthor {
        user_id: None,
        name: request.author_name,
        email: request.author_email,
    };

    match service
        .commit(
            &request.session_id,
            &request.message,
            author,
            request.messages,
        )
        .await
    {
        Ok(commit) => HttpResponse::Created().json(commit),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/vcs/commits/{id} - Get a specific commit
pub async fn get_commit(
    service: web::Data<VersionControlService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.get_commit(&path.into_inner()).await {
        Ok(commit) => HttpResponse::Ok().json(commit),
        Err(e) => HttpResponse::NotFound().json(serde_json::json!({ "error": e })),
    }
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub session_id: String,
    pub limit: Option<usize>,
}

/// GET /api/vcs/history - Get commit history
pub async fn get_history(
    service: web::Data<VersionControlService>,
    query: web::Query<HistoryQuery>,
) -> HttpResponse {
    match service.get_history(&query.session_id, query.limit).await {
        Ok(history) => HttpResponse::Ok().json(history),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/vcs/commits/{id}/snapshot - Get snapshot for a commit
pub async fn get_snapshot(
    service: web::Data<VersionControlService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.get_snapshot(&path.into_inner()).await {
        Ok(snapshot) => HttpResponse::Ok().json(snapshot),
        Err(e) => HttpResponse::NotFound().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/vcs/diff/{base}/{compare} - Get diff between commits
pub async fn get_diff(
    service: web::Data<VersionControlService>,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (base, compare) = path.into_inner();
    match service.diff(&base, &compare).await {
        Ok(diff) => HttpResponse::Ok().json(diff),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// POST /api/vcs/checkout/{id} - Checkout a commit
pub async fn checkout(
    service: web::Data<VersionControlService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.checkout(&path.into_inner()).await {
        Ok(snapshot) => HttpResponse::Ok().json(snapshot),
        Err(e) => HttpResponse::NotFound().json(serde_json::json!({ "error": e })),
    }
}

#[derive(Debug, Deserialize)]
pub struct RevertRequest {
    pub session_id: String,
    pub target_commit_id: String,
    pub author_name: String,
    pub author_email: Option<String>,
}

/// POST /api/vcs/revert - Revert to a previous commit
pub async fn revert(
    service: web::Data<VersionControlService>,
    body: web::Json<RevertRequest>,
) -> HttpResponse {
    let request = body.into_inner();
    let author = CommitAuthor {
        user_id: None,
        name: request.author_name,
        email: request.author_email,
    };

    match service
        .revert(&request.session_id, &request.target_commit_id, author)
        .await
    {
        Ok(commit) => HttpResponse::Created().json(commit),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub commit_id: String,
    pub message: Option<String>,
}

/// POST /api/vcs/tags - Create a tag
pub async fn create_tag(
    service: web::Data<VersionControlService>,
    body: web::Json<CreateTagRequest>,
) -> HttpResponse {
    let request = body.into_inner();
    match service
        .create_tag(&request.name, &request.commit_id, request.message, None)
        .await
    {
        Ok(tag) => HttpResponse::Created().json(tag),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

#[derive(Debug, Deserialize)]
pub struct ListTagsQuery {
    pub session_id: String,
}

/// GET /api/vcs/tags - List tags
pub async fn list_tags(
    service: web::Data<VersionControlService>,
    query: web::Query<ListTagsQuery>,
) -> HttpResponse {
    match service.list_tags(&query.session_id).await {
        Ok(tags) => HttpResponse::Ok().json(tags),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// DELETE /api/vcs/tags/{name} - Delete a tag
pub async fn delete_tag(
    service: web::Data<VersionControlService>,
    path: web::Path<String>,
) -> HttpResponse {
    match service.delete_tag(&path.into_inner()).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::NotFound().json(serde_json::json!({ "error": e })),
    }
}

/// Configure version control routes
pub fn configure_vcs_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/vcs")
            .route("/commit", web::post().to(create_commit))
            .route("/commits/{id}", web::get().to(get_commit))
            .route("/commits/{id}/snapshot", web::get().to(get_snapshot))
            .route("/history", web::get().to(get_history))
            .route("/diff/{base}/{compare}", web::get().to(get_diff))
            .route("/checkout/{id}", web::post().to(checkout))
            .route("/revert", web::post().to(revert))
            .route("/tags", web::post().to(create_tag))
            .route("/tags", web::get().to(list_tags))
            .route("/tags/{name}", web::delete().to(delete_tag)),
    );
}
