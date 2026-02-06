use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

use meddler_server::app_state::AppState;
use meddler_server::session;

use meddler_store::PgStore;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://meddler:meddler@localhost:5432/meddler".to_string()
        });
    let host = std::env::var("MEDDLER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("MEDDLER_PORT").unwrap_or_else(|_| "3000".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    let store = PgStore::new(pool);
    store.migrate().await.expect("Failed to run migrations");

    let state = AppState {
        agent_registry: Arc::new(store.clone()),
        message_store: Arc::new(store.clone()),
        task_store: Arc::new(store),
        sessions: Arc::new(session::SessionManager::new()),
    };

    let app = meddler_server::router::create_router(state);

    let addr = format!("{host}:{port}");
    tracing::info!("Meddler server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
