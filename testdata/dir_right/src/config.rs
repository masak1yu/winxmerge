use serde::Deserialize;
use std::env;

/// Server and application configuration loaded from environment variables.
#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub database_pool_size: u32,
    pub jwt_secret: String,
    pub jwt_expiry_secs: u64,
    pub log_level: String,
    pub metrics_enabled: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            database_url: "postgres://localhost/myapp".to_string(),
            database_pool_size: 10,
            jwt_secret: "change-me-in-production".to_string(),
            jwt_expiry_secs: 3600,
            log_level: "info".to_string(),
            metrics_enabled: true,
        }
    }
}

/// Errors that can occur during configuration loading.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingVar(String),
    #[error("Invalid value for {0}: {1}")]
    InvalidValue(String, String),
}

/// Load configuration from environment variables, falling back to defaults.
pub fn load() -> Result<ServerConfig, ConfigError> {
    let cfg = ServerConfig {
        host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
        port: env::var("PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8080),
        database_url: env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/myapp".to_string()),
        database_pool_size: env::var("DATABASE_POOL_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10),
        jwt_secret: env::var("JWT_SECRET")
            .map_err(|_| ConfigError::MissingVar("JWT_SECRET".into()))?,
        jwt_expiry_secs: env::var("JWT_EXPIRY_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3600),
        log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        metrics_enabled: env::var("METRICS_ENABLED")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(true),
    };
    Ok(cfg)
}
