use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Definition of an MCP tool exposed to the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Registry of all MCP tools available to the orchestrator.
pub struct ToolRegistry;

impl ToolRegistry {
    /// Return the list of tool definitions for the MCP `tools/list` method.
    #[must_use]
    pub fn definitions() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "list_agents".to_string(),
                description: "List all registered agents and their descriptions.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            ToolDefinition {
                name: "send_message".to_string(),
                description: "Send a message to a specific agent by name. Returns the message ID. The response will arrive via SSE notification.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "to": {
                            "type": "string",
                            "description": "Name of the recipient agent"
                        },
                        "content": {
                            "type": "string",
                            "description": "Message content to send"
                        },
                        "task_id": {
                            "type": "string",
                            "description": "Optional task ID to group related messages"
                        }
                    },
                    "required": ["to", "content"]
                }),
            },
            ToolDefinition {
                name: "get_messages".to_string(),
                description: "Retrieve message history with optional filters.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "Filter by task ID"
                        },
                        "sender": {
                            "type": "string",
                            "description": "Filter by sender agent name"
                        },
                        "recipient": {
                            "type": "string",
                            "description": "Filter by recipient agent name"
                        }
                    },
                    "required": []
                }),
            },
            ToolDefinition {
                name: "create_task".to_string(),
                description: "Create a new task to group related messages. Optionally set a time budget in seconds.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Title of the task"
                        },
                        "time_budget_secs": {
                            "type": "integer",
                            "description": "Optional time budget in seconds (e.g., 28800 for 8 hours)"
                        }
                    },
                    "required": ["title"]
                }),
            },
            ToolDefinition {
                name: "get_task_status".to_string(),
                description: "Get the status of a task, including elapsed and remaining time.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "The task ID to check"
                        }
                    },
                    "required": ["task_id"]
                }),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tools_defined() {
        let tools = ToolRegistry::definitions();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

        assert!(names.contains(&"list_agents"));
        assert!(names.contains(&"send_message"));
        assert!(names.contains(&"get_messages"));
        assert!(names.contains(&"create_task"));
        assert!(names.contains(&"get_task_status"));
        assert_eq!(tools.len(), 5);
    }

    #[test]
    fn tools_serialize() {
        let tools = ToolRegistry::definitions();
        let json = serde_json::to_string(&tools).unwrap();
        assert!(json.contains("list_agents"));
        assert!(json.contains("inputSchema"));
    }

    #[test]
    fn send_message_has_required_params() {
        let tools = ToolRegistry::definitions();
        let send = tools.iter().find(|t| t.name == "send_message").unwrap();
        let required = send.input_schema["required"].as_array().unwrap();

        let required_names: Vec<&str> = required.iter().filter_map(Value::as_str).collect();
        assert!(required_names.contains(&"to"));
        assert!(required_names.contains(&"content"));
    }
}
