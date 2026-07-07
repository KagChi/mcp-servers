-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;

-- Add scope column for project binding
ALTER TABLE memories ADD COLUMN IF NOT EXISTS scope TEXT;

-- Add embedding column for semantic search (1536 dimensions)
ALTER TABLE memories ADD COLUMN IF NOT EXISTS embedding vector(1536);

-- Create indexes for scope filtering
CREATE INDEX IF NOT EXISTS idx_memories_scope ON memories(scope) WHERE scope IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_memories_scope_created_at ON memories(scope, created_at DESC) WHERE scope IS NOT NULL;

-- Create vector index for semantic search (IVFFLAT with cosine distance)
CREATE INDEX IF NOT EXISTS idx_memories_embedding ON memories USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
