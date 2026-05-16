use std::sync::Arc;

use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{AppState, auth::SessionData};

#[derive(Debug, Deserialize)]
pub struct TaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct MockTask {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub created_at: String,
}

pub async fn public() -> impl IntoResponse {
    Json(json!({
        "message": "This is a public endpoint",
        "timestamp": Utc::now(),
        "authenticated": false
    }))
}

pub async fn config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({
        "validation_mode": state.settings.auth_validation_mode,
        "flowless_url": state.settings.flowless_api_url,
        "bridge_endpoint": state.settings.bridge_validation_endpoint,
        "cache_enabled": state.settings.cache_enabled,
        "environment": state.settings.environment
    }))
}

pub async fn protected(Extension(session): Extension<SessionData>) -> impl IntoResponse {
    Json(json!({
        "message": "This is a protected endpoint",
        "user": {
            "id": session.user_id,
            "email": session.email
        }
    }))
}

pub async fn optional(session: Option<Extension<SessionData>>) -> impl IntoResponse {
    if let Some(Extension(session)) = session {
        Json(json!({
            "message": "Authenticated user",
            "user_id": session.user_id,
            "authenticated": true
        }))
    } else {
        Json(json!({
            "message": "Anonymous user",
            "authenticated": false
        }))
    }
}

pub async fn profile(Extension(session): Extension<SessionData>) -> impl IntoResponse {
    Json(json!({
        "user": {
            "id": session.user_id,
            "email": session.email,
            "name": session.name,
            "user_type": session.user_type,
            "organization_id": session.organization_id,
            "permissions": session.permissions
        }
    }))
}

pub async fn admin_dashboard(Extension(session): Extension<SessionData>) -> impl IntoResponse {
    Json(json!({
        "message": "Admin dashboard",
        "user_id": session.user_id,
        "user_type": session.user_type,
        "timestamp": Utc::now()
    }))
}

pub async fn list_tasks(Extension(session): Extension<SessionData>) -> impl IntoResponse {
    let tasks = vec![
        MockTask {
            id: "1".to_string(),
            user_id: session.user_id.clone(),
            title: "Example Task 1".to_string(),
            description: "This is a mock task".to_string(),
            completed: false,
            created_at: Utc::now().to_rfc3339(),
        },
        MockTask {
            id: "2".to_string(),
            user_id: session.user_id,
            title: "Example Task 2".to_string(),
            description: "Another mock task".to_string(),
            completed: true,
            created_at: Utc::now().to_rfc3339(),
        },
    ];

    Json(json!({
        "tasks": tasks,
        "count": tasks.len(),
        "message": "Mock data - not stored in database"
    }))
}

pub async fn create_task(
    Extension(session): Extension<SessionData>,
    Json(body): Json<TaskRequest>,
) -> impl IntoResponse {
    let task = MockTask {
        id: format!("mock-{}", Uuid::new_v4()),
        user_id: session.user_id,
        title: body.title.unwrap_or_else(|| "Untitled task".to_string()),
        description: body.description.unwrap_or_default(),
        completed: false,
        created_at: Utc::now().to_rfc3339(),
    };

    (
        StatusCode::CREATED,
        Json(json!({
            "message": "Mock task created (not saved to database)",
            "task": task
        })),
    )
}

pub async fn get_task(
    Extension(session): Extension<SessionData>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    Json(json!({
        "task": {
            "id": task_id,
            "user_id": session.user_id,
            "title": "Mock Task",
            "description": "This is a mock task retrieved by ID",
            "completed": false,
            "created_at": Utc::now()
        },
        "message": "Mock data - not from database"
    }))
}

pub async fn update_task(
    Extension(session): Extension<SessionData>,
    Path(task_id): Path<String>,
    Json(body): Json<TaskRequest>,
) -> impl IntoResponse {
    Json(json!({
        "message": "Mock task updated (not saved to database)",
        "task": {
            "id": task_id,
            "user_id": session.user_id,
            "title": body.title,
            "description": body.description,
            "completed": body.completed.unwrap_or(false),
            "updated_at": Utc::now()
        }
    }))
}

pub async fn delete_task(Path(task_id): Path<String>) -> impl IntoResponse {
    Json(json!({
        "message": "Mock task deleted (not removed from database)",
        "task_id": task_id,
        "deleted": true,
        "is_mock": true
    }))
}
