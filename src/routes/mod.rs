pub mod api;
pub mod health;

use std::sync::Arc;

use axum::{Json, extract::State, response::IntoResponse};
use serde_json::json;

use crate::AppState;

pub async fn root(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({
        "service": "flowfull-rust-starter",
        "version": env!("CARGO_PKG_VERSION"),
        "environment": state.settings.environment,
        "status": "running"
    }))
}
