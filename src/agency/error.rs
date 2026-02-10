// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Agency Error types

#![allow(dead_code)]

use thiserror::Error;

/// Agency-specific errors
#[derive(Error, Debug)]
pub enum AgencyError {
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Tool execution failed: {tool} - {message}")]
    ToolExecutionFailed { tool: String, message: String },

    #[error("Model error: {0}")]
    ModelError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Orchestration error: {0}")]
    OrchestrationError(String),

    #[error("Timeout after {0} seconds")]
    Timeout(u64),

    #[error("Max iterations ({0}) exceeded")]
    MaxIterationsExceeded(u32),

    #[error("Cancelled by user")]
    Cancelled,

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<serde_json::Error> for AgencyError {
    fn from(err: serde_json::Error) -> Self {
        AgencyError::SerializationError(err.to_string())
    }
}

impl From<rusqlite::Error> for AgencyError {
    fn from(err: rusqlite::Error) -> Self {
        AgencyError::DatabaseError(err.to_string())
    }
}

pub type AgencyResult<T> = Result<T, AgencyError>;
