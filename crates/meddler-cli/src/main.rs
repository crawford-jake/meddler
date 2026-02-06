use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod agent_cmd;
mod send_cmd;

#[derive(Parser)]
#[command(name = "meddler", about = "Meddler CLI - AI agent orchestration transport")]
struct Cli {
    /// Meddler server URL
    #[arg(long, env = "MEDDLER_URL", default_value = "http://localhost:3000")]
    meddler_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a worker agent that connects to meddler and bridges to an LLM
    Agent {
        /// Agent name (unique, stable identity)
        #[arg(long, env = "AGENT_NAME")]
        name: String,

        /// Agent description
        #[arg(long, env = "AGENT_DESC")]
        desc: String,

        /// LLM API URL (OpenAI-compatible)
        #[arg(long, env = "LLM_URL")]
        llm_url: Option<String>,

        /// LLM model name
        #[arg(long, env = "LLM_MODEL")]
        model: Option<String>,

        /// Run in mock mode (echo responses). Set `AGENT_MODE=mock` via env.
        #[arg(long)]
        mock: bool,
    },

    /// Send a message to an agent and print the response
    Send {
        /// Recipient agent name
        agent: String,

        /// Message content
        message: String,

        /// Your agent name (defaults to "cli")
        #[arg(long, default_value = "cli")]
        from: String,
    },

    /// List all registered agents
    ListAgents,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Agent {
            name,
            desc,
            llm_url,
            model,
            mock,
        } => {
            let is_mock = mock
                || std::env::var("AGENT_MODE")
                    .map(|v| v.eq_ignore_ascii_case("mock"))
                    .unwrap_or(false);

            let mode = match (is_mock, llm_url) {
                (false, Some(url)) => agent_cmd::AgentMode::Llm {
                    url,
                    model: model.unwrap_or_else(|| "default".to_string()),
                },
                _ => agent_cmd::AgentMode::Mock,
            };
            agent_cmd::run(&cli.meddler_url, &name, &desc, mode).await?;
        }
        Commands::Send {
            agent,
            message,
            from,
        } => {
            send_cmd::run(&cli.meddler_url, &from, &agent, &message).await?;
        }
        Commands::ListAgents => {
            let client = reqwest::Client::new();
            let resp = client
                .post(format!("{}/mcp", cli.meddler_url))
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "tools/call",
                    "params": {
                        "name": "list_agents",
                        "arguments": {}
                    }
                }))
                .send()
                .await?
                .json::<serde_json::Value>()
                .await?;

            if let Some(result) = resp.get("result") {
                println!("{}", serde_json::to_string_pretty(result)?);
            } else if let Some(error) = resp.get("error") {
                eprintln!("Error: {}", serde_json::to_string_pretty(error)?);
            }
        }
    }

    Ok(())
}
