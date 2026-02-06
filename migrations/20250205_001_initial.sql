CREATE TABLE agents (
    id UUID PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    description TEXT NOT NULL,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE tasks (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    created_by UUID NOT NULL REFERENCES agents(id),
    time_budget_secs BIGINT,
    started_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE messages (
    id UUID PRIMARY KEY,
    sender_id UUID NOT NULL REFERENCES agents(id),
    recipient_id UUID NOT NULL REFERENCES agents(id),
    task_id UUID REFERENCES tasks(id) ON DELETE SET NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_messages_task ON messages(task_id, created_at);
CREATE INDEX idx_messages_recipient ON messages(recipient_id, created_at);
CREATE INDEX idx_messages_sender ON messages(sender_id, created_at);
