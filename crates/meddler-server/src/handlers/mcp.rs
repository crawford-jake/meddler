use std::convert::Infallible;

use axum::{
    extract::State,
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Response, Sse,
    },
    Json,
};
use serde_json::Value;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

use meddler_core::types::{CreateMessage, CreateTask, MessageFilter};
use meddler_mcp::jsonrpc::{INTERNAL_ERROR, INVALID_PARAMS, METHOD_NOT_FOUND};
use meddler_mcp::{JsonRpcRequest, JsonRpcResponse, ToolRegistry};

use crate::app_state::AppState;

const MCP_ORCHESTRATOR_NAME: &str = "__orchestrator__";

/// SSE stream for the orchestrator (Cursor/Claude Desktop).
///
/// Kept for the legacy MCP SSE transport. The primary transport is now
/// Streamable HTTP (POST to the same URL returns JSON directly).
pub async fn mcp_sse(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    // Register the orchestrator as a special agent if not exists
    let _ = state
        .agent_registry
        .register(meddler_core::types::RegisterAgent {
            name: MCP_ORCHESTRATOR_NAME.to_string(),
            description: "MCP orchestrator (Cursor/Claude Desktop)".to_string(),
        })
        .await;

    tracing::info!("Orchestrator connected via MCP SSE");

    let rx = state.sessions.subscribe(MCP_ORCHESTRATOR_NAME).await;

    // Send initial endpoint event as required by MCP SSE spec.
    // The endpoint tells the client where to POST JSON-RPC requests.
    let init_stream = tokio_stream::once(Ok(Event::default()
        .event("endpoint")
        .data("/mcp/sse")));

    let message_stream = BroadcastStream::new(rx).filter_map(|result| {
        result.ok().map(|msg| {
            // Wrap message as an MCP notification
            let notification = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/message",
                "params": {
                    "message": *msg,
                }
            });
            Ok(Event::default()
                .event("message")
                .json_data(&notification)
                .unwrap_or_else(|_| Event::default().data("error")))
        })
    });

    let stream = init_stream.chain(message_stream);

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Handle MCP JSON-RPC requests from the orchestrator (Streamable HTTP).
///
/// Returns JSON-RPC response directly in the HTTP body for requests,
/// or 202 Accepted for notifications (no `id` field).
#[allow(clippy::missing_errors_doc)]
pub async fn mcp_request(
    State(state): State<AppState>,
    Json(req): Json<JsonRpcRequest>,
) -> Response {
    tracing::info!("MCP request: method={}", req.method);

    // Handle notifications (no id / null id) - no response needed
    if req.id.is_null() {
        tracing::info!("Received MCP notification: {}", req.method);
        return StatusCode::ACCEPTED.into_response();
    }

    // Handle notifications/initialized (has id but is an ack, return empty success)
    if req.method == "notifications/initialized" {
        return StatusCode::ACCEPTED.into_response();
    }

    // Ensure orchestrator agent is registered
    let _ = state
        .agent_registry
        .register(meddler_core::types::RegisterAgent {
            name: MCP_ORCHESTRATOR_NAME.to_string(),
            description: "MCP orchestrator (Cursor/Claude Desktop)".to_string(),
        })
        .await;

    let response = match req.method.as_str() {
        "initialize" => handle_initialize(&req),
        "tools/list" => handle_tools_list(&req),
        "tools/call" => handle_tools_call(&state, &req).await,
        _ => JsonRpcResponse::error(req.id, METHOD_NOT_FOUND, "Method not found"),
    };

    Json(response).into_response()
}

fn handle_initialize(req: &JsonRpcRequest) -> JsonRpcResponse {
    JsonRpcResponse::success(
        req.id.clone(),
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "meddler",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

fn handle_tools_list(req: &JsonRpcRequest) -> JsonRpcResponse {
    let tools = ToolRegistry::definitions();
    JsonRpcResponse::success(
        req.id.clone(),
        serde_json::json!({ "tools": tools }),
    )
}

async fn handle_tools_call(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let Some(params) = &req.params else {
        return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, "Missing params");
    };

    let tool_name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or(Value::Object(serde_json::Map::new()));

    let result = match tool_name {
        "list_agents" => tool_list_agents(state).await,
        "send_message" => tool_send_message(state, &arguments).await,
        "get_messages" => tool_get_messages(state, &arguments).await,
        "create_task" => tool_create_task(state, &arguments).await,
        "get_task_status" => tool_get_task_status(state, &arguments).await,
        _ => Err(format!("Unknown tool: {tool_name}")),
    };

    match result {
        Ok(value) => JsonRpcResponse::success(
            req.id.clone(),
            serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&value).unwrap_or_default()
                }]
            }),
        ),
        Err(err) => JsonRpcResponse::error(req.id.clone(), INTERNAL_ERROR, err),
    }
}

async fn tool_list_agents(state: &AppState) -> Result<Value, String> {
    let agents = state
        .agent_registry
        .list()
        .await
        .map_err(|e| e.to_string())?;

    // Filter out the internal orchestrator agent
    let mut agent_list = Vec::new();
    for a in agents {
        if a.name == MCP_ORCHESTRATOR_NAME {
            continue;
        }
        let connected = state.sessions.is_connected(&a.name).await;
        agent_list.push(serde_json::json!({
            "name": a.name,
            "description": a.description,
            "connected": connected,
        }));
    }
    let agents = agent_list;

    Ok(serde_json::json!({ "agents": agents }))
}

async fn tool_send_message(state: &AppState, args: &Value) -> Result<Value, String> {
    let to = args
        .get("to")
        .and_then(Value::as_str)
        .ok_or("Missing 'to' parameter")?;

    let content = args
        .get("content")
        .and_then(Value::as_str)
        .ok_or("Missing 'content' parameter")?;

    let task_id = args
        .get("task_id")
        .and_then(Value::as_str)
        .map(|s| {
            s.parse::<uuid::Uuid>()
                .map(meddler_core::types::TaskId)
                .map_err(|e| format!("Invalid task_id: {e}"))
        })
        .transpose()?;

    // Resolve orchestrator as sender
    let sender = state
        .agent_registry
        .get_by_name(MCP_ORCHESTRATOR_NAME)
        .await
        .map_err(|e| e.to_string())?;

    // Resolve recipient
    let recipient = state
        .agent_registry
        .get_by_name(to)
        .await
        .map_err(|e| format!("Recipient agent '{to}' not found: {e}"))?;

    // If there's a task, mark it as started
    if let Some(tid) = task_id {
        let _ = state.task_store.mark_started(tid).await;
    }

    let message = state
        .message_store
        .create(CreateMessage {
            sender_id: sender.id,
            recipient_id: recipient.id,
            task_id,
            content: content.to_string(),
        })
        .await
        .map_err(|e| e.to_string())?;

    // Push to recipient's SSE
    let delivered = state.sessions.notify(to, message.clone()).await;

    Ok(serde_json::json!({
        "message_id": message.id,
        "delivered": delivered,
    }))
}

async fn tool_get_messages(state: &AppState, args: &Value) -> Result<Value, String> {
    let task_id = args
        .get("task_id")
        .and_then(Value::as_str)
        .map(|s| {
            s.parse::<uuid::Uuid>()
                .map(meddler_core::types::TaskId)
                .map_err(|e| format!("Invalid task_id: {e}"))
        })
        .transpose()?;

    let sender_id = if let Some(name) = args.get("sender").and_then(Value::as_str) {
        Some(
            state
                .agent_registry
                .get_by_name(name)
                .await
                .map_err(|e| format!("Sender '{name}' not found: {e}"))?
                .id,
        )
    } else {
        None
    };

    let recipient_id = if let Some(name) = args.get("recipient").and_then(Value::as_str) {
        Some(
            state
                .agent_registry
                .get_by_name(name)
                .await
                .map_err(|e| format!("Recipient '{name}' not found: {e}"))?
                .id,
        )
    } else {
        None
    };

    let messages = state
        .message_store
        .query(MessageFilter {
            task_id,
            sender_id,
            recipient_id,
        })
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "messages": messages }))
}

async fn tool_create_task(state: &AppState, args: &Value) -> Result<Value, String> {
    let title = args
        .get("title")
        .and_then(Value::as_str)
        .ok_or("Missing 'title' parameter")?;

    let time_budget_secs = args.get("time_budget_secs").and_then(Value::as_i64);

    // Resolve orchestrator as creator
    let creator = state
        .agent_registry
        .get_by_name(MCP_ORCHESTRATOR_NAME)
        .await
        .map_err(|e| e.to_string())?;

    let task = state
        .task_store
        .create(CreateTask {
            title: title.to_string(),
            created_by: creator.id,
            time_budget_secs,
        })
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "task_id": task.id,
        "title": task.title,
    }))
}

async fn tool_get_task_status(state: &AppState, args: &Value) -> Result<Value, String> {
    let task_id = args
        .get("task_id")
        .and_then(Value::as_str)
        .ok_or("Missing 'task_id' parameter")?;

    let id: uuid::Uuid = task_id
        .parse()
        .map_err(|e| format!("Invalid task_id: {e}"))?;

    let status = state
        .task_store
        .get_status(meddler_core::types::TaskId(id))
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!(status))
}
