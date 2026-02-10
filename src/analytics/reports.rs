// Copyright (c) 2024-2027 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Report generation module
//!
//! Generates PDF and CSV reports for analytics data.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::Write;
use uuid::Uuid;

use super::dashboard::{TeamDashboard, AnalyticsPeriod, MemberStats, ProviderStats};

// ============================================================================
// Report Types
// ============================================================================

/// Report format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    Pdf,
    Csv,
    Json,
    Html,
}

/// Report type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportType {
    /// Full team analytics report
    TeamAnalytics,
    /// Member activity report
    MemberActivity,
    /// Provider usage report
    ProviderUsage,
    /// Session summary report
    SessionSummary,
    /// Collaboration report
    Collaboration,
}

/// Report request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportRequest {
    /// Team ID
    pub team_id: Uuid,
    /// Report type
    pub report_type: ReportType,
    /// Report format
    pub format: ReportFormat,
    /// Time period
    pub period: AnalyticsPeriod,
    /// Custom start date (for custom period)
    pub start_date: Option<DateTime<Utc>>,
    /// Custom end date (for custom period)
    pub end_date: Option<DateTime<Utc>>,
    /// Include detailed breakdowns
    pub include_details: bool,
    /// Requested by user ID
    pub requested_by: Uuid,
}

/// Generated report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Report ID
    pub id: Uuid,
    /// Report type
    pub report_type: ReportType,
    /// Report format
    pub format: ReportFormat,
    /// Team ID
    pub team_id: Uuid,
    /// Time period
    pub period: AnalyticsPeriod,
    /// Generated at
    pub generated_at: DateTime<Utc>,
    /// Report title
    pub title: String,
    /// File name
    pub filename: String,
    /// Content (base64 encoded for binary formats)
    pub content: String,
    /// Content size in bytes
    pub size_bytes: usize,
}

// ============================================================================
// Report Generator
// ============================================================================

/// Report generator
pub struct ReportGenerator;

impl ReportGenerator {
    /// Create a new report generator
    pub fn new() -> Self {
        Self
    }

    /// Generate a report from dashboard data
    pub fn generate(&self, request: &ReportRequest, dashboard: &TeamDashboard) -> Report {
        let content = match request.format {
            ReportFormat::Csv => self.generate_csv(request, dashboard),
            ReportFormat::Json => self.generate_json(request, dashboard),
            ReportFormat::Html => self.generate_html(request, dashboard),
            ReportFormat::Pdf => self.generate_pdf_placeholder(request, dashboard),
        };

        let title = self.get_report_title(request);
        let filename = self.get_filename(request);

        Report {
            id: Uuid::new_v4(),
            report_type: request.report_type,
            format: request.format,
            team_id: request.team_id,
            period: request.period,
            generated_at: Utc::now(),
            title,
            filename,
            size_bytes: content.len(),
            content,
        }
    }

    /// Generate CSV report
    fn generate_csv(&self, request: &ReportRequest, dashboard: &TeamDashboard) -> String {
        let mut csv = String::new();

        match request.report_type {
            ReportType::TeamAnalytics => {
                // Overview
                csv.push_str("Team Analytics Report\n\n");
                csv.push_str("Metric,Value,Change\n");
                csv.push_str(&format!(
                    "Total Sessions,{},{:.1}%\n",
                    dashboard.overview.total_sessions,
                    dashboard.overview.sessions_change
                ));
                csv.push_str(&format!(
                    "Total Messages,{},{:.1}%\n",
                    dashboard.overview.total_messages,
                    dashboard.overview.messages_change
                ));
                csv.push_str(&format!(
                    "Total Tokens,{},{:.1}%\n",
                    dashboard.overview.total_tokens,
                    dashboard.overview.tokens_change
                ));
                csv.push_str(&format!(
                    "Active Members,{},{:.1}%\n",
                    dashboard.overview.active_members,
                    dashboard.overview.active_members_change
                ));
                csv.push_str(&format!(
                    "Avg Sessions/Member,{:.2}\n",
                    dashboard.overview.avg_sessions_per_member
                ));
                csv.push_str(&format!(
                    "Avg Messages/Session,{:.2}\n",
                    dashboard.overview.avg_messages_per_session
                ));

                if request.include_details {
                    // Provider breakdown
                    csv.push_str("\nProvider Breakdown\n");
                    csv.push_str("Provider,Sessions,Percentage,Messages,Tokens\n");
                    for provider in &dashboard.provider_breakdown {
                        csv.push_str(&format!(
                            "{},{},{:.1}%,{},{}\n",
                            provider.provider,
                            provider.sessions,
                            provider.session_percentage,
                            provider.messages,
                            provider.tokens
                        ));
                    }
                }
            }
            ReportType::MemberActivity => {
                csv.push_str("Member Activity Report\n\n");
                csv.push_str("Member,Sessions,Messages,Tokens,Avg Session Length,Activity Score,Last Active\n");
                for member in &dashboard.member_stats {
                    csv.push_str(&format!(
                        "{},{},{},{},{:.2},{},{}\n",
                        member.display_name,
                        member.sessions,
                        member.messages,
                        member.tokens,
                        member.avg_session_length,
                        member.activity_score,
                        member.last_active.map(|d| d.to_rfc3339()).unwrap_or_default()
                    ));
                }
            }
            ReportType::ProviderUsage => {
                csv.push_str("Provider Usage Report\n\n");
                csv.push_str("Provider,Sessions,Percentage,Messages,Tokens\n");
                for provider in &dashboard.provider_breakdown {
                    csv.push_str(&format!(
                        "{},{},{:.1}%,{},{}\n",
                        provider.provider,
                        provider.sessions,
                        provider.session_percentage,
                        provider.messages,
                        provider.tokens
                    ));
                }
            }
            ReportType::SessionSummary => {
                csv.push_str("Session Summary Report\n\n");
                csv.push_str("Metric,Value\n");
                csv.push_str(&format!(
                    "Average Messages,{:.2}\n",
                    dashboard.session_analytics.avg_messages
                ));
                csv.push_str(&format!(
                    "Average Tokens,{:.2}\n",
                    dashboard.session_analytics.avg_tokens
                ));
                csv.push_str(&format!(
                    "Short Sessions (1-5 msgs),{}\n",
                    dashboard.session_analytics.length_distribution.short
                ));
                csv.push_str(&format!(
                    "Medium Sessions (6-20 msgs),{}\n",
                    dashboard.session_analytics.length_distribution.medium
                ));
                csv.push_str(&format!(
                    "Long Sessions (21-50 msgs),{}\n",
                    dashboard.session_analytics.length_distribution.long
                ));
                csv.push_str(&format!(
                    "Very Long Sessions (51+ msgs),{}\n",
                    dashboard.session_analytics.length_distribution.very_long
                ));

                csv.push_str("\nTop Tags\n");
                csv.push_str("Tag,Count,Percentage\n");
                for tag in &dashboard.session_analytics.top_tags {
                    csv.push_str(&format!(
                        "{},{},{:.1}%\n",
                        tag.tag, tag.count, tag.percentage
                    ));
                }
            }
            ReportType::Collaboration => {
                csv.push_str("Collaboration Report\n\n");
                csv.push_str("Metric,Value\n");
                csv.push_str(&format!(
                    "Shared Sessions,{}\n",
                    dashboard.collaboration.shared_sessions
                ));
                csv.push_str(&format!(
                    "Total Comments,{}\n",
                    dashboard.collaboration.total_comments
                ));
                csv.push_str(&format!(
                    "Active Collaborations,{}\n",
                    dashboard.collaboration.active_collaborations
                ));
            }
        }

        csv
    }

    /// Generate JSON report
    fn generate_json(&self, request: &ReportRequest, dashboard: &TeamDashboard) -> String {
        match request.report_type {
            ReportType::TeamAnalytics => {
                serde_json::to_string_pretty(dashboard).unwrap_or_default()
            }
            ReportType::MemberActivity => {
                serde_json::to_string_pretty(&dashboard.member_stats).unwrap_or_default()
            }
            ReportType::ProviderUsage => {
                serde_json::to_string_pretty(&dashboard.provider_breakdown).unwrap_or_default()
            }
            ReportType::SessionSummary => {
                serde_json::to_string_pretty(&dashboard.session_analytics).unwrap_or_default()
            }
            ReportType::Collaboration => {
                serde_json::to_string_pretty(&dashboard.collaboration).unwrap_or_default()
            }
        }
    }

    /// Generate HTML report
    fn generate_html(&self, request: &ReportRequest, dashboard: &TeamDashboard) -> String {
        let title = self.get_report_title(request);
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str(&format!("<title>{}</title>\n", title));
        html.push_str("<style>\n");
        html.push_str("body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 40px; color: #333; }\n");
        html.push_str("h1 { color: #2563eb; border-bottom: 2px solid #2563eb; padding-bottom: 10px; }\n");
        html.push_str("h2 { color: #1f2937; margin-top: 30px; }\n");
        html.push_str("table { border-collapse: collapse; width: 100%; margin: 20px 0; }\n");
        html.push_str("th, td { border: 1px solid #e5e7eb; padding: 12px; text-align: left; }\n");
        html.push_str("th { background: #f3f4f6; font-weight: 600; }\n");
        html.push_str("tr:nth-child(even) { background: #f9fafb; }\n");
        html.push_str(".metric-card { display: inline-block; background: #f3f4f6; padding: 20px; margin: 10px; border-radius: 8px; min-width: 150px; }\n");
        html.push_str(".metric-value { font-size: 32px; font-weight: bold; color: #2563eb; }\n");
        html.push_str(".metric-label { color: #6b7280; margin-top: 5px; }\n");
        html.push_str(".change-positive { color: #059669; }\n");
        html.push_str(".change-negative { color: #dc2626; }\n");
        html.push_str(".footer { margin-top: 40px; color: #9ca3af; font-size: 12px; }\n");
        html.push_str("</style>\n</head>\n<body>\n");

        html.push_str(&format!("<h1>{}</h1>\n", title));
        html.push_str(&format!(
            "<p>Generated: {} | Period: {:?}</p>\n",
            dashboard.generated_at.format("%Y-%m-%d %H:%M UTC"),
            dashboard.period
        ));

        match request.report_type {
            ReportType::TeamAnalytics => {
                // Overview cards
                html.push_str("<h2>Overview</h2>\n<div>\n");
                html.push_str(&self.metric_card(
                    "Total Sessions",
                    &dashboard.overview.total_sessions.to_string(),
                    dashboard.overview.sessions_change,
                ));
                html.push_str(&self.metric_card(
                    "Total Messages",
                    &dashboard.overview.total_messages.to_string(),
                    dashboard.overview.messages_change,
                ));
                html.push_str(&self.metric_card(
                    "Active Members",
                    &dashboard.overview.active_members.to_string(),
                    dashboard.overview.active_members_change,
                ));
                html.push_str("</div>\n");

                if request.include_details {
                    // Provider table
                    html.push_str("<h2>Provider Breakdown</h2>\n");
                    html.push_str(
                        "<table>\n<tr><th>Provider</th><th>Sessions</th><th>%</th><th>Messages</th><th>Tokens</th></tr>\n"
                    );
                    for p in &dashboard.provider_breakdown {
                        html.push_str(&format!(
                            "<tr><td>{}</td><td>{}</td><td>{:.1}%</td><td>{}</td><td>{}</td></tr>\n",
                            p.provider, p.sessions, p.session_percentage, p.messages, p.tokens
                        ));
                    }
                    html.push_str("</table>\n");
                }
            }
            ReportType::MemberActivity => {
                html.push_str("<h2>Member Activity</h2>\n");
                html.push_str(
                    "<table>\n<tr><th>Member</th><th>Sessions</th><th>Messages</th><th>Tokens</th><th>Avg Length</th><th>Score</th></tr>\n"
                );
                for m in &dashboard.member_stats {
                    html.push_str(&format!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.1}</td><td>{}</td></tr>\n",
                        m.display_name, m.sessions, m.messages, m.tokens, m.avg_session_length, m.activity_score
                    ));
                }
                html.push_str("</table>\n");
            }
            ReportType::ProviderUsage => {
                html.push_str("<h2>Provider Usage</h2>\n");
                html.push_str(
                    "<table>\n<tr><th>Provider</th><th>Sessions</th><th>%</th><th>Messages</th><th>Tokens</th></tr>\n"
                );
                for p in &dashboard.provider_breakdown {
                    html.push_str(&format!(
                        "<tr><td>{}</td><td>{}</td><td>{:.1}%</td><td>{}</td><td>{}</td></tr>\n",
                        p.provider, p.sessions, p.session_percentage, p.messages, p.tokens
                    ));
                }
                html.push_str("</table>\n");
            }
            ReportType::SessionSummary => {
                html.push_str("<h2>Session Statistics</h2>\n<div>\n");
                html.push_str(&self.metric_card(
                    "Avg Messages",
                    &format!("{:.1}", dashboard.session_analytics.avg_messages),
                    0.0,
                ));
                html.push_str(&self.metric_card(
                    "Avg Tokens",
                    &format!("{:.0}", dashboard.session_analytics.avg_tokens),
                    0.0,
                ));
                html.push_str("</div>\n");

                html.push_str("<h2>Session Length Distribution</h2>\n");
                html.push_str("<table>\n<tr><th>Length</th><th>Count</th></tr>\n");
                html.push_str(&format!(
                    "<tr><td>Short (1-5)</td><td>{}</td></tr>\n",
                    dashboard.session_analytics.length_distribution.short
                ));
                html.push_str(&format!(
                    "<tr><td>Medium (6-20)</td><td>{}</td></tr>\n",
                    dashboard.session_analytics.length_distribution.medium
                ));
                html.push_str(&format!(
                    "<tr><td>Long (21-50)</td><td>{}</td></tr>\n",
                    dashboard.session_analytics.length_distribution.long
                ));
                html.push_str(&format!(
                    "<tr><td>Very Long (51+)</td><td>{}</td></tr>\n",
                    dashboard.session_analytics.length_distribution.very_long
                ));
                html.push_str("</table>\n");
            }
            ReportType::Collaboration => {
                html.push_str("<h2>Collaboration Metrics</h2>\n<div>\n");
                html.push_str(&self.metric_card(
                    "Shared Sessions",
                    &dashboard.collaboration.shared_sessions.to_string(),
                    0.0,
                ));
                html.push_str(&self.metric_card(
                    "Total Comments",
                    &dashboard.collaboration.total_comments.to_string(),
                    0.0,
                ));
                html.push_str("</div>\n");
            }
        }

        html.push_str("<div class=\"footer\">Generated by Chasm Analytics</div>\n");
        html.push_str("</body>\n</html>");

        html
    }

    /// Generate PDF placeholder (actual PDF generation would require a PDF library)
    fn generate_pdf_placeholder(&self, request: &ReportRequest, dashboard: &TeamDashboard) -> String {
        // In production, use a library like printpdf or wkhtmltopdf
        // For now, return HTML that can be converted to PDF
        self.generate_html(request, dashboard)
    }

    /// Create a metric card HTML
    fn metric_card(&self, label: &str, value: &str, change: f64) -> String {
        let change_class = if change >= 0.0 {
            "change-positive"
        } else {
            "change-negative"
        };
        let change_str = if change != 0.0 {
            format!(
                " <span class=\"{}\">{:+.1}%</span>",
                change_class, change
            )
        } else {
            String::new()
        };

        format!(
            "<div class=\"metric-card\"><div class=\"metric-value\">{}{}</div><div class=\"metric-label\">{}</div></div>\n",
            value, change_str, label
        )
    }

    /// Get report title
    fn get_report_title(&self, request: &ReportRequest) -> String {
        match request.report_type {
            ReportType::TeamAnalytics => "Team Analytics Report".to_string(),
            ReportType::MemberActivity => "Member Activity Report".to_string(),
            ReportType::ProviderUsage => "Provider Usage Report".to_string(),
            ReportType::SessionSummary => "Session Summary Report".to_string(),
            ReportType::Collaboration => "Collaboration Report".to_string(),
        }
    }

    /// Get filename for report
    fn get_filename(&self, request: &ReportRequest) -> String {
        let type_str = match request.report_type {
            ReportType::TeamAnalytics => "team-analytics",
            ReportType::MemberActivity => "member-activity",
            ReportType::ProviderUsage => "provider-usage",
            ReportType::SessionSummary => "session-summary",
            ReportType::Collaboration => "collaboration",
        };

        let ext = match request.format {
            ReportFormat::Csv => "csv",
            ReportFormat::Json => "json",
            ReportFormat::Html => "html",
            ReportFormat::Pdf => "pdf",
        };

        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        format!("chasm-{}-{}.{}", type_str, timestamp, ext)
    }
}

impl Default for ReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::dashboard::*;

    fn create_test_dashboard() -> TeamDashboard {
        TeamDashboard {
            team_id: Uuid::new_v4(),
            generated_at: Utc::now(),
            period: AnalyticsPeriod::Last7Days,
            overview: OverviewMetrics {
                total_sessions: 100,
                sessions_change: 10.5,
                total_messages: 1000,
                messages_change: 15.2,
                total_tokens: 50000,
                tokens_change: 8.3,
                active_members: 5,
                active_members_change: 0.0,
                avg_sessions_per_member: 20.0,
                avg_messages_per_session: 10.0,
            },
            trends: UsageTrends {
                daily_sessions: vec![],
                daily_messages: vec![],
                daily_tokens: vec![],
                hourly_distribution: vec![0; 24],
                weekday_distribution: vec![0; 7],
            },
            member_stats: vec![MemberStats {
                member_id: Uuid::new_v4(),
                display_name: "Test User".to_string(),
                sessions: 50,
                messages: 500,
                tokens: 25000,
                favorite_provider: Some("copilot".to_string()),
                avg_session_length: 10.0,
                last_active: Some(Utc::now()),
                activity_score: 85,
            }],
            provider_breakdown: vec![ProviderStats {
                provider: "copilot".to_string(),
                sessions: 60,
                session_percentage: 60.0,
                messages: 600,
                tokens: 30000,
                top_models: vec![],
            }],
            session_analytics: SessionAnalytics {
                avg_duration_minutes: 15.0,
                avg_messages: 10.0,
                avg_tokens: 500.0,
                length_distribution: SessionLengthDistribution {
                    short: 20,
                    medium: 50,
                    long: 25,
                    very_long: 5,
                },
                top_tags: vec![],
                quality_distribution: QualityDistribution {
                    excellent: 30,
                    good: 40,
                    average: 25,
                    below_average: 5,
                },
            },
            collaboration: CollaborationMetrics {
                shared_sessions: 20,
                total_comments: 100,
                active_collaborations: 5,
                top_collaborators: vec![],
            },
        }
    }

    #[test]
    fn test_generate_csv_report() {
        let generator = ReportGenerator::new();
        let dashboard = create_test_dashboard();
        let request = ReportRequest {
            team_id: dashboard.team_id,
            report_type: ReportType::TeamAnalytics,
            format: ReportFormat::Csv,
            period: AnalyticsPeriod::Last7Days,
            start_date: None,
            end_date: None,
            include_details: true,
            requested_by: Uuid::new_v4(),
        };

        let report = generator.generate(&request, &dashboard);
        assert!(report.content.contains("Total Sessions,100"));
        assert!(report.filename.ends_with(".csv"));
    }

    #[test]
    fn test_generate_html_report() {
        let generator = ReportGenerator::new();
        let dashboard = create_test_dashboard();
        let request = ReportRequest {
            team_id: dashboard.team_id,
            report_type: ReportType::MemberActivity,
            format: ReportFormat::Html,
            period: AnalyticsPeriod::Last7Days,
            start_date: None,
            end_date: None,
            include_details: true,
            requested_by: Uuid::new_v4(),
        };

        let report = generator.generate(&request, &dashboard);
        assert!(report.content.contains("<html>"));
        assert!(report.content.contains("Test User"));
        assert!(report.filename.ends_with(".html"));
    }
}
