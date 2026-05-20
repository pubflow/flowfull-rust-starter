use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    http::{HeaderName, HeaderValue, Method},
    middleware,
    routing::get,
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

pub mod auth;
pub mod cache;
pub mod config;
pub mod db;
pub mod routes;
pub mod security;
pub mod tokens;

use auth::BridgeValidator;
use cache::HybridCache;
use config::Settings;

#[derive(Clone)]
pub struct AppState {
    pub settings: Settings,
    pub db: db::Database,
    pub cache: HybridCache,
    pub bridge_validator: BridgeValidator,
}

pub fn build_app(state: Arc<AppState>) -> anyhow::Result<Router> {
    let cors = cors_layer(&state.settings)?;

    let health_routes = Router::new()
        .route("/", get(routes::health::basic))
        .route("/db", get(routes::health::database))
        .route("/cache", get(routes::health::cache))
        .route("/all", get(routes::health::all));

    let protected_tasks = Router::new()
        .route(
            "/",
            get(routes::api::list_tasks).post(routes::api::create_task),
        )
        .route(
            "/{id}",
            get(routes::api::get_task)
                .put(routes::api::update_task)
                .delete(routes::api::delete_task),
        );

    let api_routes = Router::new()
        .route("/public", get(routes::api::public))
        .route("/test/config", get(routes::api::config))
        .route(
            "/protected",
            get(routes::api::protected).layer(middleware::from_fn_with_state(
                state.clone(),
                auth::require_auth,
            )),
        )
        .route(
            "/optional",
            get(routes::api::optional).layer(middleware::from_fn_with_state(
                state.clone(),
                auth::optional_auth,
            )),
        )
        .route(
            "/profile",
            get(routes::api::profile).layer(middleware::from_fn_with_state(
                state.clone(),
                auth::require_auth,
            )),
        )
        .route(
            "/admin/dashboard",
            get(routes::api::admin_dashboard)
                .layer(auth::require_admin())
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    auth::require_auth,
                )),
        )
        .nest(
            "/tasks",
            protected_tasks.layer(middleware::from_fn_with_state(
                state.clone(),
                auth::require_auth,
            )),
        );

    Ok(Router::new()
        .route("/", get(routes::root))
        .nest("/health", health_routes)
        .nest("/api", api_routes)
        .fallback(security::not_found)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(security::request_shield))
        .with_state(state))
}

fn cors_layer(settings: &Settings) -> anyhow::Result<CorsLayer> {
    let mut layer = CorsLayer::new()
        .allow_credentials(settings.cors_credentials)
        .max_age(Duration::from_secs(settings.cors_max_age));

    let methods = settings
        .cors_methods()
        .into_iter()
        .filter_map(|method| method.parse::<Method>().ok())
        .collect::<Vec<_>>();
    layer = layer.allow_methods(methods);

    let headers = settings
        .cors_headers()
        .into_iter()
        .filter_map(|header| header.parse::<HeaderName>().ok())
        .collect::<Vec<_>>();
    layer = layer.allow_headers(headers);

    if settings.dev_cors_relaxed {
        if settings.cors_credentials {
            Ok(layer.allow_origin(AllowOrigin::mirror_request()))
        } else {
            Ok(layer.allow_origin(tower_http::cors::Any))
        }
    } else {
        let origins = settings
            .cors_origins()
            .into_iter()
            .map(|origin| origin.parse::<HeaderValue>())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(layer.allow_origin(origins))
    }
}
