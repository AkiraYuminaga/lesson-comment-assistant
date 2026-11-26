use crate::error::{AppError, AppResult};
use secrecy::Secret;
use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Directory holding courses.json / prompt.md / style.json (see `profile.rs`).
    pub config_dir: std::path::PathBuf,
    pub server: ServerConfig,
    pub gemini: GeminiConfig,
    pub security: SecurityConfig,
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct GeminiConfig {
    pub api_key: Secret<String>,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub hmac_secret: Secret<String>,
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub per_minute: u32,
}

impl AppConfig {
    pub fn from_env() -> AppResult<Self> {
        dotenvy::dotenv().ok(); // Load .env file if exists

        let server = ServerConfig {
            host: env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8787".to_string())
                .parse()
                .map_err(|e| AppError::ConfigError(format!("Invalid SERVER_PORT: {}", e)))?,
        };

        let gemini = GeminiConfig {
            api_key: Secret::new(
                env::var("GEMINI_API_KEY")
                    .map_err(|_| AppError::ConfigError("GEMINI_API_KEY not set".to_string()))?,
            ),
            model: env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-3.1-flash-lite".to_string()),
        };

        let security = SecurityConfig {
            hmac_secret: Secret::new(
                env::var("HMAC_SECRET")
                    .map_err(|_| AppError::ConfigError("HMAC_SECRET not set".to_string()))?,
            ),
            allowed_origins: env::var("CORS_ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000,http://127.0.0.1:3000".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        };

        let rate_limit = RateLimitConfig {
            per_minute: env::var("RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .map_err(|e| {
                    AppError::ConfigError(format!("Invalid RATE_LIMIT_PER_MINUTE: {}", e))
                })?,
        };

        Ok(Self {
            config_dir: env::var("CONFIG_DIR")
                .unwrap_or_else(|_| "config".to_string())
                .into(),
            server,
            gemini,
            security,
            rate_limit,
        })
    }
}
