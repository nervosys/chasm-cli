// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! HTTP API Server for Chat System Manager
//!
//! Provides a REST API for the web frontend and mobile app to interact with CSM.
//! Uses Actix-web for the HTTP server.

mod auth;
mod handlers_simple;
mod handlers_swe;
mod state;
mod sync;

// Public API exports for authentication and subscription management
// These are part of the library's public API even if not used internally
#[allow(unused_imports)]
pub use auth::{
    configure_auth_routes, require_tier, AuthenticatedUser, SubscriptionFeatures, SubscriptionTier,
};
pub use state::AppState;
pub use sync::{configure_sync_routes, create_sync_state};

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use anyhow::Result;
use std::path::PathBuf;

use crate::database::ChatDatabase;

/// API server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub database_path: String,
    pub cors_origins: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(), // Bind to all interfaces
            port: 8787,
            database_path: dirs::data_local_dir()
                .map(|p| p.join("csm").join("csm.db").to_string_lossy().to_string())
                .unwrap_or_else(|| "csm.db".to_string()),
            cors_origins: vec![
                "http://localhost:5173".to_string(),
                "http://localhost:3000".to_string(),
                "http://127.0.0.1:5173".to_string(),
                "http://127.0.0.1:3000".to_string(),
                "http://localhost:8081".to_string(),  // Expo web
                "http://127.0.0.1:8081".to_string(),  // Expo web
                "http://localhost:19006".to_string(), // Expo web alt
                "http://127.0.0.1:19006".to_string(), // Expo web alt
            ],
        }
    }
}

/// Configure API routes
fn configure_routes(cfg: &mut web::ServiceConfig) {
    use handlers_simple::*;

    eprintln!("[DEBUG] Configuring routes...");

    // Routes for /api
    cfg.service(
        web::scope("/api")
            .route("/health", web::get().to(health_check))
            .route("/workspaces", web::get().to(list_workspaces))
            .route("/workspaces/{id}", web::get().to(get_workspace))
            .route("/sessions", web::get().to(list_sessions))
            .route("/sessions/search", web::get().to(search_sessions))
            .route("/sessions/{id}", web::get().to(get_session))
            .route("/providers", web::get().to(list_providers))
            .route("/stats", web::get().to(get_stats))
            .route("/stats/overview", web::get().to(get_stats))
            // Agent routes
            .route("/agents", web::get().to(list_agents))
            .route("/agents", web::post().to(create_agent))
            .route("/agents/{id}", web::get().to(get_agent))
            .route("/agents/{id}", web::put().to(update_agent))
            .route("/agents/{id}", web::delete().to(delete_agent))
            // Swarm routes
            .route("/swarms", web::get().to(list_swarms))
            .route("/swarms", web::post().to(create_swarm))
            .route("/swarms/{id}", web::get().to(get_swarm))
            .route("/swarms/{id}", web::delete().to(delete_swarm))
            // Settings routes
            .route("/settings", web::get().to(get_settings))
            .route("/settings", web::put().to(update_settings))
            .route("/settings/accounts", web::get().to(list_accounts))
            .route("/settings/accounts", web::post().to(create_account))
            .route("/settings/accounts/{id}", web::delete().to(delete_account))
            // System routes
            .route("/system/info", web::get().to(get_system_info))
            .route("/system/health", web::get().to(get_system_health))
            .route(
                "/system/providers/health",
                web::get().to(get_provider_health),
            )
            // MCP routes
            .route("/mcp/tools", web::get().to(list_mcp_tools))
            .route("/mcp/call", web::post().to(call_mcp_tool))
            .route("/mcp/batch", web::post().to(call_mcp_tools_batch))
            .route("/mcp/system-prompt", web::get().to(get_csm_system_prompt))
            // SWE routes
            .route("/swe/projects", web::get().to(handlers_swe::list_projects))
            .route(
                "/swe/projects",
                web::post().to(handlers_swe::create_project),
            )
            .route(
                "/swe/projects/{id}",
                web::get().to(handlers_swe::get_project),
            )
            .route(
                "/swe/projects/{id}",
                web::delete().to(handlers_swe::delete_project),
            )
            .route(
                "/swe/projects/{id}/open",
                web::post().to(handlers_swe::open_project),
            )
            .route(
                "/swe/projects/{id}/context",
                web::get().to(handlers_swe::get_context),
            )
            .route(
                "/swe/projects/{id}/execute",
                web::post().to(handlers_swe::execute_tool),
            )
            .route(
                "/swe/projects/{project_id}/memory",
                web::get().to(handlers_swe::list_memory),
            )
            .route(
                "/swe/projects/{project_id}/memory",
                web::post().to(handlers_swe::create_memory),
            )
            .route(
                "/swe/projects/{project_id}/memory/{id}",
                web::get().to(handlers_swe::get_memory),
            )
            .route(
                "/swe/projects/{project_id}/memory/{id}",
                web::put().to(handlers_swe::update_memory),
            )
            .route(
                "/swe/projects/{project_id}/memory/{id}",
                web::delete().to(handlers_swe::delete_memory),
            )
            .route(
                "/swe/projects/{project_id}/rules",
                web::get().to(handlers_swe::list_rules),
            )
            .route(
                "/swe/projects/{project_id}/rules",
                web::post().to(handlers_swe::create_rule),
            )
            .route(
                "/swe/projects/{project_id}/rules/{id}",
                web::put().to(handlers_swe::update_rule),
            )
            .route(
                "/swe/projects/{project_id}/rules/{id}",
                web::delete().to(handlers_swe::delete_rule),
            ),
    );

    eprintln!("[DEBUG] Added /api routes");
}

/// Start the API server
pub async fn start_server(config: ServerConfig) -> Result<()> {
    // Ensure database directory exists
    let db_path = PathBuf::from(&config.database_path);
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Open database
    let db = ChatDatabase::open(&db_path)?;

    // Initialize SWE tables
    {
        let conn = rusqlite::Connection::open(&db_path)?;
        if let Err(e) = handlers_swe::init_swe_tables(&conn) {
            eprintln!("[WARN] Failed to initialize SWE tables: {}", e);
        }
    }

    // Initialize Auth tables
    {
        let conn = rusqlite::Connection::open(&db_path)?;
        if let Err(e) = auth::init_auth_tables(&conn) {
            eprintln!("[WARN] Failed to initialize Auth tables: {}", e);
        }
    }

    let state = web::Data::new(AppState::new(db, db_path));
    let sync_state = web::Data::new(create_sync_state());
    let cors_origins = config.cors_origins.clone();

    println!("[*] CSM API Server starting...");
    println!("   Address: http://{}:{}", config.host, config.port);
    println!("   Database: {}", config.database_path);
    println!();
    println!("[*] Mobile app endpoints:");
    println!("   GET /api/workspaces     - List workspaces");
    println!("   GET /api/sessions       - List sessions");
    println!("   GET /api/sessions/:id   - Get session details");
    println!("   GET /api/stats          - Database statistics");
    println!();
    println!("[*] SWE Mode endpoints:");
    println!("   GET /api/swe/projects   - List SWE projects");
    println!("   POST /api/swe/projects  - Create SWE project");
    println!();
    println!("[*] Sync endpoints:");
    println!("   GET /sync/version       - Get current sync version");
    println!("   GET /sync/delta?from=N  - Get changes since version N");
    println!("   POST /sync/event        - Push a sync event");
    println!("   GET /sync/snapshot      - Get full data snapshot");
    println!("   GET /sync/subscribe     - SSE stream for real-time updates");
    println!();
    println!("Press Ctrl+C to stop the server...");
    println!();

    eprintln!("[DEBUG] Creating HttpServer...");
    let server = HttpServer::new(move || {
        let origins = cors_origins.clone();
        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _req_head| {
                let origin_str = origin.to_str().unwrap_or("");
                origins.iter().any(|allowed| allowed == origin_str)
                    || origin_str.starts_with("http://localhost:")
                    || origin_str.starts_with("http://127.0.0.1:")
                    || origin_str.starts_with("exp://")
            })
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec!["Content-Type", "Authorization", "Accept"])
            .supports_credentials()
            .max_age(3600);

        App::new()
            .app_data(state.clone())
            .app_data(sync_state.clone())
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .configure(configure_routes)
            .configure(configure_sync_routes)
            .configure(configure_auth_routes)
    });

    eprintln!("[DEBUG] Binding to {}:{}...", config.host, config.port);
    let server = server.bind((config.host.as_str(), config.port))?;

    eprintln!("[DEBUG] Starting server...");
    server.run().await?;

    eprintln!("[DEBUG] Server stopped.");
    Ok(())
}
