// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Multi-Agent Orchestration
//!
//! Provides patterns for orchestrating multiple agents:
//! - Sequential: Agents execute one after another
//! - Parallel: Agents execute simultaneously
//! - Loop: Agent repeats until condition met
//! - Hierarchical: Coordinator delegates to sub-agents

#![allow(dead_code)]

use crate::agency::agent::Agent;
use crate::agency::error::{AgencyError, AgencyResult};
use crate::agency::executor::{ExecutionContext, ExecutionResult, Executor};
use crate::agency::models::{AgencyEvent, EventType, TokenUsage};
use crate::agency::session::Session;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Orchestration type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrchestrationType {
    /// Agents execute sequentially, passing state forward
    Sequential,
    /// Agents execute in parallel
    Parallel,
    /// Single agent loops until condition met
    Loop,
    /// Coordinator agent delegates to sub-agents
    Hierarchical,
}

/// A pipeline of agents
#[derive(Debug, Clone)]
pub struct Pipeline {
    /// Pipeline name
    pub name: String,
    /// Orchestration type
    pub orchestration: OrchestrationType,
    /// Agents in the pipeline
    pub agents: Vec<Arc<Agent>>,
    /// Maximum iterations for loop orchestration
    pub max_iterations: u32,
}

impl Pipeline {
    /// Create a sequential pipeline
    pub fn sequential(name: impl Into<String>, agents: Vec<Agent>) -> Self {
        Self {
            name: name.into(),
            orchestration: OrchestrationType::Sequential,
            agents: agents.into_iter().map(Arc::new).collect(),
            max_iterations: 1,
        }
    }

    /// Create a parallel pipeline
    pub fn parallel(name: impl Into<String>, agents: Vec<Agent>) -> Self {
        Self {
            name: name.into(),
            orchestration: OrchestrationType::Parallel,
            agents: agents.into_iter().map(Arc::new).collect(),
            max_iterations: 1,
        }
    }

    /// Create a loop pipeline with a single agent
    pub fn loop_agent(name: impl Into<String>, agent: Agent, max_iterations: u32) -> Self {
        Self {
            name: name.into(),
            orchestration: OrchestrationType::Loop,
            agents: vec![Arc::new(agent)],
            max_iterations,
        }
    }
}

/// A swarm of agents with a coordinator
#[derive(Debug, Clone)]
pub struct Swarm {
    /// Swarm name
    pub name: String,
    /// Description
    pub description: String,
    /// Coordinator agent
    pub coordinator: Arc<Agent>,
    /// Worker agents
    pub workers: Vec<Arc<Agent>>,
    /// Goal description
    pub goal: Option<String>,
}

impl Swarm {
    /// Create a new swarm
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        coordinator: Agent,
        workers: Vec<Agent>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            coordinator: Arc::new(coordinator),
            workers: workers.into_iter().map(Arc::new).collect(),
            goal: None,
        }
    }

    /// Set the goal
    pub fn with_goal(mut self, goal: impl Into<String>) -> Self {
        self.goal = Some(goal.into());
        self
    }
}

/// Orchestrator handles multi-agent execution
pub struct Orchestrator {
    executor: Arc<Executor>,
}

impl Orchestrator {
    /// Create a new orchestrator
    pub fn new(executor: Arc<Executor>) -> Self {
        Self { executor }
    }

    /// Run a pipeline
    pub async fn run_pipeline(
        &self,
        pipeline: &Pipeline,
        input: &str,
        ctx: &mut ExecutionContext,
    ) -> AgencyResult<OrchestratorResult> {
        match pipeline.orchestration {
            OrchestrationType::Sequential => self.run_sequential(pipeline, input, ctx).await,
            OrchestrationType::Parallel => self.run_parallel(pipeline, input, ctx).await,
            OrchestrationType::Loop => self.run_loop(pipeline, input, ctx).await,
            OrchestrationType::Hierarchical => Err(AgencyError::OrchestrationError(
                "Use run_swarm for hierarchical orchestration".to_string(),
            )),
        }
    }

    /// Run agents sequentially
    async fn run_sequential(
        &self,
        pipeline: &Pipeline,
        input: &str,
        ctx: &mut ExecutionContext,
    ) -> AgencyResult<OrchestratorResult> {
        let start_time = std::time::Instant::now();
        let mut results = Vec::new();
        let mut events = Vec::new();
        let mut token_usage = TokenUsage::default();
        let mut current_input = input.to_string();

        for agent_arc in &pipeline.agents {
            let agent = agent_arc.as_ref();
            let mut session = Session::new(agent.name(), ctx.user_id.clone());

            let result = self
                .executor
                .execute(agent, &mut session, &current_input, ctx)
                .await?;

            // Use this agent's output as next agent's input
            current_input = result.response.clone();
            token_usage.add(&result.token_usage);
            events.extend(result.events.clone());
            results.push(result);
        }

        let final_response = results
            .last()
            .map(|r| r.response.clone())
            .unwrap_or_default();

        Ok(OrchestratorResult {
            response: final_response,
            agent_results: results,
            events,
            token_usage,
            duration_ms: start_time.elapsed().as_millis() as u64,
            iterations: 1,
        })
    }

    /// Run agents in parallel
    async fn run_parallel(
        &self,
        pipeline: &Pipeline,
        input: &str,
        ctx: &mut ExecutionContext,
    ) -> AgencyResult<OrchestratorResult> {
        let start_time = std::time::Instant::now();
        let mut handles = Vec::new();

        for agent_arc in &pipeline.agents {
            let agent = agent_arc.clone();
            let executor = self.executor.clone();
            let input = input.to_string();
            let user_id = ctx.user_id.clone();

            handles.push(tokio::spawn(async move {
                let mut session = Session::new(agent.name(), user_id.clone());
                let mut ctx = ExecutionContext::new(&session);
                ctx.user_id = user_id;

                executor
                    .execute(agent.as_ref(), &mut session, &input, &mut ctx)
                    .await
            }));
        }

        let mut results = Vec::new();
        let mut events = Vec::new();
        let mut token_usage = TokenUsage::default();
        let mut responses = Vec::new();

        for handle in handles {
            match handle.await {
                Ok(Ok(result)) => {
                    responses.push(result.response.clone());
                    token_usage.add(&result.token_usage);
                    events.extend(result.events.clone());
                    results.push(result);
                }
                Ok(Err(e)) => {
                    return Err(e);
                }
                Err(e) => {
                    return Err(AgencyError::ExecutionFailed(e.to_string()));
                }
            }
        }

        // Combine responses
        let final_response = responses.join("\n\n---\n\n");

        Ok(OrchestratorResult {
            response: final_response,
            agent_results: results,
            events,
            token_usage,
            duration_ms: start_time.elapsed().as_millis() as u64,
            iterations: 1,
        })
    }

    /// Run agent in a loop until condition met or max iterations
    async fn run_loop(
        &self,
        pipeline: &Pipeline,
        input: &str,
        ctx: &mut ExecutionContext,
    ) -> AgencyResult<OrchestratorResult> {
        let start_time = std::time::Instant::now();
        let mut results = Vec::new();
        let mut events = Vec::new();
        let mut token_usage = TokenUsage::default();
        let mut current_input = input.to_string();
        let mut iterations = 0;

        let agent_arc = pipeline.agents.first().ok_or_else(|| {
            AgencyError::OrchestrationError("Loop pipeline requires at least one agent".to_string())
        })?;

        loop {
            iterations += 1;
            if iterations > pipeline.max_iterations {
                break;
            }

            let agent = agent_arc.as_ref();
            let mut session = Session::new(agent.name(), ctx.user_id.clone());

            let result = self
                .executor
                .execute(agent, &mut session, &current_input, ctx)
                .await?;

            token_usage.add(&result.token_usage);
            events.extend(result.events.clone());
            results.push(result.clone());

            // Check for completion signal in response
            // (e.g., "DONE", "COMPLETE", or other markers)
            if result.response.contains("DONE")
                || result.response.contains("COMPLETE")
                || result.response.contains("FINISHED")
            {
                break;
            }

            // Use output as next input
            current_input = result.response;
        }

        let final_response = results
            .last()
            .map(|r| r.response.clone())
            .unwrap_or_default();

        Ok(OrchestratorResult {
            response: final_response,
            agent_results: results,
            events,
            token_usage,
            duration_ms: start_time.elapsed().as_millis() as u64,
            iterations,
        })
    }

    /// Run a swarm with coordinator
    pub async fn run_swarm(
        &self,
        swarm: &Swarm,
        input: &str,
        ctx: &mut ExecutionContext,
    ) -> AgencyResult<OrchestratorResult> {
        let start_time = std::time::Instant::now();
        let mut results = Vec::new();
        let mut events = Vec::new();
        let mut token_usage = TokenUsage::default();

        // First, have coordinator analyze and delegate
        let coordinator = swarm.coordinator.as_ref();
        let mut coord_session = Session::new(coordinator.name(), ctx.user_id.clone());

        // Build context for coordinator including available workers
        let worker_info: Vec<_> = swarm
            .workers
            .iter()
            .map(|w| format!("- {}: {}", w.name(), w.description()))
            .collect();

        let coordinator_input = format!(
            "Task: {}\n\nAvailable workers:\n{}\n\nAnalyze the task and delegate to appropriate workers.",
            input,
            worker_info.join("\n")
        );

        let coord_result = self
            .executor
            .execute(coordinator, &mut coord_session, &coordinator_input, ctx)
            .await?;

        token_usage.add(&coord_result.token_usage);
        events.extend(coord_result.events.clone());
        results.push(coord_result.clone());

        // Emit handoff event
        let handoff_event = AgencyEvent {
            event_type: EventType::Handoff,
            agent_name: coordinator.name().to_string(),
            data: serde_json::json!({
                "from": coordinator.name(),
                "task": input
            }),
            timestamp: Utc::now(),
            session_id: Some(coord_session.id.clone()),
        };
        events.push(handoff_event.clone());
        ctx.emit(handoff_event).await;

        // TODO: Parse coordinator response to determine which workers to call
        // For now, run all workers in parallel with the original input
        for worker_arc in &swarm.workers {
            let worker = worker_arc.as_ref();
            let mut worker_session = Session::new(worker.name(), ctx.user_id.clone());

            let worker_result = self
                .executor
                .execute(worker, &mut worker_session, input, ctx)
                .await?;

            token_usage.add(&worker_result.token_usage);
            events.extend(worker_result.events.clone());
            results.push(worker_result);
        }

        // Have coordinator synthesize results
        let worker_results: Vec<_> = results
            .iter()
            .skip(1) // Skip coordinator's initial result
            .map(|r| format!("Result: {}", r.response))
            .collect();

        let synthesis_input = format!(
            "Original task: {}\n\nWorker results:\n{}\n\nSynthesize these results into a final response.",
            input,
            worker_results.join("\n\n")
        );

        let final_result = self
            .executor
            .execute(coordinator, &mut coord_session, &synthesis_input, ctx)
            .await?;

        token_usage.add(&final_result.token_usage);
        events.extend(final_result.events.clone());
        results.push(final_result.clone());

        Ok(OrchestratorResult {
            response: final_result.response,
            agent_results: results,
            events,
            token_usage,
            duration_ms: start_time.elapsed().as_millis() as u64,
            iterations: 1,
        })
    }
}

/// Result from orchestrator execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorResult {
    /// Final combined response
    pub response: String,
    /// Individual agent results
    pub agent_results: Vec<ExecutionResult>,
    /// All events emitted
    pub events: Vec<AgencyEvent>,
    /// Total token usage
    pub token_usage: TokenUsage,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Number of iterations (for loop orchestration)
    pub iterations: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agency::agent::AgentBuilder;
    use crate::agency::tools::ToolRegistry;

    fn create_test_agent(name: &str) -> Agent {
        AgentBuilder::new(name)
            .description(format!("{} agent", name))
            .instruction("You are a helpful assistant.")
            .model("gemini-2.5-flash")
            .build()
    }

    #[tokio::test]
    #[ignore = "Integration test - requires API credentials"]
    async fn test_sequential_pipeline() {
        let tool_registry = Arc::new(ToolRegistry::new());
        let executor = Arc::new(Executor::new(tool_registry));
        let orchestrator = Orchestrator::new(executor);

        let agents = vec![create_test_agent("researcher"), create_test_agent("writer")];
        let pipeline = Pipeline::sequential("research_pipeline", agents);

        let session = Session::new("test", None);
        let mut ctx = ExecutionContext::new(&session);

        let result = orchestrator
            .run_pipeline(&pipeline, "Tell me about Rust", &mut ctx)
            .await
            .unwrap();

        assert!(!result.response.is_empty());
        assert_eq!(result.agent_results.len(), 2);
    }

    #[tokio::test]
    #[ignore = "Integration test - requires API credentials"]
    async fn test_parallel_pipeline() {
        let tool_registry = Arc::new(ToolRegistry::new());
        let executor = Arc::new(Executor::new(tool_registry));
        let orchestrator = Orchestrator::new(executor);

        let agents = vec![create_test_agent("analyst1"), create_test_agent("analyst2")];
        let pipeline = Pipeline::parallel("analysis_pipeline", agents);

        let session = Session::new("test", None);
        let mut ctx = ExecutionContext::new(&session);

        let result = orchestrator
            .run_pipeline(&pipeline, "Analyze this data", &mut ctx)
            .await
            .unwrap();

        assert!(!result.response.is_empty());
        assert_eq!(result.agent_results.len(), 2);
    }
}
