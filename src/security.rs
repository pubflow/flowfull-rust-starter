use std::net::IpAddr;

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

pub async fn request_shield(request: Request, next: Next) -> Response {
    if is_suspicious_probe_path(request.uri().path()) {
        return StatusCode::NOT_FOUND.into_response();
    }

    next.run(request).await
}

pub async fn not_found() -> Response {
    StatusCode::NOT_FOUND.into_response()
}

pub fn real_client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(first_valid_forwarded_ip)
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .and_then(normalize_ip)
        })
}

fn first_valid_forwarded_ip(value: &str) -> Option<String> {
    value.split(',').find_map(normalize_ip)
}

fn normalize_ip(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches('"');
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.parse::<IpAddr>().is_ok() {
        return Some(trimmed.to_string());
    }

    if let Some((inside_brackets, _)) = trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']'))
    {
        if inside_brackets.parse::<IpAddr>().is_ok() {
            return Some(inside_brackets.to_string());
        }
    }

    if let Some((host, _port)) = trimmed.rsplit_once(':') {
        if host.parse::<IpAddr>().is_ok() {
            return Some(host.to_string());
        }
    }

    None
}

fn is_suspicious_probe_path(path: &str) -> bool {
    if path.starts_with("/.well-known/acme-challenge/") {
        return false;
    }

    let lower = path.to_ascii_lowercase();
    lower == "/.ds_store"
        || lower == "/wp-login.php"
        || lower == "/dockerfile"
        || lower == "/procfile"
        || lower == "/serverless.yml"
        || lower == "/serverless.yaml"
        || lower == "/@vite/env"
        || lower.starts_with("/.env")
        || lower.starts_with("/.git")
        || lower.starts_with("/.aws/credentials")
        || lower.starts_with("/actuator")
        || lower.starts_with("/swagger")
        || lower.starts_with("/server-status")
        || lower.starts_with("/server-info")
        || lower.starts_with("/telescope")
        || lower.starts_with("/debug")
        || lower == "/v2/api-docs"
        || lower == "/v3/api-docs"
        || lower == "/graphql"
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};

    use super::{is_suspicious_probe_path, real_client_ip};

    #[test]
    fn extracts_first_forwarded_ip() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("1.2.3.4, 172.18.0.2"),
        );

        assert_eq!(real_client_ip(&headers).as_deref(), Some("1.2.3.4"));
    }

    #[test]
    fn falls_back_to_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("5.6.7.8"));

        assert_eq!(real_client_ip(&headers).as_deref(), Some("5.6.7.8"));
    }

    #[test]
    fn ignores_invalid_forwarded_values() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("unknown, 2001:db8::1"),
        );

        assert_eq!(real_client_ip(&headers).as_deref(), Some("2001:db8::1"));
    }

    #[test]
    fn detects_probe_paths_without_blocking_acme() {
        assert!(is_suspicious_probe_path("/.env"));
        assert!(is_suspicious_probe_path("/.git/config"));
        assert!(is_suspicious_probe_path("/wp-login.php"));
        assert!(!is_suspicious_probe_path(
            "/.well-known/acme-challenge/token"
        ));
        assert!(!is_suspicious_probe_path("/api/public"));
    }
}
