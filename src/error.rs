// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Error types for csm

use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum CsmError {
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Invalid session format: {0}")]
    InvalidSessionFormat(String),

    #[error("VS Code storage not found")]
    StorageNotFound,

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),

    #[error("Git error: {0}")]
    GitError(String),

    #[error("VS Code is running. Close it and try again, or use --force")]
    VSCodeRunning,

    #[error("No chat sessions found")]
    NoSessionsFound,

    #[error("Must specify either --hash or --path")]
    MissingTargetSpecifier,
}

pub type Result<T> = std::result::Result<T, CsmError>;
