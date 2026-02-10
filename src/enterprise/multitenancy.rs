// Copyright (c) 2024-2027 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Multi-tenant architecture
//!
//! Supports multiple organizations sharing the same infrastructure with
//! complete data isolation and tenant-specific configurations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// Tenant Types
// ============================================================================

/// Tenant (organization) in multi-tenant system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    /// Unique tenant ID
    pub id: Uuid,
    /// Tenant slug (URL-safe identifier)
    pub slug: String,
    /// Organization name
    pub name: String,
    /// Organization domain
    pub domain: Option<String>,
    /// Tenant status
    pub status: TenantStatus,
    /// Subscription tier
    pub tier: SubscriptionTier,
    /// Settings
    pub settings: TenantSettings,
    /// Limits
    pub limits: TenantLimits,
    /// Usage statistics
    pub usage: TenantUsage,
    /// Custom branding (white-label reference)
    pub branding_id: Option<Uuid>,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Updated at
    pub updated_at: DateTime<Utc>,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Tenant status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantStatus {
    /// Active and operational
    Active,
    /// Trial period
    Trial,
    /// Suspended (payment issue)
    Suspended,
    /// Deactivated by admin
    Deactivated,
    /// Pending setup
    Pending,
    /// Archived (soft deleted)
    Archived,
}

/// Subscription tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionTier {
    /// Free tier
    Free,
    /// Starter/Basic
    Starter,
    /// Professional
    Professional,
    /// Enterprise
    Enterprise,
    /// Custom agreement
    Custom,
}

impl SubscriptionTier {
    /// Get default limits for tier
    pub fn default_limits(&self) -> TenantLimits {
        match self {
            SubscriptionTier::Free => TenantLimits {
                max_users: 5,
                max_workspaces: 3,
                max_sessions: 1000,
                max_storage_gb: 1.0,
                max_api_calls_per_day: 1000,
                retention_days: 30,
                features: vec![
                    Feature::BasicHarvest,
                    Feature::SessionView,
                ],
            },
            SubscriptionTier::Starter => TenantLimits {
                max_users: 25,
                max_workspaces: 10,
                max_sessions: 10000,
                max_storage_gb: 10.0,
                max_api_calls_per_day: 10000,
                retention_days: 90,
                features: vec![
                    Feature::BasicHarvest,
                    Feature::SessionView,
                    Feature::Export,
                    Feature::Search,
                    Feature::Tags,
                ],
            },
            SubscriptionTier::Professional => TenantLimits {
                max_users: 100,
                max_workspaces: 50,
                max_sessions: 100000,
                max_storage_gb: 100.0,
                max_api_calls_per_day: 100000,
                retention_days: 365,
                features: vec![
                    Feature::BasicHarvest,
                    Feature::SessionView,
                    Feature::Export,
                    Feature::Search,
                    Feature::Tags,
                    Feature::Teams,
                    Feature::Analytics,
                    Feature::Api,
                    Feature::Sync,
                ],
            },
            SubscriptionTier::Enterprise | SubscriptionTier::Custom => TenantLimits {
                max_users: 0, // Unlimited
                max_workspaces: 0,
                max_sessions: 0,
                max_storage_gb: 0.0,
                max_api_calls_per_day: 0,
                retention_days: 0, // Custom
                features: vec![
                    Feature::BasicHarvest,
                    Feature::SessionView,
                    Feature::Export,
                    Feature::Search,
                    Feature::Tags,
                    Feature::Teams,
                    Feature::Analytics,
                    Feature::Api,
                    Feature::Sync,
                    Feature::Sso,
                    Feature::AuditLog,
                    Feature::Compliance,
                    Feature::WhiteLabel,
                    Feature::CustomIntegrations,
                    Feature::PrioritySupport,
                ],
            },
        }
    }
}

/// Feature flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    BasicHarvest,
    SessionView,
    Export,
    Search,
    Tags,
    Teams,
    Analytics,
    Api,
    Sync,
    Sso,
    AuditLog,
    Compliance,
    WhiteLabel,
    CustomIntegrations,
    PrioritySupport,
}

/// Tenant limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantLimits {
    /// Maximum users (0 = unlimited)
    pub max_users: usize,
    /// Maximum workspaces
    pub max_workspaces: usize,
    /// Maximum sessions
    pub max_sessions: usize,
    /// Maximum storage in GB
    pub max_storage_gb: f64,
    /// Maximum API calls per day
    pub max_api_calls_per_day: usize,
    /// Data retention in days (0 = unlimited)
    pub retention_days: usize,
    /// Enabled features
    pub features: Vec<Feature>,
}

/// Tenant settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantSettings {
    /// Default timezone
    pub timezone: String,
    /// Default language
    pub language: String,
    /// Allowed authentication methods
    pub auth_methods: Vec<AuthMethod>,
    /// SSO configuration
    pub sso_config: Option<SsoConfig>,
    /// Data residency region
    pub data_region: String,
    /// Encryption key ID (for tenant-specific encryption)
    pub encryption_key_id: Option<String>,
    /// Allowed IP ranges (CIDR)
    pub ip_allowlist: Vec<String>,
    /// Session timeout in minutes
    pub session_timeout_minutes: u32,
    /// Require MFA
    pub require_mfa: bool,
}

impl Default for TenantSettings {
    fn default() -> Self {
        Self {
            timezone: "UTC".to_string(),
            language: "en".to_string(),
            auth_methods: vec![AuthMethod::Password, AuthMethod::OAuth],
            sso_config: None,
            data_region: "us-east-1".to_string(),
            encryption_key_id: None,
            ip_allowlist: vec![],
            session_timeout_minutes: 60,
            require_mfa: false,
        }
    }
}

/// Authentication method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    Password,
    OAuth,
    Saml,
    Oidc,
    ApiKey,
}

/// SSO configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoConfig {
    /// SSO provider type
    pub provider: SsoProvider,
    /// Identity provider URL
    pub idp_url: String,
    /// Client ID
    pub client_id: String,
    /// Client secret (encrypted)
    pub client_secret_encrypted: String,
    /// SAML metadata URL
    pub metadata_url: Option<String>,
    /// Certificate for SAML
    pub certificate: Option<String>,
    /// Attribute mappings
    pub attribute_mappings: HashMap<String, String>,
    /// Auto-provision users
    pub auto_provision: bool,
    /// Default role for new users
    pub default_role: String,
}

/// SSO provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SsoProvider {
    Okta,
    AzureAd,
    Google,
    OneLogin,
    Auth0,
    Custom,
}

/// Tenant usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantUsage {
    /// Current user count
    pub user_count: usize,
    /// Current workspace count
    pub workspace_count: usize,
    /// Current session count
    pub session_count: usize,
    /// Storage used in GB
    pub storage_used_gb: f64,
    /// API calls today
    pub api_calls_today: usize,
    /// API calls this month
    pub api_calls_month: usize,
    /// Last updated
    pub updated_at: DateTime<Utc>,
}

impl Default for TenantUsage {
    fn default() -> Self {
        Self {
            user_count: 0,
            workspace_count: 0,
            session_count: 0,
            storage_used_gb: 0.0,
            api_calls_today: 0,
            api_calls_month: 0,
            updated_at: Utc::now(),
        }
    }
}

// ============================================================================
// Tenant User
// ============================================================================

/// User within a tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantUser {
    /// User ID (global)
    pub user_id: Uuid,
    /// Tenant ID
    pub tenant_id: Uuid,
    /// Email
    pub email: String,
    /// Display name
    pub display_name: String,
    /// Role in tenant
    pub role: TenantRole,
    /// Status
    pub status: UserStatus,
    /// Joined at
    pub joined_at: DateTime<Utc>,
    /// Last active
    pub last_active: Option<DateTime<Utc>>,
    /// SSO subject ID (from IdP)
    pub sso_subject_id: Option<String>,
    /// MFA enabled
    pub mfa_enabled: bool,
}

/// Tenant role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantRole {
    /// Full control
    Owner,
    /// Administrative access
    Admin,
    /// Standard user
    Member,
    /// Read-only access
    Viewer,
    /// Billing only
    Billing,
}

/// User status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    Active,
    Invited,
    Suspended,
    Deactivated,
}

// ============================================================================
// Tenant Manager
// ============================================================================

/// Manages multi-tenant operations
pub struct TenantManager {
    /// All tenants
    tenants: HashMap<Uuid, Tenant>,
    /// Tenant by slug lookup
    slug_index: HashMap<String, Uuid>,
    /// Tenant by domain lookup
    domain_index: HashMap<String, Uuid>,
    /// Users by tenant
    tenant_users: HashMap<Uuid, Vec<TenantUser>>,
}

impl TenantManager {
    /// Create a new tenant manager
    pub fn new() -> Self {
        Self {
            tenants: HashMap::new(),
            slug_index: HashMap::new(),
            domain_index: HashMap::new(),
            tenant_users: HashMap::new(),
        }
    }

    /// Create a new tenant
    pub fn create_tenant(
        &mut self,
        name: &str,
        slug: &str,
        tier: SubscriptionTier,
        domain: Option<&str>,
    ) -> Result<Tenant, TenantError> {
        // Validate slug uniqueness
        if self.slug_index.contains_key(slug) {
            return Err(TenantError::SlugTaken(slug.to_string()));
        }

        // Validate domain uniqueness
        if let Some(d) = domain {
            if self.domain_index.contains_key(d) {
                return Err(TenantError::DomainTaken(d.to_string()));
            }
        }

        let id = Uuid::new_v4();
        let tenant = Tenant {
            id,
            slug: slug.to_string(),
            name: name.to_string(),
            domain: domain.map(String::from),
            status: TenantStatus::Active,
            tier,
            settings: TenantSettings::default(),
            limits: tier.default_limits(),
            usage: TenantUsage::default(),
            branding_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: HashMap::new(),
        };

        // Index
        self.slug_index.insert(slug.to_string(), id);
        if let Some(d) = domain {
            self.domain_index.insert(d.to_string(), id);
        }
        self.tenant_users.insert(id, vec![]);
        self.tenants.insert(id, tenant.clone());

        Ok(tenant)
    }

    /// Get tenant by ID
    pub fn get_tenant(&self, id: Uuid) -> Option<&Tenant> {
        self.tenants.get(&id)
    }

    /// Get tenant by slug
    pub fn get_tenant_by_slug(&self, slug: &str) -> Option<&Tenant> {
        self.slug_index.get(slug).and_then(|id| self.tenants.get(id))
    }

    /// Get tenant by domain
    pub fn get_tenant_by_domain(&self, domain: &str) -> Option<&Tenant> {
        self.domain_index.get(domain).and_then(|id| self.tenants.get(id))
    }

    /// Update tenant status
    pub fn update_status(&mut self, tenant_id: Uuid, status: TenantStatus) -> bool {
        if let Some(tenant) = self.tenants.get_mut(&tenant_id) {
            tenant.status = status;
            tenant.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Update tenant tier
    pub fn update_tier(&mut self, tenant_id: Uuid, tier: SubscriptionTier) -> bool {
        if let Some(tenant) = self.tenants.get_mut(&tenant_id) {
            tenant.tier = tier;
            tenant.limits = tier.default_limits();
            tenant.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Add user to tenant
    pub fn add_user(
        &mut self,
        tenant_id: Uuid,
        user_id: Uuid,
        email: &str,
        display_name: &str,
        role: TenantRole,
    ) -> Result<TenantUser, TenantError> {
        let tenant = self.tenants.get(&tenant_id)
            .ok_or(TenantError::NotFound(tenant_id))?;

        // Check user limit
        let current_users = self.tenant_users.get(&tenant_id).map(|u| u.len()).unwrap_or(0);
        if tenant.limits.max_users > 0 && current_users >= tenant.limits.max_users {
            return Err(TenantError::LimitExceeded("users".to_string()));
        }

        let user = TenantUser {
            user_id,
            tenant_id,
            email: email.to_string(),
            display_name: display_name.to_string(),
            role,
            status: UserStatus::Active,
            joined_at: Utc::now(),
            last_active: None,
            sso_subject_id: None,
            mfa_enabled: false,
        };

        self.tenant_users.entry(tenant_id).or_default().push(user.clone());

        // Update usage
        if let Some(tenant) = self.tenants.get_mut(&tenant_id) {
            tenant.usage.user_count += 1;
            tenant.usage.updated_at = Utc::now();
        }

        Ok(user)
    }

    /// Get tenant users
    pub fn get_users(&self, tenant_id: Uuid) -> Vec<&TenantUser> {
        self.tenant_users.get(&tenant_id)
            .map(|users| users.iter().collect())
            .unwrap_or_default()
    }

    /// Check if tenant has feature
    pub fn has_feature(&self, tenant_id: Uuid, feature: Feature) -> bool {
        self.tenants.get(&tenant_id)
            .map(|t| t.limits.features.contains(&feature))
            .unwrap_or(false)
    }

    /// Check if tenant is within limits
    pub fn check_limit(&self, tenant_id: Uuid, limit_type: &str, value: usize) -> bool {
        let tenant = match self.tenants.get(&tenant_id) {
            Some(t) => t,
            None => return false,
        };

        match limit_type {
            "users" => tenant.limits.max_users == 0 || value < tenant.limits.max_users,
            "workspaces" => tenant.limits.max_workspaces == 0 || value < tenant.limits.max_workspaces,
            "sessions" => tenant.limits.max_sessions == 0 || value < tenant.limits.max_sessions,
            "api_calls" => tenant.limits.max_api_calls_per_day == 0 || value < tenant.limits.max_api_calls_per_day,
            _ => true,
        }
    }

    /// Record API call
    pub fn record_api_call(&mut self, tenant_id: Uuid) {
        if let Some(tenant) = self.tenants.get_mut(&tenant_id) {
            tenant.usage.api_calls_today += 1;
            tenant.usage.api_calls_month += 1;
            tenant.usage.updated_at = Utc::now();
        }
    }

    /// Get all tenants
    pub fn list_tenants(&self) -> Vec<&Tenant> {
        self.tenants.values().collect()
    }
}

impl Default for TenantManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Tenant error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TenantError {
    NotFound(Uuid),
    SlugTaken(String),
    DomainTaken(String),
    LimitExceeded(String),
    FeatureNotAvailable(Feature),
    InvalidStatus(TenantStatus),
}

impl std::fmt::Display for TenantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantError::NotFound(id) => write!(f, "Tenant not found: {}", id),
            TenantError::SlugTaken(slug) => write!(f, "Slug already taken: {}", slug),
            TenantError::DomainTaken(domain) => write!(f, "Domain already taken: {}", domain),
            TenantError::LimitExceeded(limit) => write!(f, "Limit exceeded: {}", limit),
            TenantError::FeatureNotAvailable(feature) => write!(f, "Feature not available: {:?}", feature),
            TenantError::InvalidStatus(status) => write!(f, "Invalid status: {:?}", status),
        }
    }
}

impl std::error::Error for TenantError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tenant() {
        let mut manager = TenantManager::new();
        
        let tenant = manager.create_tenant(
            "Acme Corp",
            "acme",
            SubscriptionTier::Professional,
            Some("acme.com"),
        ).unwrap();

        assert_eq!(tenant.name, "Acme Corp");
        assert_eq!(tenant.slug, "acme");
        assert_eq!(tenant.tier, SubscriptionTier::Professional);
    }

    #[test]
    fn test_slug_uniqueness() {
        let mut manager = TenantManager::new();
        
        manager.create_tenant("First", "unique", SubscriptionTier::Free, None).unwrap();
        
        let result = manager.create_tenant("Second", "unique", SubscriptionTier::Free, None);
        assert!(matches!(result, Err(TenantError::SlugTaken(_))));
    }

    #[test]
    fn test_tier_limits() {
        let free_limits = SubscriptionTier::Free.default_limits();
        let enterprise_limits = SubscriptionTier::Enterprise.default_limits();

        assert_eq!(free_limits.max_users, 5);
        assert_eq!(enterprise_limits.max_users, 0); // Unlimited
        assert!(enterprise_limits.features.contains(&Feature::Sso));
        assert!(!free_limits.features.contains(&Feature::Sso));
    }

    #[test]
    fn test_add_user() {
        let mut manager = TenantManager::new();
        
        let tenant = manager.create_tenant("Test", "test", SubscriptionTier::Starter, None).unwrap();
        
        let user = manager.add_user(
            tenant.id,
            Uuid::new_v4(),
            "user@test.com",
            "Test User",
            TenantRole::Member,
        ).unwrap();

        assert_eq!(user.email, "user@test.com");
        assert_eq!(user.role, TenantRole::Member);
    }
}
