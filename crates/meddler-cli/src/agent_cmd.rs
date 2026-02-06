use reqwest::Client;
use reqwest_eventsource::{Event, EventSource};
use tokio_stream::StreamExt;

/// How the agent processes incoming messages.
pub enum AgentMode {
    /// Echo the message back (for testing).
    Mock,
    /// Forward to an LLM and return the response.
    Llm { url: String, model: String },
}

/// Run the agent: register, connect SSE, process messages.
pub async fn run(
    meddler_url: &str,
    name: &str,
    desc: &str,
    mode: AgentMode,
) -> anyhow::Result<()> {
    let client = Client::new();

    // Step 1: Register with meddler
    let resp = client
        .post(format!("{meddler_url}/agent/register"))
        .json(&serde_json::json!({
            "name": name,
            "description": desc,
        }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await?;
        anyhow::bail!("Failed to register: {body}");
    }

    let reg: serde_json::Value = resp.json().await?;
    tracing::info!("Registered as '{}' (id: {})", name, reg["agent_id"]);

    // Step 2: Connect to SSE stream
    let sse_url = format!("{meddler_url}/agent/sse/{name}");
    tracing::info!("Connecting to SSE: {sse_url}");

    let mut es = EventSource::get(&sse_url);

    // Step 3: Process incoming messages
    while let Some(event) = es.next().await {
        match event {
            Ok(Event::Open) => {
                tracing::info!("SSE connection established");
            }
            Ok(Event::Message(msg)) => {
                tracing::info!("[recv] {}", msg.data);

                // Parse the message
                let message: serde_json::Value = match serde_json::from_str(&msg.data) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!("Failed to parse message: {e}");
                        continue;
                    }
                };

                let content = message["content"]
                    .as_str()
                    .unwrap_or_default();
                let _sender_name = message["sender_id"]
                    .as_str()
                    .unwrap_or("unknown");

                // Generate response
                let response = match &mode {
                    AgentMode::Mock => format!("Echo: {content}"),
                    AgentMode::Llm { url, model } => {
                        call_llm(&client, url, model, content).await.unwrap_or_else(|e| {
                            format!("LLM error: {e}")
                        })
                    }
                };

                tracing::info!("[sent] {response}");

                // Send response back through meddler
                // We need to resolve the sender name - for now, we send back to the orchestrator
                let _ = client
                    .post(format!("{meddler_url}/agent/message"))
                    .json(&serde_json::json!({
                        "from": name,
                        "to": "__orchestrator__",
                        "content": response,
                        "task_id": message.get("task_id").and_then(|v| v.as_str()),
                    }))
                    .send()
                    .await;
            }
            Err(err) => {
                tracing::error!("SSE error: {err}");
                // Attempt reconnect after a delay
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}

/// Call an OpenAI-compatible LLM API.
async fn call_llm(
    client: &Client,
    url: &str,
    model: &str,
    prompt: &str,
) -> anyhow::Result<String> {
    let resp = client
        .post(format!("{url}/chat/completions"))
        .json(&serde_json::json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let content = resp["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("No response from LLM")
        .to_string();

    Ok(content)
}
