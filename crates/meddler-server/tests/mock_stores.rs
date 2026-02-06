use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;
use chrono::Utc;

use meddler_core::error::Error;
use meddler_core::traits::{AgentRegistry, MessageStore, TaskStore};
use meddler_core::types::{
    Agent, AgentId, CreateMessage, CreateTask, Message, MessageFilter, MessageId, RegisterAgent,
    Task, TaskId, TaskStatus,
};

/// In-memory mock agent registry.
#[derive(Default)]
pub struct MockAgentRegistry {
    agents: RwLock<HashMap<String, Agent>>,
}

impl MockAgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AgentRegistry for MockAgentRegistry {
    async fn register(&self, params: RegisterAgent) -> Result<Agent, Error> {
        let mut agents = self.agents.write().unwrap();
        if let Some(existing) = agents.get(&params.name) {
            return Ok(existing.clone());
        }

        let agent = Agent {
            id: AgentId::new(),
            name: params.name.clone(),
            description: params.description,
            registered_at: Utc::now(),
            last_seen_at: Utc::now(),
        };
        agents.insert(params.name, agent.clone());
        Ok(agent)
    }

    async fn get_by_name(&self, name: &str) -> Result<Agent, Error> {
        let agents = self.agents.read().unwrap();
        agents
            .get(name)
            .cloned()
            .ok_or_else(|| Error::AgentNotFound(name.to_string()))
    }

    async fn get_by_id(&self, id: AgentId) -> Result<Agent, Error> {
        let agents = self.agents.read().unwrap();
        agents
            .values()
            .find(|a| a.id == id)
            .cloned()
            .ok_or(Error::AgentNotFoundById(id))
    }

    async fn list(&self) -> Result<Vec<Agent>, Error> {
        let agents = self.agents.read().unwrap();
        Ok(agents.values().cloned().collect())
    }

    async fn touch(&self, _id: AgentId) -> Result<(), Error> {
        Ok(())
    }
}

/// In-memory mock message store.
#[derive(Default)]
pub struct MockMessageStore {
    messages: RwLock<Vec<Message>>,
}

impl MockMessageStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl MessageStore for MockMessageStore {
    async fn create(&self, params: CreateMessage) -> Result<Message, Error> {
        let message = Message {
            id: MessageId::new(),
            sender_id: params.sender_id,
            recipient_id: params.recipient_id,
            task_id: params.task_id,
            content: params.content,
            created_at: Utc::now(),
        };
        self.messages.write().unwrap().push(message.clone());
        Ok(message)
    }

    async fn query(&self, filter: MessageFilter) -> Result<Vec<Message>, Error> {
        let messages = self.messages.read().unwrap();
        let result = messages
            .iter()
            .filter(|m| filter.task_id.is_none() || m.task_id == filter.task_id)
            .filter(|m| filter.sender_id.is_none_or(|id| m.sender_id == id))
            .filter(|m| filter.recipient_id.is_none_or(|id| m.recipient_id == id))
            .cloned()
            .collect();
        Ok(result)
    }
}

/// In-memory mock task store.
#[derive(Default)]
pub struct MockTaskStore {
    tasks: RwLock<HashMap<TaskId, Task>>,
}

impl MockTaskStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TaskStore for MockTaskStore {
    async fn create(&self, params: CreateTask) -> Result<Task, Error> {
        let task = Task {
            id: TaskId::new(),
            title: params.title,
            created_by: params.created_by,
            time_budget_secs: params.time_budget_secs,
            started_at: None,
            created_at: Utc::now(),
        };
        self.tasks.write().unwrap().insert(task.id, task.clone());
        Ok(task)
    }

    async fn get(&self, id: TaskId) -> Result<Task, Error> {
        let tasks = self.tasks.read().unwrap();
        tasks.get(&id).cloned().ok_or(Error::TaskNotFound(id))
    }

    async fn get_status(&self, id: TaskId) -> Result<TaskStatus, Error> {
        let task = self.get(id).await?;
        Ok(TaskStatus::compute(task, Utc::now()))
    }

    async fn mark_started(&self, id: TaskId) -> Result<(), Error> {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(&id) {
            if task.started_at.is_none() {
                task.started_at = Some(Utc::now());
            }
        }
        Ok(())
    }
}
