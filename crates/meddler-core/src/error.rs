use crate::types::AgentId;

/// Core error type for the meddler system.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("agent not found: {0}")]
    AgentNotFound(String),

    #[error("agent not found by id: {0}")]
    AgentNotFoundById(AgentId),

    #[error("task not found: {0}")]
    TaskNotFound(crate::types::TaskId),

    #[error("database error: {0}")]
    Database(String),

    #[error("internal error: {0}")]
    Internal(String),
}
