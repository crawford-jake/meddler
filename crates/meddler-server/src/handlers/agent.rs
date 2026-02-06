use std::convert::Infallible;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
    Json,
};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

use meddler_core::types::{CreateMessage, RegisterAgent};

use crate::app_state::AppState;
use crate::session::SseEvent;

/// Request body for registering a worker agent.
#[derive(serde::Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub description: String,
}

/// Request body for a worker agent sending a message.
#[derive(serde::Deserialize)]
pub struct AgentMessageRequest {
    pub from: String,
    pub to: String,
    pub content: String,
    pub task_id: Option<String>,
}

/// Register a worker agent (called by CLI).
#[allow(clippy::missing_errors_doc)]
pub async fn agent_register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let agent = state
        .agent_registry
        .register(RegisterAgent {
            name: req.name,
            description: req.description,
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "agent_id": agent.id,
        "name": agent.name,
    })))
}

/// SSE stream for a worker agent to receive messages.
#[allow(clippy::missing_errors_doc)]
pub async fn agent_sse(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, (StatusCode, String)>
{
    // Verify agent exists
    let agent = state
        .agent_registry
        .get_by_name(&name)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    // Touch last_seen_at
    let _ = state.agent_registry.touch(agent.id).await;

    tracing::info!("Agent '{}' connected via SSE", name);

    let rx = state.sessions.subscribe(&name).await;
    let stream = BroadcastStream::new(rx).filter_map(|result| {
        result.ok().and_then(|evt| match evt {
            SseEvent::AgentMessage(msg) => Some(Ok(Event::default()
                .event("message")
                .json_data(&*msg)
                .unwrap_or_else(|_| Event::default().data("error serializing message")))),
            SseEvent::JsonRpc(_) => None, // Agent SSE doesn't handle JSON-RPC events
        })
    });

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// Worker agent sends a message through meddler.
#[allow(clippy::missing_errors_doc)]
pub async fn agent_message(
    State(state): State<AppState>,
    Json(req): Json<AgentMessageRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let sender = state
        .agent_registry
        .get_by_name(&req.from)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("Sender not found: {e}")))?;

    let recipient = state
        .agent_registry
        .get_by_name(&req.to)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("Recipient not found: {e}")))?;

    let task_id = req
        .task_id
        .as_deref()
        .map(|s| {
            s.parse::<uuid::Uuid>()
                .map(meddler_core::types::TaskId)
                .map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Invalid task_id: {e}"),
                    )
                })
        })
        .transpose()?;

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
            content: req.content,
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Notify the recipient via SSE
    let delivered = state.sessions.notify(&req.to, message.clone()).await;

    Ok(Json(serde_json::json!({
        "message_id": message.id,
        "delivered": delivered,
    })))
}
