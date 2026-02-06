use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;

use crate::app_state::AppState;
use crate::handlers;

/// Create the main application router with all routes.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(handlers::health))
        // MCP endpoints (for orchestrator via Cursor/Claude Desktop)
        // Supports both Streamable HTTP (POST) and legacy SSE (GET) on the same URL.
        .route("/mcp/sse", get(handlers::mcp_sse).post(handlers::mcp_request))
        .route("/mcp", post(handlers::mcp_request))
        // Agent endpoints (for worker agents via CLI)
        .route("/agent/register", post(handlers::agent_register))
        .route("/agent/sse/{name}", get(handlers::agent_sse))
        .route("/agent/message", post(handlers::agent_message))
        // CORS: allow any origin (MCP clients like Cursor may run in various contexts)
        .layer(CorsLayer::permissive())
        .with_state(state)
}
