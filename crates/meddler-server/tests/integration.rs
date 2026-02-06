use std::sync::Arc;

use axum_test::TestServer;

mod mock_stores;
use mock_stores::{MockAgentRegistry, MockMessageStore, MockTaskStore};

fn build_test_app() -> TestServer {
    let agent_registry = Arc::new(MockAgentRegistry::new());
    let message_store = Arc::new(MockMessageStore::new());
    let task_store = Arc::new(MockTaskStore::new());

    let state = meddler_server::app_state::AppState {
        agent_registry,
        message_store,
        task_store,
        sessions: Arc::new(meddler_server::session::SessionManager::new()),
    };

    let app = meddler_server::router::create_router(state);
    TestServer::new(app).unwrap()
}

#[tokio::test]
async fn health_check() {
    let server = build_test_app();
    let resp = server.get("/health").await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn register_agent() {
    let server = build_test_app();

    let resp = server
        .post("/agent/register")
        .json(&serde_json::json!({
            "name": "test-agent",
            "description": "A test agent"
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["name"], "test-agent");
    assert!(body.get("agent_id").is_some());
}

#[tokio::test]
async fn register_agent_idempotent() {
    let server = build_test_app();

    let resp1 = server
        .post("/agent/register")
        .json(&serde_json::json!({
            "name": "test-agent",
            "description": "A test agent"
        }))
        .await;
    resp1.assert_status_ok();
    let body1: serde_json::Value = resp1.json();

    let resp2 = server
        .post("/agent/register")
        .json(&serde_json::json!({
            "name": "test-agent",
            "description": "Updated description"
        }))
        .await;
    resp2.assert_status_ok();
    let body2: serde_json::Value = resp2.json();

    // Same agent_id returned
    assert_eq!(body1["agent_id"], body2["agent_id"]);
}

#[tokio::test]
async fn send_message_between_agents() {
    let server = build_test_app();

    // Register sender and recipient
    server
        .post("/agent/register")
        .json(&serde_json::json!({
            "name": "sender",
            "description": "Sender agent"
        }))
        .await
        .assert_status_ok();

    server
        .post("/agent/register")
        .json(&serde_json::json!({
            "name": "recipient",
            "description": "Recipient agent"
        }))
        .await
        .assert_status_ok();

    // Send message
    let resp = server
        .post("/agent/message")
        .json(&serde_json::json!({
            "from": "sender",
            "to": "recipient",
            "content": "Hello from sender!"
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body.get("message_id").is_some());
}

#[tokio::test]
async fn send_message_unknown_recipient_returns_404() {
    let server = build_test_app();

    server
        .post("/agent/register")
        .json(&serde_json::json!({
            "name": "sender",
            "description": "Sender agent"
        }))
        .await
        .assert_status_ok();

    let resp = server
        .post("/agent/message")
        .json(&serde_json::json!({
            "from": "sender",
            "to": "nonexistent",
            "content": "Hello?"
        }))
        .await;

    resp.assert_status_not_found();
}

#[tokio::test]
async fn mcp_initialize() {
    let server = build_test_app();

    let resp = server
        .post("/mcp")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["result"]["serverInfo"]["name"], "meddler");
}

#[tokio::test]
async fn mcp_tools_list() {
    let server = build_test_app();

    let resp = server
        .post("/mcp")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let tools = body["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 5);
}

#[tokio::test]
async fn mcp_unknown_method() {
    let server = build_test_app();

    let resp = server
        .post("/mcp")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "nonexistent/method",
            "params": {}
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body.get("error").is_some());
    assert_eq!(body["error"]["code"], -32601);
}

#[tokio::test]
async fn mcp_streamable_http_initialize() {
    let server = build_test_app();

    // POST to /mcp/sse (Streamable HTTP transport) should work the same as /mcp
    let resp = server
        .post("/mcp/sse")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["result"]["serverInfo"]["name"], "meddler");
}

#[tokio::test]
async fn mcp_notification_returns_accepted() {
    let server = build_test_app();

    // Notifications have null id and should return 202 Accepted
    let resp = server
        .post("/mcp/sse")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::ACCEPTED);
}
