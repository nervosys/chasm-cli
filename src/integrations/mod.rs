// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Life Integrations Module
//!
//! Comprehensive hooks for AI agents to interact with daily life applications.
//! Makes ChatSystemManager the home for AI power users.
//!
//! ## Categories
//!
//! - **Productivity**: Calendar, Email, Notes, Tasks, Documents
//! - **Communication**: Slack, Discord, Teams, Telegram, SMS
//! - **Browser**: Chrome, Arc, Firefox, web automation
//! - **Development**: GitHub, GitLab, Terminal, Docker
//! - **Smart Home**: Home Assistant, HomeKit, IoT
//! - **Finance**: Banking, Crypto, Trading
//! - **Health**: Apple Health, Fitness trackers
//! - **Media**: Spotify, YouTube, Podcasts

#![allow(dead_code, unused_imports)]
//! - **Travel**: Maps, Uber, Flights
//! - **Shopping**: Amazon, Groceries

pub mod browser;
pub mod communication;
pub mod hooks;
pub mod productivity;
pub mod registry;
pub mod smart_home;
pub mod system;

pub use hooks::{Hook, HookAction, HookConfig, HookResult, HookTrigger};
pub use registry::{Integration, IntegrationCategory, IntegrationRegistry, IntegrationStatus};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Integration capability that an app provides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    /// Capability identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this capability does
    pub description: String,
    /// Required permissions
    pub permissions: Vec<String>,
    /// Input parameters
    pub parameters: Vec<Parameter>,
    /// Whether this capability requires user confirmation
    pub requires_confirmation: bool,
}

/// Parameter for a capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub description: String,
    pub param_type: ParameterType,
    pub required: bool,
    pub default: Option<serde_json::Value>,
}

/// Parameter types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Date,
    DateTime,
    Duration,
    Email,
    Url,
    FilePath,
    Json,
    Array(Box<ParameterType>),
    Object(HashMap<String, ParameterType>),
}

/// Result of executing an integration action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl IntegrationResult {
    pub fn ok(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            metadata: HashMap::new(),
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
            metadata: HashMap::new(),
        }
    }
}

/// OAuth configuration for integrations requiring authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub scopes: Vec<String>,
    pub redirect_uri: String,
}

/// API key configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    pub key: String,
    pub header_name: Option<String>,
    pub prefix: Option<String>,
}

/// Authentication method for integrations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthMethod {
    None,
    ApiKey(ApiKeyConfig),
    OAuth2(OAuthConfig),
    Bearer { token: String },
    Basic { username: String, password: String },
    Custom { headers: HashMap<String, String> },
}
