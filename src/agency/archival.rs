// Copyright (c) 2024-2027 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Autonomous Session Archival Agent
//!
//! An AI agent that automatically archives old or inactive sessions based on
//! configurable rules and policies.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agency::{Agent, AgentBuilder, AgentConfig, AgentRole, AgentStatus};
use crate::database::ChatDatabase;

/// Archival policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalPolicy {
    /// Policy name
    pub name: String,
    /// Whether policy is enabled
    pub enabled: bool,
    /// Days of inactivity before archival
    pub inactive_days: u32,
    /// Minimum message count to archive (skip small sessions)
    pub min_messages: u32,
    /// Maximum message count (archive large sessions sooner)
    pub max_messages: Option<u32>,
    /// Providers to include (empty = all)
    pub providers: Vec<String>,
    /// Workspaces to include (empty = all)
    pub workspace_ids: Vec<String>,
    /// Tags that prevent archival
    pub exclude_tags: Vec<String>,
    /// Tags that trigger immediate archival
    pub include_tags: Vec<String>,
    /// Whether to compress archived sessions
    pub compress: bool,
    /// Whether to notify on archival
    pub notify: bool,
}

impl Default for ArchivalPolicy {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            enabled: true,
            inactive_days: 30,
            min_messages: 5,
            max_messages: None,
            providers: vec![],
            workspace_ids: vec![],
            exclude_tags: vec!["pinned".to_string(), "important".to_string()],
            include_tags: vec!["archive".to_string()],
            compress: true,
            notify: true,
        }
    }
}

/// Session candidate for archival
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalCandidate {
    /// Session ID
    pub session_id: String,
    /// Session title
    pub title: String,
    /// Provider
    pub provider: String,
    /// Workspace ID
    pub workspace_id: Option<String>,
    /// Message count
    pub message_count: u32,
    /// Last activity
    pub last_activity: DateTime<Utc>,
    /// Days inactive
    pub days_inactive: u32,
    /// Matching policy
    pub policy: String,
    /// Reason for archival
    pub reason: String,
    /// Priority (higher = archive sooner)
    pub priority: u8,
}

/// Archival decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalDecision {
    /// Session ID
    pub session_id: String,
    /// Whether to archive
    pub should_archive: bool,
    /// Confidence (0.0 - 1.0)
    pub confidence: f64,
    /// Reasoning
    pub reasoning: String,
    /// Matched policies
    pub policies: Vec<String>,
}

/// Archival result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalResult {
    /// Sessions archived
    pub archived_count: u32,
    /// Sessions skipped
    pub skipped_count: u32,
    /// Total size saved (bytes)
    pub bytes_saved: u64,
    /// Errors
    pub errors: Vec<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Duration (ms)
    pub duration_ms: u64,
}

/// Archival agent state
pub struct ArchivalAgentState {
    /// Policies
    policies: Vec<ArchivalPolicy>,
    /// Last run time
    last_run: Option<DateTime<Utc>>,
    /// Statistics
    stats: ArchivalStats,
    /// Pending candidates
    pending: Vec<ArchivalCandidate>,
}

/// Archival statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArchivalStats {
    /// Total runs
    pub total_runs: u64,
    /// Total archived
    pub total_archived: u64,
    /// Total skipped
    pub total_skipped: u64,
    /// Total bytes saved
    pub total_bytes_saved: u64,
    /// Average confidence
    pub avg_confidence: f64,
}

/// Autonomous session archival agent
pub struct ArchivalAgent {
    /// Agent configuration
    config: AgentConfig,
    /// Agent state
    state: Arc<RwLock<ArchivalAgentState>>,
    /// Database reference
    db: Option<Arc<ChatDatabase>>,
    /// Whether agent is running
    running: Arc<RwLock<bool>>,
}

impl ArchivalAgent {
    /// Create a new archival agent
    pub fn new() -> Self {
        let config = AgentConfig {
            name: "archival-agent".to_string(),
            description: "Autonomous session archival agent".to_string(),
            instruction: ARCHIVAL_SYSTEM_PROMPT.to_string(),
            ..Default::default()
        };

        let state = ArchivalAgentState {
            policies: vec![ArchivalPolicy::default()],
            last_run: None,
            stats: ArchivalStats::default(),
            pending: vec![],
        };

        Self {
            config,
            state: Arc::new(RwLock::new(state)),
            db: None,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Create with custom policies
    pub fn with_policies(policies: Vec<ArchivalPolicy>) -> Self {
        let agent = Self::new();
        let mut state = agent.state.blocking_write();
        state.policies = policies;
        drop(state);
        agent
    }

    /// Set database reference
    pub fn with_database(mut self, db: Arc<ChatDatabase>) -> Self {
        self.db = Some(db);
        self
    }

    /// Add a policy
    pub async fn add_policy(&self, policy: ArchivalPolicy) {
        let mut state = self.state.write().await;
        state.policies.push(policy);
    }

    /// Remove a policy by name
    pub async fn remove_policy(&self, name: &str) -> bool {
        let mut state = self.state.write().await;
        let len_before = state.policies.len();
        state.policies.retain(|p| p.name != name);
        state.policies.len() < len_before
    }

    /// Get all policies
    pub async fn get_policies(&self) -> Vec<ArchivalPolicy> {
        let state = self.state.read().await;
        state.policies.clone()
    }

    /// Scan for archival candidates
    pub async fn scan_candidates(&self) -> Vec<ArchivalCandidate> {
        let state = self.state.read().await;
        let candidates = Vec::new();
        let _now = Utc::now();

        // In real implementation, query database for sessions
        // For now, return placeholder logic
        for policy in &state.policies {
            if !policy.enabled {
                continue;
            }

            // Would query: SELECT * FROM sessions WHERE
            // - updated_at < now - inactive_days
            // - message_count >= min_messages
            // - NOT archived
            // - provider IN policies.providers (if specified)
            // - workspace_id IN policies.workspace_ids (if specified)
            // - tags NOT IN exclude_tags
        }

        candidates
    }

    /// Evaluate a session for archival
    pub async fn evaluate_session(&self, session_id: &str) -> ArchivalDecision {
        let state = self.state.read().await;
        let mut matched_policies = Vec::new();
        let mut reasons = Vec::new();
        let mut should_archive = false;
        let mut confidence = 0.0;

        // Check against all enabled policies
        for policy in &state.policies {
            if !policy.enabled {
                continue;
            }

            // In real implementation:
            // 1. Fetch session from database
            // 2. Check each policy condition
            // 3. Use LLM for nuanced decisions if needed

            // Placeholder decision logic
            matched_policies.push(policy.name.clone());
        }

        if !matched_policies.is_empty() {
            should_archive = true;
            confidence = 0.85;
            reasons.push("Matched archival policies".to_string());
        }

        ArchivalDecision {
            session_id: session_id.to_string(),
            should_archive,
            confidence,
            reasoning: reasons.join("; "),
            policies: matched_policies,
        }
    }

    /// Archive a single session
    pub async fn archive_session(&self, _session_id: &str) -> Result<bool, String> {
        // In real implementation:
        // 1. Mark session as archived in database
        // 2. Optionally compress/export
        // 3. Update statistics

        let mut state = self.state.write().await;
        state.stats.total_archived += 1;

        Ok(true)
    }

    /// Run the archival agent
    pub async fn run(&self) -> ArchivalResult {
        let start = std::time::Instant::now();
        let mut result = ArchivalResult {
            archived_count: 0,
            skipped_count: 0,
            bytes_saved: 0,
            errors: vec![],
            timestamp: Utc::now(),
            duration_ms: 0,
        };

        // Set running flag
        {
            let mut running = self.running.write().await;
            if *running {
                result.errors.push("Agent already running".to_string());
                return result;
            }
            *running = true;
        }

        // Scan for candidates
        let candidates = self.scan_candidates().await;

        // Evaluate and archive each candidate
        for candidate in candidates {
            let decision = self.evaluate_session(&candidate.session_id).await;

            if decision.should_archive && decision.confidence >= 0.7 {
                match self.archive_session(&candidate.session_id).await {
                    Ok(true) => {
                        result.archived_count += 1;
                    }
                    Ok(false) => {
                        result.skipped_count += 1;
                    }
                    Err(e) => {
                        result
                            .errors
                            .push(format!("Failed to archive {}: {}", candidate.session_id, e));
                    }
                }
            } else {
                result.skipped_count += 1;
            }
        }

        // Update state
        {
            let mut state = self.state.write().await;
            state.last_run = Some(Utc::now());
            state.stats.total_runs += 1;
            state.stats.total_bytes_saved += result.bytes_saved;
        }

        // Clear running flag
        {
            let mut running = self.running.write().await;
            *running = false;
        }

        result.duration_ms = start.elapsed().as_millis() as u64;
        result
    }

    /// Get agent statistics
    pub async fn get_stats(&self) -> ArchivalStats {
        let state = self.state.read().await;
        state.stats.clone()
    }

    /// Get last run time
    pub async fn get_last_run(&self) -> Option<DateTime<Utc>> {
        let state = self.state.read().await;
        state.last_run
    }

    /// Check if agent is running
    pub async fn is_running(&self) -> bool {
        let running = self.running.read().await;
        *running
    }

    /// Stop the agent
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }
}

impl Default for ArchivalAgent {
    fn default() -> Self {
        Self::new()
    }
}

/// System prompt for the archival agent
const ARCHIVAL_SYSTEM_PROMPT: &str = r#"You are an autonomous session archival agent for Chasm.

Your role is to analyze chat sessions and determine which should be archived based on:
1. Inactivity period (days since last message)
2. Session size and importance
3. Content relevance and quality
4. User-defined policies and tags

When evaluating a session for archival, consider:
- Is the conversation complete or ongoing?
- Does it contain important information that should be preserved?
- Are there pinned or important tags?
- How much space would archiving save?

Provide clear reasoning for your archival decisions.
"#;

/// Archival scheduler for periodic runs
pub struct ArchivalScheduler {
    /// Agent reference
    agent: Arc<ArchivalAgent>,
    /// Run interval
    interval: Duration,
    /// Whether scheduler is active
    active: Arc<RwLock<bool>>,
}

impl ArchivalScheduler {
    /// Create a new scheduler
    pub fn new(agent: Arc<ArchivalAgent>, interval_hours: u32) -> Self {
        Self {
            agent,
            interval: Duration::hours(interval_hours as i64),
            active: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the scheduler
    /// Note: This currently logs a start message. Full background scheduling
    /// requires a LocalSet or refactoring ChatDatabase for Send+Sync.
    pub async fn start(&self) {
        let mut active = self.active.write().await;
        *active = true;
        drop(active);

        // TODO: Implement background scheduling with LocalSet
        // For now, just mark as active - call run() manually
        println!(
            "[ArchivalScheduler] Started with interval {:?}. Call run() to execute.",
            self.interval
        );
    }

    /// Stop the scheduler
    pub async fn stop(&self) {
        let mut active = self.active.write().await;
        *active = false;
    }

    /// Check if scheduler is active
    pub async fn is_active(&self) -> bool {
        let active = self.active.read().await;
        *active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_archival_agent_creation() {
        let agent = ArchivalAgent::new();
        let policies = agent.get_policies().await;
        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0].name, "default");
    }

    #[tokio::test]
    async fn test_add_remove_policy() {
        let agent = ArchivalAgent::new();

        let custom_policy = ArchivalPolicy {
            name: "aggressive".to_string(),
            inactive_days: 7,
            ..Default::default()
        };

        agent.add_policy(custom_policy).await;
        let policies = agent.get_policies().await;
        assert_eq!(policies.len(), 2);

        agent.remove_policy("aggressive").await;
        let policies = agent.get_policies().await;
        assert_eq!(policies.len(), 1);
    }

    #[tokio::test]
    async fn test_evaluate_session() {
        let agent = ArchivalAgent::new();
        let decision = agent.evaluate_session("test-session-123").await;
        assert!(!decision.session_id.is_empty());
    }
}
