-- Migration: Optimize embeddings for 384d and HNSW index
-- UP Migration

-- Drop old IVFFLAT index
DROP INDEX IF EXISTS idx_memories_embedding;

-- Change embedding dimension from 1536 to 384
ALTER TABLE memories ALTER COLUMN embedding TYPE vector(384);

-- Create HNSW index for better performance (m=16, ef_construction=64)
CREATE INDEX idx_memories_embedding_hnsw ON memories 
USING hnsw (embedding vector_cosine_ops) 
WITH (m = 16, ef_construction = 64);

-- Add full-text search column and index for hybrid search
ALTER TABLE memories ADD COLUMN IF NOT EXISTS textsearch tsvector;

-- Create function to update textsearch column
CREATE OR REPLACE FUNCTION memories_textsearch_trigger() RETURNS trigger AS $$
BEGIN
    NEW.textsearch := 
        setweight(to_tsvector('english', coalesce(NEW.content, '')), 'A') ||
        setweight(to_tsvector('english', coalesce(NEW.context, '')), 'B') ||
        setweight(to_tsvector('english', coalesce(array_to_string(NEW.tags, ' '), '')), 'C');
    RETURN NEW;
END
$$ LANGUAGE plpgsql IMMUTABLE;

-- Create trigger to automatically update textsearch on insert/update
CREATE TRIGGER memories_textsearch_update 
    BEFORE INSERT OR UPDATE ON memories
    FOR EACH ROW EXECUTE FUNCTION memories_textsearch_trigger();

-- Populate existing rows
UPDATE memories SET textsearch = 
    setweight(to_tsvector('english', coalesce(content, '')), 'A') ||
    setweight(to_tsvector('english', coalesce(context, '')), 'B') ||
    setweight(to_tsvector('english', coalesce(array_to_string(tags, ' '), '')), 'C');

CREATE INDEX idx_memories_textsearch ON memories USING GIN (textsearch);

-- Set optimal HNSW search parameters
-- Note: Run this manually after migration if needed:
-- ALTER DATABASE neondb SET hnsw.ef_search = 100;
-- Or set in postgresql.conf: hnsw.ef_search = 100

-- DOWN Migration (commented out, uncomment to rollback)
-- DROP INDEX IF EXISTS idx_memories_textsearch;
-- ALTER TABLE memories DROP COLUMN IF EXISTS textsearch;
-- DROP INDEX IF EXISTS idx_memories_embedding_hnsw;
-- ALTER TABLE memories ALTER COLUMN embedding TYPE vector(1536);
-- CREATE INDEX idx_memories_embedding ON memories USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
