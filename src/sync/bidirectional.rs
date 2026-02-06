// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Bidirectional Sync Module
//!
//! Provides two-way synchronization between CSM and provider-native storage.
//! Handles conflict resolution, change tracking, and state reconciliation.

use crate::models::ChatSession;
use crate::providers::ProviderType;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

// =============================================================================
// Sync State and Tracking
// =============================================================================

/// Sync state for a single session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSyncState {
    pub session_id: String,
    pub provider: ProviderType,
    pub last_sync: DateTime<Utc>,
    pub local_hash: String,
    pub remote_hash: String,
    pub status: SyncStatus,
    pub pending_changes: Vec<SyncChange>,
}

/// Sync status for a session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Synced,
    LocalAhead,
    RemoteAhead,
    Conflict,
    Unsynced,
    Error,
}

/// Represents a single change to sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncChange {
    pub id: String,
    pub change_type: ChangeType,
    pub entity_type: EntityType,
    pub entity_id: String,
    pub timestamp: DateTime<Utc>,
    pub payload: serde_json::Value,
    pub origin: ChangeOrigin,
}

/// Type of change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Create,
    Update,
    Delete,
    Merge,
}

/// Entity type for changes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Session,
    Message,
    Metadata,
    Tag,
}

/// Origin of a change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeOrigin {
    Local,
    Remote,
}

// =============================================================================
// Conflict Resolution
// =============================================================================

/// Conflict resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStrategy {
    LocalWins,
    RemoteWins,
    KeepBoth,
    AutoMerge,
    Manual,
    MostRecent,
}

/// A sync conflict that needs resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflict {
    pub id: String,
    pub session_id: String,
    pub local_version: ConflictVersion,
    pub remote_version: ConflictVersion,
    pub conflict_type: ConflictType,
    pub suggested_strategy: ConflictStrategy,
    pub created_at: DateTime<Utc>,
    pub resolved: bool,
}

/// Version info for conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictVersion {
    pub hash: String,
    pub timestamp: DateTime<Utc>,
    pub message_count: usize,
}

/// Type of conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    MessageEdit,
    SessionMetadata,
    Deletion,
    ConcurrentAdd,
}

/// Result of a sync operation
#[derive(Debug, Clone)]
pub enum SyncResult {
    NoChanges,
    Pushed,
    Pulled,
    Merged,
    ConflictDetected(SyncConflict),
}

// =============================================================================
// Hash Computation
// =============================================================================

/// Compute a content hash for a session
pub fn compute_session_hash(session: &ChatSession) -> String {
    let mut hasher = Sha256::new();

    // Hash session metadata
    let session_id = session.session_id.clone().unwrap_or_default();
    hasher.update(session_id.as_bytes());

    if let Some(title) = &session.custom_title {
        hasher.update(title.as_bytes());
    }

    hasher.update(session.last_message_date.to_le_bytes());

    // Hash requests content
    for request in &session.requests {
        if let Some(msg) = &request.message {
            if let Some(text) = &msg.text {
                hasher.update(text.as_bytes());
            }
        }
        if let Some(resp) = &request.response {
            if let Some(result) = resp.get("result").and_then(|v| v.as_str()) {
                hasher.update(result.as_bytes());
            }
        }
    }

    format!("{:x}", hasher.finalize())
}

// =============================================================================
// Bidirectional Sync Engine
// =============================================================================

/// Configuration for bidirectional sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidirectionalSyncConfig {
    pub conflict_strategy: ConflictStrategy,
    pub auto_sync_interval_secs: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub batch_size: usize,
}

impl Default for BidirectionalSyncConfig {
    fn default() -> Self {
        Self {
            conflict_strategy: ConflictStrategy::MostRecent,
            auto_sync_interval_secs: 300,
            max_retries: 3,
            retry_delay_ms: 1000,
            batch_size: 50,
        }
    }
}

/// Bidirectional sync engine
pub struct BidirectionalSyncEngine {
    config: BidirectionalSyncConfig,
    state: HashMap<String, SessionSyncState>,
    conflicts: Vec<SyncConflict>,
}

impl BidirectionalSyncEngine {
    pub fn new(config: BidirectionalSyncConfig) -> Self {
        Self {
            config,
            state: HashMap::new(),
            conflicts: Vec::new(),
        }
    }

    /// Check sync status for a session
    pub fn check_status(
        &self,
        session_id: &str,
        local_session: &ChatSession,
        remote_session: Option<&ChatSession>,
    ) -> SyncStatus {
        let local_hash = compute_session_hash(local_session);
        let remote_hash = remote_session.map(compute_session_hash).unwrap_or_default();

        match self.state.get(session_id) {
            None => SyncStatus::Unsynced,
            Some(prev_state) => {
                let local_changed = local_hash != prev_state.local_hash;
                let remote_changed = remote_hash != prev_state.remote_hash;

                match (local_changed, remote_changed) {
                    (false, false) => SyncStatus::Synced,
                    (true, false) => SyncStatus::LocalAhead,
                    (false, true) => SyncStatus::RemoteAhead,
                    (true, true) => SyncStatus::Conflict,
                }
            }
        }
    }

    /// Sync a session bidirectionally
    pub fn sync_session(
        &mut self,
        local_session: &mut ChatSession,
        remote_session: Option<&ChatSession>,
        push_fn: impl FnOnce(&ChatSession) -> Result<()>,
        pull_fn: impl FnOnce() -> Result<Option<ChatSession>>,
    ) -> Result<SyncResult> {
        let session_id = local_session.session_id.clone().unwrap_or_default();
        let status = self.check_status(&session_id, local_session, remote_session);

        match status {
            SyncStatus::Synced => Ok(SyncResult::NoChanges),

            SyncStatus::LocalAhead => {
                push_fn(local_session)?;
                self.update_state(&session_id, local_session, local_session);
                Ok(SyncResult::Pushed)
            }

            SyncStatus::RemoteAhead => {
                if let Some(remote) = pull_fn()? {
                    *local_session = remote.clone();
                    self.update_state(&session_id, local_session, &remote);
                    Ok(SyncResult::Pulled)
                } else {
                    Ok(SyncResult::NoChanges)
                }
            }

            SyncStatus::Conflict => {
                let remote = remote_session
                    .ok_or_else(|| anyhow!("Remote required for conflict resolution"))?;
                self.resolve_conflict(local_session, remote)
            }

            SyncStatus::Unsynced => {
                push_fn(local_session)?;
                self.update_state(&session_id, local_session, local_session);
                Ok(SyncResult::Pushed)
            }

            SyncStatus::Error => Err(anyhow!("Sync in error state")),
        }
    }

    /// Update sync state after successful sync
    fn update_state(&mut self, session_id: &str, local: &ChatSession, remote: &ChatSession) {
        let state = SessionSyncState {
            session_id: session_id.to_string(),
            provider: ProviderType::Custom,
            last_sync: Utc::now(),
            local_hash: compute_session_hash(local),
            remote_hash: compute_session_hash(remote),
            status: SyncStatus::Synced,
            pending_changes: Vec::new(),
        };
        self.state.insert(session_id.to_string(), state);
    }

    /// Resolve a conflict between local and remote versions
    fn resolve_conflict(
        &mut self,
        local_session: &mut ChatSession,
        remote_session: &ChatSession,
    ) -> Result<SyncResult> {
        let session_id = local_session.session_id.clone().unwrap_or_default();

        let conflict = SyncConflict {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.clone(),
            local_version: ConflictVersion {
                hash: compute_session_hash(local_session),
                timestamp: DateTime::from_timestamp_millis(local_session.last_message_date)
                    .unwrap_or_else(Utc::now),
                message_count: local_session.requests.len(),
            },
            remote_version: ConflictVersion {
                hash: compute_session_hash(remote_session),
                timestamp: DateTime::from_timestamp_millis(remote_session.last_message_date)
                    .unwrap_or_else(Utc::now),
                message_count: remote_session.requests.len(),
            },
            conflict_type: ConflictType::ConcurrentAdd,
            suggested_strategy: self.config.conflict_strategy,
            created_at: Utc::now(),
            resolved: false,
        };

        match self.config.conflict_strategy {
            ConflictStrategy::LocalWins => {
                self.update_state(&session_id, local_session, local_session);
                Ok(SyncResult::Pushed)
            }
            ConflictStrategy::RemoteWins => {
                *local_session = remote_session.clone();
                self.update_state(&session_id, local_session, remote_session);
                Ok(SyncResult::Pulled)
            }
            ConflictStrategy::MostRecent => {
                if local_session.last_message_date >= remote_session.last_message_date {
                    self.update_state(&session_id, local_session, local_session);
                    Ok(SyncResult::Pushed)
                } else {
                    *local_session = remote_session.clone();
                    self.update_state(&session_id, local_session, remote_session);
                    Ok(SyncResult::Pulled)
                }
            }
            ConflictStrategy::Manual => {
                self.conflicts.push(conflict.clone());
                Ok(SyncResult::ConflictDetected(conflict))
            }
            _ => {
                self.conflicts.push(conflict.clone());
                Ok(SyncResult::ConflictDetected(conflict))
            }
        }
    }

    /// Get all unresolved conflicts
    pub fn get_conflicts(&self) -> &[SyncConflict] {
        &self.conflicts
    }

    /// Resolve a conflict manually
    pub fn resolve_conflict_manually(
        &mut self,
        conflict_id: &str,
        _resolution: ConflictStrategy,
        resolved_session: ChatSession,
    ) -> Result<()> {
        if let Some(conflict) = self.conflicts.iter_mut().find(|c| c.id == conflict_id) {
            conflict.resolved = true;
            let session_id = resolved_session.session_id.clone().unwrap_or_default();
            self.update_state(&session_id, &resolved_session, &resolved_session);
            Ok(())
        } else {
            Err(anyhow!("Conflict not found: {}", conflict_id))
        }
    }

    /// Get sync state for a session
    pub fn get_state(&self, session_id: &str) -> Option<&SessionSyncState> {
        self.state.get(session_id)
    }
}

// =============================================================================
// Provider-specific Sync Adapters
// =============================================================================

/// Trait for provider-specific sync operations
pub trait ProviderSyncAdapter: Send + Sync {
    fn provider_type(&self) -> ProviderType;
    fn push_session(&self, session: &ChatSession) -> Result<()>;
    fn pull_session(&self, session_id: &str) -> Result<Option<ChatSession>>;
    fn list_remote_sessions(&self) -> Result<Vec<String>>;
    fn delete_remote_session(&self, session_id: &str) -> Result<()>;
}

/// VSCode Copilot Chat sync adapter
pub struct VSCodeSyncAdapter {
    workspace_path: PathBuf,
}

impl VSCodeSyncAdapter {
    pub fn new(workspace_path: PathBuf) -> Self {
        Self { workspace_path }
    }

    fn sessions_dir(&self) -> PathBuf {
        self.workspace_path.join("chatSessions")
    }
}

impl ProviderSyncAdapter for VSCodeSyncAdapter {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Copilot
    }

    fn push_session(&self, session: &ChatSession) -> Result<()> {
        let session_id = session.session_id.clone().unwrap_or_default();
        let path = self.sessions_dir().join(format!("{}.json", session_id));
        std::fs::create_dir_all(self.sessions_dir())?;
        let json = serde_json::to_string_pretty(session)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    fn pull_session(&self, session_id: &str) -> Result<Option<ChatSession>> {
        let path = self.sessions_dir().join(format!("{}.json", session_id));
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)?;
        let session: ChatSession = serde_json::from_str(&content)?;
        Ok(Some(session))
    }

    fn list_remote_sessions(&self) -> Result<Vec<String>> {
        let dir = self.sessions_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut sessions = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".json") {
                    sessions.push(name.trim_end_matches(".json").to_string());
                }
            }
        }
        Ok(sessions)
    }

    fn delete_remote_session(&self, session_id: &str) -> Result<()> {
        let path = self.sessions_dir().join(format!("{}.json", session_id));
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_status() {
        assert_eq!(SyncStatus::Synced, SyncStatus::Synced);
    }

    #[test]
    fn test_config_default() {
        let config = BidirectionalSyncConfig::default();
        assert_eq!(config.auto_sync_interval_secs, 300);
    }

    #[test]
    fn test_engine_creation() {
        let engine = BidirectionalSyncEngine::new(BidirectionalSyncConfig::default());
        assert!(engine.conflicts.is_empty());
    }
}
