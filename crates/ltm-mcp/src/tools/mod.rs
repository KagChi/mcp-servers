use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::embedding::EmbeddingService;
use crate::memory::{
    CreateMemory, ListQuery, Memory, MemoryStore, PostgresStore, SearchQuery, UpdateMemory,
};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EmptyParams {}

// Store memory tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct StoreMemoryParams {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

pub async fn store_memory(
    store: Arc<PostgresStore>,
    embedding_service: Option<Arc<EmbeddingService>>,
    params: StoreMemoryParams,
) -> Result<Memory> {
    // Auto-generate embedding if not provided and service is available
    let embedding = match params.embedding {
        Some(emb) => Some(emb),
        None => {
            if let Some(service) = embedding_service {
                info!("Auto-generating embedding for memory content");
                match service.embed(&params.content) {
                    Ok(emb) => {
                        info!(
                            "Successfully generated embedding with {} dimensions",
                            emb.len()
                        );
                        Some(emb)
                    }
                    Err(e) => {
                        warn!(
                            "Failed to generate embedding: {}. Storing without embedding.",
                            e
                        );
                        None
                    }
                }
            } else {
                None
            }
        }
    };

    let create = CreateMemory {
        content: params.content,
        context: params.context,
        tags: params.tags,
        collection: params.collection,
        metadata: Default::default(),
        scope: params.scope,
        embedding,
    };

    store
        .store(create)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

// Get memory tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetMemoryParams {
    pub id: String,
}

pub async fn get_memory(
    store: Arc<PostgresStore>,
    params: GetMemoryParams,
) -> Result<Option<Memory>> {
    let id = Uuid::parse_str(&params.id).map_err(|e| anyhow::anyhow!("Invalid UUID: {}", e))?;

    store.get(id).await.map_err(|e| anyhow::anyhow!("{}", e))
}

// Search memories tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchMemoriesParams {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_embedding: Option<Vec<f32>>,
    #[serde(default)]
    pub search_mode: crate::memory::SearchMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

fn default_limit() -> i64 {
    10
}

pub async fn search_memories(
    store: Arc<PostgresStore>,
    embedding_service: Option<Arc<EmbeddingService>>,
    params: SearchMemoriesParams,
) -> Result<Vec<Memory>> {
    // Auto-generate query embedding if not provided and service is available
    let query_embedding = match params.query_embedding {
        Some(emb) => Some(emb),
        None => {
            if let Some(service) = embedding_service {
                // Only generate if search mode requires embeddings
                if params.search_mode == crate::memory::SearchMode::Semantic
                    || params.search_mode == crate::memory::SearchMode::Hybrid
                {
                    info!("Auto-generating query embedding for search");
                    match service.embed(&params.query) {
                        Ok(emb) => {
                            info!(
                                "Successfully generated query embedding with {} dimensions",
                                emb.len()
                            );
                            Some(emb)
                        }
                        Err(e) => {
                            warn!("Failed to generate query embedding: {}. Falling back to keyword search.", e);
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
    };

    let query = SearchQuery {
        query: params.query,
        limit: params.limit,
        offset: params.offset,
        query_embedding,
        search_mode: params.search_mode,
        scope: params.scope,
        collection: params.collection,
        tags: params.tags,
    };

    store
        .search(query)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

// List memories tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListMemoriesParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

pub async fn list_memories(
    store: Arc<PostgresStore>,
    params: ListMemoriesParams,
) -> Result<Vec<Memory>> {
    let query = ListQuery {
        collection: params.collection,
        tags: params.tags,
        limit: params.limit,
        offset: params.offset,
        scope: params.scope,
    };

    store
        .list(query)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

// Update memory tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateMemoryParams {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

pub async fn update_memory(
    store: Arc<PostgresStore>,
    params: UpdateMemoryParams,
) -> Result<Memory> {
    let id = Uuid::parse_str(&params.id).map_err(|e| anyhow::anyhow!("Invalid UUID: {}", e))?;

    let update = UpdateMemory {
        content: params.content,
        context: params.context,
        tags: params.tags,
        collection: params.collection,
        metadata: None,
        scope: params.scope,
        embedding: params.embedding,
    };

    store
        .update(id, update)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

// Delete memory tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteMemoryParams {
    pub id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DeleteMemoryResult {
    pub success: bool,
}

pub async fn delete_memory(
    store: Arc<PostgresStore>,
    params: DeleteMemoryParams,
) -> Result<DeleteMemoryResult> {
    let id = Uuid::parse_str(&params.id).map_err(|e| anyhow::anyhow!("Invalid UUID: {}", e))?;

    store
        .delete(id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok(DeleteMemoryResult { success: true })
}

// Add tags tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddTagsParams {
    pub id: String,
    pub tags: Vec<String>,
}

pub async fn add_tags(store: Arc<PostgresStore>, params: AddTagsParams) -> Result<Memory> {
    let id = Uuid::parse_str(&params.id).map_err(|e| anyhow::anyhow!("Invalid UUID: {}", e))?;

    store
        .add_tags(id, params.tags)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

// Remove tags tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveTagsParams {
    pub id: String,
    pub tags: Vec<String>,
}

pub async fn remove_tags(store: Arc<PostgresStore>, params: RemoveTagsParams) -> Result<Memory> {
    let id = Uuid::parse_str(&params.id).map_err(|e| anyhow::anyhow!("Invalid UUID: {}", e))?;

    store
        .remove_tags(id, params.tags)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

// List tags tool
pub async fn list_tags(store: Arc<PostgresStore>) -> Result<Vec<String>> {
    store
        .list_tags()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

// List collections tool
pub async fn list_collections(store: Arc<PostgresStore>) -> Result<Vec<String>> {
    store
        .list_collections()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
