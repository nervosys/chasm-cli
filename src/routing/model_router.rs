// Copyright (c) 2024-2027 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Multi-model conversation routing
//!
//! Routes conversations to optimal models based on task type, complexity,
//! cost constraints, and performance requirements.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// Task Classification
// ============================================================================

/// Task type detected from conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// General conversation/chat
    Chat,
    /// Code generation or assistance
    Coding,
    /// Code review and analysis
    CodeReview,
    /// Bug fixing and debugging
    Debugging,
    /// Writing and editing text
    Writing,
    /// Creative writing and brainstorming
    Creative,
    /// Mathematical reasoning
    Math,
    /// Data analysis
    Analysis,
    /// Research and information retrieval
    Research,
    /// Translation between languages
    Translation,
    /// Summarization of content
    Summarization,
    /// Question answering
    QuestionAnswering,
    /// Image understanding (multi-modal)
    Vision,
    /// Complex reasoning tasks
    Reasoning,
    /// Simple/quick queries
    Quick,
}

impl TaskType {
    /// Get complexity weight (1-10)
    pub fn complexity_weight(&self) -> u8 {
        match self {
            TaskType::Quick => 1,
            TaskType::Chat => 2,
            TaskType::QuestionAnswering => 3,
            TaskType::Translation => 4,
            TaskType::Summarization => 4,
            TaskType::Writing => 5,
            TaskType::Coding => 6,
            TaskType::CodeReview => 6,
            TaskType::Creative => 6,
            TaskType::Analysis => 7,
            TaskType::Debugging => 7,
            TaskType::Research => 7,
            TaskType::Math => 8,
            TaskType::Vision => 8,
            TaskType::Reasoning => 9,
        }
    }

    /// Detect task type from message content
    pub fn detect(content: &str) -> Self {
        let lower = content.to_lowercase();

        // Code-related keywords
        if lower.contains("```") || lower.contains("code") || lower.contains("function") 
            || lower.contains("class") || lower.contains("implement") {
            if lower.contains("review") || lower.contains("check") {
                return TaskType::CodeReview;
            }
            if lower.contains("bug") || lower.contains("fix") || lower.contains("error") 
                || lower.contains("debug") {
                return TaskType::Debugging;
            }
            return TaskType::Coding;
        }

        // Math keywords
        if lower.contains("calculate") || lower.contains("equation") || lower.contains("solve")
            || lower.contains("math") || lower.contains("formula") {
            return TaskType::Math;
        }

        // Analysis keywords
        if lower.contains("analyze") || lower.contains("analysis") || lower.contains("data")
            || lower.contains("statistics") || lower.contains("trend") {
            return TaskType::Analysis;
        }

        // Research keywords
        if lower.contains("research") || lower.contains("find out") || lower.contains("look up")
            || lower.contains("search for") {
            return TaskType::Research;
        }

        // Writing keywords
        if lower.contains("write") || lower.contains("draft") || lower.contains("compose")
            || lower.contains("edit") {
            if lower.contains("creative") || lower.contains("story") || lower.contains("poem") {
                return TaskType::Creative;
            }
            return TaskType::Writing;
        }

        // Translation
        if lower.contains("translate") || lower.contains("translation") {
            return TaskType::Translation;
        }

        // Summarization
        if lower.contains("summarize") || lower.contains("summary") || lower.contains("tldr") {
            return TaskType::Summarization;
        }

        // Reasoning
        if lower.contains("why") || lower.contains("reason") || lower.contains("explain")
            || lower.contains("logic") {
            return TaskType::Reasoning;
        }

        // Image/vision
        if lower.contains("image") || lower.contains("picture") || lower.contains("photo")
            || lower.contains("see") || lower.contains("look at") {
            return TaskType::Vision;
        }

        // Question answering
        if lower.ends_with('?') || lower.starts_with("what") || lower.starts_with("how")
            || lower.starts_with("when") || lower.starts_with("where") {
            return TaskType::QuestionAnswering;
        }

        // Short messages are quick queries
        if content.len() < 50 {
            return TaskType::Quick;
        }

        TaskType::Chat
    }
}

// ============================================================================
// Model Capabilities
// ============================================================================

/// Model capability profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Model identifier
    pub model_id: String,
    /// Provider (e.g., "openai", "anthropic", "google")
    pub provider: String,
    /// Display name
    pub name: String,
    /// Task type scores (0.0 - 1.0)
    pub task_scores: HashMap<TaskType, f64>,
    /// Context window size
    pub context_window: usize,
    /// Whether supports vision/images
    pub supports_vision: bool,
    /// Whether supports function calling
    pub supports_functions: bool,
    /// Whether supports streaming
    pub supports_streaming: bool,
    /// Cost per 1K input tokens (USD)
    pub cost_per_1k_input: f64,
    /// Cost per 1K output tokens (USD)
    pub cost_per_1k_output: f64,
    /// Average latency in ms
    pub avg_latency_ms: u32,
    /// Whether model is available
    pub available: bool,
}

impl ModelCapabilities {
    /// Create a new model capabilities profile
    pub fn new(model_id: &str, provider: &str, name: &str) -> Self {
        Self {
            model_id: model_id.to_string(),
            provider: provider.to_string(),
            name: name.to_string(),
            task_scores: HashMap::new(),
            context_window: 4096,
            supports_vision: false,
            supports_functions: false,
            supports_streaming: true,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            avg_latency_ms: 1000,
            available: true,
        }
    }

    /// Set task score
    pub fn with_task_score(mut self, task: TaskType, score: f64) -> Self {
        self.task_scores.insert(task, score.clamp(0.0, 1.0));
        self
    }

    /// Set context window
    pub fn with_context_window(mut self, size: usize) -> Self {
        self.context_window = size;
        self
    }

    /// Set vision support
    pub fn with_vision(mut self, supports: bool) -> Self {
        self.supports_vision = supports;
        self
    }

    /// Set function calling support
    pub fn with_functions(mut self, supports: bool) -> Self {
        self.supports_functions = supports;
        self
    }

    /// Set cost
    pub fn with_cost(mut self, input: f64, output: f64) -> Self {
        self.cost_per_1k_input = input;
        self.cost_per_1k_output = output;
        self
    }

    /// Set latency
    pub fn with_latency(mut self, ms: u32) -> Self {
        self.avg_latency_ms = ms;
        self
    }

    /// Get score for a task type
    pub fn score_for_task(&self, task: TaskType) -> f64 {
        self.task_scores.get(&task).copied().unwrap_or(0.5)
    }
}

// ============================================================================
// Routing Configuration
// ============================================================================

/// Routing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    /// Best quality regardless of cost
    BestQuality,
    /// Lowest cost that meets quality threshold
    LowestCost,
    /// Fastest response time
    FastestResponse,
    /// Balance quality and cost
    Balanced,
    /// Custom weighted scoring
    Custom,
}

/// Routing constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConstraints {
    /// Maximum cost per request (USD)
    pub max_cost: Option<f64>,
    /// Maximum latency (ms)
    pub max_latency_ms: Option<u32>,
    /// Minimum context window required
    pub min_context_window: Option<usize>,
    /// Required providers (whitelist)
    pub allowed_providers: Option<Vec<String>>,
    /// Blocked providers (blacklist)
    pub blocked_providers: Vec<String>,
    /// Require vision support
    pub require_vision: bool,
    /// Require function calling
    pub require_functions: bool,
}

impl Default for RoutingConstraints {
    fn default() -> Self {
        Self {
            max_cost: None,
            max_latency_ms: None,
            min_context_window: None,
            allowed_providers: None,
            blocked_providers: vec![],
            require_vision: false,
            require_functions: false,
        }
    }
}

/// Routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Routing strategy
    pub strategy: RoutingStrategy,
    /// Constraints
    pub constraints: RoutingConstraints,
    /// Quality weight (0.0 - 1.0)
    pub quality_weight: f64,
    /// Cost weight (0.0 - 1.0)
    pub cost_weight: f64,
    /// Latency weight (0.0 - 1.0)
    pub latency_weight: f64,
    /// Fallback model if routing fails
    pub fallback_model: Option<String>,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            strategy: RoutingStrategy::Balanced,
            constraints: RoutingConstraints::default(),
            quality_weight: 0.5,
            cost_weight: 0.3,
            latency_weight: 0.2,
            fallback_model: None,
        }
    }
}

// ============================================================================
// Routing Request/Response
// ============================================================================

/// Routing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRequest {
    /// Request ID
    pub id: Uuid,
    /// Message content to route
    pub content: String,
    /// Conversation context (previous messages)
    pub context: Vec<String>,
    /// Estimated token count
    pub estimated_tokens: usize,
    /// User preferences
    pub config: RoutingConfig,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Routing decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Request ID
    pub request_id: Uuid,
    /// Selected model
    pub model_id: String,
    /// Provider
    pub provider: String,
    /// Detected task type
    pub task_type: TaskType,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Estimated cost
    pub estimated_cost: f64,
    /// Estimated latency
    pub estimated_latency_ms: u32,
    /// Alternative models considered
    pub alternatives: Vec<ModelScore>,
    /// Reasoning for selection
    pub reasoning: String,
    /// Decision timestamp
    pub decided_at: DateTime<Utc>,
}

/// Model score during routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelScore {
    /// Model ID
    pub model_id: String,
    /// Provider
    pub provider: String,
    /// Quality score
    pub quality_score: f64,
    /// Cost score
    pub cost_score: f64,
    /// Latency score
    pub latency_score: f64,
    /// Total weighted score
    pub total_score: f64,
    /// Why not selected (if applicable)
    pub rejection_reason: Option<String>,
}

// ============================================================================
// Model Router
// ============================================================================

/// Multi-model conversation router
pub struct ModelRouter {
    /// Available models
    models: Vec<ModelCapabilities>,
    /// Default configuration
    default_config: RoutingConfig,
    /// Routing history for learning
    history: Vec<RoutingDecision>,
}

impl ModelRouter {
    /// Create a new model router
    pub fn new() -> Self {
        Self {
            models: Self::default_models(),
            default_config: RoutingConfig::default(),
            history: vec![],
        }
    }

    /// Create router with custom models
    pub fn with_models(models: Vec<ModelCapabilities>) -> Self {
        Self {
            models,
            default_config: RoutingConfig::default(),
            history: vec![],
        }
    }

    /// Add a model
    pub fn add_model(&mut self, model: ModelCapabilities) {
        self.models.push(model);
    }

    /// Route a request to the optimal model
    pub fn route(&mut self, request: &RoutingRequest) -> RoutingDecision {
        // Detect task type
        let task_type = TaskType::detect(&request.content);

        // Score all models
        let mut scores: Vec<ModelScore> = self
            .models
            .iter()
            .filter(|m| m.available)
            .filter(|m| self.meets_constraints(m, &request.config.constraints))
            .map(|m| self.score_model(m, task_type, request))
            .collect();

        // Sort by total score (descending)
        scores.sort_by(|a, b| b.total_score.partial_cmp(&a.total_score).unwrap());

        // Select best model
        let selected = scores.first().cloned().unwrap_or_else(|| {
            // Fallback
            ModelScore {
                model_id: request.config.fallback_model.clone()
                    .unwrap_or_else(|| "gpt-4o-mini".to_string()),
                provider: "openai".to_string(),
                quality_score: 0.5,
                cost_score: 0.5,
                latency_score: 0.5,
                total_score: 0.5,
                rejection_reason: None,
            }
        });

        let decision = RoutingDecision {
            request_id: request.id,
            model_id: selected.model_id.clone(),
            provider: selected.provider.clone(),
            task_type,
            confidence: selected.total_score,
            estimated_cost: self.estimate_cost(&selected.model_id, request.estimated_tokens),
            estimated_latency_ms: self.estimate_latency(&selected.model_id),
            alternatives: scores.into_iter().skip(1).take(3).collect(),
            reasoning: self.generate_reasoning(&selected, task_type),
            decided_at: Utc::now(),
        };

        // Store in history
        self.history.push(decision.clone());

        decision
    }

    /// Check if model meets constraints
    fn meets_constraints(&self, model: &ModelCapabilities, constraints: &RoutingConstraints) -> bool {
        // Check cost
        if let Some(max_cost) = constraints.max_cost {
            if model.cost_per_1k_output > max_cost * 10.0 {
                return false;
            }
        }

        // Check latency
        if let Some(max_latency) = constraints.max_latency_ms {
            if model.avg_latency_ms > max_latency {
                return false;
            }
        }

        // Check context window
        if let Some(min_context) = constraints.min_context_window {
            if model.context_window < min_context {
                return false;
            }
        }

        // Check allowed providers
        if let Some(ref allowed) = constraints.allowed_providers {
            if !allowed.contains(&model.provider) {
                return false;
            }
        }

        // Check blocked providers
        if constraints.blocked_providers.contains(&model.provider) {
            return false;
        }

        // Check vision requirement
        if constraints.require_vision && !model.supports_vision {
            return false;
        }

        // Check function requirement
        if constraints.require_functions && !model.supports_functions {
            return false;
        }

        true
    }

    /// Score a model for routing
    fn score_model(&self, model: &ModelCapabilities, task: TaskType, request: &RoutingRequest) -> ModelScore {
        let config = &request.config;

        // Quality score based on task
        let quality_score = model.score_for_task(task);

        // Cost score (inverse - lower cost = higher score)
        let max_cost = 0.1; // $0.10 per 1K tokens as baseline
        let cost_score = 1.0 - (model.cost_per_1k_output / max_cost).min(1.0);

        // Latency score (inverse - lower latency = higher score)
        let max_latency = 5000.0; // 5 seconds as baseline
        let latency_score = 1.0 - (model.avg_latency_ms as f64 / max_latency).min(1.0);

        // Calculate total based on strategy
        let total_score = match config.strategy {
            RoutingStrategy::BestQuality => quality_score,
            RoutingStrategy::LowestCost => cost_score,
            RoutingStrategy::FastestResponse => latency_score,
            RoutingStrategy::Balanced => {
                (quality_score + cost_score + latency_score) / 3.0
            }
            RoutingStrategy::Custom => {
                config.quality_weight * quality_score
                    + config.cost_weight * cost_score
                    + config.latency_weight * latency_score
            }
        };

        ModelScore {
            model_id: model.model_id.clone(),
            provider: model.provider.clone(),
            quality_score,
            cost_score,
            latency_score,
            total_score,
            rejection_reason: None,
        }
    }

    /// Estimate cost for a request
    fn estimate_cost(&self, model_id: &str, tokens: usize) -> f64 {
        self.models
            .iter()
            .find(|m| m.model_id == model_id)
            .map(|m| (tokens as f64 / 1000.0) * (m.cost_per_1k_input + m.cost_per_1k_output))
            .unwrap_or(0.0)
    }

    /// Estimate latency for a model
    fn estimate_latency(&self, model_id: &str) -> u32 {
        self.models
            .iter()
            .find(|m| m.model_id == model_id)
            .map(|m| m.avg_latency_ms)
            .unwrap_or(1000)
    }

    /// Generate reasoning for the selection
    fn generate_reasoning(&self, selected: &ModelScore, task: TaskType) -> String {
        format!(
            "Selected {} for {:?} task. Quality: {:.0}%, Cost efficiency: {:.0}%, Speed: {:.0}%",
            selected.model_id,
            task,
            selected.quality_score * 100.0,
            selected.cost_score * 100.0,
            selected.latency_score * 100.0
        )
    }

    /// Get default model profiles
    fn default_models() -> Vec<ModelCapabilities> {
        vec![
            // OpenAI models
            ModelCapabilities::new("gpt-4o", "openai", "GPT-4o")
                .with_context_window(128000)
                .with_vision(true)
                .with_functions(true)
                .with_cost(0.005, 0.015)
                .with_latency(800)
                .with_task_score(TaskType::Coding, 0.95)
                .with_task_score(TaskType::Reasoning, 0.95)
                .with_task_score(TaskType::Vision, 0.90)
                .with_task_score(TaskType::Writing, 0.90)
                .with_task_score(TaskType::Analysis, 0.90),

            ModelCapabilities::new("gpt-4o-mini", "openai", "GPT-4o Mini")
                .with_context_window(128000)
                .with_vision(true)
                .with_functions(true)
                .with_cost(0.00015, 0.0006)
                .with_latency(500)
                .with_task_score(TaskType::Chat, 0.85)
                .with_task_score(TaskType::Quick, 0.90)
                .with_task_score(TaskType::QuestionAnswering, 0.85)
                .with_task_score(TaskType::Coding, 0.80),

            ModelCapabilities::new("o1", "openai", "o1")
                .with_context_window(200000)
                .with_vision(true)
                .with_functions(false)
                .with_cost(0.015, 0.06)
                .with_latency(3000)
                .with_task_score(TaskType::Reasoning, 0.99)
                .with_task_score(TaskType::Math, 0.98)
                .with_task_score(TaskType::Coding, 0.97)
                .with_task_score(TaskType::Analysis, 0.95),

            // Anthropic models
            ModelCapabilities::new("claude-sonnet-4-20250514", "anthropic", "Claude Sonnet 4")
                .with_context_window(200000)
                .with_vision(true)
                .with_functions(true)
                .with_cost(0.003, 0.015)
                .with_latency(700)
                .with_task_score(TaskType::Coding, 0.95)
                .with_task_score(TaskType::Writing, 0.95)
                .with_task_score(TaskType::Reasoning, 0.92)
                .with_task_score(TaskType::Analysis, 0.90),

            ModelCapabilities::new("claude-3-5-haiku-20241022", "anthropic", "Claude 3.5 Haiku")
                .with_context_window(200000)
                .with_vision(true)
                .with_functions(true)
                .with_cost(0.0008, 0.004)
                .with_latency(400)
                .with_task_score(TaskType::Chat, 0.85)
                .with_task_score(TaskType::Quick, 0.90)
                .with_task_score(TaskType::Coding, 0.80),

            // Google models
            ModelCapabilities::new("gemini-2.5-flash", "google", "Gemini 2.5 Flash")
                .with_context_window(1000000)
                .with_vision(true)
                .with_functions(true)
                .with_cost(0.000075, 0.0003)
                .with_latency(300)
                .with_task_score(TaskType::Chat, 0.85)
                .with_task_score(TaskType::Quick, 0.95)
                .with_task_score(TaskType::Coding, 0.85)
                .with_task_score(TaskType::Analysis, 0.85),

            ModelCapabilities::new("gemini-2.5-pro", "google", "Gemini 2.5 Pro")
                .with_context_window(1000000)
                .with_vision(true)
                .with_functions(true)
                .with_cost(0.00125, 0.005)
                .with_latency(600)
                .with_task_score(TaskType::Coding, 0.92)
                .with_task_score(TaskType::Reasoning, 0.90)
                .with_task_score(TaskType::Analysis, 0.90)
                .with_task_score(TaskType::Research, 0.90),

            // Local models (Ollama)
            ModelCapabilities::new("llama3.3:70b", "ollama", "Llama 3.3 70B")
                .with_context_window(128000)
                .with_vision(false)
                .with_functions(true)
                .with_cost(0.0, 0.0)
                .with_latency(2000)
                .with_task_score(TaskType::Chat, 0.80)
                .with_task_score(TaskType::Coding, 0.75)
                .with_task_score(TaskType::Writing, 0.80),

            ModelCapabilities::new("qwen2.5-coder:32b", "ollama", "Qwen 2.5 Coder 32B")
                .with_context_window(32000)
                .with_vision(false)
                .with_functions(false)
                .with_cost(0.0, 0.0)
                .with_latency(1500)
                .with_task_score(TaskType::Coding, 0.85)
                .with_task_score(TaskType::CodeReview, 0.85)
                .with_task_score(TaskType::Debugging, 0.80),
        ]
    }

    /// Get routing statistics
    pub fn stats(&self) -> RouterStats {
        let mut task_counts: HashMap<TaskType, usize> = HashMap::new();
        let mut model_counts: HashMap<String, usize> = HashMap::new();
        let mut total_cost = 0.0;

        for decision in &self.history {
            *task_counts.entry(decision.task_type).or_insert(0) += 1;
            *model_counts.entry(decision.model_id.clone()).or_insert(0) += 1;
            total_cost += decision.estimated_cost;
        }

        RouterStats {
            total_requests: self.history.len(),
            task_distribution: task_counts,
            model_distribution: model_counts,
            total_estimated_cost: total_cost,
            avg_confidence: self.history.iter().map(|d| d.confidence).sum::<f64>()
                / self.history.len().max(1) as f64,
        }
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Router statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterStats {
    /// Total routing requests
    pub total_requests: usize,
    /// Task type distribution
    pub task_distribution: HashMap<TaskType, usize>,
    /// Model usage distribution
    pub model_distribution: HashMap<String, usize>,
    /// Total estimated cost
    pub total_estimated_cost: f64,
    /// Average confidence score
    pub avg_confidence: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_detection() {
        assert_eq!(TaskType::detect("Write a function to sort an array"), TaskType::Coding);
        assert_eq!(TaskType::detect("Review this code for bugs"), TaskType::CodeReview);
        assert_eq!(TaskType::detect("Calculate 2 + 2"), TaskType::Math);
        assert_eq!(TaskType::detect("Translate this to Spanish"), TaskType::Translation);
        assert_eq!(TaskType::detect("What is the weather?"), TaskType::QuestionAnswering);
        assert_eq!(TaskType::detect("Hi"), TaskType::Quick);
    }

    #[test]
    fn test_routing_decision() {
        let mut router = ModelRouter::new();
        let request = RoutingRequest {
            id: Uuid::new_v4(),
            content: "Write a Python function to parse JSON".to_string(),
            context: vec![],
            estimated_tokens: 500,
            config: RoutingConfig::default(),
            timestamp: Utc::now(),
        };

        let decision = router.route(&request);
        assert_eq!(decision.task_type, TaskType::Coding);
        assert!(decision.confidence > 0.0);
        assert!(!decision.model_id.is_empty());
    }

    #[test]
    fn test_constraints() {
        let mut router = ModelRouter::new();
        let mut config = RoutingConfig::default();
        config.constraints.max_cost = Some(0.001);
        config.constraints.allowed_providers = Some(vec!["google".to_string()]);

        let request = RoutingRequest {
            id: Uuid::new_v4(),
            content: "Quick question".to_string(),
            context: vec![],
            estimated_tokens: 100,
            config,
            timestamp: Utc::now(),
        };

        let decision = router.route(&request);
        assert_eq!(decision.provider, "google");
    }
}
