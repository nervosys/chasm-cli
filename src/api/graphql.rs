// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! GraphQL API module
//!
//! Provides a GraphQL endpoint alongside REST for flexible querying.

use actix_web::{web, HttpResponse, Responder};
use async_graphql::{
    Context, EmptySubscription, FieldResult, InputObject, Object, Schema, SimpleObject, ID,
};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};
use chrono::{DateTime, Utc};
use std::sync::Arc;

use super::state::AppState;

/// GraphQL schema type
pub type ChasmSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

// ============================================================================
// GraphQL Types
// ============================================================================

/// Workspace type for GraphQL
#[derive(SimpleObject, Clone)]
pub struct Workspace {
    /// Unique identifier
    pub id: ID,
    /// Workspace name
    pub name: String,
    /// Workspace path
    pub path: String,
    /// Provider (e.g., "vscode", "cursor")
    pub provider: String,
    /// Number of sessions
    pub session_count: i32,
    /// Last harvested timestamp
    pub last_harvested: Option<DateTime<Utc>>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Session type for GraphQL
#[derive(SimpleObject, Clone)]
pub struct Session {
    /// Unique identifier
    pub id: ID,
    /// Session title
    pub title: String,
    /// Workspace ID
    pub workspace_id: Option<ID>,
    /// Provider (e.g., "copilot", "cursor")
    pub provider: String,
    /// Model used
    pub model: Option<String>,
    /// Number of messages
    pub message_count: i32,
    /// Total tokens
    pub token_count: i32,
    /// Whether archived
    pub archived: bool,
    /// Tags
    pub tags: Vec<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Message type for GraphQL
#[derive(SimpleObject, Clone)]
pub struct Message {
    /// Unique identifier
    pub id: ID,
    /// Session ID
    pub session_id: ID,
    /// Message role (user, assistant, system)
    pub role: String,
    /// Message content
    pub content: String,
    /// Model used
    pub model: Option<String>,
    /// Token count
    pub token_count: i32,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Provider type for GraphQL
#[derive(SimpleObject, Clone)]
pub struct Provider {
    /// Unique identifier
    pub id: ID,
    /// Provider name
    pub name: String,
    /// Provider type
    pub provider_type: String,
    /// Whether enabled
    pub enabled: bool,
    /// Base URL
    pub base_url: Option<String>,
}

/// Agent type for GraphQL
#[derive(SimpleObject, Clone)]
pub struct Agent {
    /// Unique identifier
    pub id: ID,
    /// Agent name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// System prompt
    pub system_prompt: Option<String>,
    /// Provider ID
    pub provider_id: Option<ID>,
    /// Model
    pub model: String,
    /// Temperature
    pub temperature: f64,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Statistics overview
#[derive(SimpleObject, Clone)]
pub struct StatsOverview {
    /// Total sessions
    pub total_sessions: i32,
    /// Total messages
    pub total_messages: i32,
    /// Total tokens
    pub total_tokens: i64,
    /// Active providers
    pub active_providers: i32,
    /// Total workspaces
    pub workspaces: i32,
    /// Sessions today
    pub sessions_today: i32,
    /// Messages today
    pub messages_today: i32,
}

/// Search result
#[derive(SimpleObject, Clone)]
pub struct SearchResult {
    /// Sessions matching query
    pub sessions: Vec<Session>,
    /// Messages matching query
    pub messages: Vec<Message>,
    /// Total count
    pub total: i32,
}

// ============================================================================
// Input Types
// ============================================================================

/// Input for creating a session
#[derive(InputObject)]
pub struct CreateSessionInput {
    pub title: String,
    pub workspace_id: Option<ID>,
    pub provider: String,
}

/// Input for updating a session
#[derive(InputObject)]
pub struct UpdateSessionInput {
    pub title: Option<String>,
    pub tags: Option<Vec<String>>,
    pub archived: Option<bool>,
}

/// Input for creating an agent
#[derive(InputObject)]
pub struct CreateAgentInput {
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub provider_id: Option<ID>,
    pub model: String,
    pub temperature: Option<f64>,
}

/// Filter for sessions
#[derive(InputObject, Default)]
pub struct SessionFilter {
    pub workspace_id: Option<ID>,
    pub provider: Option<String>,
    pub archived: Option<bool>,
    pub search: Option<String>,
}

/// Pagination input
#[derive(InputObject, Default)]
pub struct Pagination {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// ============================================================================
// Query Root
// ============================================================================

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get all workspaces
    async fn workspaces(
        &self,
        ctx: &Context<'_>,
        pagination: Option<Pagination>,
    ) -> FieldResult<Vec<Workspace>> {
        let _state = ctx.data::<Arc<AppState>>()?;
        let pagination = pagination.unwrap_or_default();
        let _limit = pagination.limit.unwrap_or(20);
        let _offset = pagination.offset.unwrap_or(0);

        // TODO: Implement actual database query
        Ok(vec![])
    }

    /// Get a workspace by ID
    async fn workspace(&self, ctx: &Context<'_>, id: ID) -> FieldResult<Option<Workspace>> {
        let _state = ctx.data::<Arc<AppState>>()?;
        let _id = id.to_string();

        // TODO: Implement actual database query
        Ok(None)
    }

    /// Get all sessions with optional filtering
    async fn sessions(
        &self,
        ctx: &Context<'_>,
        filter: Option<SessionFilter>,
        pagination: Option<Pagination>,
    ) -> FieldResult<Vec<Session>> {
        let _state = ctx.data::<Arc<AppState>>()?;
        let _filter = filter.unwrap_or_default();
        let pagination = pagination.unwrap_or_default();
        let _limit = pagination.limit.unwrap_or(20);
        let _offset = pagination.offset.unwrap_or(0);

        // TODO: Implement actual database query
        Ok(vec![])
    }

    /// Get a session by ID
    async fn session(&self, ctx: &Context<'_>, id: ID) -> FieldResult<Option<Session>> {
        let _state = ctx.data::<Arc<AppState>>()?;
        let _id = id.to_string();

        // TODO: Implement actual database query
        Ok(None)
    }

    /// Get messages for a session
    async fn messages(
        &self,
        ctx: &Context<'_>,
        session_id: ID,
        pagination: Option<Pagination>,
    ) -> FieldResult<Vec<Message>> {
        let _state = ctx.data::<Arc<AppState>>()?;
        let _session_id = session_id.to_string();
        let pagination = pagination.unwrap_or_default();
        let _limit = pagination.limit.unwrap_or(100);
        let _offset = pagination.offset.unwrap_or(0);

        // TODO: Implement actual database query
        Ok(vec![])
    }

    /// Get all providers
    async fn providers(&self, ctx: &Context<'_>) -> FieldResult<Vec<Provider>> {
        let _state = ctx.data::<Arc<AppState>>()?;

        // Return common providers
        Ok(vec![
            Provider {
                id: "copilot".into(),
                name: "GitHub Copilot".to_string(),
                provider_type: "copilot".to_string(),
                enabled: true,
                base_url: None,
            },
            Provider {
                id: "cursor".into(),
                name: "Cursor".to_string(),
                provider_type: "cursor".to_string(),
                enabled: true,
                base_url: None,
            },
            Provider {
                id: "chatgpt".into(),
                name: "ChatGPT".to_string(),
                provider_type: "chatgpt".to_string(),
                enabled: true,
                base_url: Some("https://chatgpt.com".to_string()),
            },
        ])
    }

    /// Get all agents
    async fn agents(&self, ctx: &Context<'_>) -> FieldResult<Vec<Agent>> {
        let _state = ctx.data::<Arc<AppState>>()?;

        // TODO: Implement actual database query
        Ok(vec![])
    }

    /// Get an agent by ID
    async fn agent(&self, ctx: &Context<'_>, id: ID) -> FieldResult<Option<Agent>> {
        let _state = ctx.data::<Arc<AppState>>()?;
        let _id = id.to_string();

        // TODO: Implement actual database query
        Ok(None)
    }

    /// Get statistics overview
    async fn stats(&self, ctx: &Context<'_>) -> FieldResult<StatsOverview> {
        let _state = ctx.data::<Arc<AppState>>()?;

        // TODO: Implement actual database query
        Ok(StatsOverview {
            total_sessions: 0,
            total_messages: 0,
            total_tokens: 0,
            active_providers: 3,
            workspaces: 0,
            sessions_today: 0,
            messages_today: 0,
        })
    }

    /// Search across sessions and messages
    async fn search(
        &self,
        ctx: &Context<'_>,
        query: String,
        limit: Option<i32>,
    ) -> FieldResult<SearchResult> {
        let _state = ctx.data::<Arc<AppState>>()?;
        let _limit = limit.unwrap_or(20);
        let _query = query;

        // TODO: Implement actual search
        Ok(SearchResult {
            sessions: vec![],
            messages: vec![],
            total: 0,
        })
    }
}

// ============================================================================
// Mutation Root
// ============================================================================

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Create a new session
    async fn create_session(
        &self,
        ctx: &Context<'_>,
        input: CreateSessionInput,
    ) -> FieldResult<Session> {
        let _state = ctx.data::<Arc<AppState>>()?;

        // TODO: Implement actual creation
        Ok(Session {
            id: uuid::Uuid::new_v4().to_string().into(),
            title: input.title,
            workspace_id: input.workspace_id,
            provider: input.provider,
            model: None,
            message_count: 0,
            token_count: 0,
            archived: false,
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Update a session
    async fn update_session(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateSessionInput,
    ) -> FieldResult<Session> {
        let _state = ctx.data::<Arc<AppState>>()?;
        let _id = id.to_string();

        // TODO: Implement actual update
        Ok(Session {
            id,
            title: input.title.unwrap_or_default(),
            workspace_id: None,
            provider: "unknown".to_string(),
            model: None,
            message_count: 0,
            token_count: 0,
            archived: input.archived.unwrap_or(false),
            tags: input.tags.unwrap_or_default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Delete a session
    async fn delete_session(&self, ctx: &Context<'_>, id: ID) -> FieldResult<bool> {
        let _state = ctx.data::<Arc<AppState>>()?;
        let _id = id.to_string();

        // TODO: Implement actual deletion
        Ok(true)
    }

    /// Archive a session
    async fn archive_session(&self, ctx: &Context<'_>, id: ID) -> FieldResult<Session> {
        let _state = ctx.data::<Arc<AppState>>()?;

        // TODO: Implement actual archiving
        Ok(Session {
            id,
            title: "Archived".to_string(),
            workspace_id: None,
            provider: "unknown".to_string(),
            model: None,
            message_count: 0,
            token_count: 0,
            archived: true,
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Create an agent
    async fn create_agent(&self, ctx: &Context<'_>, input: CreateAgentInput) -> FieldResult<Agent> {
        let _state = ctx.data::<Arc<AppState>>()?;

        Ok(Agent {
            id: uuid::Uuid::new_v4().to_string().into(),
            name: input.name,
            description: input.description,
            system_prompt: input.system_prompt,
            provider_id: input.provider_id,
            model: input.model,
            temperature: input.temperature.unwrap_or(0.7),
            created_at: Utc::now(),
        })
    }

    /// Delete an agent
    async fn delete_agent(&self, ctx: &Context<'_>, id: ID) -> FieldResult<bool> {
        let _state = ctx.data::<Arc<AppState>>()?;
        let _id = id.to_string();

        // TODO: Implement actual deletion
        Ok(true)
    }

    /// Trigger a harvest
    async fn harvest(&self, ctx: &Context<'_>, providers: Option<Vec<String>>) -> FieldResult<i32> {
        let _state = ctx.data::<Arc<AppState>>()?;
        let _providers = providers;

        // TODO: Implement actual harvest
        Ok(0)
    }

    /// Trigger sync
    async fn sync(&self, ctx: &Context<'_>) -> FieldResult<bool> {
        let _state = ctx.data::<Arc<AppState>>()?;

        // TODO: Implement actual sync
        Ok(true)
    }
}

// ============================================================================
// HTTP Handlers
// ============================================================================

/// GraphQL endpoint handler
pub async fn graphql_handler(
    schema: web::Data<ChasmSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

/// GraphQL Playground UI
pub async fn graphql_playground() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(GRAPHQL_PLAYGROUND_HTML)
}

/// GraphQL introspection endpoint
pub async fn graphql_sdl(schema: web::Data<ChasmSchema>) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/plain")
        .body(schema.sdl())
}

/// Create the GraphQL schema
pub fn create_schema(state: Arc<AppState>) -> ChasmSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(state)
        .finish()
}

/// Configure GraphQL routes
pub fn configure_graphql_routes(cfg: &mut web::ServiceConfig, schema: ChasmSchema) {
    cfg.app_data(web::Data::new(schema))
        .service(
            web::resource("/graphql")
                .route(web::get().to(graphql_handler))
                .route(web::post().to(graphql_handler)),
        )
        .route("/graphql/playground", web::get().to(graphql_playground))
        .route("/graphql/sdl", web::get().to(graphql_sdl));
}

/// Embedded GraphQL Playground HTML
const GRAPHQL_PLAYGROUND_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Chasm GraphQL Playground</title>
    <style>
        body {
            margin: 0;
            padding: 0;
            height: 100vh;
            overflow: hidden;
        }
        .custom-header {
            background: linear-gradient(135deg, #e535ab 0%, #9c27b0 100%);
            color: white;
            padding: 12px 24px;
            display: flex;
            align-items: center;
            gap: 16px;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
        }
        .custom-header h1 {
            margin: 0;
            font-size: 20px;
            font-weight: 600;
        }
        .custom-header .badge {
            background: rgba(255,255,255,0.2);
            padding: 4px 12px;
            border-radius: 12px;
            font-size: 12px;
        }
        .custom-header a {
            color: white;
            text-decoration: none;
            margin-left: auto;
            opacity: 0.9;
            font-size: 14px;
        }
        .custom-header a:hover {
            opacity: 1;
        }
        #graphiql {
            height: calc(100vh - 48px);
        }
    </style>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/graphiql@3/graphiql.min.css" />
</head>
<body>
    <div class="custom-header">
        <h1>◈ Chasm GraphQL</h1>
        <span class="badge">v1.3.0</span>
        <a href="/docs">REST API →</a>
    </div>
    <div id="graphiql"></div>
    <script crossorigin src="https://cdn.jsdelivr.net/npm/react@18/umd/react.production.min.js"></script>
    <script crossorigin src="https://cdn.jsdelivr.net/npm/react-dom@18/umd/react-dom.production.min.js"></script>
    <script crossorigin src="https://cdn.jsdelivr.net/npm/graphiql@3/graphiql.min.js"></script>
    <script>
        const fetcher = GraphiQL.createFetcher({
            url: '/graphql',
        });

        const root = ReactDOM.createRoot(document.getElementById('graphiql'));
        root.render(
            React.createElement(GraphiQL, {
                fetcher,
                defaultEditorToolsVisibility: true,
            })
        );
    </script>
</body>
</html>
"#;
