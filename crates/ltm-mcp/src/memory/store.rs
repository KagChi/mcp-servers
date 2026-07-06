use async_trait::async_trait;
use uuid::Uuid;
use mcp_common::error::Result;

use super::types::{Memory, CreateMemory, UpdateMemory, SearchQuery, ListQuery};

#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn store(&self, memory: CreateMemory) -> Result<Memory>;
    async fn get(&self, id: Uuid) -> Result<Option<Memory>>;
    async fn search(&self, query: SearchQuery) -> Result<Vec<Memory>>;
    async fn list(&self, query: ListQuery) -> Result<Vec<Memory>>;
    async fn update(&self, id: Uuid, update: UpdateMemory) -> Result<Memory>;
    async fn delete(&self, id: Uuid) -> Result<()>;
    async fn add_tags(&self, id: Uuid, tags: Vec<String>) -> Result<Memory>;
    async fn remove_tags(&self, id: Uuid, tags: Vec<String>) -> Result<Memory>;
    async fn list_tags(&self) -> Result<Vec<String>>;
    async fn list_collections(&self) -> Result<Vec<String>>;
}
