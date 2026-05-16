use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;

use crate::{AppState, db};

pub async fn basic() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "flowfull-rust-starter"
    }))
}

pub async fn database(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match db::ping(&state.db).await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({"status": "ok", "database": "connected"})),
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"status": "error", "error": "database ping failed"})),
        ),
    }
}

pub async fn cache(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let metrics = state.cache.metrics().await;
    Json(json!({
        "status": "ok",
        "cache": if state.cache.enabled() { "enabled" } else { "disabled" },
        "redis": if state.cache.redis_enabled() { "connected" } else { "disabled" },
        "metrics": {
            "local_hits": metrics.local_hits,
            "local_misses": metrics.local_misses,
            "redis_hits": metrics.redis_hits,
            "redis_misses": metrics.redis_misses
        }
    }))
}

pub async fn all(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db_ok = db::ping(&state.db).await.is_ok();
    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(json!({
            "status": if db_ok { "ok" } else { "degraded" },
            "service": "flowfull-rust-starter",
            "database": if db_ok { "ok" } else { "error" },
            "cache": if state.cache.enabled() { "enabled" } else { "disabled" }
        })),
    )
}
