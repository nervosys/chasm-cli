// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Tool System
//!
//! Define and register tools that agents can use.

#![allow(dead_code)]

use crate::agency::error::AgencyResult;
use crate::agency::models::ToolResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Tool parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// Parameter name
    pub name: String,
    /// Parameter type (string, number, boolean, array, object)
    #[serde(rename = "type")]
    pub param_type: String,
    /// Parameter description
    pub description: String,
    /// Whether the parameter is required
    #[serde(default)]
    pub required: bool,
    /// Enum values (if applicable)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    /// Default value
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name (must be unique)
    pub name: String,
    /// Tool description
    pub description: String,
    /// Tool parameters
    #[serde(default)]
    pub parameters: Vec<ToolParameter>,
    /// Tool category
    #[serde(default)]
    pub category: ToolCategory,
    /// Whether the tool requires confirmation before execution
    #[serde(default)]
    pub requires_confirmation: bool,
    /// Custom metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, Value>,
}

impl Tool {
    /// Create a new tool
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: Vec::new(),
            category: ToolCategory::Custom,
            requires_confirmation: false,
            metadata: HashMap::new(),
        }
    }

    /// Convert to function definition for model API
    pub fn to_function_definition(&self) -> Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for param in &self.parameters {
            let mut prop = serde_json::Map::new();
            prop.insert("type".to_string(), Value::String(param.param_type.clone()));
            prop.insert(
                "description".to_string(),
                Value::String(param.description.clone()),
            );

            if let Some(enum_vals) = &param.enum_values {
                prop.insert(
                    "enum".to_string(),
                    Value::Array(enum_vals.iter().map(|v| Value::String(v.clone())).collect()),
                );
            }

            properties.insert(param.name.clone(), Value::Object(prop));

            if param.required {
                required.push(Value::String(param.name.clone()));
            }
        }

        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": {
                    "type": "object",
                    "properties": properties,
                    "required": required
                }
            }
        })
    }
}

/// Tool category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    #[default]
    Custom,
    Search,
    Code,
    File,
    Data,
    Communication,
    System,
    Builtin,
}

/// Fluent builder for tools
pub struct ToolBuilder {
    tool: Tool,
}

impl ToolBuilder {
    /// Create a new tool builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            tool: Tool {
                name: name.into(),
                description: String::new(),
                parameters: Vec::new(),
                category: ToolCategory::Custom,
                requires_confirmation: false,
                metadata: HashMap::new(),
            },
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.tool.description = desc.into();
        self
    }

    /// Add a parameter
    pub fn parameter(
        mut self,
        name: impl Into<String>,
        param_type: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        self.tool.parameters.push(ToolParameter {
            name: name.into(),
            param_type: param_type.into(),
            description: description.into(),
            required,
            enum_values: None,
            default: None,
        });
        self
    }

    /// Add a string parameter
    pub fn string_param(
        self,
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        self.parameter(name, "string", description, required)
    }

    /// Add a number parameter
    pub fn number_param(
        self,
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        self.parameter(name, "number", description, required)
    }

    /// Add a boolean parameter
    pub fn bool_param(
        self,
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        self.parameter(name, "boolean", description, required)
    }

    /// Set category
    pub fn category(mut self, category: ToolCategory) -> Self {
        self.tool.category = category;
        self
    }

    /// Set requires confirmation
    pub fn requires_confirmation(mut self, requires: bool) -> Self {
        self.tool.requires_confirmation = requires;
        self
    }

    /// Build the tool
    pub fn build(self) -> Tool {
        self.tool
    }
}

/// Trait for executable tools
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Get the tool definition
    fn definition(&self) -> &Tool;

    /// Execute the tool with the given arguments
    async fn execute(&self, args: Value) -> AgencyResult<ToolResult>;
}

/// Type alias for tool execution function
pub type ToolFn = Box<
    dyn Fn(Value) -> Pin<Box<dyn Future<Output = AgencyResult<ToolResult>> + Send>> + Send + Sync,
>;

/// Tool registry for managing available tools
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<Tool>>,
    executors: HashMap<String, Arc<dyn ToolExecutor>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry with builtin tools
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.register_builtins();
        registry
    }

    /// Register a tool
    pub fn register(&mut self, tool: Tool) {
        self.tools.insert(tool.name.clone(), Arc::new(tool));
    }

    /// Register a tool with its executor
    pub fn register_with_executor(&mut self, executor: impl ToolExecutor + 'static) {
        let tool = executor.definition().clone();
        let name = tool.name.clone();
        self.tools.insert(name.clone(), Arc::new(tool));
        self.executors.insert(name, Arc::new(executor));
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&Arc<Tool>> {
        self.tools.get(name)
    }

    /// Get an executor by name
    pub fn get_executor(&self, name: &str) -> Option<&Arc<dyn ToolExecutor>> {
        self.executors.get(name)
    }

    /// List all tools
    pub fn list(&self) -> Vec<&Tool> {
        self.tools.values().map(|t| t.as_ref()).collect()
    }

    /// Get tool definitions for model API
    pub fn to_definitions(&self) -> Vec<Value> {
        self.tools
            .values()
            .map(|t| t.to_function_definition())
            .collect()
    }

    /// Register builtin tools
    fn register_builtins(&mut self) {
        for tool in BuiltinTools::all() {
            self.register(tool);
        }
    }
}

/// Builtin tools provided by the Agency
pub struct BuiltinTools;

impl BuiltinTools {
    /// Get all builtin tools
    pub fn all() -> Vec<Tool> {
        vec![
            Self::web_search(),
            Self::code_execution(),
            Self::read_file(),
            Self::write_file(),
            Self::list_directory(),
            Self::http_request(),
            Self::calculator(),
        ]
    }

    /// Web search tool
    pub fn web_search() -> Tool {
        ToolBuilder::new("web_search")
            .description("Search the web for information. Returns relevant snippets and URLs.")
            .string_param("query", "The search query", true)
            .number_param(
                "max_results",
                "Maximum number of results (default: 5)",
                false,
            )
            .category(ToolCategory::Search)
            .build()
    }

    /// Code execution tool
    pub fn code_execution() -> Tool {
        ToolBuilder::new("code_execution")
            .description("Execute code in a sandboxed environment. Supports Python, JavaScript, and shell scripts.")
            .string_param("code", "The code to execute", true)
            .string_param("language", "Programming language (python, javascript, shell)", true)
            .number_param("timeout", "Execution timeout in seconds (default: 30)", false)
            .category(ToolCategory::Code)
            .requires_confirmation(true)
            .build()
    }

    /// Read file tool
    pub fn read_file() -> Tool {
        ToolBuilder::new("read_file")
            .description("Read the contents of a file from the filesystem.")
            .string_param("path", "Path to the file to read", true)
            .string_param("encoding", "File encoding (default: utf-8)", false)
            .category(ToolCategory::File)
            .build()
    }

    /// Write file tool
    pub fn write_file() -> Tool {
        ToolBuilder::new("write_file")
            .description("Write content to a file. Creates the file if it doesn't exist.")
            .string_param("path", "Path to the file to write", true)
            .string_param("content", "Content to write to the file", true)
            .bool_param("append", "Append to file instead of overwriting", false)
            .category(ToolCategory::File)
            .requires_confirmation(true)
            .build()
    }

    /// List directory tool
    pub fn list_directory() -> Tool {
        ToolBuilder::new("list_directory")
            .description("List the contents of a directory.")
            .string_param("path", "Path to the directory", true)
            .bool_param("recursive", "Include subdirectories", false)
            .bool_param("include_hidden", "Include hidden files", false)
            .category(ToolCategory::File)
            .build()
    }

    /// HTTP request tool
    pub fn http_request() -> Tool {
        ToolBuilder::new("http_request")
            .description("Make an HTTP request to a URL.")
            .string_param("url", "The URL to request", true)
            .string_param("method", "HTTP method (GET, POST, PUT, DELETE)", false)
            .string_param("body", "Request body (for POST/PUT)", false)
            .string_param("headers", "JSON object of headers", false)
            .category(ToolCategory::Communication)
            .build()
    }

    /// Calculator tool
    pub fn calculator() -> Tool {
        ToolBuilder::new("calculator")
            .description("Evaluate mathematical expressions. Supports basic arithmetic, functions, and constants.")
            .string_param("expression", "The mathematical expression to evaluate", true)
            .category(ToolCategory::Data)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_builder() {
        let tool = ToolBuilder::new("test_tool")
            .description("A test tool")
            .string_param("input", "Input parameter", true)
            .number_param("count", "Count parameter", false)
            .category(ToolCategory::Custom)
            .build();

        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, "A test tool");
        assert_eq!(tool.parameters.len(), 2);
        assert!(tool.parameters[0].required);
        assert!(!tool.parameters[1].required);
    }

    #[test]
    fn test_function_definition() {
        let tool = BuiltinTools::web_search();
        let def = tool.to_function_definition();

        assert_eq!(def["type"], "function");
        assert_eq!(def["function"]["name"], "web_search");
    }

    #[test]
    fn test_registry() {
        let registry = ToolRegistry::with_builtins();
        assert!(registry.get("web_search").is_some());
        assert!(registry.get("code_execution").is_some());
        assert!(registry.get("nonexistent").is_none());
    }
}
