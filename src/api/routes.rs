// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! API route configuration

use actix_web::web;

use super::handlers;

/// Configure all API routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            // Health check
            .route("/health", web::get().to(handlers::health_check))
            // System
            .service(
                web::scope("/system")
                    .route("/info", web::get().to(handlers::system_info))
                    .route("/vacuum", web::post().to(handlers::vacuum_database))
                    .route("/cache/clear", web::post().to(handlers::clear_cache)),
            )
            // Workspaces
            .service(
                web::scope("/workspaces")
                    .route("", web::get().to(handlers::list_workspaces))
                    .route("", web::post().to(handlers::create_workspace))
                    .route("/discover", web::post().to(handlers::discover_workspaces))
                    .route("/by-path", web::get().to(handlers::get_workspace_by_path))
                    .route("/{id}", web::get().to(handlers::get_workspace))
                    .route("/{id}", web::put().to(handlers::update_workspace))
                    .route("/{id}", web::delete().to(handlers::delete_workspace))
                    .route("/{id}/refresh", web::post().to(handlers::refresh_workspace))
                    .route("/{id}/link", web::post().to(handlers::link_workspace))
                    .route("/{id}/git", web::get().to(handlers::get_workspace_git)),
            )
            // Sessions
            .service(
                web::scope("/sessions")
                    .route("", web::get().to(handlers::list_sessions))
                    .route("", web::post().to(handlers::create_session))
                    .route("/merge", web::post().to(handlers::merge_sessions))
                    .route("/{id}", web::get().to(handlers::get_session))
                    .route("/{id}", web::put().to(handlers::update_session))
                    .route("/{id}", web::delete().to(handlers::delete_session))
                    .route("/{id}/archive", web::post().to(handlers::archive_session))
                    .route("/{id}/fork", web::post().to(handlers::fork_session))
                    .route("/{id}/export", web::get().to(handlers::export_session))
                    .route("/{id}/messages", web::get().to(handlers::list_messages))
                    .route("/{id}/messages", web::post().to(handlers::create_message))
                    .route("/{id}/messages/{msg_id}", web::get().to(handlers::get_message))
                    .route("/{id}/messages/{msg_id}", web::put().to(handlers::update_message))
                    .route("/{id}/messages/{msg_id}", web::delete().to(handlers::delete_message))
                    .route("/{id}/messages/{msg_id}/regenerate", web::post().to(handlers::regenerate_message))
                    .route("/{id}/checkpoints", web::get().to(handlers::list_checkpoints))
                    .route("/{id}/checkpoints", web::post().to(handlers::create_checkpoint))
                    .route("/{id}/share", web::get().to(handlers::list_share_links))
                    .route("/{id}/share", web::post().to(handlers::create_share_link))
                    .route("/{id}/commits", web::get().to(handlers::list_session_commits)),
            )
            // Providers
            .service(
                web::scope("/providers")
                    .route("", web::get().to(handlers::list_providers))
                    .route("", web::post().to(handlers::create_provider))
                    .route("/health", web::get().to(handlers::provider_health_check))
                    .route("/{id}", web::get().to(handlers::get_provider))
                    .route("/{id}", web::put().to(handlers::update_provider))
                    .route("/{id}", web::delete().to(handlers::delete_provider))
                    .route("/{id}/health", web::get().to(handlers::check_provider_health))
                    .route("/{id}/models", web::get().to(handlers::list_provider_models))
                    .route("/{id}/test", web::post().to(handlers::test_provider)),
            )
            // Agents
            .service(
                web::scope("/agents")
                    .route("", web::get().to(handlers::list_agents))
                    .route("", web::post().to(handlers::create_agent))
                    .route("/{id}", web::get().to(handlers::get_agent))
                    .route("/{id}", web::put().to(handlers::update_agent))
                    .route("/{id}", web::delete().to(handlers::delete_agent))
                    .route("/{id}/clone", web::post().to(handlers::clone_agent)),
            )
            // Swarms
            .service(
                web::scope("/swarms")
                    .route("", web::get().to(handlers::list_swarms))
                    .route("", web::post().to(handlers::create_swarm))
                    .route("/{id}", web::get().to(handlers::get_swarm))
                    .route("/{id}", web::put().to(handlers::update_swarm))
                    .route("/{id}", web::delete().to(handlers::delete_swarm))
                    .route("/{id}/start", web::post().to(handlers::start_swarm))
                    .route("/{id}/pause", web::post().to(handlers::pause_swarm))
                    .route("/{id}/resume", web::post().to(handlers::resume_swarm))
                    .route("/{id}/stop", web::post().to(handlers::stop_swarm)),
            )
            // Chat completions
            .service(
                web::scope("/chat")
                    .route("/completions", web::post().to(handlers::chat_completions)),
            )
            // Search
            .service(
                web::scope("/search")
                    .route("", web::get().to(handlers::search_all))
                    .route("/sessions", web::get().to(handlers::search_sessions))
                    .route("/messages", web::get().to(handlers::search_messages))
                    .route("/semantic", web::get().to(handlers::semantic_search)),
            )
            // Statistics
            .service(
                web::scope("/stats")
                    .route("/overview", web::get().to(handlers::stats_overview))
                    .route("/workspace/{id}", web::get().to(handlers::stats_workspace))
                    .route("/providers", web::get().to(handlers::stats_providers))
                    .route("/timeline", web::get().to(handlers::stats_timeline)),
            )
            // Import/Export
            .route("/import", web::post().to(handlers::import_sessions))
            .route("/export", web::post().to(handlers::export_sessions))
            .route("/harvest", web::post().to(handlers::harvest_sessions))
            .route("/sync", web::post().to(handlers::sync_sessions))
            // Settings
            .service(
                web::scope("/settings")
                    .route("", web::get().to(handlers::get_settings))
                    .route("", web::put().to(handlers::update_settings))
                    .route("/accounts", web::get().to(handlers::list_accounts))
                    .route("/accounts", web::post().to(handlers::add_account))
                    .route("/accounts/{id}", web::delete().to(handlers::remove_account)),
            ),
    );
}
