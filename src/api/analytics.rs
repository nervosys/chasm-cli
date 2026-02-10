// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Usage Analytics Module
//!
//! Provides analytics tracking and reporting for enterprise insights.
//! Tracks session counts, provider usage, API metrics, and team activity.

use actix_web::{web, HttpResponse};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::db::Database;

// =============================================================================
// Analytics Event Types
// =============================================================================

/// Types of trackable analytics events
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsEventType {
    /// Session harvested
    SessionHarvested,
    /// Session viewed
    SessionViewed,
    /// Session exported
    SessionExported,
    /// Session deleted
    SessionDeleted,
    /// Session shared
    SessionShared,
    /// Search performed
    SearchPerformed,
    /// API request made
    ApiRequest,
    /// User login
    UserLogin,
    /// User logout
    UserLogout,
    /// Feature used
    FeatureUsed,
    /// Provider synced
    ProviderSync,
    /// Export completed
    ExportCompleted,
}

impl AnalyticsEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SessionHarvested => "session_harvested",
            Self::SessionViewed => "session_viewed",
            Self::SessionExported => "session_exported",
            Self::SessionDeleted => "session_deleted",
            Self::SessionShared => "session_shared",
            Self::SearchPerformed => "search_performed",
            Self::ApiRequest => "api_request",
            Self::UserLogin => "user_login",
            Self::UserLogout => "user_logout",
            Self::FeatureUsed => "feature_used",
            Self::ProviderSync => "provider_sync",
            Self::ExportCompleted => "export_completed",
        }
    }
}

// =============================================================================
// Analytics Event
// =============================================================================

/// Analytics event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    /// Event ID
    pub id: String,
    /// Event type
    pub event_type: AnalyticsEventType,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// User ID (if authenticated)
    pub user_id: Option<String>,
    /// Organization ID
    pub organization_id: Option<String>,
    /// Provider name
    pub provider: Option<String>,
    /// Feature name
    pub feature: Option<String>,
    /// Resource type
    pub resource_type: Option<String>,
    /// Resource ID
    pub resource_id: Option<String>,
    /// Additional properties
    pub properties: HashMap<String, serde_json::Value>,
    /// Client info
    pub client: Option<ClientInfo>,
}

/// Client information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub platform: Option<String>,
    pub app_version: Option<String>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

// =============================================================================
// Analytics Metrics
// =============================================================================

/// System-wide statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    /// Total registered users
    pub total_users: u64,
    /// Active users (logged in within period)
    pub active_users: u64,
    /// Total sessions harvested
    pub total_sessions: u64,
    /// Total workspaces
    pub total_workspaces: u64,
    /// Total storage used (bytes)
    pub storage_used_bytes: u64,
    /// Total storage available (bytes)
    pub storage_total_bytes: u64,
    /// System uptime percentage
    pub uptime_percent: f64,
    /// API requests in period
    pub api_requests: u64,
    /// Errors in period
    pub error_count: u64,
}

/// Usage metrics over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageMetrics {
    /// Time period
    pub period: TimePeriod,
    /// Start of period
    pub start_time: DateTime<Utc>,
    /// End of period
    pub end_time: DateTime<Utc>,
    /// Sessions harvested
    pub sessions_harvested: u64,
    /// Sessions viewed
    pub sessions_viewed: u64,
    /// Sessions exported
    pub sessions_exported: u64,
    /// Active users
    pub active_users: u64,
    /// API requests
    pub api_requests: u64,
    /// Searches performed
    pub searches: u64,
    /// Exports completed
    pub exports: u64,
}

/// Provider usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStats {
    /// Provider name
    pub provider: String,
    /// Total sessions
    pub sessions_count: u64,
    /// Total messages
    pub messages_count: u64,
    /// Active users
    pub active_users: u64,
    /// Last sync time
    pub last_sync: Option<DateTime<Utc>>,
    /// Sync success rate
    pub sync_success_rate: f64,
}

/// User activity summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActivitySummary {
    /// User ID
    pub user_id: String,
    /// User email
    pub email: String,
    /// Sessions count
    pub sessions_count: u64,
    /// Last active time
    pub last_active: Option<DateTime<Utc>>,
    /// Total API requests
    pub api_requests: u64,
    /// Favorite provider
    pub favorite_provider: Option<String>,
}

/// Team/organization statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamStats {
    /// Organization ID
    pub organization_id: String,
    /// Total members
    pub member_count: u64,
    /// Active members
    pub active_members: u64,
    /// Total sessions
    pub total_sessions: u64,
    /// Total workspaces
    pub total_workspaces: u64,
    /// Storage used
    pub storage_used_bytes: u64,
}

/// Time series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    /// Timestamp (bucket start)
    pub timestamp: DateTime<Utc>,
    /// Value
    pub value: u64,
}

/// Time series response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries {
    /// Metric name
    pub metric: String,
    /// Data points
    pub data: Vec<TimeSeriesPoint>,
    /// Aggregation period
    pub granularity: Granularity,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimePeriod {
    Today,
    Yesterday,
    Last7Days,
    Last30Days,
    Last90Days,
    ThisMonth,
    LastMonth,
    ThisYear,
    AllTime,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Granularity {
    Hour,
    Day,
    Week,
    Month,
}

// =============================================================================
// Analytics Service
// =============================================================================

/// Analytics service for tracking and reporting
pub struct AnalyticsService {
    db: Database,
}

impl AnalyticsService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // =========================================================================
    // Event Tracking
    // =========================================================================

    /// Track an analytics event
    pub async fn track(&self, event: AnalyticsEvent) -> Result<(), String> {
        self.db
            .insert_analytics_event(&event)
            .map_err(|e| format!("Failed to track event: {}", e))
    }

    /// Track event using builder
    pub async fn track_event(
        &self,
        event_type: AnalyticsEventType,
        user_id: Option<&str>,
        organization_id: Option<&str>,
    ) -> Result<(), String> {
        let event = AnalyticsEvent {
            id: Uuid::new_v4().to_string(),
            event_type,
            timestamp: Utc::now(),
            user_id: user_id.map(String::from),
            organization_id: organization_id.map(String::from),
            provider: None,
            feature: None,
            resource_type: None,
            resource_id: None,
            properties: HashMap::new(),
            client: None,
        };
        self.track(event).await
    }

    /// Track session event
    pub async fn track_session_event(
        &self,
        event_type: AnalyticsEventType,
        user_id: Option<&str>,
        session_id: &str,
        provider: &str,
    ) -> Result<(), String> {
        let mut properties = HashMap::new();
        properties.insert("session_id".to_string(), serde_json::json!(session_id));

        let event = AnalyticsEvent {
            id: Uuid::new_v4().to_string(),
            event_type,
            timestamp: Utc::now(),
            user_id: user_id.map(String::from),
            organization_id: None,
            provider: Some(provider.to_string()),
            feature: None,
            resource_type: Some("session".to_string()),
            resource_id: Some(session_id.to_string()),
            properties,
            client: None,
        };
        self.track(event).await
    }

    /// Track API request
    pub async fn track_api_request(
        &self,
        user_id: Option<&str>,
        method: &str,
        path: &str,
        status: u16,
        duration_ms: u64,
    ) -> Result<(), String> {
        let mut properties = HashMap::new();
        properties.insert("method".to_string(), serde_json::json!(method));
        properties.insert("path".to_string(), serde_json::json!(path));
        properties.insert("status".to_string(), serde_json::json!(status));
        properties.insert("duration_ms".to_string(), serde_json::json!(duration_ms));

        let event = AnalyticsEvent {
            id: Uuid::new_v4().to_string(),
            event_type: AnalyticsEventType::ApiRequest,
            timestamp: Utc::now(),
            user_id: user_id.map(String::from),
            organization_id: None,
            provider: None,
            feature: None,
            resource_type: Some("api".to_string()),
            resource_id: None,
            properties,
            client: None,
        };
        self.track(event).await
    }

    // =========================================================================
    // Metrics & Reporting
    // =========================================================================

    /// Get system-wide statistics
    pub async fn get_system_stats(&self) -> Result<SystemStats, String> {
        self.db
            .get_system_stats()
            .map_err(|e| format!("Failed to get stats: {}", e))
    }

    /// Get usage metrics for a period
    pub async fn get_usage_metrics(
        &self,
        period: TimePeriod,
        organization_id: Option<&str>,
    ) -> Result<UsageMetrics, String> {
        let (start, end) = self.period_to_range(period);
        self.db
            .get_usage_metrics(start, end, organization_id)
            .map_err(|e| format!("Failed to get metrics: {}", e))
    }

    /// Get provider statistics
    pub async fn get_provider_stats(
        &self,
        period: TimePeriod,
        organization_id: Option<&str>,
    ) -> Result<Vec<ProviderStats>, String> {
        let (start, end) = self.period_to_range(period);
        self.db
            .get_provider_stats(start, end, organization_id)
            .map_err(|e| format!("Failed to get provider stats: {}", e))
    }

    /// Get top users by activity
    pub async fn get_top_users(
        &self,
        period: TimePeriod,
        organization_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<UserActivitySummary>, String> {
        let (start, end) = self.period_to_range(period);
        self.db
            .get_top_users(start, end, organization_id, limit)
            .map_err(|e| format!("Failed to get top users: {}", e))
    }

    /// Get team statistics
    pub async fn get_team_stats(&self, organization_id: &str) -> Result<TeamStats, String> {
        self.db
            .get_team_stats(organization_id)
            .map_err(|e| format!("Failed to get team stats: {}", e))
    }

    /// Get time series data for a metric
    pub async fn get_time_series(
        &self,
        metric: &str,
        period: TimePeriod,
        granularity: Granularity,
        organization_id: Option<&str>,
    ) -> Result<TimeSeries, String> {
        let (start, end) = self.period_to_range(period);
        let data = self
            .db
            .get_time_series(metric, start, end, granularity, organization_id)
            .map_err(|e| format!("Failed to get time series: {}", e))?;

        Ok(TimeSeries {
            metric: metric.to_string(),
            data,
            granularity,
        })
    }

    /// Get dashboard summary
    pub async fn get_dashboard_summary(
        &self,
        organization_id: Option<&str>,
    ) -> Result<DashboardSummary, String> {
        let stats = self.get_system_stats().await?;
        let usage = self
            .get_usage_metrics(TimePeriod::Last7Days, organization_id)
            .await?;
        let providers = self
            .get_provider_stats(TimePeriod::Last30Days, organization_id)
            .await?;
        let top_users = self
            .get_top_users(TimePeriod::Last30Days, organization_id, 10)
            .await?;

        // Get time series for charts
        let sessions_trend = self
            .get_time_series(
                "sessions",
                TimePeriod::Last30Days,
                Granularity::Day,
                organization_id,
            )
            .await?;

        let users_trend = self
            .get_time_series(
                "active_users",
                TimePeriod::Last30Days,
                Granularity::Day,
                organization_id,
            )
            .await?;

        Ok(DashboardSummary {
            stats,
            usage,
            providers,
            top_users,
            sessions_trend,
            users_trend,
        })
    }

    /// Export analytics data
    pub async fn export_analytics(
        &self,
        period: TimePeriod,
        organization_id: Option<&str>,
        format: ExportFormat,
    ) -> Result<Vec<u8>, String> {
        let summary = self.get_dashboard_summary(organization_id).await?;

        match format {
            ExportFormat::Json => {
                serde_json::to_vec_pretty(&summary).map_err(|e| format!("JSON error: {}", e))
            }
            ExportFormat::Csv => {
                // Simplified CSV export
                let mut output = String::new();
                output.push_str("metric,value\n");
                output.push_str(&format!("total_users,{}\n", summary.stats.total_users));
                output.push_str(&format!("active_users,{}\n", summary.stats.active_users));
                output.push_str(&format!(
                    "total_sessions,{}\n",
                    summary.stats.total_sessions
                ));
                output.push_str(&format!("api_requests,{}\n", summary.stats.api_requests));
                Ok(output.into_bytes())
            }
        }
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    fn period_to_range(&self, period: TimePeriod) -> (DateTime<Utc>, DateTime<Utc>) {
        let now = Utc::now();
        let start = match period {
            TimePeriod::Today => now
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
                .unwrap_or(now),
            TimePeriod::Yesterday => (now - Duration::days(1))
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
                .unwrap_or(now),
            TimePeriod::Last7Days => now - Duration::days(7),
            TimePeriod::Last30Days => now - Duration::days(30),
            TimePeriod::Last90Days => now - Duration::days(90),
            TimePeriod::ThisMonth => now
                .date_naive()
                .with_day(1)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
                .unwrap_or(now),
            TimePeriod::LastMonth => (now - Duration::days(30))
                .date_naive()
                .with_day(1)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
                .unwrap_or(now),
            TimePeriod::ThisYear => now
                .date_naive()
                .with_ordinal(1)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
                .unwrap_or(now),
            TimePeriod::AllTime | TimePeriod::Custom => {
                DateTime::from_timestamp(0, 0).unwrap_or(now)
            }
        };
        (start, now)
    }
}

// =============================================================================
// Response Types
// =============================================================================

/// Dashboard summary response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub stats: SystemStats,
    pub usage: UsageMetrics,
    pub providers: Vec<ProviderStats>,
    pub top_users: Vec<UserActivitySummary>,
    pub sessions_trend: TimeSeries,
    pub users_trend: TimeSeries,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    #[default]
    Json,
    Csv,
}

// =============================================================================
// HTTP Handlers
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub period: Option<TimePeriod>,
    pub organization_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TimeSeriesQuery {
    pub metric: String,
    pub period: Option<TimePeriod>,
    pub granularity: Option<Granularity>,
    pub organization_id: Option<String>,
}

/// GET /api/analytics/stats - Get system statistics
pub async fn get_stats(analytics: web::Data<AnalyticsService>) -> HttpResponse {
    match analytics.get_system_stats().await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/analytics/usage - Get usage metrics
pub async fn get_usage(
    analytics: web::Data<AnalyticsService>,
    query: web::Query<MetricsQuery>,
) -> HttpResponse {
    let period = query.period.unwrap_or(TimePeriod::Last30Days);
    let org_id = query.organization_id.as_deref();

    match analytics.get_usage_metrics(period, org_id).await {
        Ok(metrics) => HttpResponse::Ok().json(metrics),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/analytics/providers - Get provider statistics
pub async fn get_providers(
    analytics: web::Data<AnalyticsService>,
    query: web::Query<MetricsQuery>,
) -> HttpResponse {
    let period = query.period.unwrap_or(TimePeriod::Last30Days);
    let org_id = query.organization_id.as_deref();

    match analytics.get_provider_stats(period, org_id).await {
        Ok(providers) => HttpResponse::Ok().json(providers),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/analytics/users - Get top users
pub async fn get_users(
    analytics: web::Data<AnalyticsService>,
    query: web::Query<MetricsQuery>,
) -> HttpResponse {
    let period = query.period.unwrap_or(TimePeriod::Last30Days);
    let org_id = query.organization_id.as_deref();

    match analytics.get_top_users(period, org_id, 50).await {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/analytics/team/{org_id} - Get team statistics
pub async fn get_team(
    analytics: web::Data<AnalyticsService>,
    path: web::Path<String>,
) -> HttpResponse {
    let org_id = path.into_inner();

    match analytics.get_team_stats(&org_id).await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/analytics/timeseries - Get time series data
pub async fn get_timeseries(
    analytics: web::Data<AnalyticsService>,
    query: web::Query<TimeSeriesQuery>,
) -> HttpResponse {
    let period = query.period.unwrap_or(TimePeriod::Last30Days);
    let granularity = query.granularity.unwrap_or(Granularity::Day);
    let org_id = query.organization_id.as_deref();

    match analytics
        .get_time_series(&query.metric, period, granularity, org_id)
        .await
    {
        Ok(series) => HttpResponse::Ok().json(series),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/analytics/dashboard - Get dashboard summary
pub async fn get_dashboard(
    analytics: web::Data<AnalyticsService>,
    query: web::Query<MetricsQuery>,
) -> HttpResponse {
    let org_id = query.organization_id.as_deref();

    match analytics.get_dashboard_summary(org_id).await {
        Ok(summary) => HttpResponse::Ok().json(summary),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub period: Option<TimePeriod>,
    pub organization_id: Option<String>,
    pub format: Option<ExportFormat>,
}

/// POST /api/analytics/export - Export analytics data
pub async fn export_analytics(
    analytics: web::Data<AnalyticsService>,
    query: web::Query<ExportQuery>,
) -> HttpResponse {
    let period = query.period.unwrap_or(TimePeriod::Last30Days);
    let org_id = query.organization_id.as_deref();
    let format = query.format.unwrap_or_default();

    let content_type = match format {
        ExportFormat::Json => "application/json",
        ExportFormat::Csv => "text/csv",
    };

    match analytics.export_analytics(period, org_id, format).await {
        Ok(data) => HttpResponse::Ok()
            .content_type(content_type)
            .append_header((
                "Content-Disposition",
                "attachment; filename=\"analytics.export\"",
            ))
            .body(data),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// POST /api/analytics/track - Track custom event
pub async fn track_event(
    analytics: web::Data<AnalyticsService>,
    event: web::Json<AnalyticsEvent>,
) -> HttpResponse {
    match analytics.track(event.into_inner()).await {
        Ok(()) => HttpResponse::Accepted().finish(),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// Configure analytics routes
pub fn configure_analytics_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/analytics")
            .route("/stats", web::get().to(get_stats))
            .route("/usage", web::get().to(get_usage))
            .route("/providers", web::get().to(get_providers))
            .route("/users", web::get().to(get_users))
            .route("/team/{org_id}", web::get().to(get_team))
            .route("/timeseries", web::get().to(get_timeseries))
            .route("/dashboard", web::get().to(get_dashboard))
            .route("/export", web::post().to(export_analytics))
            .route("/track", web::post().to(track_event)),
    );
}
