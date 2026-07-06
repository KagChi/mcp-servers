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
    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting LTM-MCP server");

    // Load configuration from environment variables
    let config = Config::from_env()?;
    tracing::info!("Configuration loaded successfully");
    tracing::info!("Server will listen on {}:{}", config.server.host, config.server.port);
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

    // Create LtmServer instance
    let server = LtmServer::new(store, config.clone());
    tracing::info!("LTM server instance created");

    // Use rmcp stdio transport for MCP protocol
    use rmcp::transport::io::stdio;
    use rmcp::ServiceExt;

    tracing::info!("Starting MCP server on stdio transport");
    
    let running = server.serve(stdio()).await?;
    
    tracing::info!("MCP server initialized and running");
    
    // Wait for the server to finish
    let result = running.waiting().await;
    
    match result {
        Ok(_) => {
            tracing::info!("Server shut down gracefully");
            Ok(())
        }
        Err(e) => {
            tracing::error!("Server error: {}", e);
            Err(anyhow::anyhow!("Server error: {}", e))
        }
    }
}
