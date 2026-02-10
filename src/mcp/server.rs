// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! MCP Server - Main server implementation using stdio transport

#![allow(dead_code, unused_imports)]

use super::resources;
use super::tools;
use super::types::*;
use serde_json::json;
use std::io::{self, BufRead, Write};

/// MCP Server for Chat System Manager
pub struct McpServer {
    initialized: bool,
}

impl McpServer {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    /// Run the MCP server using stdio transport
    pub fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut stdout = stdout.lock();

        eprintln!("[csm-mcp] Server starting...");

        for line in stdin.lock().lines() {
            let line = line?;

            if line.is_empty() {
                continue;
            }

            eprintln!("[csm-mcp] Received: {}", &line[..line.len().min(100)]);

            match serde_json::from_str::<JsonRpcRequest>(&line) {
                Ok(request) => {
                    let response = self.handle_request(request);
                    let response_str = serde_json::to_string(&response)?;
                    eprintln!(
                        "[csm-mcp] Sending: {}",
                        &response_str[..response_str.len().min(100)]
                    );
                    writeln!(stdout, "{}", response_str)?;
                    stdout.flush()?;
                }
                Err(e) => {
                    eprintln!("[csm-mcp] Parse error: {}", e);
                    let error_response =
                        JsonRpcResponse::error(None, -32700, format!("Parse error: {}", e));
                    writeln!(stdout, "{}", serde_json::to_string(&error_response)?)?;
                    stdout.flush()?;
                }
            }
        }

        Ok(())
    }

    fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request),
            "initialized" => {
                // Notification, no response needed
                JsonRpcResponse::success(request.id, json!({}))
            }
            "tools/list" => self.handle_tools_list(request),
            "tools/call" => self.handle_tools_call(request),
            "resources/list" => self.handle_resources_list(request),
            "resources/read" => self.handle_resources_read(request),
            "ping" => JsonRpcResponse::success(request.id, json!({})),
            _ => JsonRpcResponse::error(
                request.id,
                -32601,
                format!("Method not found: {}", request.method),
            ),
        }
    }

    fn handle_initialize(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        self.initialized = true;

        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                resources: Some(ResourcesCapability {
                    list_changed: Some(false),
                    subscribe: Some(false),
                }),
                prompts: None,
            },
            server_info: ServerInfo {
                name: "csm-mcp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            },
        };

        JsonRpcResponse::success(request.id, serde_json::to_value(result).unwrap())
    }

    fn handle_tools_list(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let tools = tools::list_tools();
        JsonRpcResponse::success(request.id, json!({ "tools": tools }))
    }

    fn handle_tools_call(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let params: Result<CallToolParams, _> = serde_json::from_value(request.params.clone());

        match params {
            Ok(params) => {
                let result = tools::call_tool(&params.name, &params.arguments);
                JsonRpcResponse::success(request.id, serde_json::to_value(result).unwrap())
            }
            Err(e) => JsonRpcResponse::error(request.id, -32602, format!("Invalid params: {}", e)),
        }
    }

    fn handle_resources_list(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let resources = resources::list_resources();
        JsonRpcResponse::success(request.id, json!({ "resources": resources }))
    }

    fn handle_resources_read(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let params: Result<ReadResourceParams, _> = serde_json::from_value(request.params.clone());

        match params {
            Ok(params) => {
                let result = resources::read_resource(&params.uri);
                JsonRpcResponse::success(request.id, serde_json::to_value(result).unwrap())
            }
            Err(e) => JsonRpcResponse::error(request.id, -32602, format!("Invalid params: {}", e)),
        }
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}
