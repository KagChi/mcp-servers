use std::sync::Arc;
use serde_json;

use rmcp::{
    ServerHandler,
    model::{
        Tool, ServerInfo, ServerCapabilities, ToolsCapability,
        CallToolRequestParams, CallToolResult,
        ListToolsResult, InitializeResult, ProtocolVersion,
        Content, PaginatedRequestParams, InitializeRequestParams,
        Implementation,
    },
    service::{RequestContext, RoleServer},
    ErrorData,
};

use crate::config::Config;
use crate::memory::postgres::PostgresStore;
use crate::tools::*;

#[derive(Clone)]
pub struct LtmServer {
    #[allow(dead_code)]
    store: Arc<PostgresStore>,
    #[allow(dead_code)]
    config: Config,
}

impl LtmServer {
    pub fn new(store: Arc<PostgresStore>, config: Config) -> Self {
        Self { store, config }
    }
}

impl ServerHandler for LtmServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: None,
                }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "ltm-mcp".to_string(),
                title: None,
                version: env!("CARGO_PKG_VERSION").to_string(),
                description: Some("Long-Term Memory MCP server with PostgreSQL storage".to_string()),
                icons: None,
                website_url: None,
            },
            instructions: Some("Use the provided tools to store, retrieve, search, and manage memory entries in PostgreSQL.".to_string()),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        Ok(self.get_info())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let tools = vec![
            Tool::new("store_memory", "Store a new memory entry", Arc::new(serde_json::to_value(&schemars::schema_for!(StoreMemoryParams).schema).unwrap().as_object().unwrap().clone())),
            Tool::new("get_memory", "Retrieve a memory entry by ID", Arc::new(serde_json::to_value(&schemars::schema_for!(GetMemoryParams).schema).unwrap().as_object().unwrap().clone())),
            Tool::new("search_memories", "Search memories by text query using full-text search", Arc::new(serde_json::to_value(&schemars::schema_for!(SearchMemoriesParams).schema).unwrap().as_object().unwrap().clone())),
            Tool::new("list_memories", "List all memories with optional filtering", Arc::new(serde_json::to_value(&schemars::schema_for!(ListMemoriesParams).schema).unwrap().as_object().unwrap().clone())),
            Tool::new("update_memory", "Update an existing memory entry", Arc::new(serde_json::to_value(&schemars::schema_for!(UpdateMemoryParams).schema).unwrap().as_object().unwrap().clone())),
            Tool::new("delete_memory", "Delete a memory entry", Arc::new(serde_json::to_value(&schemars::schema_for!(DeleteMemoryParams).schema).unwrap().as_object().unwrap().clone())),
            Tool::new("add_tags", "Add tags to a memory entry", Arc::new(serde_json::to_value(&schemars::schema_for!(AddTagsParams).schema).unwrap().as_object().unwrap().clone())),
            Tool::new("remove_tags", "Remove tags from a memory entry", Arc::new(serde_json::to_value(&schemars::schema_for!(RemoveTagsParams).schema).unwrap().as_object().unwrap().clone())),
            Tool::new("list_tags", "List all unique tags", Arc::new(serde_json::to_value(&schemars::schema_for!(()).schema).unwrap().as_object().unwrap().clone())),
            Tool::new("list_collections", "List all unique collections", Arc::new(serde_json::to_value(&schemars::schema_for!(()).schema).unwrap().as_object().unwrap().clone())),
        ];

        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let tool_name = request.name.as_ref();
        let store = Arc::clone(&self.store);

        let result_json = match tool_name {
            "store_memory" => {
                let params: StoreMemoryParams = serde_json::from_value(
                    serde_json::to_value(&request.arguments).unwrap()
                )
                .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                
                let result = store_memory(store, params)
                    .await
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                
                serde_json::to_string(&result)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            "get_memory" => {
                let params: GetMemoryParams = serde_json::from_value(
                    serde_json::to_value(&request.arguments).unwrap()
                )
                .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                
                let result = get_memory(store, params)
                    .await
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                
                serde_json::to_string(&result)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            "search_memories" => {
                let params: SearchMemoriesParams = serde_json::from_value(
                    serde_json::to_value(&request.arguments).unwrap()
                )
                .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                
                let result = search_memories(store, params)
                    .await
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                
                serde_json::to_string(&result)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            "list_memories" => {
                let params: ListMemoriesParams = serde_json::from_value(
                    serde_json::to_value(&request.arguments).unwrap()
                )
                .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                
                let result = list_memories(store, params)
                    .await
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                
                serde_json::to_string(&result)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            "update_memory" => {
                let params: UpdateMemoryParams = serde_json::from_value(
                    serde_json::to_value(&request.arguments).unwrap()
                )
                .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                
                let result = update_memory(store, params)
                    .await
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                
                serde_json::to_string(&result)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            "delete_memory" => {
                let params: DeleteMemoryParams = serde_json::from_value(
                    serde_json::to_value(&request.arguments).unwrap()
                )
                .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                
                let result = delete_memory(store, params)
                    .await
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                
                serde_json::to_string(&result)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            "add_tags" => {
                let params: AddTagsParams = serde_json::from_value(
                    serde_json::to_value(&request.arguments).unwrap()
                )
                .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                
                let result = add_tags(store, params)
                    .await
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                
                serde_json::to_string(&result)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            "remove_tags" => {
                let params: RemoveTagsParams = serde_json::from_value(
                    serde_json::to_value(&request.arguments).unwrap()
                )
                .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                
                let result = remove_tags(store, params)
                    .await
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                
                serde_json::to_string(&result)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            "list_tags" => {
                let result = list_tags(store)
                    .await
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                
                serde_json::to_string(&result)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            "list_collections" => {
                let result = list_collections(store)
                    .await
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                
                serde_json::to_string(&result)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            _ => return Err(ErrorData::invalid_params(format!("Tool not found: {}", tool_name), None)),
        };

        Ok(CallToolResult {
            content: vec![Content {
                raw: rmcp::model::RawContent::Text(rmcp::model::RawTextContent {
                    text: result_json,
                    meta: None,
                }),
                annotations: None,
            }],
            structured_content: None,
            is_error: None,
            meta: None,
        })
    }
}
