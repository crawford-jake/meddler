use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};

use meddler_core::types::Message;

/// Manages active SSE sessions for connected agents.
pub struct SessionManager {
    /// Map of agent name -> broadcast sender for SSE notifications.
    sessions: RwLock<HashMap<String, broadcast::Sender<Arc<Message>>>>,
}

impl SessionManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Register a session for an agent and return a receiver for SSE events.
    pub async fn subscribe(&self, agent_name: &str) -> broadcast::Receiver<Arc<Message>> {
        let mut sessions = self.sessions.write().await;
        let sender = sessions
            .entry(agent_name.to_string())
            .or_insert_with(|| broadcast::channel(100).0);
        sender.subscribe()
    }

    /// Send a message notification to a connected agent.
    /// Returns true if the message was delivered to at least one listener.
    pub async fn notify(&self, agent_name: &str, message: Message) -> bool {
        let sessions = self.sessions.read().await;
        if let Some(sender) = sessions.get(agent_name) {
            sender.send(Arc::new(message)).is_ok()
        } else {
            false
        }
    }

    /// Remove a session when an agent disconnects.
    #[allow(dead_code)]
    pub async fn remove(&self, agent_name: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(agent_name);
    }

    /// Check if an agent has an active session.
    pub async fn is_connected(&self, agent_name: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions.contains_key(agent_name)
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
