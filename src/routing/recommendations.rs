// Copyright (c) 2024-2027 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! AI-powered session recommendations
//!
//! Recommends relevant sessions based on context, history, and user behavior.

use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

// ============================================================================
// Session Features
// ============================================================================

/// Session features for recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFeatures {
    /// Session ID
    pub session_id: Uuid,
    /// Session title
    pub title: String,
    /// Provider
    pub provider: String,
    /// Model used
    pub model: Option<String>,
    /// Tags
    pub tags: Vec<String>,
    /// Topics extracted
    pub topics: Vec<String>,
    /// Message count
    pub message_count: usize,
    /// Token count
    pub token_count: usize,
    /// Quality score (0-100)
    pub quality_score: u8,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last accessed
    pub last_accessed: DateTime<Utc>,
    /// Access count
    pub access_count: usize,
    /// Whether bookmarked
    pub bookmarked: bool,
    /// Whether archived
    pub archived: bool,
    /// Content embedding (if available)
    pub embedding: Option<Vec<f32>>,
}

/// User interaction with a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInteraction {
    /// User ID
    pub user_id: Uuid,
    /// Session ID
    pub session_id: Uuid,
    /// Interaction type
    pub interaction_type: InteractionType,
    /// Duration (if view)
    pub duration_seconds: Option<u32>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Interaction type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionType {
    View,
    Search,
    Export,
    Share,
    Bookmark,
    Continue,
    Archive,
}

// ============================================================================
// Recommendation Types
// ============================================================================

/// Recommendation reason
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationReason {
    /// Similar to current context
    SimilarContent,
    /// Related topics
    RelatedTopics,
    /// Same tags
    SameTags,
    /// Frequently accessed
    FrequentlyAccessed,
    /// Recently active
    RecentlyActive,
    /// High quality
    HighQuality,
    /// Related to search query
    SearchRelevant,
    /// Collaborative (others viewed)
    Collaborative,
    /// Continuation suggestion
    ContinueSuggestion,
    /// Trending
    Trending,
}

/// Session recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecommendation {
    /// Session ID
    pub session_id: Uuid,
    /// Session title
    pub title: String,
    /// Provider
    pub provider: String,
    /// Relevance score (0.0 - 1.0)
    pub score: f64,
    /// Primary reason for recommendation
    pub reason: RecommendationReason,
    /// Additional reasons
    pub additional_reasons: Vec<RecommendationReason>,
    /// Explanation text
    pub explanation: String,
    /// Preview snippet
    pub preview: Option<String>,
    /// Tags
    pub tags: Vec<String>,
    /// Message count
    pub message_count: usize,
    /// Created at
    pub created_at: DateTime<Utc>,
}

/// Recommendation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationRequest {
    /// User ID
    pub user_id: Uuid,
    /// Current context (what user is looking at)
    pub context: RecommendationContext,
    /// Number of recommendations to return
    pub limit: usize,
    /// Exclude session IDs
    pub exclude: Vec<Uuid>,
    /// Filter by provider
    pub provider_filter: Option<Vec<String>>,
    /// Filter by tags
    pub tag_filter: Option<Vec<String>>,
    /// Include archived sessions
    pub include_archived: bool,
}

/// Context for generating recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationContext {
    /// Currently viewing a session
    ViewingSession { session_id: Uuid },
    /// Searching for sessions
    Searching { query: String },
    /// On dashboard/home
    Dashboard,
    /// In a workspace
    Workspace { workspace_id: Uuid },
    /// Working with a provider
    Provider { provider: String },
    /// Custom context
    Custom { topics: Vec<String>, tags: Vec<String> },
}

/// Recommendation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationResponse {
    /// Recommendations
    pub recommendations: Vec<SessionRecommendation>,
    /// Context used
    pub context: RecommendationContext,
    /// Generation timestamp
    pub generated_at: DateTime<Utc>,
    /// Model/algorithm used
    pub algorithm: String,
}

// ============================================================================
// User Profile
// ============================================================================

/// User preference profile for recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// User ID
    pub user_id: Uuid,
    /// Preferred providers
    pub preferred_providers: Vec<String>,
    /// Preferred topics
    pub preferred_topics: Vec<String>,
    /// Tag preferences (tag -> weight)
    pub tag_weights: HashMap<String, f64>,
    /// Recent interactions
    pub recent_interactions: Vec<SessionInteraction>,
    /// Session view history
    pub view_history: Vec<Uuid>,
    /// Bookmarked sessions
    pub bookmarked: HashSet<Uuid>,
    /// Last updated
    pub updated_at: DateTime<Utc>,
}

impl UserProfile {
    /// Create a new user profile
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            preferred_providers: vec![],
            preferred_topics: vec![],
            tag_weights: HashMap::new(),
            recent_interactions: vec![],
            view_history: vec![],
            bookmarked: HashSet::new(),
            updated_at: Utc::now(),
        }
    }

    /// Record an interaction
    pub fn record_interaction(&mut self, session_id: Uuid, interaction_type: InteractionType) {
        self.recent_interactions.push(SessionInteraction {
            user_id: self.user_id,
            session_id,
            interaction_type,
            duration_seconds: None,
            timestamp: Utc::now(),
        });

        // Keep only recent interactions (last 30 days)
        let cutoff = Utc::now() - Duration::days(30);
        self.recent_interactions.retain(|i| i.timestamp > cutoff);

        if interaction_type == InteractionType::View {
            self.view_history.push(session_id);
            if self.view_history.len() > 100 {
                self.view_history.remove(0);
            }
        }

        if interaction_type == InteractionType::Bookmark {
            self.bookmarked.insert(session_id);
        }

        self.updated_at = Utc::now();
    }

    /// Get provider preferences based on history
    pub fn infer_provider_preferences(&self, sessions: &[SessionFeatures]) -> HashMap<String, f64> {
        let mut counts: HashMap<String, usize> = HashMap::new();

        for session_id in &self.view_history {
            if let Some(session) = sessions.iter().find(|s| s.session_id == *session_id) {
                *counts.entry(session.provider.clone()).or_insert(0) += 1;
            }
        }

        let total = counts.values().sum::<usize>().max(1) as f64;
        counts.into_iter().map(|(k, v)| (k, v as f64 / total)).collect()
    }

    /// Get topic preferences based on history
    pub fn infer_topic_preferences(&self, sessions: &[SessionFeatures]) -> HashMap<String, f64> {
        let mut counts: HashMap<String, usize> = HashMap::new();

        for session_id in &self.view_history {
            if let Some(session) = sessions.iter().find(|s| s.session_id == *session_id) {
                for topic in &session.topics {
                    *counts.entry(topic.clone()).or_insert(0) += 1;
                }
            }
        }

        let total = counts.values().sum::<usize>().max(1) as f64;
        counts.into_iter().map(|(k, v)| (k, v as f64 / total)).collect()
    }
}

// ============================================================================
// Recommendation Engine
// ============================================================================

/// AI-powered session recommendation engine
pub struct RecommendationEngine {
    /// All session features
    sessions: Vec<SessionFeatures>,
    /// User profiles
    profiles: HashMap<Uuid, UserProfile>,
    /// Global topic frequencies
    topic_frequencies: HashMap<String, usize>,
    /// Global tag frequencies
    tag_frequencies: HashMap<String, usize>,
}

impl RecommendationEngine {
    /// Create a new recommendation engine
    pub fn new() -> Self {
        Self {
            sessions: vec![],
            profiles: HashMap::new(),
            topic_frequencies: HashMap::new(),
            tag_frequencies: HashMap::new(),
        }
    }

    /// Index a session for recommendations
    pub fn index_session(&mut self, session: SessionFeatures) {
        // Update frequencies
        for topic in &session.topics {
            *self.topic_frequencies.entry(topic.clone()).or_insert(0) += 1;
        }
        for tag in &session.tags {
            *self.tag_frequencies.entry(tag.clone()).or_insert(0) += 1;
        }

        // Add or update session
        if let Some(existing) = self.sessions.iter_mut().find(|s| s.session_id == session.session_id) {
            *existing = session;
        } else {
            self.sessions.push(session);
        }
    }

    /// Get or create user profile
    pub fn get_or_create_profile(&mut self, user_id: Uuid) -> &mut UserProfile {
        self.profiles.entry(user_id).or_insert_with(|| UserProfile::new(user_id))
    }

    /// Record a user interaction
    pub fn record_interaction(&mut self, user_id: Uuid, session_id: Uuid, interaction_type: InteractionType) {
        let profile = self.get_or_create_profile(user_id);
        profile.record_interaction(session_id, interaction_type);
    }

    /// Generate recommendations
    pub fn recommend(&self, request: &RecommendationRequest) -> RecommendationResponse {
        let profile = self.profiles.get(&request.user_id);
        
        // Filter sessions
        let candidates: Vec<&SessionFeatures> = self.sessions.iter()
            .filter(|s| !request.exclude.contains(&s.session_id))
            .filter(|s| request.include_archived || !s.archived)
            .filter(|s| {
                request.provider_filter.as_ref()
                    .map(|p| p.contains(&s.provider))
                    .unwrap_or(true)
            })
            .filter(|s| {
                request.tag_filter.as_ref()
                    .map(|t| s.tags.iter().any(|st| t.contains(st)))
                    .unwrap_or(true)
            })
            .collect();

        // Score sessions based on context
        let mut scored: Vec<(SessionRecommendation, f64)> = candidates.iter()
            .map(|s| self.score_session(s, &request.context, profile))
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Take top N
        let recommendations: Vec<SessionRecommendation> = scored.into_iter()
            .take(request.limit)
            .map(|(r, _)| r)
            .collect();

        RecommendationResponse {
            recommendations,
            context: request.context.clone(),
            generated_at: Utc::now(),
            algorithm: "hybrid_scoring_v1".to_string(),
        }
    }

    /// Score a session for recommendation
    fn score_session(
        &self,
        session: &SessionFeatures,
        context: &RecommendationContext,
        profile: Option<&UserProfile>,
    ) -> (SessionRecommendation, f64) {
        let mut score = 0.0;
        let mut reasons: Vec<(RecommendationReason, f64)> = vec![];

        // Context-based scoring
        match context {
            RecommendationContext::ViewingSession { session_id } => {
                if let Some(current) = self.sessions.iter().find(|s| s.session_id == *session_id) {
                    // Topic similarity
                    let topic_sim = self.topic_similarity(&current.topics, &session.topics);
                    if topic_sim > 0.3 {
                        reasons.push((RecommendationReason::RelatedTopics, topic_sim));
                    }

                    // Tag similarity
                    let tag_sim = self.tag_similarity(&current.tags, &session.tags);
                    if tag_sim > 0.3 {
                        reasons.push((RecommendationReason::SameTags, tag_sim));
                    }

                    // Same provider bonus
                    if current.provider == session.provider {
                        reasons.push((RecommendationReason::SimilarContent, 0.2));
                    }
                }
            }
            RecommendationContext::Searching { query } => {
                let query_lower = query.to_lowercase();
                
                // Title match
                if session.title.to_lowercase().contains(&query_lower) {
                    reasons.push((RecommendationReason::SearchRelevant, 0.8));
                }

                // Topic match
                let topic_match = session.topics.iter()
                    .any(|t| t.to_lowercase().contains(&query_lower));
                if topic_match {
                    reasons.push((RecommendationReason::SearchRelevant, 0.6));
                }

                // Tag match
                let tag_match = session.tags.iter()
                    .any(|t| t.to_lowercase().contains(&query_lower));
                if tag_match {
                    reasons.push((RecommendationReason::SameTags, 0.5));
                }
            }
            RecommendationContext::Dashboard => {
                // Recency score
                let age_days = (Utc::now() - session.last_accessed).num_days() as f64;
                let recency = 1.0 / (1.0 + age_days / 7.0);
                reasons.push((RecommendationReason::RecentlyActive, recency));

                // Quality score
                if session.quality_score > 70 {
                    reasons.push((RecommendationReason::HighQuality, session.quality_score as f64 / 100.0));
                }

                // Frequency score
                if session.access_count > 5 {
                    reasons.push((RecommendationReason::FrequentlyAccessed, (session.access_count as f64).ln() / 10.0));
                }
            }
            RecommendationContext::Workspace { .. } => {
                // Recency within workspace
                let age_days = (Utc::now() - session.created_at).num_days() as f64;
                let recency = 1.0 / (1.0 + age_days / 30.0);
                reasons.push((RecommendationReason::RecentlyActive, recency * 0.5));
            }
            RecommendationContext::Provider { provider } => {
                if &session.provider == provider {
                    reasons.push((RecommendationReason::SimilarContent, 0.5));
                }
            }
            RecommendationContext::Custom { topics, tags } => {
                let topic_sim = self.topic_similarity(topics, &session.topics);
                if topic_sim > 0.2 {
                    reasons.push((RecommendationReason::RelatedTopics, topic_sim));
                }

                let tag_sim = self.tag_similarity(tags, &session.tags);
                if tag_sim > 0.2 {
                    reasons.push((RecommendationReason::SameTags, tag_sim));
                }
            }
        }

        // User profile-based scoring
        if let Some(profile) = profile {
            // Viewed similar sessions
            let view_count = profile.view_history.iter()
                .filter(|id| {
                    self.sessions.iter()
                        .find(|s| s.session_id == **id)
                        .map(|viewed| self.topic_similarity(&viewed.topics, &session.topics) > 0.5)
                        .unwrap_or(false)
                })
                .count();
            
            if view_count > 0 {
                reasons.push((RecommendationReason::Collaborative, (view_count as f64).ln() / 5.0));
            }

            // Bookmarked boost
            if profile.bookmarked.contains(&session.session_id) {
                reasons.push((RecommendationReason::FrequentlyAccessed, 0.3));
            }
        }

        // Calculate total score
        for (_, reason_score) in &reasons {
            score += reason_score;
        }

        // Normalize score to 0-1
        score = (score / (reasons.len().max(1) as f64)).min(1.0);

        // Sort reasons by score
        reasons.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let primary_reason = reasons.first().map(|(r, _)| *r).unwrap_or(RecommendationReason::RecentlyActive);
        let additional_reasons: Vec<RecommendationReason> = reasons.iter().skip(1).take(2).map(|(r, _)| *r).collect();

        let recommendation = SessionRecommendation {
            session_id: session.session_id,
            title: session.title.clone(),
            provider: session.provider.clone(),
            score,
            reason: primary_reason,
            additional_reasons,
            explanation: self.generate_explanation(primary_reason, session),
            preview: None,
            tags: session.tags.clone(),
            message_count: session.message_count,
            created_at: session.created_at,
        };

        (recommendation, score)
    }

    /// Calculate topic similarity (Jaccard)
    fn topic_similarity(&self, a: &[String], b: &[String]) -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let set_a: HashSet<&String> = a.iter().collect();
        let set_b: HashSet<&String> = b.iter().collect();

        let intersection = set_a.intersection(&set_b).count();
        let union = set_a.union(&set_b).count();

        intersection as f64 / union as f64
    }

    /// Calculate tag similarity (Jaccard)
    fn tag_similarity(&self, a: &[String], b: &[String]) -> f64 {
        self.topic_similarity(a, b)
    }

    /// Generate explanation text
    fn generate_explanation(&self, reason: RecommendationReason, session: &SessionFeatures) -> String {
        match reason {
            RecommendationReason::SimilarContent => {
                format!("Similar to what you're viewing")
            }
            RecommendationReason::RelatedTopics => {
                let topics = session.topics.iter().take(2).cloned().collect::<Vec<_>>().join(", ");
                format!("Related topics: {}", topics)
            }
            RecommendationReason::SameTags => {
                let tags = session.tags.iter().take(2).cloned().collect::<Vec<_>>().join(", ");
                format!("Tagged with: {}", tags)
            }
            RecommendationReason::FrequentlyAccessed => {
                format!("Frequently accessed session")
            }
            RecommendationReason::RecentlyActive => {
                format!("Recently active")
            }
            RecommendationReason::HighQuality => {
                format!("High quality session ({}% score)", session.quality_score)
            }
            RecommendationReason::SearchRelevant => {
                format!("Matches your search")
            }
            RecommendationReason::Collaborative => {
                format!("Popular with similar users")
            }
            RecommendationReason::ContinueSuggestion => {
                format!("You might want to continue this")
            }
            RecommendationReason::Trending => {
                format!("Trending in your team")
            }
        }
    }

    /// Get trending sessions (most accessed recently)
    pub fn get_trending(&self, limit: usize, days: i64) -> Vec<SessionRecommendation> {
        let cutoff = Utc::now() - Duration::days(days);

        let mut trending: Vec<&SessionFeatures> = self.sessions.iter()
            .filter(|s| s.last_accessed > cutoff)
            .filter(|s| !s.archived)
            .collect();

        trending.sort_by(|a, b| b.access_count.cmp(&a.access_count));

        trending.into_iter()
            .take(limit)
            .map(|s| SessionRecommendation {
                session_id: s.session_id,
                title: s.title.clone(),
                provider: s.provider.clone(),
                score: s.access_count as f64 / 100.0,
                reason: RecommendationReason::Trending,
                additional_reasons: vec![],
                explanation: format!("Viewed {} times recently", s.access_count),
                preview: None,
                tags: s.tags.clone(),
                message_count: s.message_count,
                created_at: s.created_at,
            })
            .collect()
    }
}

impl Default for RecommendationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_session(id: Uuid, title: &str, topics: Vec<&str>, tags: Vec<&str>) -> SessionFeatures {
        SessionFeatures {
            session_id: id,
            title: title.to_string(),
            provider: "copilot".to_string(),
            model: Some("gpt-4".to_string()),
            tags: tags.into_iter().map(String::from).collect(),
            topics: topics.into_iter().map(String::from).collect(),
            message_count: 10,
            token_count: 1000,
            quality_score: 80,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 5,
            bookmarked: false,
            archived: false,
            embedding: None,
        }
    }

    #[test]
    fn test_recommendations() {
        let mut engine = RecommendationEngine::new();

        let session1 = create_test_session(
            Uuid::new_v4(),
            "Rust async programming",
            vec!["rust", "async", "tokio"],
            vec!["programming", "rust"],
        );
        let session2 = create_test_session(
            Uuid::new_v4(),
            "Python web development",
            vec!["python", "web", "flask"],
            vec!["programming", "python"],
        );
        let session3 = create_test_session(
            Uuid::new_v4(),
            "Rust error handling",
            vec!["rust", "errors", "result"],
            vec!["programming", "rust"],
        );

        engine.index_session(session1.clone());
        engine.index_session(session2);
        engine.index_session(session3.clone());

        let request = RecommendationRequest {
            user_id: Uuid::new_v4(),
            context: RecommendationContext::ViewingSession { session_id: session1.session_id },
            limit: 5,
            exclude: vec![session1.session_id],
            provider_filter: None,
            tag_filter: None,
            include_archived: false,
        };

        let response = engine.recommend(&request);
        assert!(!response.recommendations.is_empty());
        
        // Session3 should score higher than session2 due to topic similarity
        let first = &response.recommendations[0];
        assert_eq!(first.session_id, session3.session_id);
    }

    #[test]
    fn test_user_profile() {
        let mut profile = UserProfile::new(Uuid::new_v4());
        let session_id = Uuid::new_v4();

        profile.record_interaction(session_id, InteractionType::View);
        profile.record_interaction(session_id, InteractionType::Bookmark);

        assert_eq!(profile.view_history.len(), 1);
        assert!(profile.bookmarked.contains(&session_id));
    }
}
