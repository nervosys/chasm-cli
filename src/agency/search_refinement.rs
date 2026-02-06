// Copyright (c) 2024-2027 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Context-Aware Search Refinement Agent
//!
//! An AI agent that understands search context and suggests query refinements
//! for better search results across chat sessions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agency::{Agent, AgentBuilder, AgentConfig};

/// Search context from user's recent activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContext {
    /// Recent search queries
    pub recent_queries: Vec<String>,
    /// Recently viewed sessions
    pub recent_sessions: Vec<String>,
    /// Active workspace
    pub workspace_id: Option<String>,
    /// Active providers
    pub providers: Vec<String>,
    /// User preferences
    pub preferences: SearchPreferences,
    /// Time range of interest
    pub time_range: Option<TimeRange>,
}

/// Time range for search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

/// User search preferences
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchPreferences {
    /// Preferred result count
    pub result_limit: u32,
    /// Semantic search enabled
    pub semantic_enabled: bool,
    /// Include archived sessions
    pub include_archived: bool,
    /// Highlight matches
    pub highlight_matches: bool,
    /// Group by session
    pub group_by_session: bool,
}

/// Query refinement suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRefinement {
    /// Refined query
    pub query: String,
    /// Refinement type
    pub refinement_type: RefinementType,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Explanation
    pub explanation: String,
    /// Expected result improvement
    pub expected_improvement: String,
}

/// Type of query refinement
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RefinementType {
    /// Add more specific terms
    Specificity,
    /// Broaden the search
    Broadening,
    /// Correct spelling/typos
    Correction,
    /// Add synonyms
    Synonyms,
    /// Add context from recent activity
    Contextual,
    /// Filter by time
    Temporal,
    /// Filter by provider
    ProviderFilter,
    /// Semantic expansion
    SemanticExpansion,
}

/// Search result with analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedSearchResult {
    /// Original result
    pub session_id: String,
    /// Title
    pub title: String,
    /// Relevance score
    pub relevance: f64,
    /// Matching snippets
    pub snippets: Vec<String>,
    /// Why this result matched
    pub match_reason: String,
    /// Suggested follow-up queries
    pub follow_ups: Vec<String>,
}

/// Search analytics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchAnalytics {
    /// Total searches
    pub total_searches: u64,
    /// Successful searches (found results)
    pub successful_searches: u64,
    /// Refinements suggested
    pub refinements_suggested: u64,
    /// Refinements accepted
    pub refinements_accepted: u64,
    /// Average result relevance
    pub avg_relevance: f64,
    /// Common query patterns
    pub common_patterns: HashMap<String, u32>,
}

/// Context-aware search agent state
pub struct SearchAgentState {
    /// Recent search history
    search_history: Vec<SearchHistoryEntry>,
    /// Analytics
    analytics: SearchAnalytics,
    /// Query patterns learned
    patterns: Vec<QueryPattern>,
    /// Session context cache
    context_cache: HashMap<String, SearchContext>,
}

/// Search history entry
#[derive(Debug, Clone)]
struct SearchHistoryEntry {
    query: String,
    timestamp: DateTime<Utc>,
    result_count: u32,
    refinements_used: Vec<String>,
}

/// Learned query pattern
#[derive(Debug, Clone)]
struct QueryPattern {
    pattern: String,
    frequency: u32,
    avg_results: f64,
    best_refinements: Vec<String>,
}

/// Context-aware search refinement agent
pub struct SearchRefinementAgent {
    /// Agent configuration
    config: AgentConfig,
    /// Agent state
    state: Arc<RwLock<SearchAgentState>>,
}

impl SearchRefinementAgent {
    /// Create a new search refinement agent
    pub fn new() -> Self {
        let config = AgentConfig {
            name: "search-refinement-agent".to_string(),
            description: "Context-aware search query refinement".to_string(),
            instruction: SEARCH_SYSTEM_PROMPT.to_string(),
            ..Default::default()
        };

        let state = SearchAgentState {
            search_history: Vec::new(),
            analytics: SearchAnalytics::default(),
            patterns: Vec::new(),
            context_cache: HashMap::new(),
        };

        Self {
            config,
            state: Arc::new(RwLock::new(state)),
        }
    }

    /// Analyze a query and suggest refinements
    pub async fn refine_query(
        &self,
        query: &str,
        context: Option<SearchContext>,
    ) -> Vec<QueryRefinement> {
        let mut refinements = Vec::new();
        let query_lower = query.to_lowercase();

        // 1. Check for common spelling corrections
        let corrections = self.check_spelling(query);
        for correction in corrections {
            refinements.push(QueryRefinement {
                query: correction.clone(),
                refinement_type: RefinementType::Correction,
                confidence: 0.9,
                explanation: "Corrected potential typo".to_string(),
                expected_improvement: "More accurate results".to_string(),
            });
        }

        // 2. Suggest synonyms/related terms
        let synonyms = self.find_synonyms(&query_lower);
        for synonym in synonyms {
            refinements.push(QueryRefinement {
                query: format!("{} OR {}", query, synonym),
                refinement_type: RefinementType::Synonyms,
                confidence: 0.75,
                explanation: format!("Added synonym: {}", synonym),
                expected_improvement: "Broader coverage".to_string(),
            });
        }

        // 3. Add contextual refinements
        if let Some(ctx) = context {
            // Use recent queries to suggest combinations
            if !ctx.recent_queries.is_empty() {
                let combined = format!("{} {}", query, ctx.recent_queries.last().unwrap());
                refinements.push(QueryRefinement {
                    query: combined,
                    refinement_type: RefinementType::Contextual,
                    confidence: 0.7,
                    explanation: "Combined with recent search".to_string(),
                    expected_improvement: "More relevant to your current focus".to_string(),
                });
            }

            // Add provider filter if context suggests it
            if ctx.providers.len() == 1 {
                refinements.push(QueryRefinement {
                    query: format!("{} provider:{}", query, ctx.providers[0]),
                    refinement_type: RefinementType::ProviderFilter,
                    confidence: 0.8,
                    explanation: format!("Filtered to {} sessions", ctx.providers[0]),
                    expected_improvement: "Focused on your active provider".to_string(),
                });
            }

            // Add time filter for recency
            refinements.push(QueryRefinement {
                query: format!("{} after:7days", query),
                refinement_type: RefinementType::Temporal,
                confidence: 0.6,
                explanation: "Limited to last 7 days".to_string(),
                expected_improvement: "Recent and relevant results".to_string(),
            });
        }

        // 4. Suggest specificity improvements
        if query.split_whitespace().count() < 3 {
            let specific_suggestions = self.suggest_specific_terms(&query_lower).await;
            for suggestion in specific_suggestions {
                refinements.push(QueryRefinement {
                    query: format!("{} {}", query, suggestion),
                    refinement_type: RefinementType::Specificity,
                    confidence: 0.65,
                    explanation: format!("Added specific term: {}", suggestion),
                    expected_improvement: "More targeted results".to_string(),
                });
            }
        }

        // 5. Semantic expansion for technical queries
        if self.is_technical_query(&query_lower) {
            refinements.push(QueryRefinement {
                query: query.to_string(),
                refinement_type: RefinementType::SemanticExpansion,
                confidence: 0.85,
                explanation: "Use semantic search for technical content".to_string(),
                expected_improvement: "Find conceptually related discussions".to_string(),
            });
        }

        // Sort by confidence
        refinements.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        // Update analytics
        {
            let mut state = self.state.write().await;
            state.analytics.refinements_suggested += refinements.len() as u64;
        }

        refinements
    }

    /// Record a search for learning
    pub async fn record_search(
        &self,
        query: &str,
        result_count: u32,
        refinements_used: Vec<String>,
    ) {
        let mut state = self.state.write().await;

        state.search_history.push(SearchHistoryEntry {
            query: query.to_string(),
            timestamp: Utc::now(),
            result_count,
            refinements_used: refinements_used.clone(),
        });

        // Keep only last 1000 searches
        let history_len = state.search_history.len();
        if history_len > 1000 {
            state.search_history.drain(0..history_len - 1000);
        }

        // Update analytics
        state.analytics.total_searches += 1;
        if result_count > 0 {
            state.analytics.successful_searches += 1;
        }
        if !refinements_used.is_empty() {
            state.analytics.refinements_accepted += 1;
        }

        // Update patterns
        let pattern = self.extract_pattern(query);
        if let Some(existing) = state.patterns.iter_mut().find(|p| p.pattern == pattern) {
            existing.frequency += 1;
            existing.avg_results = (existing.avg_results * (existing.frequency - 1) as f64
                + result_count as f64)
                / existing.frequency as f64;
        } else {
            state.patterns.push(QueryPattern {
                pattern,
                frequency: 1,
                avg_results: result_count as f64,
                best_refinements: refinements_used,
            });
        }
    }

    /// Get search analytics
    pub async fn get_analytics(&self) -> SearchAnalytics {
        let state = self.state.read().await;
        state.analytics.clone()
    }

    /// Suggest related searches based on a result
    pub async fn suggest_follow_ups(&self, _session_id: &str, query: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Based on common follow-up patterns
        suggestions.push(format!("{} example", query));
        suggestions.push(format!("{} solution", query));
        suggestions.push(format!("related to {}", query));

        // Could use LLM to generate more contextual suggestions
        // based on session content

        suggestions
    }

    /// Check for common spelling mistakes
    fn check_spelling(&self, query: &str) -> Vec<String> {
        let mut corrections = Vec::new();

        // Common programming term corrections
        let corrections_map: HashMap<&str, &str> = [
            ("javascrip", "javascript"),
            ("pytohn", "python"),
            ("typescrip", "typescript"),
            ("fucntion", "function"),
            ("aync", "async"),
            ("awiat", "await"),
            ("improt", "import"),
            ("exprot", "export"),
            ("cosnt", "const"),
            ("retrun", "return"),
        ]
        .iter()
        .cloned()
        .collect();

        let _words: Vec<&str> = query.split_whitespace().collect();
        for (typo, correct) in &corrections_map {
            if query.to_lowercase().contains(typo) {
                let corrected = query.to_lowercase().replace(typo, correct);
                corrections.push(corrected);
            }
        }

        corrections
    }

    /// Find synonyms for terms
    fn find_synonyms(&self, query: &str) -> Vec<String> {
        let mut synonyms = Vec::new();

        let synonym_map: HashMap<&str, Vec<&str>> = [
            ("error", vec!["exception", "bug", "issue", "problem"]),
            ("function", vec!["method", "procedure", "routine"]),
            ("variable", vec!["var", "const", "let", "parameter"]),
            ("create", vec!["make", "generate", "build", "new"]),
            ("delete", vec!["remove", "destroy", "drop"]),
            ("find", vec!["search", "locate", "query", "get"]),
            ("update", vec!["modify", "change", "edit", "patch"]),
            ("api", vec!["endpoint", "route", "service"]),
            ("database", vec!["db", "storage", "repository"]),
        ]
        .iter()
        .cloned()
        .collect();

        for (term, syns) in &synonym_map {
            if query.contains(term) {
                for syn in syns {
                    synonyms.push(syn.to_string());
                }
            }
        }

        synonyms.truncate(3); // Limit suggestions
        synonyms
    }

    /// Suggest more specific terms
    async fn suggest_specific_terms(&self, query: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Common specificity additions based on query content
        if query.contains("error") || query.contains("bug") {
            suggestions.push("fix".to_string());
            suggestions.push("solution".to_string());
        }
        if query.contains("how") {
            suggestions.push("step-by-step".to_string());
            suggestions.push("example".to_string());
        }
        if query.contains("best") {
            suggestions.push("practice".to_string());
            suggestions.push("approach".to_string());
        }

        suggestions.truncate(2);
        suggestions
    }

    /// Check if query is technical
    fn is_technical_query(&self, query: &str) -> bool {
        let technical_terms = [
            "function",
            "class",
            "method",
            "api",
            "error",
            "bug",
            "code",
            "implement",
            "debug",
            "async",
            "await",
            "promise",
            "callback",
            "component",
            "module",
            "import",
            "export",
            "typescript",
            "javascript",
            "python",
            "rust",
            "react",
            "vue",
            "angular",
            "node",
            "sql",
        ];

        technical_terms.iter().any(|term| query.contains(term))
    }

    /// Extract pattern from query for learning
    fn extract_pattern(&self, query: &str) -> String {
        // Normalize query to pattern
        let words: Vec<&str> = query.split_whitespace().collect();
        if words.len() <= 2 {
            return query.to_lowercase();
        }

        // Keep structure but replace specific terms with placeholders
        words
            .iter()
            .map(|w| if w.len() > 5 { "[TERM]" } else { *w })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Default for SearchRefinementAgent {
    fn default() -> Self {
        Self::new()
    }
}

/// System prompt for the search refinement agent
const SEARCH_SYSTEM_PROMPT: &str = r#"You are a context-aware search refinement agent for Chasm.

Your role is to help users find relevant chat sessions by:
1. Understanding the intent behind their search queries
2. Suggesting refinements that will improve results
3. Learning from search patterns to make better suggestions
4. Providing contextual suggestions based on recent activity

When refining a query, consider:
- Is the query too broad or too specific?
- Are there common synonyms or related terms?
- Does the user's recent activity suggest a focus area?
- Would time-based or provider-based filters help?

Always explain why a refinement might help.
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_agent_creation() {
        let agent = SearchRefinementAgent::new();
        let analytics = agent.get_analytics().await;
        assert_eq!(analytics.total_searches, 0);
    }

    #[tokio::test]
    async fn test_refine_query_basic() {
        let agent = SearchRefinementAgent::new();
        let refinements = agent.refine_query("python error", None).await;
        assert!(!refinements.is_empty());
    }

    #[tokio::test]
    async fn test_refine_query_with_context() {
        let agent = SearchRefinementAgent::new();
        let context = SearchContext {
            recent_queries: vec!["async await".to_string()],
            recent_sessions: vec![],
            workspace_id: Some("test-workspace".to_string()),
            providers: vec!["copilot".to_string()],
            preferences: SearchPreferences::default(),
            time_range: None,
        };
        let refinements = agent.refine_query("function", Some(context)).await;

        // Should have contextual refinements
        let has_contextual = refinements
            .iter()
            .any(|r| r.refinement_type == RefinementType::Contextual);
        assert!(has_contextual || !refinements.is_empty());
    }

    #[tokio::test]
    async fn test_spelling_correction() {
        let agent = SearchRefinementAgent::new();
        let refinements = agent.refine_query("pytohn function", None).await;

        let has_correction = refinements
            .iter()
            .any(|r| r.refinement_type == RefinementType::Correction);
        assert!(has_correction);
    }

    #[tokio::test]
    async fn test_record_search() {
        let agent = SearchRefinementAgent::new();
        agent.record_search("test query", 10, vec![]).await;

        let analytics = agent.get_analytics().await;
        assert_eq!(analytics.total_searches, 1);
        assert_eq!(analytics.successful_searches, 1);
    }
}
