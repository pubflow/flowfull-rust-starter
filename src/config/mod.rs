use serde::Deserialize;
use std::{env, net::SocketAddr, time::Duration};

use crate::auth::ValidationMode;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub port: u16,
    pub host: String,
    pub environment: String,
    pub base_url: String,

    pub database_url: String,
    pub database_max_connections: u32,
    pub database_min_connections: u32,
    pub database_connect_timeout_seconds: u64,

    pub flowless_api_url: String,
    pub bridge_validation_endpoint: String,
    pub bridge_validation_secret: String,
    pub bridge_secret_in_body: bool,
    pub bridge_validation_timeout_ms: u64,
    pub bridge_retry_attempts: usize,

    pub session_validation_cache_ttl: u64,
    pub session_header_name: String,
    pub session_cookie_name: String,
    pub session_allow_query: bool,

    pub auth_validation_mode: ValidationMode,
    pub auth_enable_validation_mode: bool,
    pub auth_ip_validation: bool,
    pub auth_user_agent_validation: bool,
    pub auth_device_validation: bool,
    pub auth_auto_invalidate: bool,
    pub auth_log_violations: bool,

    pub cache_enabled: bool,
    pub cache_max_capacity: u64,
    pub redis_url: Option<String>,

    pub trust_token_private_key: Option<String>,
    pub token_ttl_hours: u64,
    pub token_email_verification_ttl_hours: u64,
    pub token_password_reset_ttl_hours: u64,
    pub token_invitation_ttl_hours: u64,

    pub cors_origins: String,
    pub cors_methods: String,
    pub cors_headers: String,
    pub cors_credentials: bool,
    pub cors_max_age: u64,

    pub rate_limit_enabled: bool,
    pub rate_limit_requests: u64,
    pub rate_limit_window: u64,

    pub log_level: String,
    pub log_format: String,
    pub dev_mode: bool,
    pub dev_cors_relaxed: bool,
    pub dev_log_requests: bool,
}

impl Settings {
    pub fn load() -> anyhow::Result<Self> {
        let settings = config::Config::builder()
            .set_default("PORT", 3001)?
            .set_default("HOST", "0.0.0.0")?
            .set_default("ENVIRONMENT", "development")?
            .set_default("BASE_URL", "http://localhost:3001")?
            .set_default("DATABASE_URL", "sqlite://flowfull.db")?
            .set_default("DATABASE_MAX_CONNECTIONS", 20)?
            .set_default("DATABASE_MIN_CONNECTIONS", 1)?
            .set_default("DATABASE_CONNECT_TIMEOUT_SECONDS", 10)?
            .set_default("FLOWLESS_API_URL", "https://api.pubflow.com")?
            .set_default("BRIDGE_VALIDATION_ENDPOINT", "/auth/bridge/validate")?
            .set_default(
                "BRIDGE_VALIDATION_SECRET",
                "change-me-change-me-change-me-32chars",
            )?
            .set_default("BRIDGE_SECRET_IN_BODY", false)?
            .set_default("BRIDGE_VALIDATION_TIMEOUT_MS", 5000)?
            .set_default("BRIDGE_RETRY_ATTEMPTS", 3)?
            .set_default("SESSION_VALIDATION_CACHE_TTL", 300)?
            .set_default("SESSION_HEADER_NAME", "X-Session-Id")?
            .set_default("SESSION_COOKIE_NAME", "session_id")?
            .set_default("SESSION_ALLOW_QUERY", false)?
            .set_default("AUTH_VALIDATION_MODE", "STANDARD")?
            .set_default("AUTH_ENABLE_VALIDATION_MODE", true)?
            .set_default("AUTH_IP_VALIDATION", true)?
            .set_default("AUTH_USER_AGENT_VALIDATION", true)?
            .set_default("AUTH_DEVICE_VALIDATION", false)?
            .set_default("AUTH_AUTO_INVALIDATE", false)?
            .set_default("AUTH_LOG_VIOLATIONS", true)?
            .set_default("CACHE_ENABLED", true)?
            .set_default("CACHE_MAX_CAPACITY", 50000)?
            .set_default("TOKEN_TTL_HOURS", 168)?
            .set_default("TOKEN_EMAIL_VERIFICATION_TTL_HOURS", 24)?
            .set_default("TOKEN_PASSWORD_RESET_TTL_HOURS", 1)?
            .set_default("TOKEN_INVITATION_TTL_HOURS", 168)?
            .set_default("CORS_ORIGINS", "http://localhost:3000")?
            .set_default("CORS_METHODS", "GET,POST,PUT,DELETE,OPTIONS")?
            .set_default("CORS_HEADERS", "Content-Type,Authorization,X-Session-Id")?
            .set_default("CORS_CREDENTIALS", true)?
            .set_default("CORS_MAX_AGE", 86400)?
            .set_default("RATE_LIMIT_ENABLED", true)?
            .set_default("RATE_LIMIT_REQUESTS", 100)?
            .set_default("RATE_LIMIT_WINDOW", 60)?
            .set_default("LOG_LEVEL", "info")?
            .set_default("LOG_FORMAT", "json")?
            .set_default("DEV_MODE", true)?
            .set_default("DEV_CORS_RELAXED", true)?
            .set_default("DEV_LOG_REQUESTS", true)?
            .add_source(config::File::with_name(".env").required(false))
            .add_source(config::Environment::default())
            .build()?
            .try_deserialize::<Self>()?;

        settings.validate()?;
        Ok(settings)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.bridge_validation_secret.len() < 32 {
            anyhow::bail!("BRIDGE_VALIDATION_SECRET must be at least 32 characters");
        }
        if !(self.flowless_api_url.starts_with("http://")
            || self.flowless_api_url.starts_with("https://"))
        {
            anyhow::bail!("FLOWLESS_API_URL must start with http:// or https://");
        }
        if self.database_url.is_empty() {
            anyhow::bail!("DATABASE_URL is required");
        }
        if self.is_production() && self.trust_token_private_key.is_none() {
            anyhow::bail!("TRUST_TOKEN_PRIVATE_KEY is required in production");
        }
        Ok(())
    }

    pub fn addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(format!("{}:{}", self.host, self.port).parse()?)
    }

    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }

    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    pub fn cors_origins(&self) -> Vec<String> {
        split_csv(&self.cors_origins)
    }

    pub fn cors_methods(&self) -> Vec<String> {
        split_csv(&self.cors_methods)
    }

    pub fn cors_headers(&self) -> Vec<String> {
        split_csv(&self.cors_headers)
    }

    pub fn bridge_timeout(&self) -> Duration {
        Duration::from_millis(self.bridge_validation_timeout_ms)
    }
}

pub fn init_tracing(settings: &Settings) {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&settings.log_level));

    if settings.log_format == "json" {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
}

pub fn env_or_default(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}
