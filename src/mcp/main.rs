// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! CSM MCP Server - Entry point
//!
//! This binary provides a Model Context Protocol (MCP) server interface
//! for Chat System Manager, enabling AI agents to programmatically interact
//! with chat session management functionality.
//!
//! # Usage
//!
//! The server communicates via stdio (stdin/stdout) using JSON-RPC 2.0.
//!
//! ## Available Tools
//!
//! - `csm_list_workspaces` - List all registered workspaces
//! - `csm_find_workspace` - Find workspace by name or path
//! - `csm_list_sessions` - List sessions in a workspace
//! - `csm_list_orphaned` - Find orphaned session files
//! - `csm_register_all` - Register all sessions in current directory
//! - `csm_register_sessions` - Register specific sessions by ID or title
//! - `csm_show_session` - Show details of a specific session
//! - `csm_show_history` - Show history of workspace sessions
//! - `csm_merge_sessions` - Merge multiple sessions
//! - `csm_search` - Search sessions by content
//! - `csm_detect` - Detect chat provider and sessions
//!
//! ## Available Resources
//!
//! - `csm://workspaces` - List of all registered workspaces
//! - `csm://sessions` - List of all sessions across workspaces
//! - `csm://orphaned` - List of orphaned session files
//! - `csm://providers` - List of supported chat providers
//! - `csm://workspace/{hash}` - Details of a specific workspace
//! - `csm://session/{id}` - Details of a specific session
//!
//! # Configuration
//!
//! Add to your MCP client configuration (e.g., Claude Desktop):
//!
//! ```json
//! {
//!   "mcpServers": {
//!     "csm": {
//!       "command": "csm-mcp",
//!       "args": []
//!     }
//!   }
//! }
//! ```

use chasm::mcp::server::McpServer;

fn main() {
    let mut server = McpServer::new();

    if let Err(e) = server.run() {
        eprintln!("[csm-mcp] Server error: {}", e);
        std::process::exit(1);
    }
}
