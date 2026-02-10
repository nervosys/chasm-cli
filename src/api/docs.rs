// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! OpenAPI documentation module
//!
//! Serves the OpenAPI specification and Swagger UI.

use actix_web::{web, HttpResponse, Responder};

/// OpenAPI specification as YAML
const OPENAPI_YAML: &str = include_str!("../../openapi.yaml");

/// Get OpenAPI specification (YAML)
pub async fn openapi_yaml() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/yaml")
        .body(OPENAPI_YAML)
}

/// Get OpenAPI specification (JSON)
pub async fn openapi_json() -> impl Responder {
    // Parse YAML and convert to JSON
    match serde_yaml_to_json(OPENAPI_YAML) {
        Ok(json) => HttpResponse::Ok()
            .content_type("application/json")
            .body(json),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error: {}", e)),
    }
}

/// Swagger UI HTML page
pub async fn swagger_ui() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(SWAGGER_UI_HTML)
}

/// Convert YAML to JSON string
fn serde_yaml_to_json(yaml: &str) -> Result<String, String> {
    // Simple YAML to JSON conversion using serde_json
    let value: serde_json::Value =
        serde_yaml::from_str(yaml).map_err(|e| format!("YAML parse error: {}", e))?;
    serde_json::to_string_pretty(&value).map_err(|e| format!("JSON serialize error: {}", e))
}

/// Configure documentation routes
pub fn configure_docs_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/docs")
            .route("", web::get().to(swagger_ui))
            .route("/", web::get().to(swagger_ui))
            .route("/openapi.yaml", web::get().to(openapi_yaml))
            .route("/openapi.json", web::get().to(openapi_json)),
    );
}

/// Embedded Swagger UI HTML
const SWAGGER_UI_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Chasm API Documentation</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui.css">
    <style>
        body {
            margin: 0;
            padding: 0;
        }
        .swagger-ui .topbar {
            display: none;
        }
        .swagger-ui .info {
            margin: 20px 0;
        }
        .swagger-ui .info .title {
            color: #3b4151;
        }
        .swagger-ui .info hgroup.main {
            margin: 0;
        }
        .custom-header {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 20px 40px;
            display: flex;
            align-items: center;
            gap: 20px;
        }
        .custom-header h1 {
            margin: 0;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            font-size: 24px;
            font-weight: 600;
        }
        .custom-header .version {
            background: rgba(255,255,255,0.2);
            padding: 4px 12px;
            border-radius: 12px;
            font-size: 14px;
        }
        .custom-header a {
            color: white;
            text-decoration: none;
            margin-left: auto;
            opacity: 0.9;
        }
        .custom-header a:hover {
            opacity: 1;
        }
    </style>
</head>
<body>
    <div class="custom-header">
        <h1>🔗 Chasm API</h1>
        <span class="version">v1.3.0</span>
        <a href="https://github.com/nervosys/chasm" target="_blank">GitHub →</a>
    </div>
    <div id="swagger-ui"></div>
    <script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script>
        window.onload = function() {
            SwaggerUIBundle({
                url: "/docs/openapi.yaml",
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIBundle.SwaggerUIStandalonePreset
                ],
                layout: "BaseLayout",
                defaultModelsExpandDepth: 1,
                docExpansion: "list",
                filter: true,
                showExtensions: true,
                showCommonExtensions: true,
                syntaxHighlight: {
                    theme: "monokai"
                }
            });
        };
    </script>
</body>
</html>
"#;
