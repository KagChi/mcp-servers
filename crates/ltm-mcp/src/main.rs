use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod config;
mod memory;
mod server;
mod tools;

use config::Config;
use memory::postgres::PostgresStore;
use server::LtmServer;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if it exists (for local development)
    // This will not override existing environment variables
    let _ = dotenvy::dotenv();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting LTM-MCP server");

    // Load configuration from environment variables
    let config = Config::from_env()?;
    tracing::info!("Configuration loaded successfully");
    tracing::info!(
        "Server will listen on {}:{}",
        config.server.host,
        config.server.port
    );
    tracing::info!("Log level: {}", config.log.level);

    // Create database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database.url)
        .await?;
    tracing::info!("Database connection pool created");

    // Create PostgresStore and run migrations
    let store = PostgresStore::new(pool);
    store.run_migrations().await?;
    tracing::info!("Database migrations completed");

    let store = Arc::new(store);
    tracing::info!("LTM server store initialized");

    // HTTP/SSE transport for remote access
    use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
    use rmcp::transport::streamable_http_server::StreamableHttpServerConfig;
    use rmcp::transport::StreamableHttpService;
    use std::net::SocketAddr;

    let addr = format!("{}:{}", config.server.host, config.server.port).parse::<SocketAddr>()?;

    tracing::info!("Starting MCP server on HTTP/SSE transport at {}", addr);

    // Create session manager for stateful MCP sessions
    let session_manager = Arc::new(LocalSessionManager::default());

    // Create service factory that returns new server instances
    let store_clone = store.clone();
    let config_clone = config.clone();
    let service_factory = move || {
        let server_instance = LtmServer::new(store_clone.clone(), config_clone.clone());
        Ok(server_instance)
    };

    // Create StreamableHttpService with default config
    let mcp_service = StreamableHttpService::new(
        service_factory,
        session_manager,
        StreamableHttpServerConfig::default(),
    );

    // Mount the MCP service at /mcp endpoint
    let router = axum::Router::new().nest_service("/mcp", mcp_service);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("HTTP server listening on {}", addr);
    tracing::info!("MCP endpoint available at http://{}/mcp", addr);

    axum::serve(listener, router).await?;
    Ok(())
}
