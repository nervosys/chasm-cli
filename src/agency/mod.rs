// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Agent Development Kit (Agency) - Rust Implementation
//!
//! A Rust-native framework for building, orchestrating, and deploying AI agents.
//! Inspired by multi-agent patterns but built from scratch for performance and control.
//!
//! ## Key Features

#![allow(dead_code, unused_imports)]
//!
//! - **Code-First Development**: Define agents and tools in Rust for type safety and performance
//! - **Modular Architecture**: Compose agents into hierarchies (sequential, parallel, loop)
//! - **Tool Ecosystem**: Built-in tools + custom function registration
//! - **Multi-Model Support**: Works with any LLM provider (Gemini, OpenAI, Anthropic, local)
//! - **Session Management**: Persistent conversation state with SQLite backend
//! - **Streaming Support**: Real-time response streaming via SSE
//!
//! ## Example
//!
//! ```rust,ignore
//! use csm::Agency::{Agent, AgentBuilder, Tool, Runtime};
//!
//! let search_agent = AgentBuilder::new("researcher")
//!     .model("gemini-2.5-flash")
//!     .instruction("You are a helpful research assistant.")
//!     .tool(Tool::web_search())
//!     .build();
//!
//! let runtime = Runtime::new();
//! let result = runtime.run(&search_agent, "What is quantum computing?").await?;
//! ```

pub mod agent;
pub mod error;
pub mod executor;
pub mod memory;
pub mod modality;
pub mod models;
pub mod orchestrator;
pub mod proactive;
pub mod remote;
pub mod runtime;
pub mod session;
pub mod tools;

// Autonomous agents
pub mod archival;
pub mod search_refinement;

// Re-export main types
pub use agent::{Agent, AgentBuilder, AgentConfig, AgentRole, AgentStatus};
pub use error::AgencyError;
pub use executor::{ExecutionContext, ExecutionResult, Executor};
pub use memory::{
    AgentCache, CacheEntry, ChunkingConfig, ChunkingStrategy, ContextSegment, ContextSegmentType,
    ContextWindow, Document, DocumentChunk, DocumentType, Embedding, EmbeddingModel,
    EmbeddingProvider, KnowledgeBase, MemoryConfig, MemoryEntry, MemoryError, MemoryManager,
    MemorySource, MemoryStats, MemoryType, SearchResult, SimilarityMetric, VectorStore,
    VectorStoreConfig, VectorStoreStats,
};
pub use modality::{
    vla_models, vlm_models, ActionCommand, ActionParameters, ActionType, AudioContent, AudioData,
    BoundingBoxRegion, ContentPart, ImageContent, ImageData, ImageDetail, ImageFormat, Modality,
    ModalityCapabilities, ModelCategory, MultimodalMessage, MultimodalModel, SensorData,
    SensorType, SensorValues, VideoContent, VideoData, Waypoint,
};
pub use models::{AgencyEvent, AgencyMessage, EventType, ToolCall, ToolResult};
pub use orchestrator::{OrchestrationType, Orchestrator, Pipeline, Swarm};
pub use proactive::{
    business_agent_config, household_agent_config, ActionRisk, ActionStatus, DetectedProblem,
    PermissionLevel, ProactiveAction, ProactiveAgentConfig, ProactiveMonitor, ProblemCategory,
    ProblemSeverity, ProblemStatus,
};
pub use remote::{
    generate_node_id, generate_task_id, ArtifactType, GpuInfo, HardwareInfo, LogLevel,
    MonitorStats, NodeId, NodeStatus, RemoteAgentClient, RemoteEvent, RemoteMonitor,
    RemoteMonitorConfig, RemoteMonitorError, RemoteNode, RemoteTask, RemoteTaskBuilder,
    RemoteTaskId, RemoteTaskStatus, ResourceUsage, TaskArtifact, TaskLogEntry, TaskMetrics,
    TaskPriority, TaskResult,
};
pub use runtime::{Runtime, RuntimeConfig};
pub use session::{Session, SessionManager, SessionState};
pub use tools::{BuiltinTools, Tool, ToolBuilder, ToolRegistry};

// Autonomous agent exports
pub use archival::{ArchivalAgent, ArchivalPolicy, ArchivalResult, ArchivalScheduler, ArchivalStats};
pub use search_refinement::{
    EnrichedSearchResult, QueryRefinement, RefinementType, SearchAnalytics,
    SearchContext, SearchRefinementAgent,
};
