// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Universal Chat Session Database
//!
//! A unified database schema for storing, versioning, and tracking chat sessions
//! from multiple providers. This serves as an intermediate representation that
//! normalizes data from various sources (VS Code, web providers, share links, etc.)
//!
//! Note: Many types and methods are infrastructure for future integration.
#![allow(dead_code)]

//! ## Schema Overview
//!
//! ```text
//! +-----------------+     +-----------------+     +-----------------+
//! |   Workspaces    |----<|    Sessions     |----<|    Messages     |
//! +-----------------+     +-----------------+     +-----------------+
//!                                |
//!                                v
//!                        +-----------------+
//!                        |   Checkpoints   |
//!                        +-----------------+
//!                                |
//!                                v
//!                        +-----------------+
//!                        |   ShareLinks    |
//!                        +-----------------+
//! ```

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Database schema version
pub const SCHEMA_VERSION: &str = "3.0";

// =============================================================================
// Database Models
// =============================================================================

/// A workspace/project that contains chat sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub path: Option<String>,
    pub provider: String,
    pub provider_workspace_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub metadata: Option<String>, // JSON blob for provider-specific data
}

/// A chat session containing messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub workspace_id: Option<String>,
    pub provider: String,
    pub provider_session_id: Option<String>,
    pub title: String,
    pub model: Option<String>,
    pub message_count: i32,
    pub token_count: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub archived: bool,
    pub metadata: Option<String>, // JSON blob for provider-specific data
}

/// A message within a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: String, // "user", "assistant", "system"
    pub content: String,
    pub model: Option<String>,
    pub token_count: Option<i32>,
    pub created_at: i64,
    pub parent_id: Option<String>, // For branching conversations
    pub metadata: Option<String>,
}

/// A checkpoint/snapshot of a session at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub session_id: String,
    pub name: String,
    pub description: Option<String>,
    pub message_count: i32,
    pub session_snapshot: String, // JSON of session state
    pub created_at: i64,
    pub git_commit: Option<String>,
}

/// A share link for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareLink {
    pub id: String,
    pub session_id: Option<String>, // Linked session after import
    pub provider: String,
    pub url: String,
    pub share_id: String, // Provider-specific share ID extracted from URL
    pub title: Option<String>,
    pub imported: bool,
    pub imported_at: Option<i64>,
    pub created_at: i64,
    pub metadata: Option<String>,
}

/// Supported share link providers
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShareLinkProvider {
    ChatGPT,
    Claude,
    Gemini,
    Perplexity,
    Poe,
    Other(String),
}

impl ShareLinkProvider {
    /// Parse a URL to determine the provider
    pub fn from_url(url: &str) -> Option<(Self, String)> {
        let url_lower = url.to_lowercase();

        // ChatGPT: https://chat.openai.com/share/abc123 or https://chatgpt.com/share/abc123
        if url_lower.contains("chat.openai.com/share/") || url_lower.contains("chatgpt.com/share/")
        {
            if let Some(id) = extract_path_segment(url, "share") {
                return Some((ShareLinkProvider::ChatGPT, id));
            }
        }

        // Claude: https://claude.ai/share/abc123
        if url_lower.contains("claude.ai/share/") {
            if let Some(id) = extract_path_segment(url, "share") {
                return Some((ShareLinkProvider::Claude, id));
            }
        }

        // Gemini: https://g.co/gemini/share/abc123 or https://gemini.google.com/share/abc123
        if url_lower.contains("g.co/gemini/share/")
            || url_lower.contains("gemini.google.com/share/")
        {
            if let Some(id) = extract_path_segment(url, "share") {
                return Some((ShareLinkProvider::Gemini, id));
            }
        }

        // Perplexity: https://www.perplexity.ai/search/abc123
        if url_lower.contains("perplexity.ai/search/") {
            if let Some(id) = extract_path_segment(url, "search") {
                return Some((ShareLinkProvider::Perplexity, id));
            }
        }

        // Poe: https://poe.com/s/abc123
        if url_lower.contains("poe.com/s/") {
            if let Some(id) = extract_path_segment(url, "s") {
                return Some((ShareLinkProvider::Poe, id));
            }
        }

        None
    }

    pub fn name(&self) -> &str {
        match self {
            ShareLinkProvider::ChatGPT => "ChatGPT",
            ShareLinkProvider::Claude => "Claude",
            ShareLinkProvider::Gemini => "Gemini",
            ShareLinkProvider::Perplexity => "Perplexity",
            ShareLinkProvider::Poe => "Poe",
            ShareLinkProvider::Other(name) => name,
        }
    }
}

/// Parsed share link information
#[derive(Debug, Clone)]
pub struct ShareLinkInfo {
    pub provider: String,
    pub share_id: String,
}

/// Parser for share link URLs
pub struct ShareLinkParser;

impl ShareLinkParser {
    /// Parse a URL to extract provider and share ID
    pub fn parse(url: &str) -> Option<ShareLinkInfo> {
        ShareLinkProvider::from_url(url).map(|(provider, share_id)| ShareLinkInfo {
            provider: provider.name().to_string(),
            share_id,
        })
    }
}

/// Extract a path segment after a given key
fn extract_path_segment(url: &str, key: &str) -> Option<String> {
    let parts: Vec<&str> = url.split('/').collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == key && i + 1 < parts.len() {
            let id = parts[i + 1].split('?').next().unwrap_or(parts[i + 1]);
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
    }
    None
}

// =============================================================================
// Database Operations
// =============================================================================

/// Universal Chat Database manager
pub struct ChatDatabase {
    pub conn: Connection,
}

impl ChatDatabase {
    /// Open or create a database at the given path
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).context("Failed to open database")?;

        let db = ChatDatabase { conn };
        db.initialize()?;

        Ok(db)
    }

    /// Open an in-memory database (for testing)
    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("Failed to create in-memory database")?;

        let db = ChatDatabase { conn };
        db.initialize()?;

        Ok(db)
    }

    /// Initialize the database schema
    fn initialize(&self) -> Result<()> {
        // Check if this is a harvest database (has sessions but missing 'model' column)
        // If so, skip full schema initialization to preserve harvest data
        let is_harvest_db = self
            .conn
            .query_row("SELECT 1 FROM sessions LIMIT 1", [], |_| Ok(true))
            .is_ok();

        let has_model_column = self
            .conn
            .query_row("SELECT model FROM sessions LIMIT 1", [], |_| Ok(true))
            .is_ok();

        // Only apply full schema if not a harvest database, or if it's a fresh database
        if !is_harvest_db || has_model_column {
            self.conn
                .execute_batch(include_str!("sql/schema.sql"))
                .context("Failed to initialize database schema")?;
        }
        // For harvest databases, ensure we have the tables we need for API
        // (agents, metadata tables might be missing)
        else {
            // Create minimal additional tables needed for API functionality
            self.conn
                .execute_batch(
                    r#"
                -- Metadata table for version tracking
                CREATE TABLE IF NOT EXISTS metadata (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL,
                    updated_at INTEGER DEFAULT (strftime('%s', 'now'))
                );
                INSERT OR IGNORE INTO metadata (key, value) VALUES ('schema_version', 'harvest');
                
                -- Agents table for agent management
                CREATE TABLE IF NOT EXISTS agents (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    description TEXT,
                    instruction TEXT NOT NULL,
                    role TEXT DEFAULT 'assistant',
                    model TEXT,
                    provider TEXT,
                    temperature REAL DEFAULT 0.7,
                    max_tokens INTEGER,
                    tools TEXT,
                    sub_agents TEXT,
                    is_active INTEGER DEFAULT 1,
                    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                    metadata TEXT
                );
                "#,
                )
                .context("Failed to initialize harvest-compatible schema")?;
        }
        Ok(())
    }

    /// Get access to the underlying connection
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Get schema version
    pub fn get_version(&self) -> Result<String> {
        let version: String = self
            .conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "unknown".to_string());
        Ok(version)
    }

    // -------------------------------------------------------------------------
    // Workspace Operations
    // -------------------------------------------------------------------------

    /// Insert or update a workspace
    pub fn upsert_workspace(&self, workspace: &Workspace) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO workspaces (id, name, path, provider, provider_workspace_id, 
                                    created_at, updated_at, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                path = excluded.path,
                updated_at = excluded.updated_at,
                metadata = excluded.metadata
            "#,
            params![
                workspace.id,
                workspace.name,
                workspace.path,
                workspace.provider,
                workspace.provider_workspace_id,
                workspace.created_at,
                workspace.updated_at,
                workspace.metadata,
            ],
        )?;
        Ok(())
    }

    /// Get a workspace by ID
    pub fn get_workspace(&self, id: &str) -> Result<Option<Workspace>> {
        self.conn
            .query_row(
                "SELECT id, name, path, provider, provider_workspace_id, created_at, updated_at, metadata 
                 FROM workspaces WHERE id = ?",
                [id],
                |row| {
                    Ok(Workspace {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        path: row.get(2)?,
                        provider: row.get(3)?,
                        provider_workspace_id: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                        metadata: row.get(7)?,
                    })
                },
            )
            .optional()
            .context("Failed to get workspace")
    }

    /// List all workspaces
    pub fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, provider, provider_workspace_id, created_at, updated_at, metadata 
             FROM workspaces ORDER BY updated_at DESC"
        )?;

        let workspaces = stmt
            .query_map([], |row| {
                Ok(Workspace {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    provider: row.get(3)?,
                    provider_workspace_id: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    metadata: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(workspaces)
    }

    // -------------------------------------------------------------------------
    // Session Operations
    // -------------------------------------------------------------------------

    /// Insert or update a session
    pub fn upsert_session(&self, session: &Session) -> Result<bool> {
        let existing = self.get_session(&session.id)?;

        self.conn.execute(
            r#"
            INSERT INTO sessions (id, workspace_id, provider, provider_session_id, title,
                                  model, message_count, token_count, created_at, updated_at,
                                  archived, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                model = excluded.model,
                message_count = excluded.message_count,
                token_count = excluded.token_count,
                updated_at = excluded.updated_at,
                archived = excluded.archived,
                metadata = excluded.metadata
            "#,
            params![
                session.id,
                session.workspace_id,
                session.provider,
                session.provider_session_id,
                session.title,
                session.model,
                session.message_count,
                session.token_count,
                session.created_at,
                session.updated_at,
                session.archived,
                session.metadata,
            ],
        )?;

        Ok(existing.is_some())
    }

    /// Get a session by ID
    pub fn get_session(&self, id: &str) -> Result<Option<Session>> {
        self.conn
            .query_row(
                "SELECT id, workspace_id, provider, provider_session_id, title, model,
                        message_count, token_count, created_at, updated_at, archived, metadata
                 FROM sessions WHERE id = ?",
                [id],
                |row| {
                    Ok(Session {
                        id: row.get(0)?,
                        workspace_id: row.get(1)?,
                        provider: row.get(2)?,
                        provider_session_id: row.get(3)?,
                        title: row.get(4)?,
                        model: row.get(5)?,
                        message_count: row.get(6)?,
                        token_count: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                        archived: row.get(10)?,
                        metadata: row.get(11)?,
                    })
                },
            )
            .optional()
            .context("Failed to get session")
    }

    /// List sessions with optional filters
    pub fn list_sessions(
        &self,
        workspace_id: Option<&str>,
        provider: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Session>> {
        let mut query = String::from(
            "SELECT id, workspace_id, provider, provider_session_id, title, model,
                    message_count, token_count, created_at, updated_at, archived, metadata
             FROM sessions WHERE 1=1",
        );

        if workspace_id.is_some() {
            query.push_str(" AND workspace_id = ?1");
        }
        if provider.is_some() {
            query.push_str(" AND provider = ?2");
        }
        query.push_str(" ORDER BY updated_at DESC LIMIT ?3");

        let mut stmt = self.conn.prepare(&query)?;

        let sessions = stmt
            .query_map(
                params![
                    workspace_id.unwrap_or(""),
                    provider.unwrap_or(""),
                    limit as i64,
                ],
                |row| {
                    Ok(Session {
                        id: row.get(0)?,
                        workspace_id: row.get(1)?,
                        provider: row.get(2)?,
                        provider_session_id: row.get(3)?,
                        title: row.get(4)?,
                        model: row.get(5)?,
                        message_count: row.get(6)?,
                        token_count: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                        archived: row.get(10)?,
                        metadata: row.get(11)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    /// Count sessions by provider
    pub fn count_sessions_by_provider(&self) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT provider, COUNT(*) FROM sessions GROUP BY provider ORDER BY COUNT(*) DESC",
        )?;

        let counts = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(counts)
    }

    // -------------------------------------------------------------------------
    // Message Operations
    // -------------------------------------------------------------------------

    /// Insert a message
    pub fn insert_message(&self, message: &Message) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO messages (id, session_id, role, content, model, token_count,
                                  created_at, parent_id, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(id) DO UPDATE SET
                content = excluded.content,
                metadata = excluded.metadata
            "#,
            params![
                message.id,
                message.session_id,
                message.role,
                message.content,
                message.model,
                message.token_count,
                message.created_at,
                message.parent_id,
                message.metadata,
            ],
        )?;
        Ok(())
    }

    /// Get messages for a session
    pub fn get_messages(&self, session_id: &str) -> Result<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, role, content, model, token_count, created_at, parent_id, metadata
             FROM messages WHERE session_id = ? ORDER BY created_at ASC"
        )?;

        let messages = stmt
            .query_map([session_id], |row| {
                Ok(Message {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    model: row.get(4)?,
                    token_count: row.get(5)?,
                    created_at: row.get(6)?,
                    parent_id: row.get(7)?,
                    metadata: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    // -------------------------------------------------------------------------
    // Checkpoint Operations
    // -------------------------------------------------------------------------

    /// Create a checkpoint for a session
    pub fn create_checkpoint(
        &self,
        session_id: &str,
        name: &str,
        description: Option<&str>,
        git_commit: Option<&str>,
    ) -> Result<Checkpoint> {
        let session = self.get_session(session_id)?.context("Session not found")?;

        let messages = self.get_messages(session_id)?;

        let snapshot = serde_json::json!({
            "session": session,
            "messages": messages,
        });

        let checkpoint = Checkpoint {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            name: name.to_string(),
            description: description.map(String::from),
            message_count: messages.len() as i32,
            session_snapshot: serde_json::to_string(&snapshot)?,
            created_at: Utc::now().timestamp_millis(),
            git_commit: git_commit.map(String::from),
        };

        self.conn.execute(
            r#"
            INSERT INTO checkpoints (id, session_id, name, description, message_count,
                                     session_snapshot, created_at, git_commit)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                checkpoint.id,
                checkpoint.session_id,
                checkpoint.name,
                checkpoint.description,
                checkpoint.message_count,
                checkpoint.session_snapshot,
                checkpoint.created_at,
                checkpoint.git_commit,
            ],
        )?;

        Ok(checkpoint)
    }

    /// List checkpoints for a session
    pub fn list_checkpoints(&self, session_id: &str) -> Result<Vec<Checkpoint>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, name, description, message_count, session_snapshot, created_at, git_commit
             FROM checkpoints WHERE session_id = ? ORDER BY created_at DESC"
        )?;

        let checkpoints = stmt
            .query_map([session_id], |row| {
                Ok(Checkpoint {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    message_count: row.get(4)?,
                    session_snapshot: row.get(5)?,
                    created_at: row.get(6)?,
                    git_commit: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(checkpoints)
    }

    // -------------------------------------------------------------------------
    // Share Link Operations
    // -------------------------------------------------------------------------

    /// Add a share link
    pub fn add_share_link(&self, link: &ShareLink) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO share_links (id, session_id, provider, url, share_id, title,
                                     imported, imported_at, created_at, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(url) DO UPDATE SET
                session_id = COALESCE(excluded.session_id, share_links.session_id),
                imported = excluded.imported,
                imported_at = excluded.imported_at,
                metadata = excluded.metadata
            "#,
            params![
                link.id,
                link.session_id,
                link.provider,
                link.url,
                link.share_id,
                link.title,
                link.imported,
                link.imported_at,
                link.created_at,
                link.metadata,
            ],
        )?;
        Ok(())
    }

    /// Get share link by URL
    pub fn get_share_link_by_url(&self, url: &str) -> Result<Option<ShareLink>> {
        self.conn
            .query_row(
                "SELECT id, session_id, provider, url, share_id, title, imported, imported_at, created_at, metadata
                 FROM share_links WHERE url = ?",
                [url],
                |row| {
                    Ok(ShareLink {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        provider: row.get(2)?,
                        url: row.get(3)?,
                        share_id: row.get(4)?,
                        title: row.get(5)?,
                        imported: row.get(6)?,
                        imported_at: row.get(7)?,
                        created_at: row.get(8)?,
                        metadata: row.get(9)?,
                    })
                },
            )
            .optional()
            .context("Failed to get share link")
    }

    /// List all share links
    pub fn list_share_links(&self, imported_only: bool) -> Result<Vec<ShareLink>> {
        let query = if imported_only {
            "SELECT id, session_id, provider, url, share_id, title, imported, imported_at, created_at, metadata
             FROM share_links WHERE imported = 1 ORDER BY created_at DESC"
        } else {
            "SELECT id, session_id, provider, url, share_id, title, imported, imported_at, created_at, metadata
             FROM share_links ORDER BY created_at DESC"
        };

        let mut stmt = self.conn.prepare(query)?;

        let links = stmt
            .query_map([], |row| {
                Ok(ShareLink {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    provider: row.get(2)?,
                    url: row.get(3)?,
                    share_id: row.get(4)?,
                    title: row.get(5)?,
                    imported: row.get(6)?,
                    imported_at: row.get(7)?,
                    created_at: row.get(8)?,
                    metadata: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(links)
    }

    /// Mark a share link as imported
    pub fn mark_share_link_imported(&self, url: &str, session_id: &str) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        self.conn.execute(
            "UPDATE share_links SET imported = 1, imported_at = ?, session_id = ? WHERE url = ?",
            params![now, session_id, url],
        )?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Statistics
    // -------------------------------------------------------------------------

    /// Get database statistics
    pub fn get_statistics(&self) -> Result<DatabaseStats> {
        let workspace_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM workspaces", [], |row| row.get(0))?;

        let session_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;

        let message_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))?;

        let checkpoint_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM checkpoints", [], |row| row.get(0))?;

        let share_link_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM share_links", [], |row| row.get(0))?;

        let imported_link_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM share_links WHERE imported = 1",
            [],
            |row| row.get(0),
        )?;

        Ok(DatabaseStats {
            workspace_count,
            session_count,
            message_count,
            checkpoint_count,
            share_link_count,
            imported_link_count,
        })
    }
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub workspace_count: i64,
    pub session_count: i64,
    pub message_count: i64,
    pub checkpoint_count: i64,
    pub share_link_count: i64,
    pub imported_link_count: i64,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_share_link_provider_parsing() {
        // ChatGPT
        let (provider, id) =
            ShareLinkProvider::from_url("https://chat.openai.com/share/abc123").unwrap();
        assert_eq!(provider, ShareLinkProvider::ChatGPT);
        assert_eq!(id, "abc123");

        // Claude
        let (provider, id) = ShareLinkProvider::from_url("https://claude.ai/share/xyz789").unwrap();
        assert_eq!(provider, ShareLinkProvider::Claude);
        assert_eq!(id, "xyz789");

        // Perplexity
        let (provider, id) =
            ShareLinkProvider::from_url("https://www.perplexity.ai/search/test-query-123").unwrap();
        assert_eq!(provider, ShareLinkProvider::Perplexity);
        assert_eq!(id, "test-query-123");

        // Invalid URL
        assert!(ShareLinkProvider::from_url("https://example.com/test").is_none());
    }

    #[test]
    fn test_database_creation() {
        let db = ChatDatabase::open_in_memory().unwrap();
        let version = db.get_version().unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn test_workspace_crud() {
        let db = ChatDatabase::open_in_memory().unwrap();

        let workspace = Workspace {
            id: "ws-1".to_string(),
            name: "Test Workspace".to_string(),
            path: Some("/test/path".to_string()),
            provider: "vscode".to_string(),
            provider_workspace_id: Some("hash123".to_string()),
            created_at: 1000,
            updated_at: 2000,
            metadata: None,
        };

        db.upsert_workspace(&workspace).unwrap();

        let retrieved = db.get_workspace("ws-1").unwrap().unwrap();
        assert_eq!(retrieved.name, "Test Workspace");

        let workspaces = db.list_workspaces().unwrap();
        assert_eq!(workspaces.len(), 1);
    }

    #[test]
    fn test_session_crud() {
        let db = ChatDatabase::open_in_memory().unwrap();

        let session = Session {
            id: "sess-1".to_string(),
            workspace_id: None,
            provider: "chatgpt".to_string(),
            provider_session_id: Some("gpt-abc".to_string()),
            title: "Test Session".to_string(),
            model: Some("gpt-4".to_string()),
            message_count: 5,
            token_count: Some(1000),
            created_at: 1000,
            updated_at: 2000,
            archived: false,
            metadata: None,
        };

        let was_update = db.upsert_session(&session).unwrap();
        assert!(!was_update);

        let retrieved = db.get_session("sess-1").unwrap().unwrap();
        assert_eq!(retrieved.title, "Test Session");
    }

    #[test]
    fn test_share_link_operations() {
        let db = ChatDatabase::open_in_memory().unwrap();

        let link = ShareLink {
            id: "link-1".to_string(),
            session_id: None,
            provider: "ChatGPT".to_string(),
            url: "https://chat.openai.com/share/abc123".to_string(),
            share_id: "abc123".to_string(),
            title: Some("Shared Chat".to_string()),
            imported: false,
            imported_at: None,
            created_at: 1000,
            metadata: None,
        };

        db.add_share_link(&link).unwrap();

        let retrieved = db
            .get_share_link_by_url("https://chat.openai.com/share/abc123")
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.share_id, "abc123");
        assert!(!retrieved.imported);

        // Create a session to link to
        let session = Session {
            id: "sess-123".to_string(),
            workspace_id: None,
            provider: "chatgpt".to_string(),
            provider_session_id: None,
            title: "Imported Session".to_string(),
            model: None,
            message_count: 0,
            token_count: None,
            created_at: 1000,
            updated_at: 1000,
            archived: false,
            metadata: None,
        };
        db.upsert_session(&session).unwrap();

        // Mark as imported
        db.mark_share_link_imported(&link.url, "sess-123").unwrap();

        let updated = db.get_share_link_by_url(&link.url).unwrap().unwrap();
        assert!(updated.imported);
        assert_eq!(updated.session_id, Some("sess-123".to_string()));
    }

    #[test]
    fn test_checkpoint_creation() {
        let db = ChatDatabase::open_in_memory().unwrap();

        // Create a session first
        let session = Session {
            id: "sess-1".to_string(),
            workspace_id: None,
            provider: "test".to_string(),
            provider_session_id: None,
            title: "Test".to_string(),
            model: None,
            message_count: 0,
            token_count: None,
            created_at: 1000,
            updated_at: 1000,
            archived: false,
            metadata: None,
        };
        db.upsert_session(&session).unwrap();

        // Create checkpoint
        let checkpoint = db
            .create_checkpoint("sess-1", "v1.0", Some("First checkpoint"), None)
            .unwrap();
        assert_eq!(checkpoint.name, "v1.0");

        // List checkpoints
        let checkpoints = db.list_checkpoints("sess-1").unwrap();
        assert_eq!(checkpoints.len(), 1);
    }

    #[test]
    fn test_database_statistics() {
        let db = ChatDatabase::open_in_memory().unwrap();

        let stats = db.get_statistics().unwrap();
        assert_eq!(stats.workspace_count, 0);
        assert_eq!(stats.session_count, 0);
    }
}
