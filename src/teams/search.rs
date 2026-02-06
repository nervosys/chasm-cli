// Copyright (c) 2024-2027 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Team-wide session search module
//!
//! Provides aggregated search across all team members' sessions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::rbac::{AccessControl, Action, Permission, Resource};
use super::workspace::{MemberId, SessionVisibility, TeamId};

// ============================================================================
// Search Types
// ============================================================================

/// Team search query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSearchQuery {
    /// Search text
    pub text: String,
    /// Team ID to search within
    pub team_id: TeamId,
    /// Filter by providers
    pub providers: Option<Vec<String>>,
    /// Filter by members
    pub members: Option<Vec<MemberId>>,
    /// Filter by date range start
    pub from_date: Option<DateTime<Utc>>,
    /// Filter by date range end
    pub to_date: Option<DateTime<Utc>>,
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    /// Include archived sessions
    pub include_archived: bool,
    /// Search in message content
    pub search_content: bool,
    /// Maximum results
    pub limit: usize,
    /// Result offset
    pub offset: usize,
    /// Sort field
    pub sort_by: SearchSortField,
    /// Sort direction
    pub sort_order: SortOrder,
}

impl Default for TeamSearchQuery {
    fn default() -> Self {
        Self {
            text: String::new(),
            team_id: Uuid::nil(),
            providers: None,
            members: None,
            from_date: None,
            to_date: None,
            tags: None,
            include_archived: false,
            search_content: true,
            limit: 20,
            offset: 0,
            sort_by: SearchSortField::Relevance,
            sort_order: SortOrder::Descending,
        }
    }
}

/// Sort field for search results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchSortField {
    Relevance,
    CreatedAt,
    UpdatedAt,
    MessageCount,
    Title,
}

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Ascending,
    Descending,
}

/// Team search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSearchResult {
    /// Matching sessions
    pub sessions: Vec<TeamSessionResult>,
    /// Total count (for pagination)
    pub total_count: usize,
    /// Search took (milliseconds)
    pub took_ms: u64,
    /// Facets for filtering
    pub facets: SearchFacets,
    /// Search suggestions
    pub suggestions: Vec<String>,
}

/// Individual session result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSessionResult {
    /// Session ID
    pub session_id: String,
    /// Session title
    pub title: String,
    /// Owner member ID
    pub owner_id: MemberId,
    /// Owner display name
    pub owner_name: String,
    /// Provider
    pub provider: String,
    /// Model used
    pub model: Option<String>,
    /// Message count
    pub message_count: u32,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated
    pub updated_at: DateTime<Utc>,
    /// Tags
    pub tags: Vec<String>,
    /// Is archived
    pub archived: bool,
    /// Relevance score
    pub score: f32,
    /// Matching highlights
    pub highlights: Vec<SearchHighlight>,
    /// Session visibility
    pub visibility: SessionVisibility,
}

/// Search highlight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHighlight {
    /// Field that matched
    pub field: String,
    /// Highlighted snippet with match markers
    pub snippet: String,
    /// Message index (if from message content)
    pub message_index: Option<usize>,
}

/// Facets for search filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFacets {
    /// Providers with counts
    pub providers: HashMap<String, usize>,
    /// Members with counts
    pub members: HashMap<String, MemberFacet>,
    /// Tags with counts
    pub tags: HashMap<String, usize>,
    /// Date histogram
    pub date_histogram: Vec<DateBucket>,
}

/// Member facet information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberFacet {
    pub member_id: MemberId,
    pub display_name: String,
    pub count: usize,
}

/// Date bucket for histogram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateBucket {
    pub date: DateTime<Utc>,
    pub count: usize,
}

// ============================================================================
// Search Engine
// ============================================================================

/// Team search engine
pub struct TeamSearchEngine {
    /// Access control for permission checks
    access_control: AccessControl,
}

impl TeamSearchEngine {
    /// Create a new team search engine
    pub fn new(access_control: AccessControl) -> Self {
        Self { access_control }
    }

    /// Search sessions across a team
    pub async fn search(
        &self,
        query: TeamSearchQuery,
        searcher_id: MemberId,
        sessions: &[SessionData],
    ) -> TeamSearchResult {
        let start = std::time::Instant::now();

        // Filter sessions by access permissions
        let accessible_sessions: Vec<&SessionData> = sessions
            .iter()
            .filter(|s| self.can_view_session(searcher_id, query.team_id, s))
            .collect();

        // Apply search filters
        let mut matched_sessions: Vec<TeamSessionResult> = accessible_sessions
            .iter()
            .filter_map(|s| self.match_session(s, &query))
            .collect();

        // Calculate total before pagination
        let total_count = matched_sessions.len();

        // Sort results
        self.sort_results(&mut matched_sessions, query.sort_by, query.sort_order);

        // Apply pagination
        let paginated: Vec<TeamSessionResult> = matched_sessions
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .collect();

        // Calculate facets
        let facets = self.calculate_facets(&accessible_sessions, &query);

        // Generate suggestions
        let suggestions = self.generate_suggestions(&query.text);

        TeamSearchResult {
            sessions: paginated,
            total_count,
            took_ms: start.elapsed().as_millis() as u64,
            facets,
            suggestions,
        }
    }

    /// Check if a user can view a session
    fn can_view_session(&self, user_id: MemberId, team_id: TeamId, session: &SessionData) -> bool {
        // Owner can always view their own sessions
        if session.owner_id == user_id {
            return true;
        }

        // Check session visibility
        match session.visibility {
            SessionVisibility::Private => false,
            SessionVisibility::TeamOnly | SessionVisibility::Public => {
                // Check RBAC
                let resource = Resource::Session {
                    team_id,
                    session_id: session.session_id.clone(),
                    owner_id: session.owner_id,
                };
                matches!(
                    self.access_control.check(user_id, &resource, Action::View),
                    super::rbac::AccessDecision::Allow
                )
            }
        }
    }

    /// Match a session against the query
    fn match_session(&self, session: &SessionData, query: &TeamSearchQuery) -> Option<TeamSessionResult> {
        // Filter by provider
        if let Some(providers) = &query.providers {
            if !providers.contains(&session.provider) {
                return None;
            }
        }

        // Filter by member
        if let Some(members) = &query.members {
            if !members.contains(&session.owner_id) {
                return None;
            }
        }

        // Filter by date range
        if let Some(from) = query.from_date {
            if session.created_at < from {
                return None;
            }
        }
        if let Some(to) = query.to_date {
            if session.created_at > to {
                return None;
            }
        }

        // Filter by tags
        if let Some(tags) = &query.tags {
            if !tags.iter().any(|t| session.tags.contains(t)) {
                return None;
            }
        }

        // Filter archived
        if !query.include_archived && session.archived {
            return None;
        }

        // Text search
        let (score, highlights) = self.calculate_relevance(session, &query.text, query.search_content);

        // Require minimum score for text queries
        if !query.text.is_empty() && score < 0.1 {
            return None;
        }

        Some(TeamSessionResult {
            session_id: session.session_id.clone(),
            title: session.title.clone(),
            owner_id: session.owner_id,
            owner_name: session.owner_name.clone(),
            provider: session.provider.clone(),
            model: session.model.clone(),
            message_count: session.message_count,
            created_at: session.created_at,
            updated_at: session.updated_at,
            tags: session.tags.clone(),
            archived: session.archived,
            score,
            highlights,
            visibility: session.visibility,
        })
    }

    /// Calculate relevance score and highlights
    fn calculate_relevance(
        &self,
        session: &SessionData,
        query_text: &str,
        search_content: bool,
    ) -> (f32, Vec<SearchHighlight>) {
        if query_text.is_empty() {
            return (1.0, vec![]);
        }

        let query_lower = query_text.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();
        let mut score = 0.0;
        let mut highlights = vec![];

        // Title matching (high weight)
        let title_lower = session.title.to_lowercase();
        for term in &query_terms {
            if title_lower.contains(term) {
                score += 3.0;
                highlights.push(SearchHighlight {
                    field: "title".to_string(),
                    snippet: self.highlight_text(&session.title, term),
                    message_index: None,
                });
            }
        }

        // Tag matching (medium weight)
        for tag in &session.tags {
            let tag_lower = tag.to_lowercase();
            for term in &query_terms {
                if tag_lower.contains(term) {
                    score += 2.0;
                    highlights.push(SearchHighlight {
                        field: "tags".to_string(),
                        snippet: tag.clone(),
                        message_index: None,
                    });
                }
            }
        }

        // Provider matching
        if query_terms.iter().any(|t| session.provider.to_lowercase().contains(t)) {
            score += 1.0;
        }

        // Content matching
        if search_content {
            for (idx, message) in session.messages.iter().enumerate() {
                let content_lower = message.content.to_lowercase();
                for term in &query_terms {
                    if content_lower.contains(term) {
                        score += 0.5;
                        // Only include first few content highlights
                        if highlights.len() < 5 {
                            highlights.push(SearchHighlight {
                                field: "content".to_string(),
                                snippet: self.extract_snippet(&message.content, term),
                                message_index: Some(idx),
                            });
                        }
                    }
                }
            }
        }

        // Normalize score
        let max_possible = (query_terms.len() as f32) * 5.0;
        let normalized_score = (score / max_possible).min(1.0);

        (normalized_score, highlights)
    }

    /// Highlight matching text
    fn highlight_text(&self, text: &str, term: &str) -> String {
        let lower = text.to_lowercase();
        if let Some(pos) = lower.find(term) {
            let before = &text[..pos];
            let matched = &text[pos..pos + term.len()];
            let after = &text[pos + term.len()..];
            format!("{}**{}**{}", before, matched, after)
        } else {
            text.to_string()
        }
    }

    /// Extract snippet around matching term
    fn extract_snippet(&self, content: &str, term: &str) -> String {
        let lower = content.to_lowercase();
        if let Some(pos) = lower.find(term) {
            let start = pos.saturating_sub(50);
            let end = (pos + term.len() + 50).min(content.len());

            let mut snippet = String::new();
            if start > 0 {
                snippet.push_str("...");
            }
            snippet.push_str(&content[start..end]);
            if end < content.len() {
                snippet.push_str("...");
            }
            snippet
        } else {
            content.chars().take(100).collect()
        }
    }

    /// Sort search results
    fn sort_results(
        &self,
        results: &mut [TeamSessionResult],
        sort_by: SearchSortField,
        order: SortOrder,
    ) {
        results.sort_by(|a, b| {
            let cmp = match sort_by {
                SearchSortField::Relevance => a.score.partial_cmp(&b.score).unwrap(),
                SearchSortField::CreatedAt => a.created_at.cmp(&b.created_at),
                SearchSortField::UpdatedAt => a.updated_at.cmp(&b.updated_at),
                SearchSortField::MessageCount => a.message_count.cmp(&b.message_count),
                SearchSortField::Title => a.title.cmp(&b.title),
            };

            match order {
                SortOrder::Ascending => cmp,
                SortOrder::Descending => cmp.reverse(),
            }
        });
    }

    /// Calculate facets from search results
    fn calculate_facets(&self, sessions: &[&SessionData], _query: &TeamSearchQuery) -> SearchFacets {
        let mut providers: HashMap<String, usize> = HashMap::new();
        let mut members: HashMap<String, MemberFacet> = HashMap::new();
        let mut tags: HashMap<String, usize> = HashMap::new();
        let mut date_counts: HashMap<String, usize> = HashMap::new();

        for session in sessions {
            // Provider facet
            *providers.entry(session.provider.clone()).or_insert(0) += 1;

            // Member facet
            let member_key = session.owner_id.to_string();
            members
                .entry(member_key.clone())
                .and_modify(|f| f.count += 1)
                .or_insert(MemberFacet {
                    member_id: session.owner_id,
                    display_name: session.owner_name.clone(),
                    count: 1,
                });

            // Tags facet
            for tag in &session.tags {
                *tags.entry(tag.clone()).or_insert(0) += 1;
            }

            // Date histogram (by month)
            let month_key = session.created_at.format("%Y-%m").to_string();
            *date_counts.entry(month_key).or_insert(0) += 1;
        }

        // Convert date counts to histogram
        let mut date_histogram: Vec<DateBucket> = date_counts
            .into_iter()
            .filter_map(|(date_str, count)| {
                let date = chrono::NaiveDate::parse_from_str(&format!("{}-01", date_str), "%Y-%m-%d")
                    .ok()?;
                Some(DateBucket {
                    date: DateTime::from_naive_utc_and_offset(
                        date.and_hms_opt(0, 0, 0)?,
                        Utc,
                    ),
                    count,
                })
            })
            .collect();
        date_histogram.sort_by(|a, b| a.date.cmp(&b.date));

        SearchFacets {
            providers,
            members,
            tags,
            date_histogram,
        }
    }

    /// Generate search suggestions
    fn generate_suggestions(&self, query: &str) -> Vec<String> {
        // Simple suggestions based on common patterns
        let mut suggestions = vec![];

        if !query.is_empty() {
            // Add provider filter suggestion
            suggestions.push(format!("{} provider:copilot", query));
            suggestions.push(format!("{} provider:cursor", query));

            // Add date filter suggestion
            suggestions.push(format!("{} from:last-week", query));
        }

        suggestions
    }
}

/// Session data for search indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub session_id: String,
    pub title: String,
    pub owner_id: MemberId,
    pub owner_name: String,
    pub provider: String,
    pub model: Option<String>,
    pub message_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub archived: bool,
    pub visibility: SessionVisibility,
    pub messages: Vec<MessageData>,
}

/// Message data for content search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageData {
    pub role: String,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_session(
        id: &str,
        title: &str,
        owner_id: MemberId,
        provider: &str,
    ) -> SessionData {
        SessionData {
            session_id: id.to_string(),
            title: title.to_string(),
            owner_id,
            owner_name: "Test User".to_string(),
            provider: provider.to_string(),
            model: Some("gpt-4".to_string()),
            message_count: 10,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec!["rust".to_string()],
            archived: false,
            visibility: SessionVisibility::TeamOnly,
            messages: vec![MessageData {
                role: "user".to_string(),
                content: "Hello, how do I write a Rust function?".to_string(),
            }],
        }
    }

    #[tokio::test]
    async fn test_team_search() {
        let access_control = AccessControl::new();
        let engine = TeamSearchEngine::new(access_control);

        let owner_id = Uuid::new_v4();
        let sessions = vec![
            create_test_session("1", "Rust Programming Help", owner_id, "copilot"),
            create_test_session("2", "Python Tutorial", owner_id, "cursor"),
        ];

        let query = TeamSearchQuery {
            text: "rust".to_string(),
            team_id: Uuid::new_v4(),
            limit: 10,
            ..Default::default()
        };

        let result = engine.search(query, owner_id, &sessions).await;
        assert!(!result.sessions.is_empty());
    }
}
