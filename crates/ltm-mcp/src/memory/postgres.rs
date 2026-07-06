use sqlx::{PgPool, Row};
use uuid::Uuid;
use async_trait::async_trait;
use mcp_common::error::{CommonError, Result};

use super::store::MemoryStore;
use super::types::{Memory, CreateMemory, UpdateMemory, SearchQuery, ListQuery};

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

        let row = sqlx::query(
            r#"
            INSERT INTO memories (id, content, context, tags, collection, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, content, context, tags, collection, created_at, updated_at, access_count, metadata
            "#
        )
        .bind(id)
        .bind(&memory.content)
        .bind(&memory.context)
        .bind(&memory.tags)
        .bind(&memory.collection)
        .bind(&metadata_json)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| CommonError::Database(e.to_string()))?;

        let metadata_value: serde_json::Value = row.try_get("metadata")
            .map_err(|e| CommonError::Database(e.to_string()))?;
        let metadata = serde_json::from_value(metadata_value)
            .unwrap_or_default();

        Ok(Memory {
            id: row.try_get("id").map_err(|e| CommonError::Database(e.to_string()))?,
            content: row.try_get("content").map_err(|e| CommonError::Database(e.to_string()))?,
            context: row.try_get("context").ok(),
            tags: row.try_get("tags").map_err(|e| CommonError::Database(e.to_string()))?,
            collection: row.try_get("collection").ok(),
            created_at: row.try_get("created_at").map_err(|e| CommonError::Database(e.to_string()))?,
            updated_at: row.try_get("updated_at").map_err(|e| CommonError::Database(e.to_string()))?,
            access_count: row.try_get("access_count").map_err(|e| CommonError::Database(e.to_string()))?,
            metadata,
        })
    }

    async fn get(&self, id: Uuid) -> Result<Option<Memory>> {
        let row = sqlx::query(
            r#"
            SELECT id, content, context, tags, collection, created_at, updated_at, access_count, metadata
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

        let metadata_value: serde_json::Value = row.try_get("metadata")
            .map_err(|e| CommonError::Database(e.to_string()))?;
        let metadata = serde_json::from_value(metadata_value)
            .unwrap_or_default();

        Ok(Some(Memory {
            id: row.try_get("id").map_err(|e| CommonError::Database(e.to_string()))?,
            content: row.try_get("content").map_err(|e| CommonError::Database(e.to_string()))?,
            context: row.try_get("context").ok(),
            tags: row.try_get("tags").map_err(|e| CommonError::Database(e.to_string()))?,
            collection: row.try_get("collection").ok(),
            created_at: row.try_get("created_at").map_err(|e| CommonError::Database(e.to_string()))?,
            updated_at: row.try_get("updated_at").map_err(|e| CommonError::Database(e.to_string()))?,
            access_count: row.try_get("access_count").map_err(|e| CommonError::Database(e.to_string()))?,
            metadata,
        }))
    }

    async fn search(&self, query: SearchQuery) -> Result<Vec<Memory>> {
        let limit = query.limit;
        let offset = query.offset;

        let rows = sqlx::query(
            r#"
            SELECT id, content, context, tags, collection, created_at, updated_at, access_count, metadata,
                   ts_rank(content_tsv, plainto_tsquery('english', $1)) as rank
            FROM memories
            WHERE content_tsv @@ plainto_tsquery('english', $1)
            ORDER BY rank DESC, created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(&query.query)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CommonError::Database(e.to_string()))?;

        let mut memories = Vec::new();
        for row in rows {
            let metadata_value: serde_json::Value = row.try_get("metadata")
                .map_err(|e| CommonError::Database(e.to_string()))?;
            let metadata = serde_json::from_value(metadata_value)
                .unwrap_or_default();

            memories.push(Memory {
                id: row.try_get("id").map_err(|e| CommonError::Database(e.to_string()))?,
                content: row.try_get("content").map_err(|e| CommonError::Database(e.to_string()))?,
                context: row.try_get("context").ok(),
                tags: row.try_get("tags").map_err(|e| CommonError::Database(e.to_string()))?,
                collection: row.try_get("collection").ok(),
                created_at: row.try_get("created_at").map_err(|e| CommonError::Database(e.to_string()))?,
                updated_at: row.try_get("updated_at").map_err(|e| CommonError::Database(e.to_string()))?,
                access_count: row.try_get("access_count").map_err(|e| CommonError::Database(e.to_string()))?,
                metadata,
            });
        }

        Ok(memories)
    }

    async fn list(&self, query: ListQuery) -> Result<Vec<Memory>> {
        let limit = query.limit;
        let offset = query.offset;

        let mut sql = String::from(
            "SELECT id, content, context, tags, collection, created_at, updated_at, access_count, metadata FROM memories WHERE 1=1"
        );
        
        let mut bind_idx = 1;
        if query.collection.is_some() {
            sql.push_str(&format!(" AND collection = ${}", bind_idx));
            bind_idx += 1;
        }
        if query.tags.is_some() {
            sql.push_str(&format!(" AND tags @> ${}", bind_idx));
            bind_idx += 1;
        }
        
        sql.push_str(&format!(" ORDER BY created_at DESC LIMIT ${} OFFSET ${}", bind_idx, bind_idx + 1));

        let mut query_builder = sqlx::query(&sql);
        
        if let Some(ref collection) = query.collection {
            query_builder = query_builder.bind(collection);
        }
        if let Some(ref tags) = query.tags {
            query_builder = query_builder.bind(tags);
        }
        
        query_builder = query_builder.bind(limit).bind(offset);

        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CommonError::Database(e.to_string()))?;

        let mut memories = Vec::new();
        for row in rows {
            let metadata_value: serde_json::Value = row.try_get("metadata")
                .map_err(|e| CommonError::Database(e.to_string()))?;
            let metadata = serde_json::from_value(metadata_value)
                .unwrap_or_default();

            memories.push(Memory {
                id: row.try_get("id").map_err(|e| CommonError::Database(e.to_string()))?,
                content: row.try_get("content").map_err(|e| CommonError::Database(e.to_string()))?,
                context: row.try_get("context").ok(),
                tags: row.try_get("tags").map_err(|e| CommonError::Database(e.to_string()))?,
                collection: row.try_get("collection").ok(),
                created_at: row.try_get("created_at").map_err(|e| CommonError::Database(e.to_string()))?,
                updated_at: row.try_get("updated_at").map_err(|e| CommonError::Database(e.to_string()))?,
                access_count: row.try_get("access_count").map_err(|e| CommonError::Database(e.to_string()))?,
                metadata,
            });
        }

        Ok(memories)
    }

    async fn update(&self, id: Uuid, update: UpdateMemory) -> Result<Memory> {
        let mut sql = String::from("UPDATE memories SET updated_at = NOW()");
        let mut bind_idx = 1;
        
        if update.content.is_some() {
            sql.push_str(&format!(", content = ${}", bind_idx));
            bind_idx += 1;
        }
        if update.context.is_some() {
            sql.push_str(&format!(", context = ${}", bind_idx));
            bind_idx += 1;
        }
        if update.tags.is_some() {
            sql.push_str(&format!(", tags = ${}", bind_idx));
            bind_idx += 1;
        }
        if update.collection.is_some() {
            sql.push_str(&format!(", collection = ${}", bind_idx));
            bind_idx += 1;
        }
        if update.metadata.is_some() {
            sql.push_str(&format!(", metadata = ${}", bind_idx));
            bind_idx += 1;
        }
        
        sql.push_str(&format!(" WHERE id = ${} RETURNING id, content, context, tags, collection, created_at, updated_at, access_count, metadata", bind_idx));

        let mut query_builder = sqlx::query(&sql);
        
        if let Some(ref content) = update.content {
            query_builder = query_builder.bind(content);
        }
        if let Some(ref context) = update.context {
            query_builder = query_builder.bind(context);
        }
        if let Some(ref tags) = update.tags {
            query_builder = query_builder.bind(tags);
        }
        if let Some(ref collection) = update.collection {
            query_builder = query_builder.bind(collection);
        }
        if let Some(ref metadata) = update.metadata {
            let metadata_json = serde_json::to_value(metadata)
                .map_err(|e| CommonError::Serialization(e.to_string()))?;
            query_builder = query_builder.bind(metadata_json);
        }
        
        query_builder = query_builder.bind(id);

        let row = query_builder
            .fetch_one(&self.pool)
            .await
            .map_err(|_e| CommonError::NotFound(format!("Memory not found: {}", id)))?;

        let metadata_value: serde_json::Value = row.try_get("metadata")
            .map_err(|e| CommonError::Database(e.to_string()))?;
        let metadata = serde_json::from_value(metadata_value)
            .unwrap_or_default();

        Ok(Memory {
            id: row.try_get("id").map_err(|e| CommonError::Database(e.to_string()))?,
            content: row.try_get("content").map_err(|e| CommonError::Database(e.to_string()))?,
            context: row.try_get("context").ok(),
            tags: row.try_get("tags").map_err(|e| CommonError::Database(e.to_string()))?,
            collection: row.try_get("collection").ok(),
            created_at: row.try_get("created_at").map_err(|e| CommonError::Database(e.to_string()))?,
            updated_at: row.try_get("updated_at").map_err(|e| CommonError::Database(e.to_string()))?,
            access_count: row.try_get("access_count").map_err(|e| CommonError::Database(e.to_string()))?,
            metadata,
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
            RETURNING id, content, context, tags, collection, created_at, updated_at, access_count, metadata
            "#
        )
        .bind(&tags)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_e| CommonError::NotFound(format!("Memory not found: {}", id)))?;

        let metadata_value: serde_json::Value = row.try_get("metadata")
            .map_err(|e| CommonError::Database(e.to_string()))?;
        let metadata = serde_json::from_value(metadata_value)
            .unwrap_or_default();

        Ok(Memory {
            id: row.try_get("id").map_err(|e| CommonError::Database(e.to_string()))?,
            content: row.try_get("content").map_err(|e| CommonError::Database(e.to_string()))?,
            context: row.try_get("context").ok(),
            tags: row.try_get("tags").map_err(|e| CommonError::Database(e.to_string()))?,
            collection: row.try_get("collection").ok(),
            created_at: row.try_get("created_at").map_err(|e| CommonError::Database(e.to_string()))?,
            updated_at: row.try_get("updated_at").map_err(|e| CommonError::Database(e.to_string()))?,
            access_count: row.try_get("access_count").map_err(|e| CommonError::Database(e.to_string()))?,
            metadata,
        })
    }

    async fn remove_tags(&self, id: Uuid, tags: Vec<String>) -> Result<Memory> {
        let row = sqlx::query(
            r#"
            UPDATE memories
            SET tags = array(SELECT unnest(tags) EXCEPT SELECT unnest($1::text[]))
            WHERE id = $2
            RETURNING id, content, context, tags, collection, created_at, updated_at, access_count, metadata
            "#
        )
        .bind(&tags)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_e| CommonError::NotFound(format!("Memory not found: {}", id)))?;

        let metadata_value: serde_json::Value = row.try_get("metadata")
            .map_err(|e| CommonError::Database(e.to_string()))?;
        let metadata = serde_json::from_value(metadata_value)
            .unwrap_or_default();

        Ok(Memory {
            id: row.try_get("id").map_err(|e| CommonError::Database(e.to_string()))?,
            content: row.try_get("content").map_err(|e| CommonError::Database(e.to_string()))?,
            context: row.try_get("context").ok(),
            tags: row.try_get("tags").map_err(|e| CommonError::Database(e.to_string()))?,
            collection: row.try_get("collection").ok(),
            created_at: row.try_get("created_at").map_err(|e| CommonError::Database(e.to_string()))?,
            updated_at: row.try_get("updated_at").map_err(|e| CommonError::Database(e.to_string()))?,
            access_count: row.try_get("access_count").map_err(|e| CommonError::Database(e.to_string()))?,
            metadata,
        })
    }

    async fn list_tags(&self) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT unnest(tags) as tag
            FROM memories
            ORDER BY tag
            "#
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
            "#
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
