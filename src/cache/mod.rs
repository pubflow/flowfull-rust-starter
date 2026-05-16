use std::time::Duration;

use moka::future::Cache;

use crate::{auth::SessionData, config::Settings};

#[derive(Debug, Clone, Default)]
pub struct CacheMetrics {
    pub local_hits: u64,
    pub local_misses: u64,
    pub redis_hits: u64,
    pub redis_misses: u64,
}

#[derive(Clone)]
pub struct HybridCache {
    enabled: bool,
    local: Cache<String, SessionData>,
    ttl: Duration,
    #[cfg(feature = "redis-cache")]
    redis: Option<redis::aio::ConnectionManager>,
    metrics: std::sync::Arc<tokio::sync::RwLock<CacheMetrics>>,
}

impl HybridCache {
    pub async fn new(settings: &Settings) -> anyhow::Result<Self> {
        #[cfg(feature = "redis-cache")]
        let redis = if let Some(url) = &settings.redis_url {
            let client = redis::Client::open(url.as_str())?;
            Some(client.get_connection_manager().await?)
        } else {
            None
        };

        Ok(Self {
            enabled: settings.cache_enabled,
            local: Cache::builder()
                .max_capacity(settings.cache_max_capacity)
                .time_to_live(Duration::from_secs(settings.session_validation_cache_ttl))
                .build(),
            ttl: Duration::from_secs(settings.session_validation_cache_ttl),
            #[cfg(feature = "redis-cache")]
            redis,
            metrics: Default::default(),
        })
    }

    pub fn disabled_for_tests() -> Self {
        Self {
            enabled: false,
            local: Cache::new(1),
            ttl: Duration::from_secs(1),
            #[cfg(feature = "redis-cache")]
            redis: None,
            metrics: Default::default(),
        }
    }

    pub async fn get(&self, key: &str) -> Option<SessionData> {
        if !self.enabled {
            return None;
        }

        if let Some(value) = self.local.get(key).await {
            self.metrics.write().await.local_hits += 1;
            return Some(value);
        }
        self.metrics.write().await.local_misses += 1;

        #[cfg(feature = "redis-cache")]
        if let Some(redis) = &self.redis {
            let mut redis = redis.clone();
            let value: redis::RedisResult<Option<String>> =
                redis::AsyncCommands::get(&mut redis, key).await;
            match value {
                Ok(Some(json)) => {
                    self.metrics.write().await.redis_hits += 1;
                    if let Ok(session) = serde_json::from_str::<SessionData>(&json) {
                        self.local.insert(key.to_string(), session.clone()).await;
                        return Some(session);
                    }
                }
                _ => {
                    self.metrics.write().await.redis_misses += 1;
                }
            }
        }

        None
    }

    pub async fn set(&self, key: String, value: SessionData) -> anyhow::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        self.local.insert(key.clone(), value.clone()).await;

        #[cfg(feature = "redis-cache")]
        if let Some(redis) = &self.redis {
            let mut redis = redis.clone();
            let json = serde_json::to_string(&value)?;
            let _: () =
                redis::AsyncCommands::set_ex(&mut redis, key, json, self.ttl.as_secs()).await?;
        }

        Ok(())
    }

    pub async fn metrics(&self) -> CacheMetrics {
        self.metrics.read().await.clone()
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn redis_enabled(&self) -> bool {
        #[cfg(feature = "redis-cache")]
        {
            self.redis.is_some()
        }
        #[cfg(not(feature = "redis-cache"))]
        {
            false
        }
    }
}
