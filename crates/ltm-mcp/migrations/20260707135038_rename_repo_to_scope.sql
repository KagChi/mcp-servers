-- Migration: Rename repo column to scope
-- This migration renames the 'repo' column to 'scope' for better semantic clarity
-- The scope field allows filtering memories by project, context, or any organizational unit

-- Rename the column
ALTER TABLE memories RENAME COLUMN repo TO scope;

-- Rename the indexes for consistency
DROP INDEX IF EXISTS idx_memories_repo;
DROP INDEX IF EXISTS idx_memories_repo_created_at;

-- Recreate indexes with new names
CREATE INDEX IF NOT EXISTS idx_memories_scope ON memories(scope) WHERE scope IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_memories_scope_created_at ON memories(scope, created_at DESC) WHERE scope IS NOT NULL;
