use reqwest::Client;

/// Send a message to an agent and print the result.
pub async fn run(
    meddler_url: &str,
    from: &str,
    to: &str,
    content: &str,
) -> anyhow::Result<()> {
    let client = Client::new();

    // Ensure the sender is registered
    let _ = client
        .post(format!("{meddler_url}/agent/register"))
        .json(&serde_json::json!({
            "name": from,
            "description": "CLI user",
        }))
        .send()
        .await?;

    // Send the message
    let resp = client
        .post(format!("{meddler_url}/agent/message"))
        .json(&serde_json::json!({
            "from": from,
            "to": to,
            "content": content,
        }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await?;
        anyhow::bail!("Failed to send message: {body}");
    }

    let result: serde_json::Value = resp.json().await?;
    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}
