# MCP Servers

A collection of Model Context Protocol (MCP) servers built with Rust.

## Available Servers

### LTM-MCP (Long-Term Memory)

A powerful long-term memory server for AI assistants, providing persistent storage and retrieval of conversational context and information.

**Features:**
- PostgreSQL-backed persistent storage with full-text search
- RESTful HTTP/SSE API for MCP protocol
- API key authentication
- Automatic database migrations
- Docker deployment ready
- Multi-platform support (linux/amd64, linux/arm64)

**MCP Tools:**
- `store_memory` - Store new memories with content, context, tags, and metadata
- `get_memory` - Retrieve a specific memory by ID
- `search_memories` - Full-text search across memory content
- `list_memories` - List memories with filtering by collection and tags
- `update_memory` - Update existing memory fields
- `delete_memory` - Remove a memory from storage
- `add_tags` - Add tags to an existing memory
- `remove_tags` - Remove tags from a memory
- `list_tags` - Get all unique tags across memories
- `list_collections` - Get all unique collection names

## Quick Start

### Using Docker (Recommended)

```bash
# Pull the latest image
docker pull ghcr.io/kagchi/mcp-servers-ltm-mcp:latest

# Run with PostgreSQL
docker run -d \
  --name ltm-mcp \
  -p 3000:3000 \
  -e LTM_DATABASE_URL="postgresql://user:password@postgres:5432/ltm" \
  -e LTM_AUTH_API_KEY="your-secret-api-key" \
  ghcr.io/kagchi/mcp-servers-ltm-mcp:latest
```

### From Source

```bash
# Clone the repository
git clone https://github.com/KagChi/mcp-servers.git
cd mcp-servers

# Set environment variables
export LTM_DATABASE_URL="postgresql://localhost:5432/ltm"
export LTM_AUTH_API_KEY="your-secret-api-key"

# Build and run
cargo run --package ltm-mcp
```

## Configuration

All configuration is done through environment variables with the `LTM_` prefix. For local development, create a `.env` file in `crates/ltm-mcp/` (see `.env.example`).

### Required Configuration
- `LTM_DATABASE_URL` - PostgreSQL connection URL
  - Format: `postgresql://user:password@host:port/database`
  - Example: `postgresql://postgres:password@localhost:5432/ltm-mcp`
- `LTM_AUTH_API_KEY` - API key for authentication
  - Example: `your-secret-api-key-here`

### Optional Configuration
- `LTM_SERVER_HOST` - Server bind address (default: `0.0.0.0`)
- `LTM_SERVER_PORT` - Server port (default: `3000`)
- `LTM_LOG_LEVEL` - Log level (default: `info`)
  - Options: `trace`, `debug`, `info`, `warn`, `error`

## API Endpoints

### Health Check
```bash
GET /health
```
Verifies server and database connectivity.

### MCP Protocol Endpoints
- `GET /mcp/v1/info` - Server information
- `GET /mcp/v1/tools` - List available tools
- `POST /mcp/v1/tools/call` - Call a tool

All MCP endpoints require authentication via Bearer token:
```bash
curl -H "Authorization: Bearer YOUR_API_KEY" \
  http://localhost:3000/mcp/v1/tools
```

## Development

### Prerequisites
- Rust 1.95.0 or later
- PostgreSQL 14 or later
- Docker (for containerized deployment)

### Building

```bash
# Build all crates
cargo build --workspace

# Build specific crate
cargo build --package ltm-mcp

# Build for release
cargo build --release --workspace
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test --package ltm-mcp
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Run linter
cargo clippy --all-targets --all-features -- -D warnings
```

## Docker Deployment

### Building Images

```bash
# Build ltm-mcp image
docker build -f crates/ltm-mcp/Dockerfile -t ltm-mcp:latest .
```

### Multi-Platform Builds

```bash
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -f crates/ltm-mcp/Dockerfile \
  -t ltm-mcp:latest \
  .
```

## Project Structure

```
mcp-servers/
├── Cargo.toml                 # Workspace configuration
├── LICENSE                    # Apache-2.0 license
├── README.md                  # This file
├── .github/
│   └── workflows/
│       ├── check.yml          # Rust and Docker validation
│       └── docker.yml         # Docker build and push
└── crates/
    ├── mcp-common/            # Shared utilities
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       └── error.rs       # Common error types
    └── ltm-mcp/               # Long-term memory server
        ├── Cargo.toml
        ├── Dockerfile
        ├── migrations/        # Database migrations
        │   └── 20260706000001_create_memories.sql
        └── src/
            ├── main.rs        # Application entry point
            ├── config.rs      # Environment configuration
            ├── server.rs      # HTTP server and routes
            ├── tools/         # MCP tool implementations
            │   └── mod.rs
            └── memory/        # Memory storage layer
                ├── mod.rs
                ├── types.rs   # Data models
                ├── store.rs   # Storage trait
                └── postgres.rs # PostgreSQL implementation
```

## CI/CD

The project uses GitHub Actions for continuous integration and deployment:

- **Rust Check** (`check.yml`) - Validates Rust toolchain, formatting, linting, and tests
- **Docker Build** (`docker.yml`) - Builds and pushes multi-platform Docker images to GitHub Container Registry

Images are automatically published to `ghcr.io/kagchi/mcp-servers-{crate}` on:
- Push to `main` branch (tagged as `latest`)
- Version tags (e.g., `v1.0.0`)

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## Author

KagChi

## Repository

https://github.com/KagChi/mcp-servers
