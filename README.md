# Meddler

AI agent orchestration transport layer. Think Polytopia: the orchestrator agent is the player who moves each piece one by one. Agents don't talk to each other - the orchestrator manually shuttles information between them.

## Quick Start

```bash
docker compose up -d
```

This starts:
- **Postgres** - Message persistence
- **Meddler server** - Transport layer at `http://localhost:3000`
- **researcher** - Mock agent (echo mode)
- **scrutinizer** - Mock agent (echo mode)

## Connect Cursor as Orchestrator

Add to `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "meddler": {
      "url": "http://localhost:3000/mcp/sse"
    }
  }
}
```

Then in Cursor:
> "Use meddler to send 'hello world' to the researcher agent"

## CLI Setup

Build and install the CLI for debugging:

```bash
cargo build --release -p meddler-cli
```

Add to `~/.zshrc`:

```bash
# Meddler CLI
export PATH="$PATH:$HOME/Developer/meddler/target/release"
```

Then from any terminal:

```bash
meddler list-agents
meddler send researcher "Hello!"
```

## MCP Tools

Tools available to the orchestrator (via Cursor/Claude Desktop):

| Tool | Description |
|------|-------------|
| `list_agents` | Discover available agents and their descriptions |
| `send_message` | Send a message to a specific agent by name |
| `get_messages` | Retrieve message history with optional filters |
| `create_task` | Create a task to group related messages |
| `get_task_status` | Check elapsed/remaining time on a task |

## Running with Real LLMs

Edit `docker-compose.yml` to connect agents to Ollama/LMStudio:

```yaml
researcher:
  image: meddler-agent:local
  environment:
    MEDDLER_URL: http://meddler:3000
    AGENT_NAME: researcher
    AGENT_DESC: "Research agent that searches and summarizes information"
    LLM_URL: http://host.docker.internal:11434/v1
    LLM_MODEL: qwen3:32b
```

Or run agents directly:

```bash
meddler agent \
  --name researcher \
  --desc "Research and summarization" \
  --llm-url http://localhost:11434/v1 \
  --model qwen3:32b
```

## Architecture

```
meddler/
├── crates/
│   ├── meddler-core/       # Types, traits, error handling
│   ├── meddler-store/      # Postgres persistence (sqlx)
│   ├── meddler-mcp/        # MCP protocol types and tool definitions
│   ├── meddler-server/     # Axum HTTP server
│   └── meddler-cli/        # CLI binary ("meddler")
├── migrations/             # SQL migrations
├── Dockerfile              # Multi-stage build (server + agent targets)
└── docker-compose.yml      # Full dev stack
```

## Development

```bash
# Run tests
cargo test

# Run clippy (pedantic lints enabled)
cargo clippy --all-targets

# Start just postgres for local dev
docker compose up -d postgres
DATABASE_URL=postgres://meddler:meddler@localhost:5432/meddler cargo run -p meddler-server
```

## License

See [LICENSE](LICENSE).
