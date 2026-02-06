use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentId(pub Uuid);

impl AgentId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Unique identifier for a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageId(pub Uuid);

impl MessageId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Unique identifier for a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaskId(pub Uuid);

impl TaskId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A registered agent in the meddler system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub description: String,
    pub registered_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

/// A point-to-point message between two agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub sender_id: AgentId,
    pub recipient_id: AgentId,
    pub task_id: Option<TaskId>,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// A task that groups related messages and tracks time budgets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    pub created_by: AgentId,
    pub time_budget_secs: Option<i64>,
    pub started_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Computed view of a task's time status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatus {
    pub task: Task,
    pub elapsed_secs: Option<i64>,
    pub remaining_secs: Option<i64>,
}

impl TaskStatus {
    /// Compute the status of a task at the given point in time.
    #[must_use]
    pub fn compute(task: Task, now: DateTime<Utc>) -> Self {
        let elapsed_secs = task
            .started_at
            .map(|started| (now - started).num_seconds());

        let remaining_secs = match (elapsed_secs, task.time_budget_secs) {
            (Some(elapsed), Some(budget)) => Some((budget - elapsed).max(0)),
            _ => None,
        };

        Self {
            task,
            elapsed_secs,
            remaining_secs,
        }
    }
}

/// Parameters for creating a new message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessage {
    pub sender_id: AgentId,
    pub recipient_id: AgentId,
    pub task_id: Option<TaskId>,
    pub content: String,
}

/// Parameters for creating a new task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTask {
    pub title: String,
    pub created_by: AgentId,
    pub time_budget_secs: Option<i64>,
}

/// Parameters for registering an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterAgent {
    pub name: String,
    pub description: String,
}

/// Filters for querying messages.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageFilter {
    pub task_id: Option<TaskId>,
    pub sender_id: Option<AgentId>,
    pub recipient_id: Option<AgentId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_id_roundtrip() {
        let id = AgentId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: AgentId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn message_id_roundtrip() {
        let id = MessageId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: MessageId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn task_id_roundtrip() {
        let id = TaskId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: TaskId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn task_status_not_started() {
        let now = chrono::Utc::now();
        let task = Task {
            id: TaskId::new(),
            title: "Test".to_string(),
            created_by: AgentId::new(),
            time_budget_secs: Some(3600),
            started_at: None,
            created_at: now,
        };

        let status = TaskStatus::compute(task, now);
        assert!(status.elapsed_secs.is_none());
        assert!(status.remaining_secs.is_none());
    }

    #[test]
    fn task_status_in_progress() {
        let now = chrono::Utc::now();
        let started = now - chrono::Duration::seconds(1800); // 30 min ago
        let task = Task {
            id: TaskId::new(),
            title: "Test".to_string(),
            created_by: AgentId::new(),
            time_budget_secs: Some(3600), // 1 hour
            started_at: Some(started),
            created_at: now,
        };

        let status = TaskStatus::compute(task, now);
        assert_eq!(status.elapsed_secs, Some(1800));
        assert_eq!(status.remaining_secs, Some(1800));
    }

    #[test]
    fn task_status_overtime() {
        let now = chrono::Utc::now();
        let started = now - chrono::Duration::seconds(7200); // 2 hours ago
        let task = Task {
            id: TaskId::new(),
            title: "Test".to_string(),
            created_by: AgentId::new(),
            time_budget_secs: Some(3600), // 1 hour budget
            started_at: Some(started),
            created_at: now,
        };

        let status = TaskStatus::compute(task, now);
        assert_eq!(status.elapsed_secs, Some(7200));
        assert_eq!(status.remaining_secs, Some(0)); // Clamped to 0
    }

    #[test]
    fn task_status_no_budget() {
        let now = chrono::Utc::now();
        let started = now - chrono::Duration::seconds(1800);
        let task = Task {
            id: TaskId::new(),
            title: "Test".to_string(),
            created_by: AgentId::new(),
            time_budget_secs: None,
            started_at: Some(started),
            created_at: now,
        };

        let status = TaskStatus::compute(task, now);
        assert_eq!(status.elapsed_secs, Some(1800));
        assert!(status.remaining_secs.is_none()); // No budget = no remaining
    }

    #[test]
    fn agent_serialization() {
        let agent = Agent {
            id: AgentId::new(),
            name: "researcher".to_string(),
            description: "A research agent".to_string(),
            registered_at: chrono::Utc::now(),
            last_seen_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&agent).unwrap();
        let deserialized: Agent = serde_json::from_str(&json).unwrap();
        assert_eq!(agent.id, deserialized.id);
        assert_eq!(agent.name, deserialized.name);
    }

    #[test]
    fn message_serialization() {
        let msg = Message {
            id: MessageId::new(),
            sender_id: AgentId::new(),
            recipient_id: AgentId::new(),
            task_id: Some(TaskId::new()),
            content: "Hello world".to_string(),
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.id, deserialized.id);
        assert_eq!(msg.content, deserialized.content);
        assert_eq!(msg.task_id, deserialized.task_id);
    }
}
