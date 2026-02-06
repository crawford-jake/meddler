use async_trait::async_trait;

use crate::error::Error;
use crate::types::{
    Agent, AgentId, CreateMessage, CreateTask, Message, MessageFilter, RegisterAgent, Task,
    TaskId, TaskStatus,
};

/// Registry for managing agent identities.
#[async_trait]
pub trait AgentRegistry: Send + Sync {
    /// Register or reconnect an agent. If an agent with the same name exists,
    /// returns the existing agent (idempotent). Otherwise creates a new one.
    async fn register(&self, params: RegisterAgent) -> Result<Agent, Error>;

    /// Get an agent by name.
    async fn get_by_name(&self, name: &str) -> Result<Agent, Error>;

    /// Get an agent by ID.
    async fn get_by_id(&self, id: AgentId) -> Result<Agent, Error>;

    /// List all registered agents.
    async fn list(&self) -> Result<Vec<Agent>, Error>;

    /// Update the `last_seen_at` timestamp for an agent.
    async fn touch(&self, id: AgentId) -> Result<(), Error>;
}

/// Store for persisting messages.
#[async_trait]
pub trait MessageStore: Send + Sync {
    /// Create and persist a new message.
    async fn create(&self, params: CreateMessage) -> Result<Message, Error>;

    /// Query messages with optional filters.
    async fn query(&self, filter: MessageFilter) -> Result<Vec<Message>, Error>;
}

/// Store for managing tasks.
#[async_trait]
pub trait TaskStore: Send + Sync {
    /// Create a new task.
    async fn create(&self, params: CreateTask) -> Result<Task, Error>;

    /// Get a task by ID.
    async fn get(&self, id: TaskId) -> Result<Task, Error>;

    /// Get the computed status of a task.
    async fn get_status(&self, id: TaskId) -> Result<TaskStatus, Error>;

    /// Mark a task as started (sets `started_at` if not already set).
    async fn mark_started(&self, id: TaskId) -> Result<(), Error>;
}
