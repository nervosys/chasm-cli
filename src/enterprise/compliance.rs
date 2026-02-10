// Copyright (c) 2024-2027 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Compliance and audit features
//!
//! SOC2, HIPAA, and other compliance framework support.

use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// Compliance Frameworks
// ============================================================================

/// Supported compliance frameworks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ComplianceFramework {
    /// SOC 2 Type I & II
    Soc2,
    /// HIPAA
    Hipaa,
    /// GDPR
    Gdpr,
    /// CCPA
    Ccpa,
    /// ISO 27001
    Iso27001,
    /// FedRAMP
    FedRamp,
    /// PCI DSS
    PciDss,
}

impl ComplianceFramework {
    /// Get required controls for framework
    pub fn required_controls(&self) -> Vec<ComplianceControl> {
        match self {
            ComplianceFramework::Soc2 => vec![
                ComplianceControl::AccessControl,
                ComplianceControl::ChangeManagement,
                ComplianceControl::RiskAssessment,
                ComplianceControl::SystemOperations,
                ComplianceControl::LogicalAccess,
                ComplianceControl::DataClassification,
                ComplianceControl::IncidentManagement,
                ComplianceControl::BusinessContinuity,
                ComplianceControl::VendorManagement,
            ],
            ComplianceFramework::Hipaa => vec![
                ComplianceControl::AccessControl,
                ComplianceControl::AuditControls,
                ComplianceControl::IntegrityControls,
                ComplianceControl::TransmissionSecurity,
                ComplianceControl::AuthenticationControls,
                ComplianceControl::DataEncryption,
                ComplianceControl::DataRetention,
                ComplianceControl::IncidentManagement,
                ComplianceControl::RiskAssessment,
            ],
            ComplianceFramework::Gdpr => vec![
                ComplianceControl::DataMinimization,
                ComplianceControl::ConsentManagement,
                ComplianceControl::DataPortability,
                ComplianceControl::RightToErasure,
                ComplianceControl::DataProtectionByDesign,
                ComplianceControl::DataBreachNotification,
                ComplianceControl::DataProcessingRecords,
            ],
            ComplianceFramework::Ccpa => vec![
                ComplianceControl::ConsentManagement,
                ComplianceControl::DataPortability,
                ComplianceControl::RightToErasure,
                ComplianceControl::DataSaleOptOut,
                ComplianceControl::PrivacyNotice,
            ],
            _ => vec![
                ComplianceControl::AccessControl,
                ComplianceControl::AuditControls,
                ComplianceControl::DataEncryption,
            ],
        }
    }

    /// Get retention requirements (days)
    pub fn retention_requirement(&self) -> Option<u32> {
        match self {
            ComplianceFramework::Hipaa => Some(2190), // 6 years
            ComplianceFramework::Soc2 => Some(365),   // 1 year minimum
            ComplianceFramework::PciDss => Some(365), // 1 year
            _ => None,
        }
    }
}

/// Compliance control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceControl {
    AccessControl,
    AuditControls,
    ChangeManagement,
    RiskAssessment,
    SystemOperations,
    LogicalAccess,
    DataClassification,
    IncidentManagement,
    BusinessContinuity,
    VendorManagement,
    IntegrityControls,
    TransmissionSecurity,
    AuthenticationControls,
    DataEncryption,
    DataRetention,
    DataMinimization,
    ConsentManagement,
    DataPortability,
    RightToErasure,
    DataProtectionByDesign,
    DataBreachNotification,
    DataProcessingRecords,
    DataSaleOptOut,
    PrivacyNotice,
}

// ============================================================================
// Audit Events
// ============================================================================

/// Audit event severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    Info,
    Warning,
    Critical,
}

/// Audit event category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    Authentication,
    Authorization,
    DataAccess,
    DataModification,
    DataDeletion,
    Configuration,
    Security,
    System,
    Compliance,
    UserManagement,
    ApiAccess,
}

/// Audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Event ID
    pub id: Uuid,
    /// Tenant ID
    pub tenant_id: Uuid,
    /// Actor (user or system)
    pub actor: AuditActor,
    /// Event category
    pub category: AuditCategory,
    /// Action performed
    pub action: String,
    /// Resource type
    pub resource_type: String,
    /// Resource ID
    pub resource_id: Option<String>,
    /// Event severity
    pub severity: AuditSeverity,
    /// Success/failure
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Request details
    pub request: AuditRequest,
    /// Changes made
    pub changes: Option<AuditChanges>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Audit actor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditActor {
    /// Actor type
    pub actor_type: ActorType,
    /// User ID (if user)
    pub user_id: Option<Uuid>,
    /// User email
    pub email: Option<String>,
    /// Service name (if system)
    pub service_name: Option<String>,
    /// API key ID (if API)
    pub api_key_id: Option<String>,
}

/// Actor type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorType {
    User,
    System,
    ApiKey,
    Service,
}

/// Request details for audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRequest {
    /// HTTP method
    pub method: Option<String>,
    /// Request path
    pub path: Option<String>,
    /// Source IP
    pub ip_address: String,
    /// User agent
    pub user_agent: Option<String>,
    /// Request ID
    pub request_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
}

/// Changes made in event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditChanges {
    /// Fields changed
    pub fields: Vec<FieldChange>,
    /// Before state (if applicable)
    pub before: Option<serde_json::Value>,
    /// After state (if applicable)
    pub after: Option<serde_json::Value>,
}

/// Field change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    /// Field name
    pub field: String,
    /// Old value (redacted if sensitive)
    pub old_value: Option<String>,
    /// New value (redacted if sensitive)
    pub new_value: Option<String>,
    /// Whether the field is sensitive
    pub sensitive: bool,
}

// ============================================================================
// Data Classification
// ============================================================================

/// Data classification level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataClassification {
    /// Public data
    Public,
    /// Internal use only
    Internal,
    /// Confidential
    Confidential,
    /// Restricted/Highly Confidential
    Restricted,
    /// Protected Health Information (PHI)
    Phi,
    /// Personally Identifiable Information (PII)
    Pii,
}

impl DataClassification {
    /// Get required encryption for classification
    pub fn requires_encryption(&self) -> bool {
        matches!(
            self,
            DataClassification::Confidential
                | DataClassification::Restricted
                | DataClassification::Phi
                | DataClassification::Pii
        )
    }

    /// Get access restrictions
    pub fn access_restrictions(&self) -> Vec<&'static str> {
        match self {
            DataClassification::Public => vec![],
            DataClassification::Internal => vec!["authenticated_users"],
            DataClassification::Confidential => vec!["role_based_access"],
            DataClassification::Restricted => vec!["explicit_grant", "mfa_required", "audit_all_access"],
            DataClassification::Phi => vec!["hipaa_authorized", "audit_all_access", "encryption_required"],
            DataClassification::Pii => vec!["gdpr_consent", "data_minimization", "encryption_required"],
        }
    }
}

/// Data retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Policy ID
    pub id: Uuid,
    /// Policy name
    pub name: String,
    /// Description
    pub description: String,
    /// Data classification
    pub classification: DataClassification,
    /// Retention period in days
    pub retention_days: u32,
    /// Action after retention period
    pub action: RetentionAction,
    /// Legal hold override
    pub legal_hold_exempt: bool,
    /// Applicable data types
    pub data_types: Vec<String>,
    /// Active
    pub active: bool,
    /// Created at
    pub created_at: DateTime<Utc>,
}

/// Retention action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionAction {
    /// Permanently delete
    Delete,
    /// Archive to cold storage
    Archive,
    /// Anonymize data
    Anonymize,
    /// Review required
    Review,
}

// ============================================================================
// Compliance Status
// ============================================================================

/// Tenant compliance status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    /// Tenant ID
    pub tenant_id: Uuid,
    /// Enabled frameworks
    pub enabled_frameworks: Vec<ComplianceFramework>,
    /// Control statuses
    pub control_statuses: HashMap<ComplianceControl, ControlStatus>,
    /// Overall compliance score (0-100)
    pub compliance_score: u8,
    /// Last assessment
    pub last_assessment: Option<DateTime<Utc>>,
    /// Next assessment due
    pub next_assessment_due: Option<DateTime<Utc>>,
    /// Active issues
    pub issues: Vec<ComplianceIssue>,
    /// Certifications
    pub certifications: Vec<Certification>,
}

/// Control status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlStatus {
    /// Control
    pub control: ComplianceControl,
    /// Status
    pub status: ControlStatusType,
    /// Evidence collected
    pub evidence_count: usize,
    /// Last verified
    pub last_verified: Option<DateTime<Utc>>,
    /// Notes
    pub notes: Option<String>,
}

/// Control status type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlStatusType {
    /// Not implemented
    NotImplemented,
    /// In progress
    InProgress,
    /// Implemented
    Implemented,
    /// Verified
    Verified,
    /// NonCompliant
    NonCompliant,
}

/// Compliance issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceIssue {
    /// Issue ID
    pub id: Uuid,
    /// Framework
    pub framework: ComplianceFramework,
    /// Related control
    pub control: ComplianceControl,
    /// Severity
    pub severity: IssueSeverity,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Remediation steps
    pub remediation: String,
    /// Due date
    pub due_date: Option<DateTime<Utc>>,
    /// Status
    pub status: IssueStatus,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Resolved at
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Issue severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Issue status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueStatus {
    Open,
    InProgress,
    Resolved,
    Accepted,
    Waived,
}

/// Certification record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certification {
    /// Framework
    pub framework: ComplianceFramework,
    /// Certification type
    pub certification_type: String,
    /// Issued date
    pub issued_date: DateTime<Utc>,
    /// Expiry date
    pub expiry_date: DateTime<Utc>,
    /// Issuer
    pub issuer: String,
    /// Certificate URL
    pub certificate_url: Option<String>,
}

// ============================================================================
// Compliance Manager
// ============================================================================

/// Manages compliance features
pub struct ComplianceManager {
    /// Audit events
    audit_log: Vec<AuditEvent>,
    /// Retention policies
    retention_policies: HashMap<Uuid, RetentionPolicy>,
    /// Tenant compliance status
    compliance_statuses: HashMap<Uuid, ComplianceStatus>,
}

impl ComplianceManager {
    /// Create a new compliance manager
    pub fn new() -> Self {
        Self {
            audit_log: vec![],
            retention_policies: HashMap::new(),
            compliance_statuses: HashMap::new(),
        }
    }

    /// Log an audit event
    pub fn log_event(&mut self, event: AuditEvent) {
        self.audit_log.push(event);
    }

    /// Create audit event builder
    pub fn create_event(
        &self,
        tenant_id: Uuid,
        category: AuditCategory,
        action: &str,
    ) -> AuditEventBuilder {
        AuditEventBuilder::new(tenant_id, category, action)
    }

    /// Query audit log
    pub fn query_audit_log(&self, query: &AuditQuery) -> Vec<&AuditEvent> {
        self.audit_log.iter()
            .filter(|e| {
                // Filter by tenant
                if let Some(tenant_id) = query.tenant_id {
                    if e.tenant_id != tenant_id {
                        return false;
                    }
                }

                // Filter by category
                if let Some(ref categories) = query.categories {
                    if !categories.contains(&e.category) {
                        return false;
                    }
                }

                // Filter by date range
                if let Some(start) = query.start_date {
                    if e.timestamp < start {
                        return false;
                    }
                }
                if let Some(end) = query.end_date {
                    if e.timestamp > end {
                        return false;
                    }
                }

                // Filter by actor
                if let Some(user_id) = query.user_id {
                    if e.actor.user_id != Some(user_id) {
                        return false;
                    }
                }

                // Filter by success
                if let Some(success) = query.success_only {
                    if e.success != success {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Enable compliance framework for tenant
    pub fn enable_framework(&mut self, tenant_id: Uuid, framework: ComplianceFramework) {
        let status = self.compliance_statuses.entry(tenant_id).or_insert_with(|| {
            ComplianceStatus {
                tenant_id,
                enabled_frameworks: vec![],
                control_statuses: HashMap::new(),
                compliance_score: 0,
                last_assessment: None,
                next_assessment_due: None,
                issues: vec![],
                certifications: vec![],
            }
        });

        if !status.enabled_frameworks.contains(&framework) {
            status.enabled_frameworks.push(framework);

            // Initialize control statuses
            for control in framework.required_controls() {
                status.control_statuses.entry(control).or_insert(ControlStatus {
                    control,
                    status: ControlStatusType::NotImplemented,
                    evidence_count: 0,
                    last_verified: None,
                    notes: None,
                });
            }
        }
    }

    /// Get compliance status for tenant
    pub fn get_compliance_status(&self, tenant_id: Uuid) -> Option<&ComplianceStatus> {
        self.compliance_statuses.get(&tenant_id)
    }

    /// Update control status
    pub fn update_control_status(
        &mut self,
        tenant_id: Uuid,
        control: ComplianceControl,
        status: ControlStatusType,
    ) -> bool {
        if let Some(compliance) = self.compliance_statuses.get_mut(&tenant_id) {
            if let Some(control_status) = compliance.control_statuses.get_mut(&control) {
                control_status.status = status;
                control_status.last_verified = Some(Utc::now());
                
                // Recalculate compliance score
                self.recalculate_score(tenant_id);
                
                return true;
            }
        }
        false
    }

    /// Recalculate compliance score
    fn recalculate_score(&mut self, tenant_id: Uuid) {
        if let Some(status) = self.compliance_statuses.get_mut(&tenant_id) {
            let total = status.control_statuses.len();
            if total == 0 {
                status.compliance_score = 0;
                return;
            }

            let compliant = status.control_statuses.values()
                .filter(|s| matches!(s.status, ControlStatusType::Implemented | ControlStatusType::Verified))
                .count();

            status.compliance_score = ((compliant as f64 / total as f64) * 100.0) as u8;
        }
    }

    /// Create retention policy
    pub fn create_retention_policy(&mut self, policy: RetentionPolicy) -> Uuid {
        let id = policy.id;
        self.retention_policies.insert(id, policy);
        id
    }

    /// Get data requiring deletion
    pub fn get_data_for_deletion(&self, tenant_id: Uuid) -> Vec<DataDeletionTask> {
        let mut tasks = vec![];

        for policy in self.retention_policies.values() {
            if !policy.active || policy.legal_hold_exempt {
                continue;
            }

            let cutoff_date = Utc::now() - Duration::days(policy.retention_days as i64);

            tasks.push(DataDeletionTask {
                policy_id: policy.id,
                policy_name: policy.name.clone(),
                data_types: policy.data_types.clone(),
                cutoff_date,
                action: policy.action,
                classification: policy.classification,
            });
        }

        tasks
    }

    /// Generate compliance report
    pub fn generate_report(&self, tenant_id: Uuid, framework: ComplianceFramework) -> Option<ComplianceReport> {
        let status = self.compliance_statuses.get(&tenant_id)?;

        if !status.enabled_frameworks.contains(&framework) {
            return None;
        }

        let required_controls = framework.required_controls();
        let control_details: Vec<_> = required_controls.iter()
            .map(|c| {
                let status_info = status.control_statuses.get(c);
                ControlReportItem {
                    control: *c,
                    status: status_info.map(|s| s.status).unwrap_or(ControlStatusType::NotImplemented),
                    evidence_count: status_info.map(|s| s.evidence_count).unwrap_or(0),
                    last_verified: status_info.and_then(|s| s.last_verified),
                }
            })
            .collect();

        let compliant_count = control_details.iter()
            .filter(|c| matches!(c.status, ControlStatusType::Implemented | ControlStatusType::Verified))
            .count();

        Some(ComplianceReport {
            tenant_id,
            framework,
            generated_at: Utc::now(),
            compliance_score: status.compliance_score,
            total_controls: required_controls.len(),
            compliant_controls: compliant_count,
            control_details,
            open_issues: status.issues.iter()
                .filter(|i| i.framework == framework && i.status == IssueStatus::Open)
                .count(),
            certifications: status.certifications.iter()
                .filter(|c| c.framework == framework)
                .cloned()
                .collect(),
        })
    }
}

impl Default for ComplianceManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Supporting Types
// ============================================================================

/// Audit query parameters
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    pub tenant_id: Option<Uuid>,
    pub categories: Option<Vec<AuditCategory>>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub user_id: Option<Uuid>,
    pub success_only: Option<bool>,
    pub limit: Option<usize>,
}

/// Audit event builder
pub struct AuditEventBuilder {
    event: AuditEvent,
}

impl AuditEventBuilder {
    pub fn new(tenant_id: Uuid, category: AuditCategory, action: &str) -> Self {
        Self {
            event: AuditEvent {
                id: Uuid::new_v4(),
                tenant_id,
                actor: AuditActor {
                    actor_type: ActorType::System,
                    user_id: None,
                    email: None,
                    service_name: None,
                    api_key_id: None,
                },
                category,
                action: action.to_string(),
                resource_type: String::new(),
                resource_id: None,
                severity: AuditSeverity::Info,
                success: true,
                error_message: None,
                request: AuditRequest {
                    method: None,
                    path: None,
                    ip_address: "0.0.0.0".to_string(),
                    user_agent: None,
                    request_id: None,
                    session_id: None,
                },
                changes: None,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            },
        }
    }

    pub fn user(mut self, user_id: Uuid, email: &str) -> Self {
        self.event.actor.actor_type = ActorType::User;
        self.event.actor.user_id = Some(user_id);
        self.event.actor.email = Some(email.to_string());
        self
    }

    pub fn resource(mut self, resource_type: &str, resource_id: &str) -> Self {
        self.event.resource_type = resource_type.to_string();
        self.event.resource_id = Some(resource_id.to_string());
        self
    }

    pub fn severity(mut self, severity: AuditSeverity) -> Self {
        self.event.severity = severity;
        self
    }

    pub fn failed(mut self, error: &str) -> Self {
        self.event.success = false;
        self.event.error_message = Some(error.to_string());
        self
    }

    pub fn ip(mut self, ip: &str) -> Self {
        self.event.request.ip_address = ip.to_string();
        self
    }

    pub fn build(self) -> AuditEvent {
        self.event
    }
}

/// Data deletion task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDeletionTask {
    pub policy_id: Uuid,
    pub policy_name: String,
    pub data_types: Vec<String>,
    pub cutoff_date: DateTime<Utc>,
    pub action: RetentionAction,
    pub classification: DataClassification,
}

/// Control report item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlReportItem {
    pub control: ComplianceControl,
    pub status: ControlStatusType,
    pub evidence_count: usize,
    pub last_verified: Option<DateTime<Utc>>,
}

/// Compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub tenant_id: Uuid,
    pub framework: ComplianceFramework,
    pub generated_at: DateTime<Utc>,
    pub compliance_score: u8,
    pub total_controls: usize,
    pub compliant_controls: usize,
    pub control_details: Vec<ControlReportItem>,
    pub open_issues: usize,
    pub certifications: Vec<Certification>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_builder() {
        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let event = AuditEventBuilder::new(tenant_id, AuditCategory::Authentication, "login")
            .user(user_id, "user@example.com")
            .ip("192.168.1.1")
            .build();

        assert_eq!(event.tenant_id, tenant_id);
        assert_eq!(event.category, AuditCategory::Authentication);
        assert_eq!(event.actor.user_id, Some(user_id));
        assert!(event.success);
    }

    #[test]
    fn test_compliance_framework_controls() {
        let soc2_controls = ComplianceFramework::Soc2.required_controls();
        assert!(soc2_controls.contains(&ComplianceControl::AccessControl));
        assert!(soc2_controls.contains(&ComplianceControl::ChangeManagement));

        let hipaa_controls = ComplianceFramework::Hipaa.required_controls();
        assert!(hipaa_controls.contains(&ComplianceControl::DataEncryption));
    }

    #[test]
    fn test_data_classification() {
        assert!(!DataClassification::Public.requires_encryption());
        assert!(DataClassification::Phi.requires_encryption());
        assert!(DataClassification::Pii.requires_encryption());
    }

    #[test]
    fn test_enable_framework() {
        let mut manager = ComplianceManager::new();
        let tenant_id = Uuid::new_v4();

        manager.enable_framework(tenant_id, ComplianceFramework::Soc2);

        let status = manager.get_compliance_status(tenant_id).unwrap();
        assert!(status.enabled_frameworks.contains(&ComplianceFramework::Soc2));
        assert!(!status.control_statuses.is_empty());
    }
}
