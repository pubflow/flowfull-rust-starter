use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::Request,
    http::{StatusCode, header},
    middleware,
    routing::get,
};
use chrono::{Duration, Utc};
use flowfull_rust_starter::{
    AppState,
    auth::{
        BridgeValidator, SessionData, ValidationMode, ValidationModeSettings, ValidationOptions,
        ValidationSignals, extract_session_id, require_auth, require_roles_csv,
    },
    build_app,
    cache::HybridCache,
    config::Settings,
    tokens::{TrustTokenError, TrustTokenManager},
};
use http_body_util::BodyExt;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use tower::ServiceExt;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header as header_matcher, method, path},
};

fn settings(flowless_api_url: String) -> Settings {
    Settings {
        port: 3001,
        host: "127.0.0.1".to_string(),
        environment: "development".to_string(),
        base_url: "http://localhost:3001".to_string(),
        database_url: "postgres://postgres:postgres@localhost/flowfull_test".to_string(),
        database_auth_token: None,
        database_max_connections: 5,
        database_min_connections: 0,
        database_connect_timeout_seconds: 1,
        flowless_api_url,
        bridge_validation_endpoint: "/auth/bridge/validate".to_string(),
        bridge_validation_secret: "test-bridge-secret-with-32-characters".to_string(),
        bridge_secret_in_body: false,
        bridge_validation_timeout_ms: 1000,
        bridge_retry_attempts: 1,
        session_validation_cache_ttl: 60,
        session_header_name: "X-Session-Id".to_string(),
        session_cookie_name: "session_id".to_string(),
        session_allow_query: true,
        auth_validation_mode: ValidationMode::Standard,
        auth_enable_validation_mode: true,
        auth_ip_validation: true,
        auth_user_agent_validation: true,
        auth_device_validation: false,
        auth_auto_invalidate: false,
        auth_log_violations: true,
        cache_enabled: true,
        cache_max_capacity: 100,
        redis_url: None,
        trust_token_private_key: None,
        token_ttl_hours: 168,
        token_email_verification_ttl_hours: 24,
        token_password_reset_ttl_hours: 1,
        token_invitation_ttl_hours: 168,
        cors_origins: "http://localhost:3000".to_string(),
        cors_methods: "GET,POST,PUT,DELETE,OPTIONS".to_string(),
        cors_headers: "Content-Type,Authorization,X-Session-Id".to_string(),
        cors_credentials: true,
        cors_max_age: 86400,
        rate_limit_enabled: true,
        rate_limit_requests: 100,
        rate_limit_window: 60,
        log_level: "debug".to_string(),
        log_format: "text".to_string(),
        dev_mode: true,
        dev_cors_relaxed: true,
        dev_log_requests: true,
    }
}

fn test_session(user_type: &str) -> SessionData {
    SessionData {
        user_id: "user_123".to_string(),
        email: "user@example.com".to_string(),
        name: Some("Test User".to_string()),
        user_type: Some(user_type.to_string()),
        organization_id: Some("org_123".to_string()),
        permissions: vec!["tasks:read".to_string()],
        expires_at: Some(Utc::now() + Duration::hours(1)),
        validated_at: Utc::now(),
    }
}

async fn app_with_bridge(user_type: &str) -> axum::Router {
    build_app(state_with_bridge(user_type).await).expect("app")
}

async fn state_with_bridge(user_type: &str) -> Arc<AppState> {
    sqlx::any::install_default_drivers();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/auth/bridge/validate"))
        .and(header_matcher(
            "X-Bridge-Secret",
            "test-bridge-secret-with-32-characters",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "valid": true,
            "user": {
                "id": "user_123",
                "email": "user@example.com",
                "name": "Test User",
                "user_type": user_type
            },
            "session": {
                "id": "session_123",
                "user_id": "user_123",
                "expires_at": (Utc::now() + Duration::hours(1)).to_rfc3339()
            }
        })))
        .mount(&server)
        .await;

    let settings = settings(server.uri());
    let db = sqlx::any::AnyPoolOptions::new()
        .connect_lazy(&settings.database_url)
        .expect("lazy database pool");
    let cache = HybridCache::new(&settings).await.expect("cache");
    let bridge_validator = BridgeValidator::new(&settings).expect("bridge validator");
    Arc::new(AppState {
        settings,
        db: flowfull_rust_starter::db::Database::Sqlx(db),
        cache,
        bridge_validator,
    })
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("json")
}

#[test]
fn validation_mode_builds_expected_options() {
    let options = ValidationMode::Strict.build_options(
        ValidationModeSettings {
            enabled: true,
            ip_enabled: true,
            user_agent_enabled: true,
            device_enabled: true,
        },
        ValidationSignals {
            ip: Some("127.0.0.1".to_string()),
            user_agent: Some("agent".to_string()),
            device_id: Some("device".to_string()),
        },
    );

    assert_eq!(
        options,
        ValidationOptions {
            ip: Some("127.0.0.1".to_string()),
            user_agent: Some("agent".to_string()),
            device_id: Some("device".to_string())
        }
    );

    assert_eq!(
        ValidationMode::Disabled.build_options(
            ValidationModeSettings {
                enabled: true,
                ip_enabled: true,
                user_agent_enabled: true,
                device_enabled: true,
            },
            ValidationSignals {
                ip: Some("127.0.0.1".to_string()),
                user_agent: Some("agent".to_string()),
                device_id: Some("device".to_string()),
            },
        ),
        ValidationOptions::default()
    );
}

#[test]
fn extracts_session_from_header_cookie_and_query() {
    let mut settings = settings("http://localhost:3000".to_string());

    let request = Request::builder()
        .header("X-Session-Id", "from-header")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        extract_session_id(&request, &settings),
        Some("from-header".to_string())
    );

    let request = Request::builder()
        .header(header::COOKIE, "other=value; session_id=from-cookie")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        extract_session_id(&request, &settings),
        Some("from-cookie".to_string())
    );

    settings.session_allow_query = true;
    let request = Request::builder()
        .uri("/api/protected?session_id=from-query")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        extract_session_id(&request, &settings),
        Some("from-query".to_string())
    );
}

#[tokio::test]
async fn local_cache_gets_and_sets_sessions() {
    let settings = settings("http://localhost:3000".to_string());
    let cache = HybridCache::new(&settings).await.expect("cache");
    cache
        .set("session:test".to_string(), test_session("user"))
        .await
        .expect("set");

    let cached = cache.get("session:test").await.expect("cached session");
    assert_eq!(cached.user_id, "user_123");
    assert!(cache.metrics().await.local_hits >= 1);
}

#[tokio::test]
async fn bridge_validator_sends_expected_request_and_maps_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/auth/bridge/validate"))
        .and(header_matcher(
            "X-Bridge-Secret",
            "test-bridge-secret-with-32-characters",
        ))
        .and(body_json(json!({
            "session_id": "session_123",
            "ip": "127.0.0.1",
            "user_agent": "test-agent"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "valid": true,
            "user": {
                "id": "user_123",
                "email": "user@example.com",
                "name": "Test User",
                "user_type": "admin"
            },
            "session": {
                "id": "session_123",
                "user_id": "user_123",
                "expires_at": (Utc::now() + Duration::hours(1)).to_rfc3339()
            }
        })))
        .mount(&server)
        .await;

    let validator =
        BridgeValidator::for_tests(&server.uri(), "test-bridge-secret-with-32-characters")
            .expect("validator");
    let session = validator
        .validate_session(
            "session_123",
            ValidationOptions {
                ip: Some("127.0.0.1".to_string()),
                user_agent: Some("test-agent".to_string()),
                device_id: None,
            },
        )
        .await
        .expect("valid session");

    assert_eq!(session.user_id, "user_123");
    assert_eq!(session.user_type.as_deref(), Some("admin"));
}

#[tokio::test]
async fn public_route_works_without_auth() {
    let app = app_with_bridge("user").await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/public")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["authenticated"], false);
}

#[tokio::test]
async fn protected_route_rejects_missing_session() {
    let app = app_with_bridge("user").await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/protected")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn protected_route_accepts_valid_bridge_session() {
    let app = app_with_bridge("user").await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/protected")
                .header("X-Session-Id", "session_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["user"]["id"], "user_123");
}

#[tokio::test]
async fn optional_route_supports_anonymous_and_authenticated_users() {
    let app = app_with_bridge("user").await;
    let anonymous = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/optional")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(anonymous.status(), StatusCode::OK);
    assert_eq!(response_json(anonymous).await["authenticated"], false);

    let authenticated = app
        .oneshot(
            Request::builder()
                .uri("/api/optional")
                .header("X-Session-Id", "session_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(authenticated.status(), StatusCode::OK);
    assert_eq!(response_json(authenticated).await["authenticated"], true);
}

#[tokio::test]
async fn admin_guard_returns_forbidden_for_wrong_role() {
    let app = app_with_bridge("user").await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/dashboard")
                .header("X-Session-Id", "session_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_guard_rejects_superadmin_by_default() {
    let app = app_with_bridge("superadmin").await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/dashboard")
                .header("X-Session-Id", "session_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn custom_role_guard_accepts_comma_separated_roles() {
    let state = state_with_bridge("manager").await;
    let app = Router::new()
        .route(
            "/manager",
            get(|| async { StatusCode::OK })
                .layer(require_roles_csv("admin, manager, owner"))
                .layer(middleware::from_fn_with_state(state.clone(), require_auth)),
        )
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/manager")
                .header("X-Session-Id", "session_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn trust_tokens_verify_and_expire() {
    let manager = TrustTokenManager::generate();
    let token = manager
        .create_token(
            "user_123",
            "user@example.com",
            "session",
            Duration::hours(1),
            None,
        )
        .expect("token");
    let claims = manager.verify_token(&token).expect("claims");
    assert_eq!(claims.user_id, "user_123");

    let expired = manager
        .create_token(
            "user_123",
            "user@example.com",
            "session",
            Duration::seconds(-1),
            None,
        )
        .expect("expired token");
    assert!(matches!(
        manager.verify_token(&expired),
        Err(TrustTokenError::Expired)
    ));
}
