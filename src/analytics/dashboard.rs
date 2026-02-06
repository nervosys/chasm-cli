// Copyright (c) 2024-2027 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Analytics dashboard module
//!
//! Provides team usage analytics and insights.

use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// Dashboard Types
// ============================================================================

/// Team analytics dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamDashboard {
    /// Team ID
    pub team_id: Uuid,
    /// Dashboard generated at
    pub generated_at: DateTime<Utc>,
    /// Time period
    pub period: AnalyticsPeriod,
    /// Overview metrics
    pub overview: OverviewMetrics,
    /// Usage trends
    pub trends: UsageTrends,
    /// Member statistics
    pub member_stats: Vec<MemberStats>,
    /// Provider breakdown
    pub provider_breakdown: Vec<ProviderStats>,
    /// Session analytics
    pub session_analytics: SessionAnalytics,
    /// Collaboration metrics
    pub collaboration: CollaborationMetrics,
}

/// Analytics time period
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsPeriod {
    Today,
    Yesterday,
    Last7Days,
    Last30Days,
    Last90Days,
    ThisMonth,
    LastMonth,
    ThisYear,
    Custom,
}

impl AnalyticsPeriod {
    /// Get start date for period
    pub fn start_date(&self) -> DateTime<Utc> {
        let now = Utc::now();
        match self {
            Self::Today => now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc(),
            Self::Yesterday => (now - Duration::days(1)).date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc(),
            Self::Last7Days => now - Duration::days(7),
            Self::Last30Days => now - Duration::days(30),
            Self::Last90Days => now - Duration::days(90),
            Self::ThisMonth => {
                let naive = now.date_naive();
                chrono::NaiveDate::from_ymd_opt(naive.year(), naive.month(), 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
            }
            Self::LastMonth => {
                let naive = now.date_naive();
                let (year, month) = if naive.month() == 1 {
                    (naive.year() - 1, 12)
                } else {
                    (naive.year(), naive.month() - 1)
                };
                chrono::NaiveDate::from_ymd_opt(year, month, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
            }
            Self::ThisYear => {
                let naive = now.date_naive();
                chrono::NaiveDate::from_ymd_opt(naive.year(), 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
            }
            Self::Custom => now - Duration::days(30), // Default for custom
        }
    }
}

/// Overview metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewMetrics {
    /// Total sessions in period
    pub total_sessions: u64,
    /// Sessions change from previous period
    pub sessions_change: f64,
    /// Total messages in period
    pub total_messages: u64,
    /// Messages change from previous period
    pub messages_change: f64,
    /// Total tokens used
    pub total_tokens: u64,
    /// Tokens change from previous period
    pub tokens_change: f64,
    /// Active members in period
    pub active_members: u32,
    /// Active members change
    pub active_members_change: f64,
    /// Average sessions per member
    pub avg_sessions_per_member: f64,
    /// Average messages per session
    pub avg_messages_per_session: f64,
}

/// Usage trends over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageTrends {
    /// Daily session counts
    pub daily_sessions: Vec<TimeSeriesPoint>,
    /// Daily message counts
    pub daily_messages: Vec<TimeSeriesPoint>,
    /// Daily token usage
    pub daily_tokens: Vec<TimeSeriesPoint>,
    /// Hourly activity distribution (0-23)
    pub hourly_distribution: Vec<u64>,
    /// Day of week distribution (0=Sun, 6=Sat)
    pub weekday_distribution: Vec<u64>,
}

/// Time series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Value
    pub value: f64,
}

/// Individual member statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberStats {
    /// Member ID
    pub member_id: Uuid,
    /// Display name
    pub display_name: String,
    /// Total sessions
    pub sessions: u64,
    /// Total messages
    pub messages: u64,
    /// Total tokens
    pub tokens: u64,
    /// Favorite provider
    pub favorite_provider: Option<String>,
    /// Average session length (messages)
    pub avg_session_length: f64,
    /// Last active
    pub last_active: Option<DateTime<Utc>>,
    /// Activity score (0-100)
    pub activity_score: u8,
}

/// Provider statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStats {
    /// Provider name
    pub provider: String,
    /// Session count
    pub sessions: u64,
    /// Session percentage
    pub session_percentage: f64,
    /// Message count
    pub messages: u64,
    /// Token count
    pub tokens: u64,
    /// Most used models
    pub top_models: Vec<ModelUsage>,
}

/// Model usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    /// Model name
    pub model: String,
    /// Usage count
    pub count: u64,
    /// Percentage
    pub percentage: f64,
}

/// Session-level analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAnalytics {
    /// Average session duration (minutes)
    pub avg_duration_minutes: f64,
    /// Average messages per session
    pub avg_messages: f64,
    /// Average tokens per session
    pub avg_tokens: f64,
    /// Session length distribution
    pub length_distribution: SessionLengthDistribution,
    /// Top tags
    pub top_tags: Vec<TagUsage>,
    /// Quality score distribution
    pub quality_distribution: QualityDistribution,
}

/// Session length distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLengthDistribution {
    /// 1-5 messages
    pub short: u64,
    /// 6-20 messages
    pub medium: u64,
    /// 21-50 messages
    pub long: u64,
    /// 51+ messages
    pub very_long: u64,
}

/// Tag usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagUsage {
    /// Tag name
    pub tag: String,
    /// Usage count
    pub count: u64,
    /// Percentage
    pub percentage: f64,
}

/// Session quality distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityDistribution {
    /// Excellent (80-100)
    pub excellent: u64,
    /// Good (60-79)
    pub good: u64,
    /// Average (40-59)
    pub average: u64,
    /// Below average (0-39)
    pub below_average: u64,
}

/// Collaboration metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationMetrics {
    /// Total shared sessions
    pub shared_sessions: u64,
    /// Total comments
    pub total_comments: u64,
    /// Active collaborations (sessions with multiple contributors)
    pub active_collaborations: u64,
    /// Most collaborative members
    pub top_collaborators: Vec<CollaboratorStats>,
}

/// Collaborator statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaboratorStats {
    /// Member ID
    pub member_id: Uuid,
    /// Display name
    pub display_name: String,
    /// Sessions shared
    pub sessions_shared: u64,
    /// Comments made
    pub comments_made: u64,
    /// Collaboration score
    pub collaboration_score: u8,
}

// ============================================================================
// Analytics Engine
// ============================================================================

/// Analytics engine for generating dashboards
pub struct AnalyticsEngine {
    /// Cached dashboards
    cache: HashMap<(Uuid, AnalyticsPeriod), CachedDashboard>,
    /// Cache TTL in seconds
    cache_ttl: u64,
}

struct CachedDashboard {
    dashboard: TeamDashboard,
    cached_at: DateTime<Utc>,
}

impl AnalyticsEngine {
    /// Create a new analytics engine
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            cache_ttl: 300, // 5 minutes
        }
    }

    /// Generate dashboard for a team
    pub fn generate_dashboard(
        &mut self,
        team_id: Uuid,
        period: AnalyticsPeriod,
        session_data: &[SessionAnalyticsData],
        member_data: &[MemberAnalyticsData],
    ) -> TeamDashboard {
        // Check cache
        let cache_key = (team_id, period);
        if let Some(cached) = self.cache.get(&cache_key) {
            let age = (Utc::now() - cached.cached_at).num_seconds() as u64;
            if age < self.cache_ttl {
                return cached.dashboard.clone();
            }
        }

        let start_date = period.start_date();
        let now = Utc::now();

        // Filter data by period
        let period_sessions: Vec<&SessionAnalyticsData> = session_data
            .iter()
            .filter(|s| s.created_at >= start_date && s.created_at <= now)
            .collect();

        // Calculate overview metrics
        let overview = self.calculate_overview(&period_sessions, member_data, period);

        // Calculate trends
        let trends = self.calculate_trends(&period_sessions, start_date, now);

        // Calculate member stats
        let member_stats = self.calculate_member_stats(&period_sessions, member_data);

        // Calculate provider breakdown
        let provider_breakdown = self.calculate_provider_breakdown(&period_sessions);

        // Calculate session analytics
        let session_analytics = self.calculate_session_analytics(&period_sessions);

        // Calculate collaboration metrics
        let collaboration = self.calculate_collaboration_metrics(&period_sessions, member_data);

        let dashboard = TeamDashboard {
            team_id,
            generated_at: Utc::now(),
            period,
            overview,
            trends,
            member_stats,
            provider_breakdown,
            session_analytics,
            collaboration,
        };

        // Cache dashboard
        self.cache.insert(
            cache_key,
            CachedDashboard {
                dashboard: dashboard.clone(),
                cached_at: Utc::now(),
            },
        );

        dashboard
    }

    fn calculate_overview(
        &self,
        sessions: &[&SessionAnalyticsData],
        _members: &[MemberAnalyticsData],
        _period: AnalyticsPeriod,
    ) -> OverviewMetrics {
        let total_sessions = sessions.len() as u64;
        let total_messages: u64 = sessions.iter().map(|s| s.message_count as u64).sum();
        let total_tokens: u64 = sessions.iter().map(|s| s.token_count as u64).sum();

        let active_member_ids: std::collections::HashSet<_> =
            sessions.iter().map(|s| s.owner_id).collect();
        let active_members = active_member_ids.len() as u32;

        let avg_sessions_per_member = if active_members > 0 {
            total_sessions as f64 / active_members as f64
        } else {
            0.0
        };

        let avg_messages_per_session = if total_sessions > 0 {
            total_messages as f64 / total_sessions as f64
        } else {
            0.0
        };

        // Calculate changes (simplified - would need previous period data)
        let sessions_change = 0.0;
        let messages_change = 0.0;
        let tokens_change = 0.0;
        let active_members_change = 0.0;

        OverviewMetrics {
            total_sessions,
            sessions_change,
            total_messages,
            messages_change,
            total_tokens,
            tokens_change,
            active_members,
            active_members_change,
            avg_sessions_per_member,
            avg_messages_per_session,
        }
    }

    fn calculate_trends(
        &self,
        sessions: &[&SessionAnalyticsData],
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> UsageTrends {
        let mut daily_sessions: HashMap<String, u64> = HashMap::new();
        let mut daily_messages: HashMap<String, u64> = HashMap::new();
        let mut daily_tokens: HashMap<String, u64> = HashMap::new();
        let mut hourly: Vec<u64> = vec![0; 24];
        let mut weekday: Vec<u64> = vec![0; 7];

        for session in sessions {
            let date_key = session.created_at.format("%Y-%m-%d").to_string();
            *daily_sessions.entry(date_key.clone()).or_insert(0) += 1;
            *daily_messages.entry(date_key.clone()).or_insert(0) += session.message_count as u64;
            *daily_tokens.entry(date_key).or_insert(0) += session.token_count as u64;

            let hour = session.created_at.hour() as usize;
            hourly[hour] += 1;

            let weekday_idx = session.created_at.weekday().num_days_from_sunday() as usize;
            weekday[weekday_idx] += 1;
        }

        // Convert to time series
        let mut current = start;
        let mut sessions_ts = vec![];
        let mut messages_ts = vec![];
        let mut tokens_ts = vec![];

        while current <= end {
            let date_key = current.format("%Y-%m-%d").to_string();
            sessions_ts.push(TimeSeriesPoint {
                timestamp: current,
                value: *daily_sessions.get(&date_key).unwrap_or(&0) as f64,
            });
            messages_ts.push(TimeSeriesPoint {
                timestamp: current,
                value: *daily_messages.get(&date_key).unwrap_or(&0) as f64,
            });
            tokens_ts.push(TimeSeriesPoint {
                timestamp: current,
                value: *daily_tokens.get(&date_key).unwrap_or(&0) as f64,
            });
            current = current + Duration::days(1);
        }

        UsageTrends {
            daily_sessions: sessions_ts,
            daily_messages: messages_ts,
            daily_tokens: tokens_ts,
            hourly_distribution: hourly,
            weekday_distribution: weekday,
        }
    }

    fn calculate_member_stats(
        &self,
        sessions: &[&SessionAnalyticsData],
        members: &[MemberAnalyticsData],
    ) -> Vec<MemberStats> {
        let mut stats_map: HashMap<Uuid, MemberStats> = HashMap::new();

        for session in sessions {
            let entry = stats_map.entry(session.owner_id).or_insert_with(|| {
                let member = members.iter().find(|m| m.member_id == session.owner_id);
                MemberStats {
                    member_id: session.owner_id,
                    display_name: member.map(|m| m.display_name.clone()).unwrap_or_default(),
                    sessions: 0,
                    messages: 0,
                    tokens: 0,
                    favorite_provider: None,
                    avg_session_length: 0.0,
                    last_active: None,
                    activity_score: 0,
                }
            });

            entry.sessions += 1;
            entry.messages += session.message_count as u64;
            entry.tokens += session.token_count as u64;

            if entry.last_active.map(|la| session.created_at > la).unwrap_or(true) {
                entry.last_active = Some(session.created_at);
            }
        }

        // Calculate averages and scores
        for stats in stats_map.values_mut() {
            if stats.sessions > 0 {
                stats.avg_session_length = stats.messages as f64 / stats.sessions as f64;
            }
            // Simple activity score based on sessions
            stats.activity_score = (stats.sessions.min(100)) as u8;
        }

        let mut result: Vec<_> = stats_map.into_values().collect();
        result.sort_by(|a, b| b.sessions.cmp(&a.sessions));
        result
    }

    fn calculate_provider_breakdown(&self, sessions: &[&SessionAnalyticsData]) -> Vec<ProviderStats> {
        let mut provider_map: HashMap<String, ProviderStats> = HashMap::new();
        let total = sessions.len() as f64;

        for session in sessions {
            let entry = provider_map
                .entry(session.provider.clone())
                .or_insert_with(|| ProviderStats {
                    provider: session.provider.clone(),
                    sessions: 0,
                    session_percentage: 0.0,
                    messages: 0,
                    tokens: 0,
                    top_models: vec![],
                });

            entry.sessions += 1;
            entry.messages += session.message_count as u64;
            entry.tokens += session.token_count as u64;
        }

        // Calculate percentages
        for stats in provider_map.values_mut() {
            stats.session_percentage = if total > 0.0 {
                (stats.sessions as f64 / total) * 100.0
            } else {
                0.0
            };
        }

        let mut result: Vec<_> = provider_map.into_values().collect();
        result.sort_by(|a, b| b.sessions.cmp(&a.sessions));
        result
    }

    fn calculate_session_analytics(&self, sessions: &[&SessionAnalyticsData]) -> SessionAnalytics {
        let total = sessions.len();

        let mut total_messages = 0u64;
        let mut total_tokens = 0u64;
        let mut length_dist = SessionLengthDistribution {
            short: 0,
            medium: 0,
            long: 0,
            very_long: 0,
        };
        let mut tag_counts: HashMap<String, u64> = HashMap::new();
        let mut quality_dist = QualityDistribution {
            excellent: 0,
            good: 0,
            average: 0,
            below_average: 0,
        };

        for session in sessions {
            total_messages += session.message_count as u64;
            total_tokens += session.token_count as u64;

            // Length distribution
            match session.message_count {
                0..=5 => length_dist.short += 1,
                6..=20 => length_dist.medium += 1,
                21..=50 => length_dist.long += 1,
                _ => length_dist.very_long += 1,
            }

            // Tags
            for tag in &session.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }

            // Quality (simplified)
            match session.quality_score {
                80..=100 => quality_dist.excellent += 1,
                60..=79 => quality_dist.good += 1,
                40..=59 => quality_dist.average += 1,
                _ => quality_dist.below_average += 1,
            }
        }

        let avg_messages = if total > 0 {
            total_messages as f64 / total as f64
        } else {
            0.0
        };

        let avg_tokens = if total > 0 {
            total_tokens as f64 / total as f64
        } else {
            0.0
        };

        // Top tags
        let total_f = total as f64;
        let mut top_tags: Vec<_> = tag_counts
            .into_iter()
            .map(|(tag, count)| TagUsage {
                tag,
                count,
                percentage: if total_f > 0.0 {
                    (count as f64 / total_f) * 100.0
                } else {
                    0.0
                },
            })
            .collect();
        top_tags.sort_by(|a, b| b.count.cmp(&a.count));
        top_tags.truncate(10);

        SessionAnalytics {
            avg_duration_minutes: 0.0, // Would need timing data
            avg_messages,
            avg_tokens,
            length_distribution: length_dist,
            top_tags,
            quality_distribution: quality_dist,
        }
    }

    fn calculate_collaboration_metrics(
        &self,
        sessions: &[&SessionAnalyticsData],
        _members: &[MemberAnalyticsData],
    ) -> CollaborationMetrics {
        let shared_sessions = sessions.iter().filter(|s| s.is_shared).count() as u64;
        let total_comments: u64 = sessions.iter().map(|s| s.comment_count as u64).sum();

        CollaborationMetrics {
            shared_sessions,
            total_comments,
            active_collaborations: 0,
            top_collaborators: vec![],
        }
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Set cache TTL
    pub fn set_cache_ttl(&mut self, ttl_seconds: u64) {
        self.cache_ttl = ttl_seconds;
    }
}

impl Default for AnalyticsEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Session data for analytics
#[derive(Debug, Clone)]
pub struct SessionAnalyticsData {
    pub session_id: String,
    pub owner_id: Uuid,
    pub provider: String,
    pub model: Option<String>,
    pub message_count: u32,
    pub token_count: u32,
    pub created_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub quality_score: u8,
    pub is_shared: bool,
    pub comment_count: u32,
}

/// Member data for analytics
#[derive(Debug, Clone)]
pub struct MemberAnalyticsData {
    pub member_id: Uuid,
    pub display_name: String,
    pub joined_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_period_start_date() {
        let start = AnalyticsPeriod::Last7Days.start_date();
        let expected = Utc::now() - Duration::days(7);
        assert!((start - expected).num_seconds().abs() < 2);
    }

    #[test]
    fn test_generate_dashboard() {
        let mut engine = AnalyticsEngine::new();
        let team_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        let sessions = vec![SessionAnalyticsData {
            session_id: "session-1".to_string(),
            owner_id,
            provider: "copilot".to_string(),
            model: Some("gpt-4".to_string()),
            message_count: 10,
            token_count: 500,
            created_at: Utc::now(),
            tags: vec!["rust".to_string()],
            quality_score: 85,
            is_shared: false,
            comment_count: 0,
        }];

        let members = vec![MemberAnalyticsData {
            member_id: owner_id,
            display_name: "Test User".to_string(),
            joined_at: Utc::now() - Duration::days(30),
        }];

        let dashboard = engine.generate_dashboard(team_id, AnalyticsPeriod::Last7Days, &sessions, &members);

        assert_eq!(dashboard.overview.total_sessions, 1);
        assert_eq!(dashboard.overview.total_messages, 10);
    }
}
