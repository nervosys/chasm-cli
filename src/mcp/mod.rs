// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! MCP (Model Context Protocol) Server for Chat System Manager
//!
//! This module implements an MCP server that exposes csm functionality
//! to AI agents, enabling them to:
//! - List and search workspaces and chat sessions
//! - Access csm-web database sessions and messages
//! - Find orphaned sessions not in VS Code's index
//! - Register sessions to make them visible
//! - Merge sessions across workspaces
//! - Search chat history with full-text search

#![allow(dead_code, unused_imports)]

pub mod db;
pub mod resources;
pub mod server;
pub mod tools;
pub mod types;

pub use server::McpServer;
