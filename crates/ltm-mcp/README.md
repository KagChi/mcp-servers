# LTM MCP Server - Long-Term Memory

A Model Context Protocol (MCP) server that provides long-term memory storage with semantic search capabilities powered by local embeddings.

## Features

- **Persistent Memory Storage**: Store and retrieve memories with PostgreSQL + pgvector
- **Semantic Search**: Find memories by meaning, not just keywords, using local Rust-native embeddings
- **Hybrid Search**: Combines semantic similarity with keyword matching using Reciprocal Rank Fusion (RRF)
- **Automatic Embeddings**: Server-side embedding generation with configurable models
- **Optimized Performance**: HNSW indexing for fast vector similarity search
- **Tag-based Organization**: Organize memories with tags and collections
- **Scope-based Organization**: Separate memories by project/repository
- **Zero API Costs**: Uses local embedding models (no OpenAI/external API required)

## Architecture

### Embedding System

The LTM server uses **Rust-native embeddings** powered by [Candle](https://github.com/huggingface/candle) (Hugging Face's ML framework) for 100% local, cost-free semantic search:

- **Default Model**: `sentence-transformers/all-MiniLM-L6-v2` (384 dimensions, ~80MB)
- **CPU-Optimized**: Runs efficiently on CPU-only environments
- **Auto-Download**: Models download automatically from Hugging Face on first startup
- **No External APIs**: All embedding generation happens locally

### Search Modes

1. **Keyword Search**: Traditional PostgreSQL full-text search
2. **Semantic Search**: Vector similarity using embeddings (finds meaning, not just words)
3. **Hybrid Search** (Default): Combines both using Reciprocal Rank Fusion (RRF) for best results

### Performance

- **Query Speed**: 200-500 QPS on typical hardware
- **Storage**: ~1.5KB per memory (384d embeddings)
- **Accuracy**: 98%+ recall with HNSW indexing
- **Memory Usage**: ~80MB model + ~50MB runtime overhead

## Installation

### Prerequisites

- PostgreSQL 15+ with pgvector extension
- Rust 1.75+ (for building from source)

### Database Setup

```bash
# Install pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;

# Run migrations (automatic on first startup)
```

### Environment Configuration

Create a `.env` file or set environment variables:

```bash
# Required
LTM_DATABASE_URL=postgresql://user:password@localhost/ltm

# Optional - Server Configuration
LTM_SERVER_HOST=127.0.0.1
LTM_SERVER_PORT=3000
LTM_LOG_LEVEL=info

# Optional - Embedding Configuration
LTM_EMBEDDING_ENABLED=true  # Enable/disable embeddings (default: true)
LTM_EMBEDDING_MODEL=sentence-transformers/all-MiniLM-L6-v2  # Model name
LTM_EMBEDDING_DIMENSIONS=384  # Embedding dimensions (must match model)
LTM_EMBEDDING_CACHE_DIR=/path/to/cache  # Model cache directory (default: OS cache dir)

# Optional - Authentication
LTM_AUTH_API_KEY=your-secret-key  # If not set, auth is disabled
```

### Running the Server

```bash
# Development
cargo run --release

# Docker
docker build -t ltm-mcp .
docker run -p 3000:3000 --env-file .env ltm-mcp
```

## Usage

### Storing Memories

```json
{
  "method": "tools/call",
  "params": {
    "name": "store_memory",
    "arguments": {
      "content": "The authentication system uses JWT tokens with 24-hour expiry",
      "context": "Reviewing security implementation",
      "tags": ["auth", "security", "jwt"],
      "collection": "architecture",
      "scope": "my-project"
    }
  }
}
```

**Note**: The `embedding` field is optional - if not provided, the server automatically generates embeddings using the configured model.

### Searching Memories

**Hybrid Search (Default - Best Results)**
```json
{
  "method": "tools/call",
  "params": {
    "name": "search_memories",
    "arguments": {
      "query": "how does authentication work?",
      "search_mode": "hybrid",
      "limit": 10,
      "scope": "my-project"
    }
  }
}
```

**Semantic-Only Search**
```json
{
  "method": "tools/call",
  "params": {
    "name": "search_memories",
    "arguments": {
      "query": "authentication implementation",
      "search_mode": "semantic",
      "limit": 10
    }
  }
}
```

**Keyword-Only Search**
```json
{
  "method": "tools/call",
  "params": {
    "name": "search_memories",
    "arguments": {
      "query": "JWT tokens",
      "search_mode": "keyword",
      "limit": 10
    }
  }
}
```

### Other Operations

- `list_memories`: List all memories with pagination
- `get_memory`: Retrieve a specific memory by ID
- `update_memory`: Update memory content/metadata
- `delete_memory`: Remove a memory
- `add_tags`: Add tags to a memory
- `remove_tags`: Remove tags from a memory
- `list_tags`: List all unique tags
- `list_collections`: List all unique collections

## Configuration Options

### Embedding Models

You can use any sentence-transformers compatible model from Hugging Face:

**Lightweight (Fast)**
- `sentence-transformers/all-MiniLM-L6-v2` (384d, 80MB) - **Default**
- `sentence-transformers/all-MiniLM-L12-v2` (384d, 120MB)

**Higher Quality (Slower)**
- `sentence-transformers/all-mpnet-base-v2` (768d, 420MB)
- `sentence-transformers/all-roberta-large-v1` (1024d, 1.4GB)

**Important**: When changing models, you must:
1. Update `LTM_EMBEDDING_DIMENSIONS` to match the model's output dimensions
2. Run a new migration to adjust the vector column size
3. Regenerate embeddings for existing memories (or leave them as NULL)

### Search Parameters

The HNSW index is configured for balanced performance:
- `m = 16`: Connections per layer (higher = better recall, larger index)
- `ef_construction = 64`: Build quality (higher = better index, slower build)
- `ef_search = 100`: Query quality (higher = better recall, slower queries)

To adjust query-time accuracy, set PostgreSQL parameter:
```sql
SET hnsw.ef_search = 200;  -- Higher = better accuracy, slower queries
```

### Disabling Embeddings

To run without embeddings (keyword search only):

```bash
LTM_EMBEDDING_ENABLED=false
```

This skips model loading and disables semantic/hybrid search modes.

## Migration Guide

### Upgrading to Embedding-Enabled Version

If you have an existing LTM installation without embeddings:

1. **Backup your database**
2. **Run the migration**: `sqlx migrate run` (happens automatically on startup)
3. **Existing memories**: Will have `embedding = NULL` and won't appear in semantic/hybrid searches
4. **New memories**: Will automatically get embeddings generated

To regenerate embeddings for existing memories, you can:
- Re-save them via `update_memory` (embeddings auto-generate)
- Or write a script to bulk-update embeddings using the embedding service

### Downgrading

To revert to IVFFLAT with 1536d embeddings, run the DOWN migration:

```sql
DROP INDEX IF EXISTS idx_memories_textsearch;
ALTER TABLE memories DROP COLUMN IF EXISTS textsearch;
DROP INDEX IF EXISTS idx_memories_embedding_hnsw;
ALTER TABLE memories ALTER COLUMN embedding TYPE vector(1536);
CREATE INDEX idx_memories_embedding ON memories USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
```

## Performance Tuning

### PostgreSQL Configuration

For optimal performance with vector search:

```conf
# Memory (adjust based on available RAM)
shared_buffers = 8GB              # 25% of RAM
work_mem = 50MB                   # Per-operation memory
maintenance_work_mem = 2GB        # For index building
effective_cache_size = 20GB       # 50-75% of RAM

# Parallelism
max_parallel_workers_per_gather = 4
max_parallel_workers = 8
```

### Binary Quantization (Optional)

For 75% size reduction and 2-4x faster queries with minimal accuracy loss:

```sql
-- Create binary quantized index
CREATE INDEX ON memories USING hnsw 
  ((binary_quantize(embedding)::bit(384)) bit_hamming_ops);

-- Query with re-ranking
SELECT * FROM (
    SELECT * FROM memories 
    ORDER BY binary_quantize(embedding)::bit(384) <~> 
             binary_quantize('[...]') 
    LIMIT 20
) ORDER BY embedding <=> '[...]' LIMIT 5;
```

## Troubleshooting

### Model Download Issues

If model download fails:
1. Check internet connectivity
2. Manually download from Hugging Face:
   ```bash
   # Download to cache directory
   curl -L https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json -o tokenizer.json
   curl -L https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/config.json -o config.json
   curl -L https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/model.safetensors -o model.safetensors
   ```
3. Set `LTM_EMBEDDING_CACHE_DIR` to the download directory

### Slow Queries

1. Check index usage: `EXPLAIN ANALYZE SELECT ... ORDER BY embedding <=> '...' LIMIT 10`
2. Increase `hnsw.ef_search` for better recall (at cost of speed)
3. Ensure `shared_buffers` and `work_mem` are properly configured
4. Consider binary quantization for large datasets

### High Memory Usage

1. Reduce model size (use all-MiniLM-L6-v2 instead of larger models)
2. Lower `hnsw.ef_search` value
3. Reduce PostgreSQL `shared_buffers` and `work_mem`

## License

MIT

## Contributing

Contributions welcome! Please open an issue or PR.
