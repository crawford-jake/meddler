use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};

use meddler_core::types::Message;

/// An event that can be sent over an SSE connection.
#[derive(Debug, Clone)]
pub enum SseEvent {
    /// A message from another agent.
    AgentMessage(Arc<Message>),
    /// A raw JSON-RPC response (for MCP protocol).
    JsonRpc(serde_json::Value),
}

/// Manages active SSE sessions for connected agents and the MCP orchestrator.
pub struct SessionManager {
    /// Map of agent name -> broadcast sender for SSE notifications.
    sessions: RwLock<HashMap<String, broadcast::Sender<SseEvent>>>,
}

impl SessionManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Register a session and return a receiver for SSE events.
    pub async fn subscribe(&self, name: &str) -> broadcast::Receiver<SseEvent> {
        let mut sessions = self.sessions.write().await;
        let sender = sessions
            .entry(name.to_string())
            .or_insert_with(|| broadcast::channel(100).0);
        sender.subscribe()
    }

    /// Send a message notification to a connected agent.
    /// Returns true if the message was delivered to at least one listener.
    pub async fn notify(&self, agent_name: &str, message: Message) -> bool {
        let sessions = self.sessions.read().await;
        if let Some(sender) = sessions.get(agent_name) {
            sender
                .send(SseEvent::AgentMessage(Arc::new(message)))
                .is_ok()
        } else {
            false
        }
    }

    /// Send a JSON-RPC response to the MCP orchestrator's SSE stream.
    pub async fn send_jsonrpc(&self, name: &str, value: serde_json::Value) -> bool {
        let sessions = self.sessions.read().await;
        if let Some(sender) = sessions.get(name) {
            sender.send(SseEvent::JsonRpc(value)).is_ok()
        } else {
            false
        }
    }

    /// Remove a session when an agent disconnects.
    pub async fn remove(&self, name: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(name);
    }

    /// Check if an agent has an active session.
    pub async fn is_connected(&self, name: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions.contains_key(name)
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
