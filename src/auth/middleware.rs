use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use tower::{Layer, Service};

use crate::{
    AppState,
    auth::{BridgeValidator, SessionData, ValidationModeSettings, ValidationSignals},
    cache::HybridCache,
    config::Settings,
    security::real_client_ip,
};

pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(session_id) = extract_session_id(&request, &state.settings) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let signals = request_validation_signals(&request);
    let Ok(session) = validate_or_cache(&state, &session_id, signals).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    request.extensions_mut().insert(session);
    next.run(request).await
}

pub async fn optional_auth(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    if let Some(session_id) = extract_session_id(&request, &state.settings) {
        let signals = request_validation_signals(&request);
        if let Ok(session) = validate_or_cache(&state, &session_id, signals).await {
            request.extensions_mut().insert(session);
        }
    }
    next.run(request).await
}

#[derive(Debug, Clone)]
pub struct RequireRolesLayer {
    allowed_roles: Arc<[String]>,
}

#[derive(Debug, Clone)]
pub struct RequireRolesService<S> {
    inner: S,
    allowed_roles: Arc<[String]>,
}

pub fn require_roles<I, R>(roles: I) -> RequireRolesLayer
where
    I: IntoIterator<Item = R>,
    R: Into<String>,
{
    let allowed_roles = roles
        .into_iter()
        .map(Into::into)
        .map(|role| role.trim().to_string())
        .filter(|role| !role.is_empty())
        .collect::<Vec<_>>();

    RequireRolesLayer {
        allowed_roles: Arc::from(allowed_roles),
    }
}

pub fn require_roles_csv(roles: &str) -> RequireRolesLayer {
    require_roles(roles.split(','))
}

pub fn require_admin() -> RequireRolesLayer {
    require_roles(["admin"])
}

impl<S> Layer<S> for RequireRolesLayer {
    type Service = RequireRolesService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequireRolesService {
            inner,
            allowed_roles: self.allowed_roles.clone(),
        }
    }
}

impl<S> Service<Request> for RequireRolesService<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let allowed_roles = self.allowed_roles.clone();
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let Some(session) = request.extensions().get::<SessionData>() else {
                return Ok(StatusCode::UNAUTHORIZED.into_response());
            };

            let user_type = session.user_type.as_deref().unwrap_or_default();
            if allowed_roles.iter().any(|role| role == user_type) {
                inner.call(request).await
            } else {
                Ok(StatusCode::FORBIDDEN.into_response())
            }
        })
    }
}

pub fn extract_session_id(request: &Request, settings: &Settings) -> Option<String> {
    request
        .headers()
        .get(settings.session_header_name.as_str())
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| extract_cookie(request, &settings.session_cookie_name))
        .or_else(|| {
            settings
                .session_allow_query
                .then(|| extract_query_session(request))?
        })
}

async fn validate_or_cache(
    state: &Arc<AppState>,
    session_id: &str,
    signals: ValidationSignals,
) -> anyhow::Result<SessionData> {
    let cache_key = format!("session:{session_id}");
    if let Some(session) = state.cache.get(&cache_key).await {
        return Ok(session);
    }

    let options = state.settings.auth_validation_mode.build_options(
        ValidationModeSettings {
            enabled: state.settings.auth_enable_validation_mode,
            ip_enabled: state.settings.auth_ip_validation,
            user_agent_enabled: state.settings.auth_user_agent_validation,
            device_enabled: state.settings.auth_device_validation,
        },
        signals,
    );

    let session = state
        .bridge_validator
        .validate_session(session_id, options)
        .await?;
    state.cache.set(cache_key, session.clone()).await?;
    Ok(session)
}

fn request_validation_signals(request: &Request) -> ValidationSignals {
    ValidationSignals {
        ip: real_client_ip(request.headers()),
        user_agent: request
            .headers()
            .get("user-agent")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string),
        device_id: request
            .headers()
            .get("x-device-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string),
    }
}

fn extract_cookie(request: &Request, cookie_name: &str) -> Option<String> {
    request
        .headers()
        .get("cookie")
        .and_then(|value| value.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|cookie| {
                let (name, value) = cookie.trim().split_once('=')?;
                (name == cookie_name && !value.is_empty()).then(|| value.to_string())
            })
        })
}

fn extract_query_session(request: &Request) -> Option<String> {
    request.uri().query().and_then(|query| {
        query.split('&').find_map(|part| {
            let (name, value) = part.split_once('=')?;
            (name == "session_id" && !value.is_empty()).then(|| value.to_string())
        })
    })
}

#[allow(dead_code)]
fn _assert_send_sync(_: &BridgeValidator, _: &HybridCache) {}
