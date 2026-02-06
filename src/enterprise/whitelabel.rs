// Copyright (c) 2024-2027 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! White-labeling and custom branding
//!
//! Supports tenant-specific branding, themes, and customization.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// Branding Configuration
// ============================================================================

/// Complete branding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingConfig {
    /// Unique branding ID
    pub id: Uuid,
    /// Tenant ID
    pub tenant_id: Uuid,
    /// Brand name
    pub brand_name: String,
    /// Logo configuration
    pub logo: LogoConfig,
    /// Color theme
    pub theme: ThemeConfig,
    /// Typography
    pub typography: TypographyConfig,
    /// Custom CSS
    pub custom_css: Option<String>,
    /// Email templates
    pub email_templates: EmailTemplates,
    /// Custom domain
    pub custom_domain: Option<CustomDomain>,
    /// Footer configuration
    pub footer: FooterConfig,
    /// Favicon URL
    pub favicon_url: Option<String>,
    /// Meta tags
    pub meta_tags: MetaTags,
    /// Feature visibility
    pub feature_visibility: FeatureVisibility,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Updated at
    pub updated_at: DateTime<Utc>,
}

impl BrandingConfig {
    /// Create default branding for a tenant
    pub fn default_for_tenant(tenant_id: Uuid, brand_name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            tenant_id,
            brand_name: brand_name.to_string(),
            logo: LogoConfig::default(),
            theme: ThemeConfig::default(),
            typography: TypographyConfig::default(),
            custom_css: None,
            email_templates: EmailTemplates::default(),
            custom_domain: None,
            footer: FooterConfig::default(),
            favicon_url: None,
            meta_tags: MetaTags::default(),
            feature_visibility: FeatureVisibility::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

// ============================================================================
// Logo Configuration
// ============================================================================

/// Logo configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoConfig {
    /// Primary logo URL
    pub primary_url: Option<String>,
    /// Dark mode logo URL
    pub dark_mode_url: Option<String>,
    /// Icon/favicon URL
    pub icon_url: Option<String>,
    /// Logo alt text
    pub alt_text: String,
    /// Logo width (px)
    pub width: Option<u32>,
    /// Logo height (px)
    pub height: Option<u32>,
}

impl Default for LogoConfig {
    fn default() -> Self {
        Self {
            primary_url: None,
            dark_mode_url: None,
            icon_url: None,
            alt_text: "Logo".to_string(),
            width: None,
            height: Some(40),
        }
    }
}

// ============================================================================
// Theme Configuration
// ============================================================================

/// Color theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Primary brand color
    pub primary_color: String,
    /// Secondary color
    pub secondary_color: String,
    /// Accent color
    pub accent_color: String,
    /// Background color
    pub background_color: String,
    /// Surface/card color
    pub surface_color: String,
    /// Text color
    pub text_color: String,
    /// Secondary text color
    pub text_secondary_color: String,
    /// Border color
    pub border_color: String,
    /// Success color
    pub success_color: String,
    /// Warning color
    pub warning_color: String,
    /// Error color
    pub error_color: String,
    /// Info color
    pub info_color: String,
    /// Dark mode variant
    pub dark_mode: Option<Box<ThemeConfig>>,
    /// Border radius
    pub border_radius: String,
    /// Shadow style
    pub shadow: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            primary_color: "#2563eb".to_string(),
            secondary_color: "#7c3aed".to_string(),
            accent_color: "#06b6d4".to_string(),
            background_color: "#ffffff".to_string(),
            surface_color: "#f9fafb".to_string(),
            text_color: "#111827".to_string(),
            text_secondary_color: "#6b7280".to_string(),
            border_color: "#e5e7eb".to_string(),
            success_color: "#059669".to_string(),
            warning_color: "#d97706".to_string(),
            error_color: "#dc2626".to_string(),
            info_color: "#2563eb".to_string(),
            dark_mode: Some(Box::new(Self::dark_mode_defaults())),
            border_radius: "0.5rem".to_string(),
            shadow: "0 1px 3px 0 rgb(0 0 0 / 0.1)".to_string(),
        }
    }
}

impl ThemeConfig {
    /// Dark mode defaults
    pub fn dark_mode_defaults() -> Self {
        Self {
            primary_color: "#3b82f6".to_string(),
            secondary_color: "#8b5cf6".to_string(),
            accent_color: "#22d3ee".to_string(),
            background_color: "#0f172a".to_string(),
            surface_color: "#1e293b".to_string(),
            text_color: "#f1f5f9".to_string(),
            text_secondary_color: "#94a3b8".to_string(),
            border_color: "#334155".to_string(),
            success_color: "#10b981".to_string(),
            warning_color: "#f59e0b".to_string(),
            error_color: "#ef4444".to_string(),
            info_color: "#3b82f6".to_string(),
            dark_mode: None,
            border_radius: "0.5rem".to_string(),
            shadow: "0 1px 3px 0 rgb(0 0 0 / 0.3)".to_string(),
        }
    }

    /// Generate CSS variables
    pub fn to_css_variables(&self) -> String {
        format!(
            r#":root {{
  --color-primary: {};
  --color-secondary: {};
  --color-accent: {};
  --color-background: {};
  --color-surface: {};
  --color-text: {};
  --color-text-secondary: {};
  --color-border: {};
  --color-success: {};
  --color-warning: {};
  --color-error: {};
  --color-info: {};
  --border-radius: {};
  --shadow: {};
}}"#,
            self.primary_color,
            self.secondary_color,
            self.accent_color,
            self.background_color,
            self.surface_color,
            self.text_color,
            self.text_secondary_color,
            self.border_color,
            self.success_color,
            self.warning_color,
            self.error_color,
            self.info_color,
            self.border_radius,
            self.shadow
        )
    }
}

// ============================================================================
// Typography
// ============================================================================

/// Typography configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypographyConfig {
    /// Primary font family
    pub font_family: String,
    /// Heading font family
    pub heading_font_family: Option<String>,
    /// Monospace font family
    pub mono_font_family: String,
    /// Base font size
    pub base_font_size: String,
    /// Line height
    pub line_height: String,
    /// Font weights
    pub font_weights: FontWeights,
    /// Custom font URLs
    pub custom_font_urls: Vec<String>,
}

impl Default for TypographyConfig {
    fn default() -> Self {
        Self {
            font_family: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif".to_string(),
            heading_font_family: None,
            mono_font_family: "ui-monospace, SFMono-Regular, Menlo, Monaco, monospace".to_string(),
            base_font_size: "16px".to_string(),
            line_height: "1.5".to_string(),
            font_weights: FontWeights::default(),
            custom_font_urls: vec![],
        }
    }
}

/// Font weights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontWeights {
    pub light: u16,
    pub regular: u16,
    pub medium: u16,
    pub semibold: u16,
    pub bold: u16,
}

impl Default for FontWeights {
    fn default() -> Self {
        Self {
            light: 300,
            regular: 400,
            medium: 500,
            semibold: 600,
            bold: 700,
        }
    }
}

// ============================================================================
// Email Templates
// ============================================================================

/// Email templates for white-labeled communications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTemplates {
    /// Welcome email template
    pub welcome: EmailTemplate,
    /// Invitation email template
    pub invitation: EmailTemplate,
    /// Password reset email template
    pub password_reset: EmailTemplate,
    /// Session shared notification
    pub session_shared: EmailTemplate,
    /// Weekly digest
    pub weekly_digest: EmailTemplate,
    /// Custom templates
    pub custom: HashMap<String, EmailTemplate>,
}

impl Default for EmailTemplates {
    fn default() -> Self {
        Self {
            welcome: EmailTemplate::default_welcome(),
            invitation: EmailTemplate::default_invitation(),
            password_reset: EmailTemplate::default_password_reset(),
            session_shared: EmailTemplate::default_session_shared(),
            weekly_digest: EmailTemplate::default_digest(),
            custom: HashMap::new(),
        }
    }
}

/// Email template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTemplate {
    /// Subject line (supports variables)
    pub subject: String,
    /// HTML body
    pub body_html: String,
    /// Plain text body
    pub body_text: String,
    /// From name
    pub from_name: Option<String>,
    /// Reply-to address
    pub reply_to: Option<String>,
}

impl EmailTemplate {
    fn default_welcome() -> Self {
        Self {
            subject: "Welcome to {{brand_name}}".to_string(),
            body_html: r#"<h1>Welcome to {{brand_name}}</h1><p>Hello {{user_name}},</p><p>Your account has been created.</p>"#.to_string(),
            body_text: "Welcome to {{brand_name}}\n\nHello {{user_name}},\nYour account has been created.".to_string(),
            from_name: None,
            reply_to: None,
        }
    }

    fn default_invitation() -> Self {
        Self {
            subject: "You've been invited to {{brand_name}}".to_string(),
            body_html: r#"<h1>You're invited!</h1><p>{{inviter_name}} has invited you to join {{brand_name}}.</p>"#.to_string(),
            body_text: "You're invited!\n\n{{inviter_name}} has invited you to join {{brand_name}}.".to_string(),
            from_name: None,
            reply_to: None,
        }
    }

    fn default_password_reset() -> Self {
        Self {
            subject: "Reset your {{brand_name}} password".to_string(),
            body_html: r#"<h1>Password Reset</h1><p>Click the link below to reset your password.</p>"#.to_string(),
            body_text: "Password Reset\n\nClick the link below to reset your password.".to_string(),
            from_name: None,
            reply_to: None,
        }
    }

    fn default_session_shared() -> Self {
        Self {
            subject: "{{sharer_name}} shared a session with you".to_string(),
            body_html: r#"<h1>Session Shared</h1><p>{{sharer_name}} shared "{{session_title}}" with you.</p>"#.to_string(),
            body_text: "Session Shared\n\n{{sharer_name}} shared \"{{session_title}}\" with you.".to_string(),
            from_name: None,
            reply_to: None,
        }
    }

    fn default_digest() -> Self {
        Self {
            subject: "Your weekly {{brand_name}} digest".to_string(),
            body_html: r#"<h1>Weekly Digest</h1><p>Here's your activity summary.</p>"#.to_string(),
            body_text: "Weekly Digest\n\nHere's your activity summary.".to_string(),
            from_name: None,
            reply_to: None,
        }
    }

    /// Render template with variables
    pub fn render(&self, variables: &HashMap<String, String>) -> (String, String, String) {
        let mut subject = self.subject.clone();
        let mut body_html = self.body_html.clone();
        let mut body_text = self.body_text.clone();

        for (key, value) in variables {
            let placeholder = format!("{{{{{}}}}}", key);
            subject = subject.replace(&placeholder, value);
            body_html = body_html.replace(&placeholder, value);
            body_text = body_text.replace(&placeholder, value);
        }

        (subject, body_html, body_text)
    }
}

// ============================================================================
// Custom Domain
// ============================================================================

/// Custom domain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomDomain {
    /// Domain name
    pub domain: String,
    /// SSL certificate status
    pub ssl_status: SslStatus,
    /// DNS verification status
    pub dns_verified: bool,
    /// DNS verification token
    pub dns_token: String,
    /// CNAME target
    pub cname_target: String,
    /// Configured at
    pub configured_at: DateTime<Utc>,
    /// SSL expires at
    pub ssl_expires_at: Option<DateTime<Utc>>,
}

/// SSL status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SslStatus {
    Pending,
    Provisioning,
    Active,
    Failed,
    Expired,
}

// ============================================================================
// Footer Configuration
// ============================================================================

/// Footer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FooterConfig {
    /// Show powered by
    pub show_powered_by: bool,
    /// Powered by text
    pub powered_by_text: Option<String>,
    /// Copyright text
    pub copyright_text: Option<String>,
    /// Footer links
    pub links: Vec<FooterLink>,
    /// Social links
    pub social_links: Vec<SocialLink>,
}

impl Default for FooterConfig {
    fn default() -> Self {
        Self {
            show_powered_by: true,
            powered_by_text: None,
            copyright_text: None,
            links: vec![],
            social_links: vec![],
        }
    }
}

/// Footer link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FooterLink {
    pub label: String,
    pub url: String,
    pub new_tab: bool,
}

/// Social link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialLink {
    pub platform: SocialPlatform,
    pub url: String,
}

/// Social platform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialPlatform {
    Twitter,
    LinkedIn,
    GitHub,
    Facebook,
    Instagram,
    YouTube,
    Discord,
    Slack,
}

// ============================================================================
// Meta Tags
// ============================================================================

/// Meta tags for SEO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaTags {
    /// Page title template
    pub title_template: String,
    /// Default description
    pub description: String,
    /// Keywords
    pub keywords: Vec<String>,
    /// Open Graph image URL
    pub og_image_url: Option<String>,
    /// Twitter card type
    pub twitter_card: String,
    /// Additional meta tags
    pub custom: HashMap<String, String>,
}

impl Default for MetaTags {
    fn default() -> Self {
        Self {
            title_template: "{{page_title}} | {{brand_name}}".to_string(),
            description: "AI chat session management platform".to_string(),
            keywords: vec![],
            og_image_url: None,
            twitter_card: "summary_large_image".to_string(),
            custom: HashMap::new(),
        }
    }
}

// ============================================================================
// Feature Visibility
// ============================================================================

/// Control visibility of UI features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureVisibility {
    /// Show provider logos
    pub show_provider_logos: bool,
    /// Show analytics
    pub show_analytics: bool,
    /// Show collaboration features
    pub show_collaboration: bool,
    /// Show export options
    pub show_export: bool,
    /// Show API documentation
    pub show_api_docs: bool,
    /// Show help/support
    pub show_help: bool,
    /// Custom hidden features
    pub hidden_features: Vec<String>,
}

impl Default for FeatureVisibility {
    fn default() -> Self {
        Self {
            show_provider_logos: true,
            show_analytics: true,
            show_collaboration: true,
            show_export: true,
            show_api_docs: true,
            show_help: true,
            hidden_features: vec![],
        }
    }
}

// ============================================================================
// Branding Manager
// ============================================================================

/// Manages branding configurations
pub struct BrandingManager {
    /// Branding configs by tenant
    configs: HashMap<Uuid, BrandingConfig>,
}

impl BrandingManager {
    /// Create a new branding manager
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    /// Create branding for a tenant
    pub fn create_branding(&mut self, tenant_id: Uuid, brand_name: &str) -> BrandingConfig {
        let config = BrandingConfig::default_for_tenant(tenant_id, brand_name);
        self.configs.insert(tenant_id, config.clone());
        config
    }

    /// Get branding for a tenant
    pub fn get_branding(&self, tenant_id: Uuid) -> Option<&BrandingConfig> {
        self.configs.get(&tenant_id)
    }

    /// Update theme
    pub fn update_theme(&mut self, tenant_id: Uuid, theme: ThemeConfig) -> bool {
        if let Some(config) = self.configs.get_mut(&tenant_id) {
            config.theme = theme;
            config.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Update logo
    pub fn update_logo(&mut self, tenant_id: Uuid, logo: LogoConfig) -> bool {
        if let Some(config) = self.configs.get_mut(&tenant_id) {
            config.logo = logo;
            config.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Set custom domain
    pub fn set_custom_domain(&mut self, tenant_id: Uuid, domain: &str) -> Option<CustomDomain> {
        let config = self.configs.get_mut(&tenant_id)?;
        
        let custom_domain = CustomDomain {
            domain: domain.to_string(),
            ssl_status: SslStatus::Pending,
            dns_verified: false,
            dns_token: Uuid::new_v4().to_string(),
            cname_target: "app.chasm.cloud".to_string(),
            configured_at: Utc::now(),
            ssl_expires_at: None,
        };

        config.custom_domain = Some(custom_domain.clone());
        config.updated_at = Utc::now();

        Some(custom_domain)
    }

    /// Generate CSS for tenant
    pub fn generate_css(&self, tenant_id: Uuid) -> Option<String> {
        let config = self.configs.get(&tenant_id)?;
        let mut css = config.theme.to_css_variables();

        if let Some(ref custom_css) = config.custom_css {
            css.push_str("\n\n/* Custom CSS */\n");
            css.push_str(custom_css);
        }

        Some(css)
    }
}

impl Default for BrandingManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_css_variables() {
        let theme = ThemeConfig::default();
        let css = theme.to_css_variables();
        
        assert!(css.contains("--color-primary"));
        assert!(css.contains("#2563eb"));
    }

    #[test]
    fn test_email_template_render() {
        let template = EmailTemplate::default_welcome();
        let mut vars = HashMap::new();
        vars.insert("brand_name".to_string(), "Acme".to_string());
        vars.insert("user_name".to_string(), "John".to_string());

        let (subject, _, _) = template.render(&vars);
        assert_eq!(subject, "Welcome to Acme");
    }

    #[test]
    fn test_branding_manager() {
        let mut manager = BrandingManager::new();
        let tenant_id = Uuid::new_v4();

        let config = manager.create_branding(tenant_id, "Test Brand");
        assert_eq!(config.brand_name, "Test Brand");

        assert!(manager.get_branding(tenant_id).is_some());
    }
}
