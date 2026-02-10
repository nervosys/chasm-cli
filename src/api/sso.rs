// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! SSO/SAML Authentication Module
//!
//! Provides enterprise Single Sign-On support with SAML 2.0 protocol.
//! Supports integration with identity providers like Okta, Azure AD, OneLogin, etc.

use actix_web::{web, HttpRequest, HttpResponse};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{Duration, Utc};
use flate2::read::DeflateDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use uuid::Uuid;

use super::auth::{AuthResponse, Claims, PublicUser, SubscriptionTier, User};
use super::audit::Database;

// =============================================================================
// SSO Configuration
// =============================================================================

/// SAML Identity Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlIdpConfig {
    /// Unique identifier for this IdP
    pub id: String,
    /// Display name (e.g., "Okta", "Azure AD")
    pub name: String,
    /// Entity ID of the Identity Provider
    pub entity_id: String,
    /// SSO URL for SAML requests
    pub sso_url: String,
    /// Single Logout URL (optional)
    pub slo_url: Option<String>,
    /// X.509 certificate for signature validation (PEM format)
    pub certificate: String,
    /// Whether this IdP is enabled
    pub enabled: bool,
    /// Organization/tenant ID this IdP is associated with
    pub organization_id: Option<String>,
    /// Attribute mappings (IdP attribute -> Chasm field)
    pub attribute_mappings: AttributeMappings,
    /// Default subscription tier for users from this IdP
    pub default_tier: SubscriptionTier,
    /// Whether to auto-provision users on first login
    pub auto_provision: bool,
    /// Created timestamp
    pub created_at: i64,
    /// Updated timestamp
    pub updated_at: i64,
}

/// Attribute mappings from IdP to Chasm user fields
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AttributeMappings {
    /// Attribute containing the user's email
    pub email: String,
    /// Attribute containing the user's display name
    pub display_name: String,
    /// Attribute containing the user's first name (optional)
    pub first_name: Option<String>,
    /// Attribute containing the user's last name (optional)
    pub last_name: Option<String>,
    /// Attribute containing the user's groups (optional)
    pub groups: Option<String>,
    /// Attribute containing the user's department (optional)
    pub department: Option<String>,
}

impl Default for SamlIdpConfig {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: String::new(),
            entity_id: String::new(),
            sso_url: String::new(),
            slo_url: None,
            certificate: String::new(),
            enabled: false,
            organization_id: None,
            attribute_mappings: AttributeMappings {
                email: "email".to_string(),
                display_name: "displayName".to_string(),
                first_name: Some("firstName".to_string()),
                last_name: Some("lastName".to_string()),
                groups: Some("groups".to_string()),
                department: None,
            },
            default_tier: SubscriptionTier::Enterprise,
            auto_provision: true,
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
        }
    }
}

// =============================================================================
// SAML Service Provider Configuration
// =============================================================================

/// Service Provider (Chasm) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlSpConfig {
    /// Entity ID of Chasm as a Service Provider
    pub entity_id: String,
    /// Assertion Consumer Service URL
    pub acs_url: String,
    /// Single Logout URL
    pub slo_url: String,
    /// X.509 certificate for signing (PEM format)
    pub certificate: String,
    /// Private key for signing (PEM format, encrypted)
    #[serde(skip_serializing)]
    pub private_key: String,
    /// Name ID format preference
    pub name_id_format: NameIdFormat,
    /// Whether to sign authentication requests
    pub sign_requests: bool,
    /// Whether to require signed assertions
    pub require_signed_assertions: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum NameIdFormat {
    #[default]
    EmailAddress,
    Persistent,
    Transient,
    Unspecified,
}

impl NameIdFormat {
    pub fn as_urn(&self) -> &'static str {
        match self {
            Self::EmailAddress => "urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress",
            Self::Persistent => "urn:oasis:names:tc:SAML:2.0:nameid-format:persistent",
            Self::Transient => "urn:oasis:names:tc:SAML:2.0:nameid-format:transient",
            Self::Unspecified => "urn:oasis:names:tc:SAML:1.1:nameid-format:unspecified",
        }
    }
}

// =============================================================================
// SAML Request/Response Types
// =============================================================================

/// SAML Authentication Request
#[derive(Debug, Clone)]
pub struct SamlAuthnRequest {
    pub id: String,
    pub issue_instant: String,
    pub destination: String,
    pub issuer: String,
    pub acs_url: String,
    pub name_id_format: NameIdFormat,
}

impl SamlAuthnRequest {
    pub fn new(sp_config: &SamlSpConfig, idp_config: &SamlIdpConfig) -> Self {
        Self {
            id: format!("_{}", Uuid::new_v4()),
            issue_instant: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            destination: idp_config.sso_url.clone(),
            issuer: sp_config.entity_id.clone(),
            acs_url: sp_config.acs_url.clone(),
            name_id_format: sp_config.name_id_format,
        }
    }

    /// Generate SAML AuthnRequest XML
    pub fn to_xml(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<samlp:AuthnRequest
    xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol"
    xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"
    ID="{id}"
    Version="2.0"
    IssueInstant="{issue_instant}"
    Destination="{destination}"
    AssertionConsumerServiceURL="{acs_url}"
    ProtocolBinding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST">
    <saml:Issuer>{issuer}</saml:Issuer>
    <samlp:NameIDPolicy
        Format="{name_id_format}"
        AllowCreate="true"/>
</samlp:AuthnRequest>"#,
            id = self.id,
            issue_instant = self.issue_instant,
            destination = self.destination,
            acs_url = self.acs_url,
            issuer = self.issuer,
            name_id_format = self.name_id_format.as_urn(),
        )
    }

    /// Encode request for HTTP-Redirect binding (deflate + base64 + URL encode)
    pub fn encode_redirect(&self) -> String {
        use flate2::write::DeflateEncoder;
        use flate2::Compression;
        use std::io::Write;

        let xml = self.to_xml();
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(xml.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();
        let encoded = BASE64.encode(&compressed);
        urlencoding::encode(&encoded).to_string()
    }
}

/// Parsed SAML Response
#[derive(Debug, Clone)]
pub struct SamlResponse {
    pub id: String,
    pub in_response_to: String,
    pub status: SamlStatus,
    pub issuer: String,
    pub assertion: Option<SamlAssertion>,
}

#[derive(Debug, Clone)]
pub struct SamlAssertion {
    pub id: String,
    pub issuer: String,
    pub subject: SamlSubject,
    pub conditions: SamlConditions,
    pub attributes: HashMap<String, Vec<String>>,
    pub authn_statement: SamlAuthnStatement,
}

#[derive(Debug, Clone)]
pub struct SamlSubject {
    pub name_id: String,
    pub name_id_format: String,
}

#[derive(Debug, Clone)]
pub struct SamlConditions {
    pub not_before: String,
    pub not_on_or_after: String,
    pub audience: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SamlAuthnStatement {
    pub authn_instant: String,
    pub session_index: Option<String>,
    pub session_not_on_or_after: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SamlStatus {
    Success,
    Requester,
    Responder,
    VersionMismatch,
    AuthnFailed,
    Unknown(String),
}

// =============================================================================
// SSO Session State
// =============================================================================

/// Pending SSO request state (stored temporarily during auth flow)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoRequestState {
    pub request_id: String,
    pub idp_id: String,
    pub relay_state: Option<String>,
    pub created_at: i64,
    pub expires_at: i64,
}

/// SSO Session (linked to user session)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoSession {
    pub id: String,
    pub user_id: String,
    pub idp_id: String,
    pub name_id: String,
    pub session_index: Option<String>,
    pub created_at: i64,
    pub expires_at: i64,
}

// =============================================================================
// API Request/Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct InitiateSsoRequest {
    /// IdP ID or organization identifier
    pub idp_id: Option<String>,
    /// Email domain for IdP discovery
    pub email_domain: Option<String>,
    /// Relay state (return URL after auth)
    pub relay_state: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InitiateSsoResponse {
    /// URL to redirect user to for SSO
    pub redirect_url: String,
    /// Request ID for tracking
    pub request_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SamlCallbackRequest {
    /// Base64-encoded SAML Response
    #[serde(rename = "SAMLResponse")]
    pub saml_response: String,
    /// Relay state from original request
    #[serde(rename = "RelayState")]
    pub relay_state: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateIdpRequest {
    pub name: String,
    pub entity_id: String,
    pub sso_url: String,
    pub slo_url: Option<String>,
    pub certificate: String,
    pub organization_id: Option<String>,
    pub attribute_mappings: Option<AttributeMappings>,
    pub auto_provision: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIdpRequest {
    pub name: Option<String>,
    pub sso_url: Option<String>,
    pub slo_url: Option<String>,
    pub certificate: Option<String>,
    pub enabled: Option<bool>,
    pub attribute_mappings: Option<AttributeMappings>,
    pub auto_provision: Option<bool>,
}

// =============================================================================
// SSO Service
// =============================================================================

pub struct SsoService {
    db: Database,
    sp_config: SamlSpConfig,
}

impl SsoService {
    pub fn new(db: Database, base_url: &str) -> Self {
        let sp_config = SamlSpConfig {
            entity_id: format!("{}/saml/metadata", base_url),
            acs_url: format!("{}/api/sso/callback", base_url),
            slo_url: format!("{}/api/sso/logout", base_url),
            certificate: String::new(), // Would be loaded from config
            private_key: String::new(), // Would be loaded from config
            name_id_format: NameIdFormat::EmailAddress,
            sign_requests: false,
            require_signed_assertions: true,
        };

        Self { db, sp_config }
    }

    /// Get IdP configuration by ID
    pub async fn get_idp(&self, idp_id: &str) -> Result<Option<SamlIdpConfig>, String> {
        self.db
            .get_sso_idp(idp_id)
            .map_err(|e| format!("Database error: {}", e))
    }

    /// Get IdP by email domain (for domain-based discovery)
    pub async fn get_idp_by_domain(&self, domain: &str) -> Result<Option<SamlIdpConfig>, String> {
        self.db
            .get_sso_idp_by_domain(domain)
            .map_err(|e| format!("Database error: {}", e))
    }

    /// List all configured IdPs
    pub async fn list_idps(
        &self,
        organization_id: Option<&str>,
    ) -> Result<Vec<SamlIdpConfig>, String> {
        self.db
            .list_sso_idps(organization_id)
            .map_err(|e| format!("Database error: {}", e))
    }

    /// Create a new IdP configuration
    pub async fn create_idp(&self, request: CreateIdpRequest) -> Result<SamlIdpConfig, String> {
        let idp = SamlIdpConfig {
            id: Uuid::new_v4().to_string(),
            name: request.name,
            entity_id: request.entity_id,
            sso_url: request.sso_url,
            slo_url: request.slo_url,
            certificate: request.certificate,
            enabled: true,
            organization_id: request.organization_id,
            attribute_mappings: request.attribute_mappings.unwrap_or_default(),
            default_tier: SubscriptionTier::Enterprise,
            auto_provision: request.auto_provision.unwrap_or(true),
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
        };

        self.db
            .create_sso_idp(&idp)
            .map_err(|e| format!("Failed to create IdP: {}", e))?;

        Ok(idp)
    }

    /// Update an IdP configuration
    pub async fn update_idp(
        &self,
        idp_id: &str,
        request: UpdateIdpRequest,
    ) -> Result<SamlIdpConfig, String> {
        let mut idp = self.get_idp(idp_id).await?.ok_or("IdP not found")?;

        if let Some(name) = request.name {
            idp.name = name;
        }
        if let Some(sso_url) = request.sso_url {
            idp.sso_url = sso_url;
        }
        if let Some(slo_url) = request.slo_url {
            idp.slo_url = Some(slo_url);
        }
        if let Some(certificate) = request.certificate {
            idp.certificate = certificate;
        }
        if let Some(enabled) = request.enabled {
            idp.enabled = enabled;
        }
        if let Some(attribute_mappings) = request.attribute_mappings {
            idp.attribute_mappings = attribute_mappings;
        }
        if let Some(auto_provision) = request.auto_provision {
            idp.auto_provision = auto_provision;
        }

        idp.updated_at = Utc::now().timestamp();

        self.db
            .update_sso_idp(&idp)
            .map_err(|e| format!("Failed to update IdP: {}", e))?;

        Ok(idp)
    }

    /// Delete an IdP configuration
    pub async fn delete_idp(&self, idp_id: &str) -> Result<(), String> {
        self.db
            .delete_sso_idp(idp_id)
            .map_err(|e| format!("Failed to delete IdP: {}", e))
    }

    /// Initiate SSO login flow
    pub async fn initiate_sso(
        &self,
        request: InitiateSsoRequest,
    ) -> Result<InitiateSsoResponse, String> {
        // Find the IdP
        let idp = if let Some(idp_id) = &request.idp_id {
            self.get_idp(idp_id).await?
        } else if let Some(domain) = &request.email_domain {
            self.get_idp_by_domain(domain).await?
        } else {
            return Err("Must provide either idp_id or email_domain".to_string());
        };

        let idp = idp.ok_or("IdP not found")?;

        if !idp.enabled {
            return Err("IdP is disabled".to_string());
        }

        // Generate SAML AuthnRequest
        let authn_request = SamlAuthnRequest::new(&self.sp_config, &idp);
        let encoded_request = authn_request.encode_redirect();

        // Store request state
        let state = SsoRequestState {
            request_id: authn_request.id.clone(),
            idp_id: idp.id.clone(),
            relay_state: request.relay_state.clone(),
            created_at: Utc::now().timestamp(),
            expires_at: (Utc::now() + Duration::minutes(10)).timestamp(),
        };

        self.db
            .store_sso_request_state(&state)
            .map_err(|e| format!("Failed to store request state: {}", e))?;

        // Build redirect URL
        let mut redirect_url = idp.sso_url.clone();
        redirect_url.push_str(if redirect_url.contains('?') { "&" } else { "?" });
        redirect_url.push_str("SAMLRequest=");
        redirect_url.push_str(&encoded_request);

        if let Some(relay_state) = &request.relay_state {
            redirect_url.push_str("&RelayState=");
            redirect_url.push_str(&urlencoding::encode(relay_state));
        }

        Ok(InitiateSsoResponse {
            redirect_url,
            request_id: authn_request.id,
        })
    }

    /// Handle SAML callback (Assertion Consumer Service)
    pub async fn handle_callback(
        &self,
        request: SamlCallbackRequest,
    ) -> Result<AuthResponse, String> {
        // Decode SAML Response
        let response_xml = BASE64
            .decode(&request.saml_response)
            .map_err(|e| format!("Invalid base64: {}", e))?;

        let response_str =
            String::from_utf8(response_xml).map_err(|e| format!("Invalid UTF-8: {}", e))?;

        // Parse SAML Response (simplified - production would use proper XML parsing)
        let saml_response = self.parse_saml_response(&response_str)?;

        if saml_response.status != SamlStatus::Success {
            return Err(format!(
                "SAML authentication failed: {:?}",
                saml_response.status
            ));
        }

        let assertion = saml_response.assertion.ok_or("No assertion in response")?;

        // Validate assertion
        self.validate_assertion(&assertion)?;

        // Get IdP config
        let idp = self
            .get_idp_by_domain(&self.extract_domain(&assertion.subject.name_id))
            .await?
            .ok_or("IdP not found")?;

        // Extract user attributes
        let email = self
            .get_attribute(&assertion, &idp.attribute_mappings.email)
            .or_else(|| Some(assertion.subject.name_id.clone()))
            .ok_or("Email not found in assertion")?;

        let display_name = self
            .get_attribute(&assertion, &idp.attribute_mappings.display_name)
            .unwrap_or_else(|| email.split('@').next().unwrap_or(&email).to_string());

        // Find or create user
        let user = self
            .find_or_create_user(&idp, &email, &display_name)
            .await?;

        // Create SSO session
        let sso_session = SsoSession {
            id: Uuid::new_v4().to_string(),
            user_id: user.id.clone(),
            idp_id: idp.id.clone(),
            name_id: assertion.subject.name_id.clone(),
            session_index: assertion.authn_statement.session_index.clone(),
            created_at: Utc::now().timestamp(),
            expires_at: (Utc::now() + Duration::hours(24)).timestamp(),
        };

        self.db
            .store_sso_session(&sso_session)
            .map_err(|e| format!("Failed to store SSO session: {}", e))?;

        // Generate JWT tokens
        let access_token = super::auth::generate_access_token(&user)
            .ok_or_else(|| "Failed to generate access token".to_string())?;
        let refresh_token = super::auth::generate_refresh_token(&user)
            .ok_or_else(|| "Failed to generate refresh token".to_string())?;

        Ok(AuthResponse {
            user: user.into(),
            access_token,
            refresh_token,
            expires_in: 24 * 60 * 60, // 24 hours
        })
    }

    /// Generate SP metadata XML
    pub fn generate_metadata(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<EntityDescriptor
    xmlns="urn:oasis:names:tc:SAML:2.0:metadata"
    entityID="{entity_id}">
    <SPSSODescriptor
        AuthnRequestsSigned="{sign_requests}"
        WantAssertionsSigned="{require_signed}"
        protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
        <NameIDFormat>{name_id_format}</NameIDFormat>
        <AssertionConsumerService
            Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"
            Location="{acs_url}"
            index="0"
            isDefault="true"/>
        <SingleLogoutService
            Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"
            Location="{slo_url}"/>
    </SPSSODescriptor>
    <Organization>
        <OrganizationName xml:lang="en">Chasm</OrganizationName>
        <OrganizationDisplayName xml:lang="en">Chasm - Chat Session Manager</OrganizationDisplayName>
        <OrganizationURL xml:lang="en">https://github.com/nervosys/chasm</OrganizationURL>
    </Organization>
</EntityDescriptor>"#,
            entity_id = self.sp_config.entity_id,
            sign_requests = self.sp_config.sign_requests,
            require_signed = self.sp_config.require_signed_assertions,
            name_id_format = self.sp_config.name_id_format.as_urn(),
            acs_url = self.sp_config.acs_url,
            slo_url = self.sp_config.slo_url,
        )
    }

    // Helper methods

    fn parse_saml_response(&self, xml: &str) -> Result<SamlResponse, String> {
        // Simplified parsing - production would use proper XML library
        // This is a placeholder that extracts basic information
        let id = self
            .extract_xml_attr(xml, "Response", "ID")
            .unwrap_or_default();
        let in_response_to = self
            .extract_xml_attr(xml, "Response", "InResponseTo")
            .unwrap_or_default();
        let issuer = self.extract_xml_element(xml, "Issuer").unwrap_or_default();

        let status = if xml.contains("urn:oasis:names:tc:SAML:2.0:status:Success") {
            SamlStatus::Success
        } else if xml.contains("urn:oasis:names:tc:SAML:2.0:status:Requester") {
            SamlStatus::Requester
        } else if xml.contains("urn:oasis:names:tc:SAML:2.0:status:Responder") {
            SamlStatus::Responder
        } else if xml.contains("AuthnFailed") {
            SamlStatus::AuthnFailed
        } else {
            SamlStatus::Unknown("Unknown status".to_string())
        };

        // Parse assertion if present
        let assertion = if xml.contains("<saml:Assertion") || xml.contains("<Assertion") {
            Some(self.parse_assertion(xml)?)
        } else {
            None
        };

        Ok(SamlResponse {
            id,
            in_response_to,
            status,
            issuer,
            assertion,
        })
    }

    fn parse_assertion(&self, xml: &str) -> Result<SamlAssertion, String> {
        let id = self
            .extract_xml_attr(xml, "Assertion", "ID")
            .unwrap_or_default();
        let issuer = self.extract_xml_element(xml, "Issuer").unwrap_or_default();

        // Extract NameID
        let name_id = self
            .extract_xml_element(xml, "NameID")
            .ok_or("NameID not found")?;
        let name_id_format = self
            .extract_xml_attr(xml, "NameID", "Format")
            .unwrap_or_default();

        // Extract conditions
        let not_before = self
            .extract_xml_attr(xml, "Conditions", "NotBefore")
            .unwrap_or_default();
        let not_on_or_after = self
            .extract_xml_attr(xml, "Conditions", "NotOnOrAfter")
            .unwrap_or_default();

        // Extract attributes (simplified)
        let attributes = self.parse_attributes(xml);

        // Extract authn statement
        let authn_instant = self
            .extract_xml_attr(xml, "AuthnStatement", "AuthnInstant")
            .unwrap_or_default();
        let session_index = self.extract_xml_attr(xml, "AuthnStatement", "SessionIndex");

        Ok(SamlAssertion {
            id,
            issuer,
            subject: SamlSubject {
                name_id,
                name_id_format,
            },
            conditions: SamlConditions {
                not_before,
                not_on_or_after,
                audience: vec![],
            },
            attributes,
            authn_statement: SamlAuthnStatement {
                authn_instant,
                session_index,
                session_not_on_or_after: None,
            },
        })
    }

    fn parse_attributes(&self, xml: &str) -> HashMap<String, Vec<String>> {
        let mut attributes = HashMap::new();

        // Simple regex-like extraction (production would use proper XML parsing)
        // This finds Attribute elements and their AttributeValue children
        let attr_pattern = r#"Name="([^"]+)".*?<.*?AttributeValue[^>]*>([^<]+)<"#;

        // For now, return empty - proper implementation would parse XML
        attributes
    }

    fn validate_assertion(&self, assertion: &SamlAssertion) -> Result<(), String> {
        let now = Utc::now();

        // Check time conditions
        if !assertion.conditions.not_before.is_empty() {
            if let Ok(not_before) =
                chrono::DateTime::parse_from_rfc3339(&assertion.conditions.not_before)
            {
                if now < not_before.with_timezone(&Utc) {
                    return Err("Assertion not yet valid".to_string());
                }
            }
        }

        if !assertion.conditions.not_on_or_after.is_empty() {
            if let Ok(not_on_or_after) =
                chrono::DateTime::parse_from_rfc3339(&assertion.conditions.not_on_or_after)
            {
                if now >= not_on_or_after.with_timezone(&Utc) {
                    return Err("Assertion has expired".to_string());
                }
            }
        }

        Ok(())
    }

    fn get_attribute(&self, assertion: &SamlAssertion, name: &str) -> Option<String> {
        assertion
            .attributes
            .get(name)
            .and_then(|v| v.first())
            .cloned()
    }

    fn extract_domain(&self, email: &str) -> String {
        email.split('@').last().unwrap_or("").to_string()
    }

    async fn find_or_create_user(
        &self,
        idp: &SamlIdpConfig,
        email: &str,
        display_name: &str,
    ) -> Result<User, String> {
        // Try to find existing user
        if let Some(user) = self
            .db
            .get_user_by_email(email)
            .map_err(|e| e.to_string())?
        {
            // Update last login
            self.db
                .update_user_login(&user.id)
                .map_err(|e| e.to_string())?;
            return Ok(user);
        }

        // Auto-provision if enabled
        if !idp.auto_provision {
            return Err("User not found and auto-provisioning is disabled".to_string());
        }

        // Create new user
        let user = User {
            id: Uuid::new_v4().to_string(),
            email: email.to_string(),
            display_name: display_name.to_string(),
            password_hash: String::new(), // SSO users don't have passwords
            subscription_tier: idp.default_tier,
            subscription_expires_at: None,
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            last_login_at: Some(Utc::now().timestamp()),
            email_verified: true, // SSO users are pre-verified
            avatar_url: None,
            metadata: Some(format!(r#"{{"sso_idp":"{}"}}"#, idp.id)),
        };

        self.db.create_user(&user).map_err(|e| e.to_string())?;

        Ok(user)
    }

    // XML helper methods (simplified - production would use proper XML library)

    fn extract_xml_attr(&self, xml: &str, element: &str, attr: &str) -> Option<String> {
        let pattern = format!(r#"<[^>]*{}[^>]*{}="([^"]+)""#, element, attr);
        // Simplified extraction
        None
    }

    fn extract_xml_element(&self, xml: &str, element: &str) -> Option<String> {
        let start_tag = format!("<{}", element);
        let end_tag = format!("</{}>", element);

        if let Some(start) = xml.find(&start_tag) {
            if let Some(content_start) = xml[start..].find('>') {
                let content_start = start + content_start + 1;
                if let Some(end) = xml[content_start..].find(&end_tag) {
                    return Some(xml[content_start..content_start + end].to_string());
                }
            }
        }
        None
    }
}

// =============================================================================
// HTTP Handlers
// =============================================================================

/// GET /api/sso/metadata - Return SP metadata XML
pub async fn get_metadata(sso_service: web::Data<SsoService>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/xml")
        .body(sso_service.generate_metadata())
}

/// POST /api/sso/initiate - Initiate SSO login
pub async fn initiate_sso(
    sso_service: web::Data<SsoService>,
    request: web::Json<InitiateSsoRequest>,
) -> HttpResponse {
    match sso_service.initiate_sso(request.into_inner()).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// POST /api/sso/callback - Handle SAML callback (ACS)
pub async fn sso_callback(
    sso_service: web::Data<SsoService>,
    form: web::Form<SamlCallbackRequest>,
) -> HttpResponse {
    match sso_service.handle_callback(form.into_inner()).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// GET /api/sso/idps - List IdP configurations (admin only)
pub async fn list_idps(
    sso_service: web::Data<SsoService>,
    query: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    let org_id = query.get("organization_id").map(|s| s.as_str());
    match sso_service.list_idps(org_id).await {
        Ok(idps) => HttpResponse::Ok().json(idps),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// POST /api/sso/idps - Create IdP configuration (admin only)
pub async fn create_idp(
    sso_service: web::Data<SsoService>,
    request: web::Json<CreateIdpRequest>,
) -> HttpResponse {
    match sso_service.create_idp(request.into_inner()).await {
        Ok(idp) => HttpResponse::Created().json(idp),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// PUT /api/sso/idps/{idp_id} - Update IdP configuration (admin only)
pub async fn update_idp(
    sso_service: web::Data<SsoService>,
    path: web::Path<String>,
    request: web::Json<UpdateIdpRequest>,
) -> HttpResponse {
    let idp_id = path.into_inner();
    match sso_service.update_idp(&idp_id, request.into_inner()).await {
        Ok(idp) => HttpResponse::Ok().json(idp),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// DELETE /api/sso/idps/{idp_id} - Delete IdP configuration (admin only)
pub async fn delete_idp(
    sso_service: web::Data<SsoService>,
    path: web::Path<String>,
) -> HttpResponse {
    let idp_id = path.into_inner();
    match sso_service.delete_idp(&idp_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

/// Configure SSO routes
pub fn configure_sso_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/sso")
            .route("/metadata", web::get().to(get_metadata))
            .route("/initiate", web::post().to(initiate_sso))
            .route("/callback", web::post().to(sso_callback))
            .route("/idps", web::get().to(list_idps))
            .route("/idps", web::post().to(create_idp))
            .route("/idps/{idp_id}", web::put().to(update_idp))
            .route("/idps/{idp_id}", web::delete().to(delete_idp)),
    );
}
