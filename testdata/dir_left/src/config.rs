use serde::Deserialize;
use std::env;

/// Server and application configuration loaded from environment variables.
#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub log_level: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            database_url: "postgres://localhost/myapp".to_string(),
            jwt_secret: "change-me-in-production".to_string(),
            log_level: "info".to_string(),
        }
    }
}

/// Load configuration from environment variables, falling back to defaults.
pub fn load() -> Result<ServerConfig, config::ConfigError> {
    let cfg = ServerConfig {
        host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
        port: env::var("PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8080),
        database_url: env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/myapp".to_string()),
        jwt_secret: env::var("JWT_SECRET")
            .expect("JWT_SECRET must be set"),
        log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
    };
    Ok(cfg)
}
