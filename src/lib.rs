// Copyright (c) 2024-2028 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Chat System Manager (CSM) - Library
//!
//! A library for managing and merging chat sessions across workspaces and LLM providers.
//!
//! ## Supported Providers
//!
//! - **VS Code Copilot Chat** - Default, file-based sessions
//! - **Cursor** - Cursor IDE chat sessions
//! - **Ollama** - Local LLM inference
//! - **vLLM** - High-performance LLM serving
//! - **Azure AI Foundry** - Microsoft's AI platform (Foundry Local)
//! - **LM Studio** - Local model runner
//! - **LocalAI** - Drop-in OpenAI replacement
//! - **Text Generation WebUI** - oobabooga's web interface
//! - **Jan.ai** - Open source ChatGPT alternative
//! - **GPT4All** - Local privacy-focused AI
//! - **Llamafile** - Portable executable LLMs
//!
//! ## Agent Development Kit (Agency)
//!
//! The `Agency` module provides a Rust-native framework for building AI agents:
//!
//! ```rust,ignore
//! use csm::Agency::{Agent, AgentBuilder, Runtime, Tool};
//!
//! let agent = AgentBuilder::new("assistant")
//!     .instruction("You are a helpful assistant.")
//!     .model("gemini-2.5-flash")
//!     .tool(Tool::web_search())
//!     .build();
//!
//! let runtime = Runtime::new()?;
//! runtime.register_agent(agent);
//! let result = runtime.run("assistant", "Hello!").await?;
//! ```

// Library modules export public APIs for external use - suppress dead_code warnings
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::type_complexity)]

pub mod agency;
pub mod analytics;
pub mod automation;
pub mod browser;
pub mod cli;
pub mod cloud_sync;
pub mod commands;
pub mod database;
pub mod encryption;
pub mod error;
pub mod integrations;
pub mod intelligence;
pub mod mcp;
pub mod models;
pub mod plugins;
pub mod providers;
pub mod routing;
pub mod scaling;
pub mod storage;
pub mod sync;
pub mod teams;
pub mod telemetry;
pub mod tui;
pub mod workspace;

// Re-export commonly used items
pub use cli::{
    Cli, Commands, ExportCommands, FetchCommands, FindCommands, GitCommands, ImportCommands,
    ListCommands, MergeCommands, MigrationCommands, MoveCommands, ProviderCommands, RunCommands,
    ShowCommands,
};
pub use database::{ChatDatabase, ShareLinkInfo, ShareLinkParser, ShareLinkProvider};
pub use error::CsmError;
pub use models::{
    ChatMessage, ChatRequest, ChatSession, ChatSessionIndex, ChatSessionIndexEntry,
    SessionWithPath, Workspace, WorkspaceJson,
};
pub use providers::{
    CsmConfig, GenericMessage, GenericSession, ProviderConfig, ProviderRegistry, ProviderType,
};
pub use storage::{
    add_session_to_index, backup_workspace_sessions, is_vscode_running, read_chat_session_index,
    register_all_sessions_from_directory, write_chat_session_index,
};
pub use workspace::{
    decode_workspace_folder, discover_workspaces, find_workspace_by_path,
    get_chat_sessions_from_workspace, get_workspace_by_hash, get_workspace_by_path,
    get_workspace_storage_path, normalize_path,
};


