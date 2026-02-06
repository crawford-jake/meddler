use std::sync::Arc;

use meddler_core::traits::{AgentRegistry, MessageStore, TaskStore};

use crate::session::SessionManager;

/// Shared application state with injected dependencies.
#[derive(Clone)]
pub struct AppState {
    pub agent_registry: Arc<dyn AgentRegistry>,
    pub message_store: Arc<dyn MessageStore>,
    pub task_store: Arc<dyn TaskStore>,
    pub sessions: Arc<SessionManager>,
}
