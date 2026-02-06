use async_trait::async_trait;
use sqlx::PgPool;

use meddler_core::error::Error;
use meddler_core::traits::{AgentRegistry, MessageStore, TaskStore};
use meddler_core::types::{
    Agent, AgentId, CreateMessage, CreateTask, Message, MessageFilter, MessageId, RegisterAgent,
    Task, TaskId, TaskStatus,
};

/// Postgres-backed implementation of all storage traits.
#[derive(Debug, Clone)]
pub struct PgStore {
    pool: PgPool,
}

impl PgStore {
    /// Create a new `PgStore` with the given connection pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run database migrations.
    ///
    /// # Errors
    ///
    /// Returns an error if migrations fail to apply.
    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::migrate!("../../migrations").run(&self.pool).await?;
        Ok(())
    }
}

#[async_trait]
impl AgentRegistry for PgStore {
    async fn register(&self, params: RegisterAgent) -> Result<Agent, Error> {
        let id = uuid::Uuid::new_v4();
        let row = sqlx::query_as::<_, AgentRow>(
            r"
            INSERT INTO agents (id, name, description)
            VALUES ($1, $2, $3)
            ON CONFLICT (name) DO UPDATE
                SET description = EXCLUDED.description,
                    last_seen_at = NOW()
            RETURNING id, name, description, registered_at, last_seen_at
            ",
        )
        .bind(id)
        .bind(&params.name)
        .bind(&params.description)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.into())
    }

    async fn get_by_name(&self, name: &str) -> Result<Agent, Error> {
        let row = sqlx::query_as::<_, AgentRow>(
            "SELECT id, name, description, registered_at, last_seen_at FROM agents WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?
        .ok_or_else(|| Error::AgentNotFound(name.to_string()))?;

        Ok(row.into())
    }

    async fn get_by_id(&self, id: AgentId) -> Result<Agent, Error> {
        let row = sqlx::query_as::<_, AgentRow>(
            "SELECT id, name, description, registered_at, last_seen_at FROM agents WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?
        .ok_or(Error::AgentNotFoundById(id))?;

        Ok(row.into())
    }

    async fn list(&self) -> Result<Vec<Agent>, Error> {
        let rows = sqlx::query_as::<_, AgentRow>(
            "SELECT id, name, description, registered_at, last_seen_at FROM agents ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn touch(&self, id: AgentId) -> Result<(), Error> {
        sqlx::query("UPDATE agents SET last_seen_at = NOW() WHERE id = $1")
            .bind(id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl MessageStore for PgStore {
    async fn create(&self, params: CreateMessage) -> Result<Message, Error> {
        let id = uuid::Uuid::new_v4();
        let row = sqlx::query_as::<_, MessageRow>(
            r"
            INSERT INTO messages (id, sender_id, recipient_id, task_id, content)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, sender_id, recipient_id, task_id, content, created_at
            ",
        )
        .bind(id)
        .bind(params.sender_id.0)
        .bind(params.recipient_id.0)
        .bind(params.task_id.map(|t| t.0))
        .bind(&params.content)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.into())
    }

    async fn query(&self, filter: MessageFilter) -> Result<Vec<Message>, Error> {
        let rows = sqlx::query_as::<_, MessageRow>(
            r"
            SELECT id, sender_id, recipient_id, task_id, content, created_at
            FROM messages
            WHERE ($1::uuid IS NULL OR task_id = $1)
              AND ($2::uuid IS NULL OR sender_id = $2)
              AND ($3::uuid IS NULL OR recipient_id = $3)
            ORDER BY created_at ASC
            ",
        )
        .bind(filter.task_id.map(|t| t.0))
        .bind(filter.sender_id.map(|a| a.0))
        .bind(filter.recipient_id.map(|a| a.0))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[async_trait]
impl TaskStore for PgStore {
    async fn create(&self, params: CreateTask) -> Result<Task, Error> {
        let id = uuid::Uuid::new_v4();
        let row = sqlx::query_as::<_, TaskRow>(
            r"
            INSERT INTO tasks (id, title, created_by, time_budget_secs)
            VALUES ($1, $2, $3, $4)
            RETURNING id, title, created_by, time_budget_secs, started_at, created_at
            ",
        )
        .bind(id)
        .bind(&params.title)
        .bind(params.created_by.0)
        .bind(params.time_budget_secs)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.into())
    }

    async fn get(&self, id: TaskId) -> Result<Task, Error> {
        let row = sqlx::query_as::<_, TaskRow>(
            "SELECT id, title, created_by, time_budget_secs, started_at, created_at FROM tasks WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?
        .ok_or(Error::TaskNotFound(id))?;

        Ok(row.into())
    }

    async fn get_status(&self, id: TaskId) -> Result<TaskStatus, Error> {
        let task = self.get(id).await?;
        Ok(TaskStatus::compute(task, chrono::Utc::now()))
    }

    async fn mark_started(&self, id: TaskId) -> Result<(), Error> {
        sqlx::query(
            "UPDATE tasks SET started_at = NOW() WHERE id = $1 AND started_at IS NULL",
        )
        .bind(id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }
}

// --- Internal row types for sqlx ---

#[derive(sqlx::FromRow)]
struct AgentRow {
    id: uuid::Uuid,
    name: String,
    description: String,
    registered_at: chrono::DateTime<chrono::Utc>,
    last_seen_at: chrono::DateTime<chrono::Utc>,
}

impl From<AgentRow> for Agent {
    fn from(row: AgentRow) -> Self {
        Self {
            id: AgentId(row.id),
            name: row.name,
            description: row.description,
            registered_at: row.registered_at,
            last_seen_at: row.last_seen_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    id: uuid::Uuid,
    sender_id: uuid::Uuid,
    recipient_id: uuid::Uuid,
    task_id: Option<uuid::Uuid>,
    content: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<MessageRow> for Message {
    fn from(row: MessageRow) -> Self {
        Self {
            id: MessageId(row.id),
            sender_id: AgentId(row.sender_id),
            recipient_id: AgentId(row.recipient_id),
            task_id: row.task_id.map(TaskId),
            content: row.content,
            created_at: row.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: uuid::Uuid,
    title: String,
    created_by: uuid::Uuid,
    time_budget_secs: Option<i64>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<TaskRow> for Task {
    fn from(row: TaskRow) -> Self {
        Self {
            id: TaskId(row.id),
            title: row.title,
            created_by: AgentId(row.created_by),
            time_budget_secs: row.time_budget_secs,
            started_at: row.started_at,
            created_at: row.created_at,
        }
    }
}
