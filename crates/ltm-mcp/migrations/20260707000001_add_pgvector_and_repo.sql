-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;

-- Add repo column for project binding
ALTER TABLE memories ADD COLUMN IF NOT EXISTS repo TEXT;

-- Add embedding column for semantic search (1536 dimensions)
ALTER TABLE memories ADD COLUMN IF NOT EXISTS embedding vector(1536);

-- Create indexes for repo filtering
CREATE INDEX IF NOT EXISTS idx_memories_repo ON memories(repo) WHERE repo IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_memories_repo_created_at ON memories(repo, created_at DESC) WHERE repo IS NOT NULL;

-- Create vector index for semantic search (IVFFLAT with cosine distance)
CREATE INDEX IF NOT EXISTS idx_memories_embedding ON memories USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
