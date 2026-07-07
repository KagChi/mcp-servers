use async_trait::async_trait;
use mcp_common::error::{CommonError, Result};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::store::MemoryStore;
use super::types::{CreateMemory, ListQuery, Memory, SearchQuery, UpdateMemory};

pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| CommonError::Database(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl MemoryStore for PostgresStore {
    async fn store(&self, memory: CreateMemory) -> Result<Memory> {
        let id = Uuid::new_v4();
        let metadata_json = serde_json::to_value(&memory.metadata)
            .map_err(|e| CommonError::Serialization(e.to_string()))?;

        // Convert embedding Vec<f32> to pgvector::Vector if present
        let embedding_vector = memory
            .embedding
            .as_ref()
            .map(|e| pgvector::Vector::from(e.clone()));

        let row = sqlx::query(
            r#"
            INSERT INTO memories (id, content, context, tags, collection, metadata, repo, embedding)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, content, context, tags, collection, created_at, updated_at, access_count, metadata, repo, embedding
            "#
        )
        .bind(id)
        .bind(&memory.content)
        .bind(&memory.context)
        .bind(&memory.tags)
        .bind(&memory.collection)
        .bind(&metadata_json)
        .bind(&memory.repo)
        .bind(&embedding_vector)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| CommonError::Database(e.to_string()))?;

        let metadata_value: serde_json::Value = row
            .try_get("metadata")
            .map_err(|e| CommonError::Database(e.to_string()))?;
        let metadata = serde_json::from_value(metadata_value).unwrap_or_default();

        // Convert pgvector::Vector back to Vec<f32> if present
        let embedding: Option<Vec<f32>> = row
            .try_get::<Option<pgvector::Vector>, _>("embedding")
            .ok()
            .flatten()
            .map(|v| v.to_vec());

        Ok(Memory {
            id: row
                .try_get("id")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            content: row
                .try_get("content")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            context: row.try_get("context").ok(),
            tags: row
                .try_get("tags")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            collection: row.try_get("collection").ok(),
            created_at: row
                .try_get("created_at")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            updated_at: row
                .try_get("updated_at")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            access_count: row
                .try_get("access_count")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            metadata,
            repo: row.try_get("repo").ok(),
            embedding,
        })
    }

    async fn get(&self, id: Uuid) -> Result<Option<Memory>> {
        let row = sqlx::query(
            r#"
            SELECT id, content, context, tags, collection, created_at, updated_at, access_count, metadata, repo, embedding
            FROM memories
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CommonError::Database(e.to_string()))?;

        let Some(row) = row else {
            return Ok(None);
        };

        // Increment access count
        sqlx::query("UPDATE memories SET access_count = access_count + 1 WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| CommonError::Database(e.to_string()))?;

        let metadata_value: serde_json::Value = row
            .try_get("metadata")
            .map_err(|e| CommonError::Database(e.to_string()))?;
        let metadata = serde_json::from_value(metadata_value).unwrap_or_default();

        let embedding: Option<Vec<f32>> = row
            .try_get::<Option<pgvector::Vector>, _>("embedding")
            .ok()
            .flatten()
            .map(|v| v.to_vec());

        Ok(Some(Memory {
            id: row
                .try_get("id")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            content: row
                .try_get("content")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            context: row.try_get("context").ok(),
            tags: row
                .try_get("tags")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            collection: row.try_get("collection").ok(),
            created_at: row
                .try_get("created_at")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            updated_at: row
                .try_get("updated_at")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            access_count: row
                .try_get("access_count")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            metadata,
            repo: row.try_get("repo").ok(),
            embedding,
        }))
    }

    async fn search(&self, query: SearchQuery) -> Result<Vec<Memory>> {
        use super::types::SearchMode;

        match query.search_mode {
            SearchMode::Keyword => self.keyword_search(&query).await,
            SearchMode::Semantic => self.semantic_search(&query).await,
            SearchMode::Hybrid => self.hybrid_search(&query).await,
        }
    }

    async fn list(&self, query: ListQuery) -> Result<Vec<Memory>> {
        let limit = query.limit;
        let offset = query.offset;

        let mut query_builder = sqlx::QueryBuilder::new(
            "SELECT id, content, context, tags, collection, created_at, updated_at, access_count, metadata, repo, embedding FROM memories WHERE 1=1"
        );

        if let Some(ref repo) = query.repo {
            query_builder.push(" AND repo = ");
            query_builder.push_bind(repo);
        }
        if let Some(ref collection) = query.collection {
            query_builder.push(" AND collection = ");
            query_builder.push_bind(collection);
        }
        if let Some(ref tags) = query.tags {
            query_builder.push(" AND tags @> ");
            query_builder.push_bind(tags);
        }

        query_builder.push(" ORDER BY created_at DESC LIMIT ");
        query_builder.push_bind(limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);

        let rows = query_builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CommonError::Database(e.to_string()))?;

        let mut memories = Vec::new();
        for row in rows {
            let metadata_value: serde_json::Value = row
                .try_get("metadata")
                .map_err(|e| CommonError::Database(e.to_string()))?;
            let metadata = serde_json::from_value(metadata_value).unwrap_or_default();

            let embedding: Option<Vec<f32>> = row
                .try_get::<Option<pgvector::Vector>, _>("embedding")
                .ok()
                .flatten()
                .map(|v| v.to_vec());

            memories.push(Memory {
                id: row
                    .try_get("id")
                    .map_err(|e| CommonError::Database(e.to_string()))?,
                content: row
                    .try_get("content")
                    .map_err(|e| CommonError::Database(e.to_string()))?,
                context: row.try_get("context").ok(),
                tags: row
                    .try_get("tags")
                    .map_err(|e| CommonError::Database(e.to_string()))?,
                collection: row.try_get("collection").ok(),
                created_at: row
                    .try_get("created_at")
                    .map_err(|e| CommonError::Database(e.to_string()))?,
                updated_at: row
                    .try_get("updated_at")
                    .map_err(|e| CommonError::Database(e.to_string()))?,
                access_count: row
                    .try_get("access_count")
                    .map_err(|e| CommonError::Database(e.to_string()))?,
                metadata,
                repo: row.try_get("repo").ok(),
                embedding,
            });
        }

        Ok(memories)
    }

    async fn update(&self, id: Uuid, update: UpdateMemory) -> Result<Memory> {
        let mut query_builder = sqlx::QueryBuilder::new("UPDATE memories SET updated_at = NOW()");

        if let Some(ref content) = update.content {
            query_builder.push(", content = ");
            query_builder.push_bind(content);
        }
        if let Some(ref context) = update.context {
            query_builder.push(", context = ");
            query_builder.push_bind(context);
        }
        if let Some(ref tags) = update.tags {
            query_builder.push(", tags = ");
            query_builder.push_bind(tags);
        }
        if let Some(ref collection) = update.collection {
            query_builder.push(", collection = ");
            query_builder.push_bind(collection);
        }
        if let Some(ref metadata) = update.metadata {
            let metadata_json = serde_json::to_value(metadata)
                .map_err(|e| CommonError::Serialization(e.to_string()))?;
            query_builder.push(", metadata = ");
            query_builder.push_bind(metadata_json);
        }
        if let Some(ref repo) = update.repo {
            query_builder.push(", repo = ");
            query_builder.push_bind(repo);
        }
        if let Some(ref embedding) = update.embedding {
            let embedding_vector = pgvector::Vector::from(embedding.clone());
            query_builder.push(", embedding = ");
            query_builder.push_bind(embedding_vector);
        }

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(id);
        query_builder.push(" RETURNING id, content, context, tags, collection, created_at, updated_at, access_count, metadata, repo, embedding");

        let row = query_builder
            .build()
            .fetch_one(&self.pool)
            .await
            .map_err(|_e| CommonError::NotFound(format!("Memory not found: {}", id)))?;

        let metadata_value: serde_json::Value = row
            .try_get("metadata")
            .map_err(|e| CommonError::Database(e.to_string()))?;
        let metadata = serde_json::from_value(metadata_value).unwrap_or_default();

        let embedding: Option<Vec<f32>> = row
            .try_get::<Option<pgvector::Vector>, _>("embedding")
            .ok()
            .flatten()
            .map(|v| v.to_vec());

        Ok(Memory {
            id: row
                .try_get("id")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            content: row
                .try_get("content")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            context: row.try_get("context").ok(),
            tags: row
                .try_get("tags")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            collection: row.try_get("collection").ok(),
            created_at: row
                .try_get("created_at")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            updated_at: row
                .try_get("updated_at")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            access_count: row
                .try_get("access_count")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            metadata,
            repo: row.try_get("repo").ok(),
            embedding,
        })
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM memories WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| CommonError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(CommonError::NotFound(format!("Memory not found: {}", id)));
        }

        Ok(())
    }

    async fn add_tags(&self, id: Uuid, tags: Vec<String>) -> Result<Memory> {
        let row = sqlx::query(
            r#"
            UPDATE memories
            SET tags = array(SELECT DISTINCT unnest(tags || $1::text[]))
            WHERE id = $2
            RETURNING id, content, context, tags, collection, created_at, updated_at, access_count, metadata, repo, embedding
            "#
        )
        .bind(&tags)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_e| CommonError::NotFound(format!("Memory not found: {}", id)))?;

        let metadata_value: serde_json::Value = row
            .try_get("metadata")
            .map_err(|e| CommonError::Database(e.to_string()))?;
        let metadata = serde_json::from_value(metadata_value).unwrap_or_default();

        let embedding: Option<Vec<f32>> = row
            .try_get::<Option<pgvector::Vector>, _>("embedding")
            .ok()
            .flatten()
            .map(|v| v.to_vec());

        Ok(Memory {
            id: row
                .try_get("id")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            content: row
                .try_get("content")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            context: row.try_get("context").ok(),
            tags: row
                .try_get("tags")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            collection: row.try_get("collection").ok(),
            created_at: row
                .try_get("created_at")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            updated_at: row
                .try_get("updated_at")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            access_count: row
                .try_get("access_count")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            metadata,
            repo: row.try_get("repo").ok(),
            embedding,
        })
    }

    async fn remove_tags(&self, id: Uuid, tags: Vec<String>) -> Result<Memory> {
        let row = sqlx::query(
            r#"
            UPDATE memories
            SET tags = array(SELECT unnest(tags) EXCEPT SELECT unnest($1::text[]))
            WHERE id = $2
            RETURNING id, content, context, tags, collection, created_at, updated_at, access_count, metadata, repo, embedding
            "#
        )
        .bind(&tags)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_e| CommonError::NotFound(format!("Memory not found: {}", id)))?;

        let metadata_value: serde_json::Value = row
            .try_get("metadata")
            .map_err(|e| CommonError::Database(e.to_string()))?;
        let metadata = serde_json::from_value(metadata_value).unwrap_or_default();

        let embedding: Option<Vec<f32>> = row
            .try_get::<Option<pgvector::Vector>, _>("embedding")
            .ok()
            .flatten()
            .map(|v| v.to_vec());

        Ok(Memory {
            id: row
                .try_get("id")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            content: row
                .try_get("content")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            context: row.try_get("context").ok(),
            tags: row
                .try_get("tags")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            collection: row.try_get("collection").ok(),
            created_at: row
                .try_get("created_at")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            updated_at: row
                .try_get("updated_at")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            access_count: row
                .try_get("access_count")
                .map_err(|e| CommonError::Database(e.to_string()))?,
            metadata,
            repo: row.try_get("repo").ok(),
            embedding,
        })
    }

    async fn list_tags(&self) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT unnest(tags) as tag
            FROM memories
            ORDER BY tag
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CommonError::Database(e.to_string()))?;

        let mut tags = Vec::new();
        for row in rows {
            if let Ok(tag) = row.try_get::<String, _>("tag") {
                tags.push(tag);
            }
        }

        Ok(tags)
    }

    async fn list_collections(&self) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT collection
            FROM memories
            WHERE collection IS NOT NULL
            ORDER BY collection
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CommonError::Database(e.to_string()))?;

        let mut collections = Vec::new();
        for row in rows {
            if let Ok(collection) = row.try_get::<String, _>("collection") {
                collections.push(collection);
            }
        }

        Ok(collections)
    }
}

// Helper methods for PostgresStore
impl PostgresStore {
    async fn keyword_search(&self, query: &SearchQuery) -> Result<Vec<Memory>> {
        let mut query_builder = sqlx::QueryBuilder::new(
            r#"
            SELECT id, content, context, tags, collection, created_at, updated_at, access_count, metadata, repo, embedding,
                   ts_rank(content_tsv, plainto_tsquery('english', "#,
        );
        query_builder.push_bind(&query.query);
        query_builder
            .push(")) as rank FROM memories WHERE content_tsv @@ plainto_tsquery('english', ");
        query_builder.push_bind(&query.query);
        query_builder.push(")");

        if let Some(repo) = &query.repo {
            query_builder.push(" AND repo = ");
            query_builder.push_bind(repo);
        }
        if let Some(collection) = &query.collection {
            query_builder.push(" AND collection = ");
            query_builder.push_bind(collection);
        }
        if let Some(tags) = &query.tags {
            if !tags.is_empty() {
                query_builder.push(" AND tags @> ");
                query_builder.push_bind(tags);
            }
        }

        query_builder.push(" ORDER BY rank DESC, created_at DESC LIMIT ");
        query_builder.push_bind(query.limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(query.offset);

        let rows = query_builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CommonError::Database(e.to_string()))?;

        self.rows_to_memories(rows)
    }

    async fn semantic_search(&self, query: &SearchQuery) -> Result<Vec<Memory>> {
        let query_embedding = query.query_embedding.as_ref().ok_or_else(|| {
            CommonError::Validation("query_embedding is required for semantic search".to_string())
        })?;

        let query_vector = pgvector::Vector::from(query_embedding.clone());

        let mut query_builder = sqlx::QueryBuilder::new(
            r#"
            SELECT id, content, context, tags, collection, created_at, updated_at, access_count, metadata, repo, embedding,
                   1 - (embedding <=> "#,
        );
        query_builder.push_bind(query_vector);
        query_builder.push(") as similarity FROM memories WHERE embedding IS NOT NULL");

        if let Some(repo) = &query.repo {
            query_builder.push(" AND repo = ");
            query_builder.push_bind(repo);
        }
        if let Some(collection) = &query.collection {
            query_builder.push(" AND collection = ");
            query_builder.push_bind(collection);
        }
        if let Some(tags) = &query.tags {
            if !tags.is_empty() {
                query_builder.push(" AND tags @> ");
                query_builder.push_bind(tags);
            }
        }

        query_builder.push(" ORDER BY similarity DESC, created_at DESC LIMIT ");
        query_builder.push_bind(query.limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(query.offset);

        let rows = query_builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CommonError::Database(e.to_string()))?;

        self.rows_to_memories(rows)
    }

    async fn hybrid_search(&self, query: &SearchQuery) -> Result<Vec<Memory>> {
        let query_embedding = query.query_embedding.as_ref().ok_or_else(|| {
            CommonError::Validation("query_embedding is required for hybrid search".to_string())
        })?;

        let query_vector = pgvector::Vector::from(query_embedding.clone());

        // Reciprocal Rank Fusion (RRF) with k=60
        // Combines semantic and keyword search results using rank-based scoring
        let mut query_builder = sqlx::QueryBuilder::new(
            r#"
            WITH semantic_search AS (
                SELECT id, RANK() OVER (ORDER BY embedding <=> "#,
        );
        query_builder.push_bind(query_vector.clone());
        query_builder.push(") AS rank FROM memories WHERE embedding IS NOT NULL");
        
        if let Some(repo) = &query.repo {
            query_builder.push(" AND repo = ");
            query_builder.push_bind(repo);
        }
        if let Some(collection) = &query.collection {
            query_builder.push(" AND collection = ");
            query_builder.push_bind(collection);
        }
        if let Some(tags) = &query.tags {
            if !tags.is_empty() {
                query_builder.push(" AND tags @> ");
                query_builder.push_bind(tags);
            }
        }
        
        query_builder.push(" ORDER BY embedding <=> ");
        query_builder.push_bind(query_vector);
        query_builder.push(" LIMIT 20");
        
        query_builder.push(
            r#"
            ),
            keyword_search AS (
                SELECT id, RANK() OVER (ORDER BY ts_rank(textsearch, query) DESC) AS rank
                FROM memories, plainto_tsquery('english', "#,
        );
        query_builder.push_bind(&query.query);
        query_builder.push(") query WHERE textsearch @@ query");
        
        if let Some(repo) = &query.repo {
            query_builder.push(" AND repo = ");
            query_builder.push_bind(repo);
        }
        if let Some(collection) = &query.collection {
            query_builder.push(" AND collection = ");
            query_builder.push_bind(collection);
        }
        if let Some(tags) = &query.tags {
            if !tags.is_empty() {
                query_builder.push(" AND tags @> ");
                query_builder.push_bind(tags);
            }
        }
        
        query_builder.push(" ORDER BY ts_rank(textsearch, query) DESC LIMIT 20");
        
        query_builder.push(
            r#"
            )
            SELECT m.*,
                   COALESCE(1.0 / (60 + s.rank), 0.0) + COALESCE(1.0 / (60 + k.rank), 0.0) AS rrf_score
            FROM memories m
            LEFT JOIN semantic_search s ON m.id = s.id
            LEFT JOIN keyword_search k ON m.id = k.id
            WHERE s.id IS NOT NULL OR k.id IS NOT NULL
            ORDER BY rrf_score DESC
            LIMIT "#,
        );
        query_builder.push_bind(query.limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(query.offset);

        let rows = query_builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CommonError::Database(e.to_string()))?;

        self.rows_to_memories(rows)
    }

    fn rows_to_memories(&self, rows: Vec<sqlx::postgres::PgRow>) -> Result<Vec<Memory>> {
        let mut memories = Vec::new();

        for row in rows {
            let metadata_value: serde_json::Value = row
                .try_get("metadata")
                .map_err(|e| CommonError::Database(e.to_string()))?;
            let metadata = serde_json::from_value(metadata_value).unwrap_or_default();

            let embedding: Option<Vec<f32>> = row
                .try_get::<Option<pgvector::Vector>, _>("embedding")
                .ok()
                .flatten()
                .map(|v| v.to_vec());

            memories.push(Memory {
                id: row
                    .try_get("id")
                    .map_err(|e| CommonError::Database(e.to_string()))?,
                content: row
                    .try_get("content")
                    .map_err(|e| CommonError::Database(e.to_string()))?,
                context: row.try_get("context").ok(),
                tags: row
                    .try_get("tags")
                    .map_err(|e| CommonError::Database(e.to_string()))?,
                collection: row.try_get("collection").ok(),
                created_at: row
                    .try_get("created_at")
                    .map_err(|e| CommonError::Database(e.to_string()))?,
                updated_at: row
                    .try_get("updated_at")
                    .map_err(|e| CommonError::Database(e.to_string()))?,
                access_count: row
                    .try_get("access_count")
                    .map_err(|e| CommonError::Database(e.to_string()))?,
                metadata,
                repo: row.try_get("repo").ok(),
                embedding,
            });
        }

        Ok(memories)
    }
}
