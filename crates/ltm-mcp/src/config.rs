use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    #[allow(dead_code)]
    pub auth: AuthConfig,
    pub log: LogConfig,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    #[allow(dead_code)]
    pub api_key: String,
}

#[derive(Debug, Clone)]
pub struct LogConfig {
    pub level: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
        }
    }
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let server = ServerConfig {
            host: env::var("LTM_SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("LTM_SERVER_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
        };

        let database = DatabaseConfig {
            url: env::var("LTM_DATABASE_URL")
                .map_err(|_| anyhow::anyhow!("LTM_DATABASE_URL is required"))?,
        };

        let auth = AuthConfig {
            api_key: env::var("LTM_AUTH_API_KEY").unwrap_or_default(),
        };

        let log = LogConfig {
            level: env::var("LTM_LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        };

        Ok(Config {
            server,
            database,
            auth,
            log,
        })
    }
}
