// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Remote Agent Monitoring System
//!
//! Enables monitoring of agent task progress across distributed machines.
//!
//! ## Features
//!
//! - **Real-time Progress**: WebSocket-based live updates
//! - **Task Tracking**: Monitor task status, progress, and metrics
//! - **Multi-machine**: Track agents across multiple remote hosts
//! - **Heartbeat**: Automatic health monitoring with configurable intervals

#![allow(dead_code)]
//! - **Event Streaming**: Subscribe to specific agent or task events

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

// =============================================================================
// Core Types
// =============================================================================

/// Unique identifier for remote nodes
pub type NodeId = String;

/// Unique identifier for remote tasks
pub type RemoteTaskId = String;

/// Remote node representing a machine running agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteNode {
    /// Unique node identifier
    pub id: NodeId,
    /// Human-readable name
    pub name: String,
    /// Node address (hostname:port or IP:port)
    pub address: String,
    /// Node status
    pub status: NodeStatus,
    /// Node capabilities/tags
    pub tags: Vec<String>,
    /// Hardware info
    pub hardware: Option<HardwareInfo>,
    /// Number of active agents
    pub active_agents: u32,
    /// Number of running tasks
    pub running_tasks: u32,
    /// Last heartbeat received
    pub last_heartbeat: DateTime<Utc>,
    /// Node registered at
    pub registered_at: DateTime<Utc>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Node status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum NodeStatus {
    /// Node is online and healthy
    Online,
    /// Node is online but degraded (high load, etc.)
    Degraded,
    /// Node is offline or unreachable
    Offline,
    /// Node is in maintenance mode
    Maintenance,
    /// Node status is unknown
    #[default]
    Unknown,
}

/// Hardware information for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    /// CPU cores
    pub cpu_cores: u32,
    /// Total RAM in bytes
    pub ram_total: u64,
    /// Available RAM in bytes
    pub ram_available: u64,
    /// GPU information
    pub gpus: Vec<GpuInfo>,
    /// Operating system
    pub os: String,
    /// Architecture (x86_64, arm64, etc.)
    pub arch: String,
}

/// GPU information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    /// GPU name/model
    pub name: String,
    /// VRAM in bytes
    pub vram: u64,
    /// CUDA version (if NVIDIA)
    pub cuda_version: Option<String>,
}

// =============================================================================
// Remote Task
// =============================================================================

/// A task running on a remote node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTask {
    /// Unique task identifier
    pub id: RemoteTaskId,
    /// Node this task is running on
    pub node_id: NodeId,
    /// Agent executing this task
    pub agent_id: String,
    /// Agent name
    pub agent_name: String,
    /// Task title/description
    pub title: String,
    /// Detailed description
    pub description: Option<String>,
    /// Task status
    pub status: RemoteTaskStatus,
    /// Progress (0.0 - 1.0)
    pub progress: f32,
    /// Progress message
    pub progress_message: Option<String>,
    /// Current step (for multi-step tasks)
    pub current_step: Option<u32>,
    /// Total steps
    pub total_steps: Option<u32>,
    /// Task priority
    pub priority: TaskPriority,
    /// Task started at
    pub started_at: DateTime<Utc>,
    /// Task completed at
    pub completed_at: Option<DateTime<Utc>>,
    /// Estimated completion time
    pub eta: Option<DateTime<Utc>>,
    /// Task result (if completed)
    pub result: Option<TaskResult>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Resource usage
    pub resources: ResourceUsage,
    /// Task logs (recent entries)
    pub logs: Vec<TaskLogEntry>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Remote task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteTaskStatus {
    /// Task is queued waiting to start
    Queued,
    /// Task is starting up
    Starting,
    /// Task is actively running
    Running,
    /// Task is paused
    Paused,
    /// Task completed successfully
    Completed,
    /// Task failed with error
    Failed,
    /// Task was cancelled
    Cancelled,
    /// Task timed out
    TimedOut,
}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TaskPriority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Success flag
    pub success: bool,
    /// Output data
    pub output: Option<serde_json::Value>,
    /// Artifacts produced (file paths, URLs, etc.)
    pub artifacts: Vec<TaskArtifact>,
    /// Metrics collected during execution
    pub metrics: TaskMetrics,
}

/// Task artifact (output files, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskArtifact {
    /// Artifact name
    pub name: String,
    /// Artifact type
    pub artifact_type: ArtifactType,
    /// Location (path or URL)
    pub location: String,
    /// Size in bytes
    pub size: Option<u64>,
    /// Checksum (SHA256)
    pub checksum: Option<String>,
}

/// Artifact types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    File,
    Directory,
    Url,
    Database,
    Model,
    Report,
    Log,
    Custom(String),
}

/// Task execution metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskMetrics {
    /// Total execution time in milliseconds
    pub duration_ms: u64,
    /// Tokens used (for LLM tasks)
    pub tokens_used: Option<u64>,
    /// API calls made
    pub api_calls: u32,
    /// Files processed
    pub files_processed: u32,
    /// Errors encountered (but recovered)
    pub errors_recovered: u32,
    /// Retries performed
    pub retries: u32,
}

/// Resource usage during task execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage percentage (0-100)
    pub cpu_percent: f32,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// GPU memory usage in bytes (if applicable)
    pub gpu_memory_bytes: Option<u64>,
    /// Network bytes sent
    pub network_tx_bytes: u64,
    /// Network bytes received
    pub network_rx_bytes: u64,
    /// Disk read bytes
    pub disk_read_bytes: u64,
    /// Disk write bytes
    pub disk_write_bytes: u64,
}

/// Task log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLogEntry {
    /// Log timestamp
    pub timestamp: DateTime<Utc>,
    /// Log level
    pub level: LogLevel,
    /// Log message
    pub message: String,
    /// Optional structured data
    pub data: Option<serde_json::Value>,
}

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

// =============================================================================
// Events
// =============================================================================

/// Events emitted by the remote monitoring system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RemoteEvent {
    /// Node came online
    NodeOnline(NodeId),
    /// Node went offline
    NodeOffline(NodeId),
    /// Node status changed
    NodeStatusChanged {
        node_id: NodeId,
        old_status: NodeStatus,
        new_status: NodeStatus,
    },
    /// Node heartbeat received
    NodeHeartbeat {
        node_id: NodeId,
        timestamp: DateTime<Utc>,
    },

    /// Task was created/queued (boxed to reduce enum size)
    TaskCreated(Box<RemoteTask>),
    /// Task started running
    TaskStarted {
        task_id: RemoteTaskId,
        node_id: NodeId,
    },
    /// Task progress updated
    TaskProgress {
        task_id: RemoteTaskId,
        progress: f32,
        message: Option<String>,
    },
    /// Task step completed
    TaskStepCompleted {
        task_id: RemoteTaskId,
        step: u32,
        total: u32,
        description: Option<String>,
    },
    /// Task completed successfully
    TaskCompleted {
        task_id: RemoteTaskId,
        result: TaskResult,
    },
    /// Task failed
    TaskFailed {
        task_id: RemoteTaskId,
        error: String,
    },
    /// Task was cancelled
    TaskCancelled {
        task_id: RemoteTaskId,
        reason: Option<String>,
    },
    /// Task log entry added
    TaskLog {
        task_id: RemoteTaskId,
        entry: TaskLogEntry,
    },

    /// Agent registered on node
    AgentRegistered {
        node_id: NodeId,
        agent_id: String,
        agent_name: String,
    },
    /// Agent unregistered from node
    AgentUnregistered { node_id: NodeId, agent_id: String },
}

// =============================================================================
// Remote Monitor
// =============================================================================

/// Configuration for the remote monitor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteMonitorConfig {
    /// Server bind address
    pub bind_address: String,
    /// Server port
    pub port: u16,
    /// Enable TLS
    pub tls_enabled: bool,
    /// TLS certificate path
    pub tls_cert_path: Option<String>,
    /// TLS key path
    pub tls_key_path: Option<String>,
    /// Authentication token (for API access)
    pub auth_token: Option<String>,
    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,
    /// Node timeout in seconds (mark offline after this)
    pub node_timeout_secs: u64,
    /// Maximum log entries per task
    pub max_log_entries: usize,
    /// Enable metrics collection
    pub metrics_enabled: bool,
}

impl Default for RemoteMonitorConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 9876,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            auth_token: None,
            heartbeat_interval_secs: 30,
            node_timeout_secs: 90,
            max_log_entries: 1000,
            metrics_enabled: true,
        }
    }
}

/// Remote monitoring server
pub struct RemoteMonitor {
    config: RemoteMonitorConfig,
    nodes: Arc<RwLock<HashMap<NodeId, RemoteNode>>>,
    tasks: Arc<RwLock<HashMap<RemoteTaskId, RemoteTask>>>,
    event_tx: broadcast::Sender<RemoteEvent>,
}

impl RemoteMonitor {
    /// Create a new remote monitor
    pub fn new(config: RemoteMonitorConfig) -> Self {
        let (event_tx, _) = broadcast::channel(1000);

        Self {
            config,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<RemoteEvent> {
        self.event_tx.subscribe()
    }

    /// Register a new node
    pub async fn register_node(&self, node: RemoteNode) -> Result<(), RemoteMonitorError> {
        let node_id = node.id.clone();
        let mut nodes = self.nodes.write().await;
        nodes.insert(node_id.clone(), node);

        let _ = self.event_tx.send(RemoteEvent::NodeOnline(node_id));
        Ok(())
    }

    /// Unregister a node
    pub async fn unregister_node(&self, node_id: &str) -> Result<bool, RemoteMonitorError> {
        let mut nodes = self.nodes.write().await;
        let removed = nodes.remove(node_id).is_some();

        if removed {
            let _ = self
                .event_tx
                .send(RemoteEvent::NodeOffline(node_id.to_string()));
        }

        Ok(removed)
    }

    /// Update node heartbeat
    pub async fn heartbeat(&self, node_id: &str) -> Result<(), RemoteMonitorError> {
        let mut nodes = self.nodes.write().await;

        if let Some(node) = nodes.get_mut(node_id) {
            node.last_heartbeat = Utc::now();
            if node.status == NodeStatus::Offline || node.status == NodeStatus::Unknown {
                let old_status = node.status;
                node.status = NodeStatus::Online;
                let _ = self.event_tx.send(RemoteEvent::NodeStatusChanged {
                    node_id: node_id.to_string(),
                    old_status,
                    new_status: NodeStatus::Online,
                });
            }
            let _ = self.event_tx.send(RemoteEvent::NodeHeartbeat {
                node_id: node_id.to_string(),
                timestamp: node.last_heartbeat,
            });
            Ok(())
        } else {
            Err(RemoteMonitorError::NodeNotFound(node_id.to_string()))
        }
    }

    /// Get node by ID
    pub async fn get_node(&self, node_id: &str) -> Option<RemoteNode> {
        let nodes = self.nodes.read().await;
        nodes.get(node_id).cloned()
    }

    /// List all nodes
    pub async fn list_nodes(&self) -> Vec<RemoteNode> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().collect()
    }

    /// List nodes by status
    pub async fn list_nodes_by_status(&self, status: NodeStatus) -> Vec<RemoteNode> {
        let nodes = self.nodes.read().await;
        nodes
            .values()
            .filter(|n| n.status == status)
            .cloned()
            .collect()
    }

    /// Create a new task
    pub async fn create_task(&self, task: RemoteTask) -> Result<RemoteTaskId, RemoteMonitorError> {
        // Verify node exists
        {
            let nodes = self.nodes.read().await;
            if !nodes.contains_key(&task.node_id) {
                return Err(RemoteMonitorError::NodeNotFound(task.node_id.clone()));
            }
        }

        let task_id = task.id.clone();
        let mut tasks = self.tasks.write().await;
        tasks.insert(task_id.clone(), task.clone());

        let _ = self.event_tx.send(RemoteEvent::TaskCreated(Box::new(task)));
        Ok(task_id)
    }

    /// Update task status
    pub async fn update_task_status(
        &self,
        task_id: &str,
        status: RemoteTaskStatus,
    ) -> Result<(), RemoteMonitorError> {
        let mut tasks = self.tasks.write().await;

        if let Some(task) = tasks.get_mut(task_id) {
            let old_status = task.status;
            task.status = status;

            match status {
                RemoteTaskStatus::Running if old_status != RemoteTaskStatus::Running => {
                    let _ = self.event_tx.send(RemoteEvent::TaskStarted {
                        task_id: task_id.to_string(),
                        node_id: task.node_id.clone(),
                    });
                }
                RemoteTaskStatus::Completed => {
                    task.completed_at = Some(Utc::now());
                    task.progress = 1.0;
                }
                RemoteTaskStatus::Failed
                | RemoteTaskStatus::Cancelled
                | RemoteTaskStatus::TimedOut => {
                    task.completed_at = Some(Utc::now());
                }
                _ => {}
            }

            Ok(())
        } else {
            Err(RemoteMonitorError::TaskNotFound(task_id.to_string()))
        }
    }

    /// Update task progress
    pub async fn update_task_progress(
        &self,
        task_id: &str,
        progress: f32,
        message: Option<String>,
    ) -> Result<(), RemoteMonitorError> {
        let mut tasks = self.tasks.write().await;

        if let Some(task) = tasks.get_mut(task_id) {
            task.progress = progress.clamp(0.0, 1.0);
            task.progress_message = message.clone();

            let _ = self.event_tx.send(RemoteEvent::TaskProgress {
                task_id: task_id.to_string(),
                progress: task.progress,
                message,
            });

            Ok(())
        } else {
            Err(RemoteMonitorError::TaskNotFound(task_id.to_string()))
        }
    }

    /// Update task step
    pub async fn update_task_step(
        &self,
        task_id: &str,
        step: u32,
        total: u32,
        description: Option<String>,
    ) -> Result<(), RemoteMonitorError> {
        let mut tasks = self.tasks.write().await;

        if let Some(task) = tasks.get_mut(task_id) {
            task.current_step = Some(step);
            task.total_steps = Some(total);
            task.progress = step as f32 / total as f32;

            let _ = self.event_tx.send(RemoteEvent::TaskStepCompleted {
                task_id: task_id.to_string(),
                step,
                total,
                description,
            });

            Ok(())
        } else {
            Err(RemoteMonitorError::TaskNotFound(task_id.to_string()))
        }
    }

    /// Complete a task
    pub async fn complete_task(
        &self,
        task_id: &str,
        result: TaskResult,
    ) -> Result<(), RemoteMonitorError> {
        let node_id = {
            let mut tasks = self.tasks.write().await;

            if let Some(task) = tasks.get_mut(task_id) {
                task.status = RemoteTaskStatus::Completed;
                task.completed_at = Some(Utc::now());
                task.progress = 1.0;
                task.result = Some(result.clone());
                task.node_id.clone()
            } else {
                return Err(RemoteMonitorError::TaskNotFound(task_id.to_string()));
            }
        };

        // Update node task count
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.get_mut(&node_id) {
            node.running_tasks = node.running_tasks.saturating_sub(1);
        }

        let _ = self.event_tx.send(RemoteEvent::TaskCompleted {
            task_id: task_id.to_string(),
            result,
        });

        Ok(())
    }

    /// Fail a task
    pub async fn fail_task(&self, task_id: &str, error: String) -> Result<(), RemoteMonitorError> {
        let node_id = {
            let mut tasks = self.tasks.write().await;

            if let Some(task) = tasks.get_mut(task_id) {
                task.status = RemoteTaskStatus::Failed;
                task.completed_at = Some(Utc::now());
                task.error = Some(error.clone());
                task.node_id.clone()
            } else {
                return Err(RemoteMonitorError::TaskNotFound(task_id.to_string()));
            }
        };

        // Update node task count
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.get_mut(&node_id) {
            node.running_tasks = node.running_tasks.saturating_sub(1);
        }

        let _ = self.event_tx.send(RemoteEvent::TaskFailed {
            task_id: task_id.to_string(),
            error,
        });

        Ok(())
    }

    /// Cancel a task
    pub async fn cancel_task(
        &self,
        task_id: &str,
        reason: Option<String>,
    ) -> Result<(), RemoteMonitorError> {
        let node_id = {
            let mut tasks = self.tasks.write().await;

            if let Some(task) = tasks.get_mut(task_id) {
                task.status = RemoteTaskStatus::Cancelled;
                task.completed_at = Some(Utc::now());
                task.node_id.clone()
            } else {
                return Err(RemoteMonitorError::TaskNotFound(task_id.to_string()));
            }
        };

        // Update node task count
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.get_mut(&node_id) {
            node.running_tasks = node.running_tasks.saturating_sub(1);
        }

        let _ = self.event_tx.send(RemoteEvent::TaskCancelled {
            task_id: task_id.to_string(),
            reason,
        });

        Ok(())
    }

    /// Add log entry to task
    pub async fn add_task_log(
        &self,
        task_id: &str,
        entry: TaskLogEntry,
    ) -> Result<(), RemoteMonitorError> {
        let mut tasks = self.tasks.write().await;

        if let Some(task) = tasks.get_mut(task_id) {
            task.logs.push(entry.clone());

            // Trim logs if over limit
            if task.logs.len() > self.config.max_log_entries {
                let drain_count = task.logs.len() - self.config.max_log_entries;
                task.logs.drain(0..drain_count);
            }

            let _ = self.event_tx.send(RemoteEvent::TaskLog {
                task_id: task_id.to_string(),
                entry,
            });

            Ok(())
        } else {
            Err(RemoteMonitorError::TaskNotFound(task_id.to_string()))
        }
    }

    /// Get task by ID
    pub async fn get_task(&self, task_id: &str) -> Option<RemoteTask> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    /// List all tasks
    pub async fn list_tasks(&self) -> Vec<RemoteTask> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// List tasks by node
    pub async fn list_tasks_by_node(&self, node_id: &str) -> Vec<RemoteTask> {
        let tasks = self.tasks.read().await;
        tasks
            .values()
            .filter(|t| t.node_id == node_id)
            .cloned()
            .collect()
    }

    /// List tasks by status
    pub async fn list_tasks_by_status(&self, status: RemoteTaskStatus) -> Vec<RemoteTask> {
        let tasks = self.tasks.read().await;
        tasks
            .values()
            .filter(|t| t.status == status)
            .cloned()
            .collect()
    }

    /// List tasks by agent
    pub async fn list_tasks_by_agent(&self, agent_id: &str) -> Vec<RemoteTask> {
        let tasks = self.tasks.read().await;
        tasks
            .values()
            .filter(|t| t.agent_id == agent_id)
            .cloned()
            .collect()
    }

    /// Get monitoring statistics
    pub async fn get_stats(&self) -> MonitorStats {
        let nodes = self.nodes.read().await;
        let tasks = self.tasks.read().await;

        let online_nodes = nodes
            .values()
            .filter(|n| n.status == NodeStatus::Online)
            .count();
        let total_agents: u32 = nodes.values().map(|n| n.active_agents).sum();

        let running_tasks = tasks
            .values()
            .filter(|t| t.status == RemoteTaskStatus::Running)
            .count();
        let queued_tasks = tasks
            .values()
            .filter(|t| t.status == RemoteTaskStatus::Queued)
            .count();
        let completed_tasks = tasks
            .values()
            .filter(|t| t.status == RemoteTaskStatus::Completed)
            .count();
        let failed_tasks = tasks
            .values()
            .filter(|t| t.status == RemoteTaskStatus::Failed)
            .count();

        MonitorStats {
            total_nodes: nodes.len(),
            online_nodes,
            total_agents: total_agents as usize,
            total_tasks: tasks.len(),
            running_tasks,
            queued_tasks,
            completed_tasks,
            failed_tasks,
        }
    }

    /// Check for timed out nodes and update their status
    pub async fn check_node_timeouts(&self) {
        let timeout = chrono::Duration::seconds(self.config.node_timeout_secs as i64);
        let now = Utc::now();

        let mut nodes = self.nodes.write().await;
        for node in nodes.values_mut() {
            if node.status == NodeStatus::Online && now - node.last_heartbeat > timeout {
                let old_status = node.status;
                node.status = NodeStatus::Offline;
                let _ = self.event_tx.send(RemoteEvent::NodeStatusChanged {
                    node_id: node.id.clone(),
                    old_status,
                    new_status: NodeStatus::Offline,
                });
            }
        }
    }

    /// Get configuration
    pub fn config(&self) -> &RemoteMonitorConfig {
        &self.config
    }
}

/// Monitor statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorStats {
    pub total_nodes: usize,
    pub online_nodes: usize,
    pub total_agents: usize,
    pub total_tasks: usize,
    pub running_tasks: usize,
    pub queued_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
}

// =============================================================================
// Remote Agent Client
// =============================================================================

/// Client for remote agents to report progress
pub struct RemoteAgentClient {
    /// Node ID this client represents
    pub node_id: NodeId,
    /// Server URL
    server_url: String,
    /// Authentication token
    auth_token: Option<String>,
    /// HTTP client
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl RemoteAgentClient {
    /// Create a new remote agent client
    pub fn new(node_id: impl Into<String>, server_url: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            server_url: server_url.into(),
            auth_token: None,
            client: reqwest::Client::new(),
        }
    }

    /// Set authentication token
    pub fn with_auth(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    /// Send heartbeat
    pub async fn heartbeat(&self) -> Result<(), RemoteMonitorError> {
        let url = format!(
            "{}/api/v1/nodes/{}/heartbeat",
            self.server_url, self.node_id
        );

        let mut request = self.client.post(&url);
        if let Some(ref token) = self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| RemoteMonitorError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RemoteMonitorError::ApiError(
                response.status().as_u16(),
                response.text().await.unwrap_or_default(),
            ));
        }

        Ok(())
    }

    /// Report task progress
    pub async fn report_progress(
        &self,
        task_id: &str,
        progress: f32,
        message: Option<&str>,
    ) -> Result<(), RemoteMonitorError> {
        let url = format!("{}/api/v1/tasks/{}/progress", self.server_url, task_id);

        let body = serde_json::json!({
            "progress": progress,
            "message": message,
        });

        let mut request = self.client.post(&url).json(&body);
        if let Some(ref token) = self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| RemoteMonitorError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RemoteMonitorError::ApiError(
                response.status().as_u16(),
                response.text().await.unwrap_or_default(),
            ));
        }

        Ok(())
    }

    /// Report task step
    pub async fn report_step(
        &self,
        task_id: &str,
        step: u32,
        total: u32,
        description: Option<&str>,
    ) -> Result<(), RemoteMonitorError> {
        let url = format!("{}/api/v1/tasks/{}/step", self.server_url, task_id);

        let body = serde_json::json!({
            "step": step,
            "total": total,
            "description": description,
        });

        let mut request = self.client.post(&url).json(&body);
        if let Some(ref token) = self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| RemoteMonitorError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RemoteMonitorError::ApiError(
                response.status().as_u16(),
                response.text().await.unwrap_or_default(),
            ));
        }

        Ok(())
    }

    /// Report task completion
    pub async fn report_completed(
        &self,
        task_id: &str,
        result: TaskResult,
    ) -> Result<(), RemoteMonitorError> {
        let url = format!("{}/api/v1/tasks/{}/complete", self.server_url, task_id);

        let mut request = self.client.post(&url).json(&result);
        if let Some(ref token) = self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| RemoteMonitorError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RemoteMonitorError::ApiError(
                response.status().as_u16(),
                response.text().await.unwrap_or_default(),
            ));
        }

        Ok(())
    }

    /// Report task failure
    pub async fn report_failed(
        &self,
        task_id: &str,
        error: &str,
    ) -> Result<(), RemoteMonitorError> {
        let url = format!("{}/api/v1/tasks/{}/fail", self.server_url, task_id);

        let body = serde_json::json!({
            "error": error,
        });

        let mut request = self.client.post(&url).json(&body);
        if let Some(ref token) = self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| RemoteMonitorError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RemoteMonitorError::ApiError(
                response.status().as_u16(),
                response.text().await.unwrap_or_default(),
            ));
        }

        Ok(())
    }

    /// Send log entry
    pub async fn send_log(
        &self,
        task_id: &str,
        level: LogLevel,
        message: &str,
    ) -> Result<(), RemoteMonitorError> {
        let url = format!("{}/api/v1/tasks/{}/log", self.server_url, task_id);

        let body = serde_json::json!({
            "level": level,
            "message": message,
            "timestamp": Utc::now(),
        });

        let mut request = self.client.post(&url).json(&body);
        if let Some(ref token) = self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| RemoteMonitorError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RemoteMonitorError::ApiError(
                response.status().as_u16(),
                response.text().await.unwrap_or_default(),
            ));
        }

        Ok(())
    }
}

// =============================================================================
// Task Builder
// =============================================================================

/// Builder for creating remote tasks
pub struct RemoteTaskBuilder {
    task: RemoteTask,
}

impl RemoteTaskBuilder {
    /// Create a new task builder
    pub fn new(
        id: impl Into<String>,
        node_id: impl Into<String>,
        agent_id: impl Into<String>,
    ) -> Self {
        Self {
            task: RemoteTask {
                id: id.into(),
                node_id: node_id.into(),
                agent_id: agent_id.into(),
                agent_name: String::new(),
                title: String::new(),
                description: None,
                status: RemoteTaskStatus::Queued,
                progress: 0.0,
                progress_message: None,
                current_step: None,
                total_steps: None,
                priority: TaskPriority::Normal,
                started_at: Utc::now(),
                completed_at: None,
                eta: None,
                result: None,
                error: None,
                resources: ResourceUsage::default(),
                logs: Vec::new(),
                metadata: HashMap::new(),
            },
        }
    }

    /// Set agent name
    pub fn agent_name(mut self, name: impl Into<String>) -> Self {
        self.task.agent_name = name.into();
        self
    }

    /// Set task title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.task.title = title.into();
        self
    }

    /// Set task description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.task.description = Some(description.into());
        self
    }

    /// Set task priority
    pub fn priority(mut self, priority: TaskPriority) -> Self {
        self.task.priority = priority;
        self
    }

    /// Set total steps
    pub fn total_steps(mut self, steps: u32) -> Self {
        self.task.total_steps = Some(steps);
        self
    }

    /// Set ETA
    pub fn eta(mut self, eta: DateTime<Utc>) -> Self {
        self.task.eta = Some(eta);
        self
    }

    /// Add metadata
    pub fn metadata(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.task.metadata.insert(key.into(), v);
        }
        self
    }

    /// Build the task
    pub fn build(self) -> RemoteTask {
        self.task
    }
}

// =============================================================================
// Error Types
// =============================================================================

/// Remote monitor errors
#[derive(Debug, Clone)]
pub enum RemoteMonitorError {
    /// Node not found
    NodeNotFound(String),
    /// Task not found
    TaskNotFound(String),
    /// Network error
    Network(String),
    /// API error
    ApiError(u16, String),
    /// Authentication error
    AuthError(String),
    /// Invalid state
    InvalidState(String),
}

impl std::fmt::Display for RemoteMonitorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoteMonitorError::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            RemoteMonitorError::TaskNotFound(id) => write!(f, "Task not found: {}", id),
            RemoteMonitorError::Network(e) => write!(f, "Network error: {}", e),
            RemoteMonitorError::ApiError(code, msg) => write!(f, "API error {}: {}", code, msg),
            RemoteMonitorError::AuthError(e) => write!(f, "Authentication error: {}", e),
            RemoteMonitorError::InvalidState(e) => write!(f, "Invalid state: {}", e),
        }
    }
}

impl std::error::Error for RemoteMonitorError {}

// =============================================================================
// Helper Functions
// =============================================================================

/// Generate a unique task ID
pub fn generate_task_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("task_{:x}", timestamp)
}

/// Generate a unique node ID
pub fn generate_node_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("node_{:x}", timestamp)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_remote_monitor_creation() {
        let config = RemoteMonitorConfig::default();
        let monitor = RemoteMonitor::new(config);

        let stats = monitor.get_stats().await;
        assert_eq!(stats.total_nodes, 0);
        assert_eq!(stats.total_tasks, 0);
    }

    #[tokio::test]
    async fn test_node_registration() {
        let monitor = RemoteMonitor::new(RemoteMonitorConfig::default());

        let node = RemoteNode {
            id: "test-node".to_string(),
            name: "Test Node".to_string(),
            address: "localhost:9876".to_string(),
            status: NodeStatus::Online,
            tags: vec!["test".to_string()],
            hardware: None,
            active_agents: 0,
            running_tasks: 0,
            last_heartbeat: Utc::now(),
            registered_at: Utc::now(),
            metadata: HashMap::new(),
        };

        monitor.register_node(node).await.unwrap();

        let retrieved = monitor.get_node("test-node").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Node");
    }

    #[tokio::test]
    async fn test_task_creation() {
        let monitor = RemoteMonitor::new(RemoteMonitorConfig::default());

        // First register a node
        let node = RemoteNode {
            id: "test-node".to_string(),
            name: "Test Node".to_string(),
            address: "localhost:9876".to_string(),
            status: NodeStatus::Online,
            tags: vec![],
            hardware: None,
            active_agents: 1,
            running_tasks: 0,
            last_heartbeat: Utc::now(),
            registered_at: Utc::now(),
            metadata: HashMap::new(),
        };
        monitor.register_node(node).await.unwrap();

        // Create a task
        let task = RemoteTaskBuilder::new("task-1", "test-node", "agent-1")
            .agent_name("Test Agent")
            .title("Test Task")
            .description("A test task")
            .priority(TaskPriority::High)
            .build();

        let task_id = monitor.create_task(task).await.unwrap();
        assert_eq!(task_id, "task-1");

        let retrieved = monitor.get_task("task-1").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test Task");
    }

    #[tokio::test]
    async fn test_task_progress() {
        let monitor = RemoteMonitor::new(RemoteMonitorConfig::default());

        // Register node
        let node = RemoteNode {
            id: "node-1".to_string(),
            name: "Node 1".to_string(),
            address: "localhost:9876".to_string(),
            status: NodeStatus::Online,
            tags: vec![],
            hardware: None,
            active_agents: 1,
            running_tasks: 0,
            last_heartbeat: Utc::now(),
            registered_at: Utc::now(),
            metadata: HashMap::new(),
        };
        monitor.register_node(node).await.unwrap();

        // Create task
        let task = RemoteTaskBuilder::new("task-1", "node-1", "agent-1")
            .title("Progress Test")
            .build();
        monitor.create_task(task).await.unwrap();

        // Update progress
        monitor
            .update_task_progress("task-1", 0.5, Some("Halfway done".to_string()))
            .await
            .unwrap();

        let task = monitor.get_task("task-1").await.unwrap();
        assert!((task.progress - 0.5).abs() < 0.01);
        assert_eq!(task.progress_message, Some("Halfway done".to_string()));
    }

    #[test]
    fn test_task_builder() {
        let task = RemoteTaskBuilder::new("task-123", "node-1", "agent-1")
            .agent_name("My Agent")
            .title("Important Task")
            .description("Does important things")
            .priority(TaskPriority::Critical)
            .total_steps(5)
            .build();

        assert_eq!(task.id, "task-123");
        assert_eq!(task.node_id, "node-1");
        assert_eq!(task.agent_id, "agent-1");
        assert_eq!(task.agent_name, "My Agent");
        assert_eq!(task.title, "Important Task");
        assert_eq!(task.priority, TaskPriority::Critical);
        assert_eq!(task.total_steps, Some(5));
        assert_eq!(task.status, RemoteTaskStatus::Queued);
    }
}
