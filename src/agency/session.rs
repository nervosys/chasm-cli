// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Session Management
//!
//! Manages conversation sessions with persistent state.

#![allow(dead_code)]

use crate::agency::error::{AgencyError, AgencyResult};
use crate::agency::models::{AgencyMessage, MessageRole, TokenUsage};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Session state (JSON-serializable key-value store)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionState {
    /// Arbitrary state data
    pub data: HashMap<String, serde_json::Value>,
}

impl SessionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn set<T: Serialize>(&mut self, key: impl Into<String>, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.data.insert(key.into(), v);
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.data.remove(key)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

/// A conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session ID
    pub id: String,
    /// Associated agent name
    pub agent_name: String,
    /// User ID (optional)
    #[serde(default)]
    pub user_id: Option<String>,
    /// Session title/name
    #[serde(default)]
    pub title: Option<String>,
    /// Conversation messages
    pub messages: Vec<AgencyMessage>,
    /// Session state
    #[serde(default)]
    pub state: SessionState,
    /// Token usage
    #[serde(default)]
    pub token_usage: TokenUsage,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Custom metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Session {
    /// Create a new session
    pub fn new(agent_name: impl Into<String>, user_id: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: generate_session_id(),
            agent_name: agent_name.into(),
            user_id,
            title: None,
            messages: Vec::new(),
            state: SessionState::new(),
            token_usage: TokenUsage::default(),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Add a message to the session
    pub fn add_message(&mut self, message: AgencyMessage) {
        if let Some(tokens) = message.tokens {
            self.token_usage.total_tokens += tokens;
            match message.role {
                MessageRole::User | MessageRole::System => {
                    self.token_usage.prompt_tokens += tokens;
                }
                MessageRole::Assistant | MessageRole::Tool => {
                    self.token_usage.completion_tokens += tokens;
                }
            }
        }
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    /// Get messages formatted for model API
    pub fn to_api_messages(&self) -> Vec<serde_json::Value> {
        self.messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role.to_string(),
                    "content": m.content
                })
            })
            .collect()
    }

    /// Get the last N messages
    pub fn last_messages(&self, n: usize) -> &[AgencyMessage] {
        let start = self.messages.len().saturating_sub(n);
        &self.messages[start..]
    }

    /// Clear messages but keep state
    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.token_usage = TokenUsage::default();
        self.updated_at = Utc::now();
    }

    /// Rewind to before a specific message
    pub fn rewind_to(&mut self, message_id: &str) -> Option<Vec<AgencyMessage>> {
        if let Some(pos) = self.messages.iter().position(|m| m.id == message_id) {
            let removed: Vec<_> = self.messages.drain(pos..).collect();
            self.updated_at = Utc::now();
            // Recalculate token usage
            self.recalculate_tokens();
            Some(removed)
        } else {
            None
        }
    }

    fn recalculate_tokens(&mut self) {
        let mut usage = TokenUsage::default();
        for m in &self.messages {
            if let Some(tokens) = m.tokens {
                usage.total_tokens += tokens;
                match m.role {
                    MessageRole::User | MessageRole::System => {
                        usage.prompt_tokens += tokens;
                    }
                    MessageRole::Assistant | MessageRole::Tool => {
                        usage.completion_tokens += tokens;
                    }
                }
            }
        }
        self.token_usage = usage;
    }
}

/// Generate a unique session ID
fn generate_session_id() -> String {
    format!(
        "session-{}-{}",
        Utc::now().timestamp_millis(),
        &uuid::Uuid::new_v4().to_string()[..8]
    )
}

/// Generate a unique message ID
pub fn generate_message_id() -> String {
    format!(
        "msg-{}-{}",
        Utc::now().timestamp_millis(),
        &uuid::Uuid::new_v4().to_string()[..8]
    )
}

/// Session manager with SQLite persistence
pub struct SessionManager {
    conn: Arc<Mutex<Connection>>,
}

impl SessionManager {
    /// Create a new session manager with the given database path
    pub fn new(db_path: impl AsRef<Path>) -> AgencyResult<Self> {
        let conn = Connection::open(db_path)?;
        let manager = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        manager.init_schema()?;
        Ok(manager)
    }

    /// Create an in-memory session manager (for testing)
    pub fn in_memory() -> AgencyResult<Self> {
        let conn = Connection::open_in_memory()?;
        let manager = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        manager.init_schema()?;
        Ok(manager)
    }

    /// Initialize database schema
    fn init_schema(&self) -> AgencyResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AgencyError::DatabaseError(e.to_string()))?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS Agency_sessions (
                id TEXT PRIMARY KEY,
                agent_name TEXT NOT NULL,
                user_id TEXT,
                title TEXT,
                messages TEXT NOT NULL,
                state TEXT NOT NULL,
                token_usage TEXT NOT NULL,
                metadata TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_Agency_sessions_agent ON Agency_sessions(agent_name);
            CREATE INDEX IF NOT EXISTS idx_Agency_sessions_user ON Agency_sessions(user_id);
            CREATE INDEX IF NOT EXISTS idx_Agency_sessions_updated ON Agency_sessions(updated_at DESC);
            "#,
        )?;
        Ok(())
    }

    /// Create a new session
    pub fn create(
        &self,
        agent_name: impl Into<String>,
        user_id: Option<String>,
    ) -> AgencyResult<Session> {
        let session = Session::new(agent_name, user_id);
        self.save(&session)?;
        Ok(session)
    }

    /// Save a session
    pub fn save(&self, session: &Session) -> AgencyResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AgencyError::DatabaseError(e.to_string()))?;
        conn.execute(
            r#"
            INSERT OR REPLACE INTO Agency_sessions 
            (id, agent_name, user_id, title, messages, state, token_usage, metadata, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                session.id,
                session.agent_name,
                session.user_id,
                session.title,
                serde_json::to_string(&session.messages)?,
                serde_json::to_string(&session.state)?,
                serde_json::to_string(&session.token_usage)?,
                serde_json::to_string(&session.metadata)?,
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get a session by ID
    pub fn get(&self, id: &str) -> AgencyResult<Option<Session>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AgencyError::DatabaseError(e.to_string()))?;
        let session = conn
            .query_row(
                "SELECT * FROM Agency_sessions WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Session {
                        id: row.get(0)?,
                        agent_name: row.get(1)?,
                        user_id: row.get(2)?,
                        title: row.get(3)?,
                        messages: serde_json::from_str(&row.get::<_, String>(4)?)
                            .unwrap_or_default(),
                        state: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                        token_usage: serde_json::from_str(&row.get::<_, String>(6)?)
                            .unwrap_or_default(),
                        metadata: serde_json::from_str(&row.get::<_, String>(7)?)
                            .unwrap_or_default(),
                        created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                    })
                },
            )
            .optional()?;
        Ok(session)
    }

    /// List sessions for an agent
    pub fn list_by_agent(
        &self,
        agent_name: &str,
        limit: Option<u32>,
    ) -> AgencyResult<Vec<Session>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AgencyError::DatabaseError(e.to_string()))?;
        let limit = limit.unwrap_or(100);
        let mut stmt = conn.prepare(
            "SELECT * FROM Agency_sessions WHERE agent_name = ?1 ORDER BY updated_at DESC LIMIT ?2",
        )?;
        let sessions = stmt
            .query_map(params![agent_name, limit], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    agent_name: row.get(1)?,
                    user_id: row.get(2)?,
                    title: row.get(3)?,
                    messages: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                    state: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                    token_usage: serde_json::from_str(&row.get::<_, String>(6)?)
                        .unwrap_or_default(),
                    metadata: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(sessions)
    }

    /// List sessions for a user
    pub fn list_by_user(&self, user_id: &str, limit: Option<u32>) -> AgencyResult<Vec<Session>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AgencyError::DatabaseError(e.to_string()))?;
        let limit = limit.unwrap_or(100);
        let mut stmt = conn.prepare(
            "SELECT * FROM Agency_sessions WHERE user_id = ?1 ORDER BY updated_at DESC LIMIT ?2",
        )?;
        let sessions = stmt
            .query_map(params![user_id, limit], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    agent_name: row.get(1)?,
                    user_id: row.get(2)?,
                    title: row.get(3)?,
                    messages: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                    state: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                    token_usage: serde_json::from_str(&row.get::<_, String>(6)?)
                        .unwrap_or_default(),
                    metadata: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(sessions)
    }

    /// Delete a session
    pub fn delete(&self, id: &str) -> AgencyResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AgencyError::DatabaseError(e.to_string()))?;
        let rows = conn.execute("DELETE FROM Agency_sessions WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state() {
        let mut state = SessionState::new();
        state.set("count", 42);
        state.set("name", "test");

        assert_eq!(state.get::<i32>("count"), Some(42));
        assert_eq!(state.get::<String>("name"), Some("test".to_string()));
        assert!(state.contains("count"));
        assert!(!state.contains("missing"));
    }

    #[test]
    fn test_session_messages() {
        let mut session = Session::new("test_agent", None);
        session.add_message(AgencyMessage {
            id: "msg1".to_string(),
            role: MessageRole::User,
            content: "Hello".to_string(),
            tool_calls: vec![],
            tool_result: None,
            timestamp: Utc::now(),
            tokens: Some(5),
            agent_name: None,
            metadata: HashMap::new(),
        });

        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.token_usage.prompt_tokens, 5);
    }

    #[test]
    fn test_session_manager() -> AgencyResult<()> {
        let manager = SessionManager::in_memory()?;
        let session = manager.create("test_agent", Some("user1".to_string()))?;

        let loaded = manager.get(&session.id)?;
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().agent_name, "test_agent");

        let sessions = manager.list_by_agent("test_agent", None)?;
        assert_eq!(sessions.len(), 1);

        manager.delete(&session.id)?;
        let deleted = manager.get(&session.id)?;
        assert!(deleted.is_none());

        Ok(())
    }
}
