// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Authentication and Authorization Module
//!
//! Provides JWT-based authentication, user management, and subscription handling
//! for the CSM ecosystem (csm-rust, csm-web, csm-app).

use actix_web::{dev::Payload, web, FromRequest, HttpMessage, HttpRequest, HttpResponse};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::future::{ready, Ready};
use uuid::Uuid;

// =============================================================================
// Configuration
// =============================================================================

/// JWT secret key - in production, this should come from environment variables
const JWT_SECRET: &[u8] = b"csm_jwt_secret_key_change_in_production_2024";
const JWT_EXPIRY_HOURS: i64 = 24;
const REFRESH_TOKEN_EXPIRY_DAYS: i64 = 30;

// =============================================================================
// Subscription Tiers
// =============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum SubscriptionTier {
    /// Free tier - basic sync, limited features
    #[default]
    Free,
    /// Pro tier - full sync, all features, priority support
    Pro,
    /// Enterprise tier - team features, admin controls, SLA
    Enterprise,
}

impl SubscriptionTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
            Self::Enterprise => "enterprise",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "free" => Some(Self::Free),
            "pro" => Some(Self::Pro),
            "enterprise" => Some(Self::Enterprise),
            _ => None,
        }
    }
}

// =============================================================================
// Subscription Features
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionFeatures {
    /// Maximum number of workspaces
    pub max_workspaces: Option<u32>,
    /// Maximum number of sessions
    pub max_sessions: Option<u32>,
    /// Maximum number of agents
    pub max_agents: Option<u32>,
    /// Maximum number of swarms
    pub max_swarms: Option<u32>,
    /// Real-time sync enabled
    pub realtime_sync: bool,
    /// Cross-device sync enabled
    pub cross_device_sync: bool,
    /// Priority support
    pub priority_support: bool,
    /// Team collaboration features
    pub team_collaboration: bool,
    /// Advanced analytics
    pub analytics: bool,
    /// API access
    pub api_access: bool,
    /// Custom integrations
    pub custom_integrations: bool,
}

impl SubscriptionFeatures {
    pub fn for_tier(tier: SubscriptionTier) -> Self {
        match tier {
            SubscriptionTier::Free => Self {
                max_workspaces: Some(10),
                max_sessions: Some(100),
                max_agents: Some(5),
                max_swarms: Some(1),
                realtime_sync: true,
                cross_device_sync: true,
                priority_support: false,
                team_collaboration: false,
                analytics: false,
                api_access: false,
                custom_integrations: false,
            },
            SubscriptionTier::Pro => Self {
                max_workspaces: Some(100),
                max_sessions: None, // Unlimited
                max_agents: Some(100),
                max_swarms: Some(20),
                realtime_sync: true,
                cross_device_sync: true,
                priority_support: true,
                team_collaboration: false,
                analytics: true,
                api_access: true,
                custom_integrations: false,
            },
            SubscriptionTier::Enterprise => Self {
                max_workspaces: None, // Unlimited
                max_sessions: None,
                max_agents: None,
                max_swarms: None,
                realtime_sync: true,
                cross_device_sync: true,
                priority_support: true,
                team_collaboration: true,
                analytics: true,
                api_access: true,
                custom_integrations: true,
            },
        }
    }
}

// =============================================================================
// User Model
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub display_name: String,
    #[serde(skip_serializing)]
    #[allow(dead_code)]
    pub password_hash: String,
    pub subscription_tier: SubscriptionTier,
    pub subscription_expires_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_login_at: Option<i64>,
    pub email_verified: bool,
    pub avatar_url: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicUser {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub subscription_tier: SubscriptionTier,
    pub subscription_expires_at: Option<i64>,
    pub created_at: i64,
    pub email_verified: bool,
    pub avatar_url: Option<String>,
    pub features: SubscriptionFeatures,
}

impl From<User> for PublicUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
            subscription_tier: user.subscription_tier,
            subscription_expires_at: user.subscription_expires_at,
            created_at: user.created_at,
            email_verified: user.email_verified,
            avatar_url: user.avatar_url,
            features: SubscriptionFeatures::for_tier(user.subscription_tier),
        }
    }
}

// =============================================================================
// JWT Claims
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Email
    pub email: String,
    /// Subscription tier
    pub tier: String,
    /// Issued at
    pub iat: i64,
    /// Expiration
    pub exp: i64,
    /// Token type (access or refresh)
    pub token_type: String,
}

// =============================================================================
// Auth State (for middleware)
// =============================================================================

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub email: String,
    pub tier: SubscriptionTier,
}

impl FromRequest for AuthenticatedUser {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        // Try to get user from request extensions (set by middleware)
        if let Some(user) = req.extensions().get::<AuthenticatedUser>() {
            return ready(Ok(user.clone()));
        }

        // Try to extract from Authorization header directly
        if let Some(auth_header) = req.headers().get("Authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if let Some(token) = auth_str.strip_prefix("Bearer ") {
                    if let Some(user) = validate_token(token) {
                        return ready(Ok(user));
                    }
                }
            }
        }

        ready(Ok(AuthenticatedUser {
            user_id: String::new(),
            email: String::new(),
            tier: SubscriptionTier::Free,
        }))
    }
}

// =============================================================================
// Request/Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: PublicUser,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpgradeSubscriptionRequest {
    pub tier: String,
    /// Payment token from payment provider (Stripe, etc.)
    #[allow(dead_code)]
    pub payment_token: Option<String>,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Hash a password using SHA-256 with salt
pub fn hash_password(password: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(salt.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Generate a JWT access token
pub fn generate_access_token(user: &User) -> Option<String> {
    let now = Utc::now();
    let exp = now + Duration::hours(JWT_EXPIRY_HOURS);

    let claims = Claims {
        sub: user.id.clone(),
        email: user.email.clone(),
        tier: user.subscription_tier.as_str().to_string(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
        token_type: "access".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
    .ok()
}

/// Generate a refresh token
pub fn generate_refresh_token(user: &User) -> Option<String> {
    let now = Utc::now();
    let exp = now + Duration::days(REFRESH_TOKEN_EXPIRY_DAYS);

    let claims = Claims {
        sub: user.id.clone(),
        email: user.email.clone(),
        tier: user.subscription_tier.as_str().to_string(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
        token_type: "refresh".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
    .ok()
}

/// Validate a JWT token and return the authenticated user
pub fn validate_token(token: &str) -> Option<AuthenticatedUser> {
    let validation = Validation::new(Algorithm::HS256);

    let token_data =
        decode::<Claims>(token, &DecodingKey::from_secret(JWT_SECRET), &validation).ok()?;

    let claims = token_data.claims;

    // Check if token is expired
    if claims.exp < Utc::now().timestamp() {
        return None;
    }

    Some(AuthenticatedUser {
        user_id: claims.sub,
        email: claims.email,
        tier: SubscriptionTier::from_str(&claims.tier).unwrap_or_default(),
    })
}

/// Validate refresh token specifically
pub fn validate_refresh_token(token: &str) -> Option<AuthenticatedUser> {
    let validation = Validation::new(Algorithm::HS256);

    let token_data =
        decode::<Claims>(token, &DecodingKey::from_secret(JWT_SECRET), &validation).ok()?;

    let claims = token_data.claims;

    // Must be a refresh token
    if claims.token_type != "refresh" {
        return None;
    }

    // Check if token is expired
    if claims.exp < Utc::now().timestamp() {
        return None;
    }

    Some(AuthenticatedUser {
        user_id: claims.sub,
        email: claims.email,
        tier: SubscriptionTier::from_str(&claims.tier).unwrap_or_default(),
    })
}

// =============================================================================
// Database Operations
// =============================================================================

/// Initialize auth tables in the database
pub fn init_auth_tables(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            email TEXT UNIQUE NOT NULL,
            display_name TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            password_salt TEXT NOT NULL,
            subscription_tier TEXT NOT NULL DEFAULT 'free',
            subscription_expires_at INTEGER,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            last_login_at INTEGER,
            email_verified INTEGER DEFAULT 0,
            avatar_url TEXT,
            metadata TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS refresh_tokens (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            token_hash TEXT NOT NULL,
            expires_at INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            revoked INTEGER DEFAULT 0,
            device_info TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            device_id TEXT,
            device_name TEXT,
            platform TEXT,
            ip_address TEXT,
            last_active_at INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_id ON refresh_tokens(user_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_user_sessions_user_id ON user_sessions(user_id)",
        [],
    )?;

    Ok(())
}

// =============================================================================
// HTTP Handlers
// =============================================================================

/// Register a new user
pub async fn register(
    app_state: web::Data<crate::api::state::AppState>,
    body: web::Json<RegisterRequest>,
) -> HttpResponse {
    let db = app_state.db.lock().unwrap();

    // Initialize tables if needed
    if let Err(e) = init_auth_tables(&db.conn) {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("Database error: {}", e)
        }));
    }

    // Validate input
    if body.email.is_empty() || !body.email.contains('@') {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Invalid email address"
        }));
    }

    if body.password.len() < 8 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Password must be at least 8 characters"
        }));
    }

    if body.display_name.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Display name is required"
        }));
    }

    // Check if email already exists
    let existing: rusqlite::Result<Option<String>> = db
        .conn
        .query_row(
            "SELECT id FROM users WHERE email = ?1",
            rusqlite::params![body.email.to_lowercase()],
            |row| row.get(0),
        )
        .optional();

    if let Ok(Some(_)) = existing {
        return HttpResponse::Conflict().json(serde_json::json!({
            "success": false,
            "error": "Email already registered"
        }));
    }

    // Create user
    let user_id = Uuid::new_v4().to_string();
    let salt = Uuid::new_v4().to_string();
    let password_hash = hash_password(&body.password, &salt);
    let now = Utc::now().timestamp();

    let result = db.conn.execute(
        "INSERT INTO users (id, email, display_name, password_hash, password_salt, 
                           subscription_tier, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            user_id,
            body.email.to_lowercase(),
            body.display_name,
            password_hash,
            salt,
            "free",
            now,
            now,
        ],
    );

    if let Err(e) = result {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("Failed to create user: {}", e)
        }));
    }

    // Create user object for token generation
    let user = User {
        id: user_id,
        email: body.email.to_lowercase(),
        display_name: body.display_name.clone(),
        password_hash,
        subscription_tier: SubscriptionTier::Free,
        subscription_expires_at: None,
        created_at: now,
        updated_at: now,
        last_login_at: None,
        email_verified: false,
        avatar_url: None,
        metadata: None,
    };

    // Generate tokens
    let access_token = match generate_access_token(&user) {
        Some(t) => t,
        None => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to generate access token"
            }))
        }
    };

    let refresh_token = match generate_refresh_token(&user) {
        Some(t) => t,
        None => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to generate refresh token"
            }))
        }
    };

    HttpResponse::Created().json(serde_json::json!({
        "success": true,
        "data": AuthResponse {
            user: PublicUser::from(user),
            access_token,
            refresh_token,
            expires_in: JWT_EXPIRY_HOURS * 3600,
        }
    }))
}

/// Login with email and password
pub async fn login(
    app_state: web::Data<crate::api::state::AppState>,
    body: web::Json<LoginRequest>,
) -> HttpResponse {
    let db = app_state.db.lock().unwrap();

    // Initialize tables if needed
    if let Err(e) = init_auth_tables(&db.conn) {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("Database error: {}", e)
        }));
    }

    // Find user by email
    let user_result: rusqlite::Result<(
        String,
        String,
        String,
        String,
        String,
        String,
        Option<i64>,
        i64,
        i64,
        Option<i64>,
        i32,
        Option<String>,
    )> = db.conn.query_row(
        "SELECT id, email, display_name, password_hash, password_salt, 
                    subscription_tier, subscription_expires_at, created_at, updated_at,
                    last_login_at, email_verified, avatar_url
             FROM users WHERE email = ?1",
        rusqlite::params![body.email.to_lowercase()],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
                row.get(10)?,
                row.get(11)?,
            ))
        },
    );

    let (
        id,
        email,
        display_name,
        stored_hash,
        salt,
        tier_str,
        sub_expires,
        created_at,
        updated_at,
        _last_login,
        verified,
        avatar,
    ) = match user_result {
        Ok(data) => data,
        Err(_) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Invalid email or password"
            }))
        }
    };

    // Verify password
    let provided_hash = hash_password(&body.password, &salt);
    if provided_hash != stored_hash {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Invalid email or password"
        }));
    }

    // Update last login time
    let now = Utc::now().timestamp();
    let _ = db.conn.execute(
        "UPDATE users SET last_login_at = ?1 WHERE id = ?2",
        rusqlite::params![now, id],
    );

    let user = User {
        id,
        email,
        display_name,
        password_hash: stored_hash,
        subscription_tier: SubscriptionTier::from_str(&tier_str).unwrap_or_default(),
        subscription_expires_at: sub_expires,
        created_at,
        updated_at,
        last_login_at: Some(now),
        email_verified: verified == 1,
        avatar_url: avatar,
        metadata: None,
    };

    // Generate tokens
    let access_token = match generate_access_token(&user) {
        Some(t) => t,
        None => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to generate access token"
            }))
        }
    };

    let refresh_token = match generate_refresh_token(&user) {
        Some(t) => t,
        None => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to generate refresh token"
            }))
        }
    };

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": AuthResponse {
            user: PublicUser::from(user),
            access_token,
            refresh_token,
            expires_in: JWT_EXPIRY_HOURS * 3600,
        }
    }))
}

/// Refresh access token using refresh token
pub async fn refresh_token(
    app_state: web::Data<crate::api::state::AppState>,
    body: web::Json<RefreshTokenRequest>,
) -> HttpResponse {
    // Validate refresh token
    let auth_user = match validate_refresh_token(&body.refresh_token) {
        Some(u) => u,
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Invalid or expired refresh token"
            }))
        }
    };

    let db = app_state.db.lock().unwrap();

    // Get current user data
    let user_result: rusqlite::Result<(
        String,
        String,
        String,
        String,
        Option<i64>,
        i64,
        i64,
        i32,
        Option<String>,
    )> = db.conn.query_row(
        "SELECT id, email, display_name, subscription_tier, subscription_expires_at,
                    created_at, updated_at, email_verified, avatar_url
             FROM users WHERE id = ?1",
        rusqlite::params![auth_user.user_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
            ))
        },
    );

    let (id, email, display_name, tier_str, sub_expires, created_at, updated_at, verified, avatar) =
        match user_result {
            Ok(data) => data,
            Err(_) => {
                return HttpResponse::NotFound().json(serde_json::json!({
                    "success": false,
                    "error": "User not found"
                }))
            }
        };

    let user = User {
        id,
        email,
        display_name,
        password_hash: String::new(), // Not needed for token generation
        subscription_tier: SubscriptionTier::from_str(&tier_str).unwrap_or_default(),
        subscription_expires_at: sub_expires,
        created_at,
        updated_at,
        last_login_at: None,
        email_verified: verified == 1,
        avatar_url: avatar,
        metadata: None,
    };

    // Generate new tokens
    let access_token = match generate_access_token(&user) {
        Some(t) => t,
        None => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to generate access token"
            }))
        }
    };

    let new_refresh_token = match generate_refresh_token(&user) {
        Some(t) => t,
        None => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to generate refresh token"
            }))
        }
    };

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": AuthResponse {
            user: PublicUser::from(user),
            access_token,
            refresh_token: new_refresh_token,
            expires_in: JWT_EXPIRY_HOURS * 3600,
        }
    }))
}

/// Get current user profile
pub async fn get_profile(
    app_state: web::Data<crate::api::state::AppState>,
    auth_user: AuthenticatedUser,
) -> HttpResponse {
    if auth_user.user_id.is_empty() {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Not authenticated"
        }));
    }

    let db = app_state.db.lock().unwrap();

    let user_result: rusqlite::Result<(
        String,
        String,
        String,
        String,
        Option<i64>,
        i64,
        i64,
        i32,
        Option<String>,
    )> = db.conn.query_row(
        "SELECT id, email, display_name, subscription_tier, subscription_expires_at,
                    created_at, updated_at, email_verified, avatar_url
             FROM users WHERE id = ?1",
        rusqlite::params![auth_user.user_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
            ))
        },
    );

    match user_result {
        Ok((
            id,
            email,
            display_name,
            tier_str,
            sub_expires,
            created_at,
            updated_at,
            verified,
            avatar,
        )) => {
            let user = User {
                id,
                email,
                display_name,
                password_hash: String::new(),
                subscription_tier: SubscriptionTier::from_str(&tier_str).unwrap_or_default(),
                subscription_expires_at: sub_expires,
                created_at,
                updated_at,
                last_login_at: None,
                email_verified: verified == 1,
                avatar_url: avatar,
                metadata: None,
            };

            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "data": PublicUser::from(user)
            }))
        }
        Err(_) => HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "User not found"
        })),
    }
}

/// Update user profile
pub async fn update_profile(
    app_state: web::Data<crate::api::state::AppState>,
    auth_user: AuthenticatedUser,
    body: web::Json<UpdateProfileRequest>,
) -> HttpResponse {
    if auth_user.user_id.is_empty() {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Not authenticated"
        }));
    }

    // Scope the db lock so it's released before calling get_profile
    {
        let db = app_state.db.lock().unwrap();
        let now = Utc::now().timestamp();

        // Build update query
        let mut updates = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref name) = body.display_name {
            updates.push("display_name = ?");
            params.push(Box::new(name.clone()));
        }

        if let Some(ref avatar) = body.avatar_url {
            updates.push("avatar_url = ?");
            params.push(Box::new(avatar.clone()));
        }

        if updates.is_empty() {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "No fields to update"
            }));
        }

        updates.push("updated_at = ?");
        params.push(Box::new(now));

        let query = format!("UPDATE users SET {} WHERE id = ?", updates.join(", "));
        params.push(Box::new(auth_user.user_id.clone()));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        if let Err(e) = db.conn.execute(&query, params_refs.as_slice()) {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": format!("Failed to update profile: {}", e)
            }));
        }
    } // db lock released here

    // Return updated profile
    get_profile(app_state, auth_user).await
}

/// Change password
pub async fn change_password(
    app_state: web::Data<crate::api::state::AppState>,
    auth_user: AuthenticatedUser,
    body: web::Json<ChangePasswordRequest>,
) -> HttpResponse {
    if auth_user.user_id.is_empty() {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Not authenticated"
        }));
    }

    if body.new_password.len() < 8 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "New password must be at least 8 characters"
        }));
    }

    let db = app_state.db.lock().unwrap();

    // Get current password hash and salt
    let creds: rusqlite::Result<(String, String)> = db.conn.query_row(
        "SELECT password_hash, password_salt FROM users WHERE id = ?1",
        rusqlite::params![auth_user.user_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    let (stored_hash, salt) = match creds {
        Ok(c) => c,
        Err(_) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "error": "User not found"
            }))
        }
    };

    // Verify current password
    let current_hash = hash_password(&body.current_password, &salt);
    if current_hash != stored_hash {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Current password is incorrect"
        }));
    }

    // Update password
    let new_salt = Uuid::new_v4().to_string();
    let new_hash = hash_password(&body.new_password, &new_salt);
    let now = Utc::now().timestamp();

    if let Err(e) = db.conn.execute(
        "UPDATE users SET password_hash = ?1, password_salt = ?2, updated_at = ?3 WHERE id = ?4",
        rusqlite::params![new_hash, new_salt, now, auth_user.user_id],
    ) {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("Failed to update password: {}", e)
        }));
    }

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Password updated successfully"
    }))
}

/// Get subscription details
pub async fn get_subscription(
    app_state: web::Data<crate::api::state::AppState>,
    auth_user: AuthenticatedUser,
) -> HttpResponse {
    if auth_user.user_id.is_empty() {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Not authenticated"
        }));
    }

    let db = app_state.db.lock().unwrap();

    let result: rusqlite::Result<(String, Option<i64>)> = db.conn.query_row(
        "SELECT subscription_tier, subscription_expires_at FROM users WHERE id = ?1",
        rusqlite::params![auth_user.user_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    match result {
        Ok((tier_str, expires_at)) => {
            let tier = SubscriptionTier::from_str(&tier_str).unwrap_or_default();
            let features = SubscriptionFeatures::for_tier(tier);

            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "data": {
                    "tier": tier_str,
                    "expiresAt": expires_at,
                    "features": features,
                    "isActive": expires_at.map(|exp| exp > Utc::now().timestamp()).unwrap_or(true),
                }
            }))
        }
        Err(_) => HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "User not found"
        })),
    }
}

/// Upgrade subscription (simulated - in production would integrate with payment provider)
pub async fn upgrade_subscription(
    app_state: web::Data<crate::api::state::AppState>,
    auth_user: AuthenticatedUser,
    body: web::Json<UpgradeSubscriptionRequest>,
) -> HttpResponse {
    if auth_user.user_id.is_empty() {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Not authenticated"
        }));
    }

    let new_tier = match SubscriptionTier::from_str(&body.tier) {
        Some(t) => t,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "Invalid subscription tier"
            }))
        }
    };

    // In production, this would:
    // 1. Validate the payment token with Stripe/PayPal
    // 2. Create a subscription in the payment provider
    // 3. Set up webhooks for subscription events

    let db = app_state.db.lock().unwrap();
    let now = Utc::now().timestamp();
    let expires_at = if new_tier == SubscriptionTier::Free {
        None
    } else {
        // 1 year subscription
        Some(now + (365 * 24 * 3600))
    };

    if let Err(e) = db.conn.execute(
        "UPDATE users SET subscription_tier = ?1, subscription_expires_at = ?2, updated_at = ?3 WHERE id = ?4",
        rusqlite::params![new_tier.as_str(), expires_at, now, auth_user.user_id],
    ) {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("Failed to upgrade subscription: {}", e)
        }));
    }

    let features = SubscriptionFeatures::for_tier(new_tier);

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": {
            "tier": new_tier.as_str(),
            "expiresAt": expires_at,
            "features": features,
            "message": "Subscription upgraded successfully"
        }
    }))
}

/// Logout (invalidate session)
pub async fn logout(_auth_user: AuthenticatedUser) -> HttpResponse {
    // In a production system, you would:
    // 1. Add the token to a blacklist
    // 2. Remove the refresh token from the database
    // 3. Clear any server-side session data

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Logged out successfully"
    }))
}

// =============================================================================
// Route Configuration
// =============================================================================

pub fn configure_auth_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login))
            .route("/refresh", web::post().to(refresh_token))
            .route("/logout", web::post().to(logout))
            .route("/profile", web::get().to(get_profile))
            .route("/profile", web::put().to(update_profile))
            .route("/password", web::put().to(change_password))
            .route("/subscription", web::get().to(get_subscription))
            .route(
                "/subscription/upgrade",
                web::post().to(upgrade_subscription),
            ),
    );
}
