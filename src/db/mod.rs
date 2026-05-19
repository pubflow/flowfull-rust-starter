use std::{sync::Arc, time::Duration};

use libsql::Database as LibSqlDatabase;
use sqlx::{AnyPool, any::AnyPoolOptions};
use url::Url;

use crate::config::Settings;

#[derive(Clone)]
pub enum Database {
    Sqlx(AnyPool),
    LibSql(Arc<LibSqlDatabase>),
}

pub async fn connect(settings: &Settings) -> anyhow::Result<Database> {
    if database_type(&settings.database_url) == "libsql" {
        let (safe_url, auth_token) = parse_libsql_connection(
            &settings.database_url,
            settings.database_auth_token.as_deref(),
        )?;
        let db = libsql::Builder::new_remote(safe_url, auth_token)
            .build()
            .await?;
        return Ok(Database::LibSql(Arc::new(db)));
    }

    sqlx::any::install_default_drivers();

    let pool = AnyPoolOptions::new()
        .max_connections(settings.database_max_connections)
        .min_connections(settings.database_min_connections)
        .acquire_timeout(Duration::from_secs(
            settings.database_connect_timeout_seconds,
        ))
        .connect(&settings.database_url)
        .await?;
    Ok(Database::Sqlx(pool))
}

pub async fn ping(db: &Database) -> anyhow::Result<()> {
    match db {
        Database::Sqlx(pool) => {
            sqlx::query("SELECT 1").execute(pool).await?;
        }
        Database::LibSql(database) => {
            let conn = database.connect()?;
            conn.query("SELECT 1", ()).await?;
        }
    }
    Ok(())
}

pub fn database_type(database_url: &str) -> &'static str {
    if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        "postgres"
    } else if database_url.starts_with("mysql://") {
        "mysql"
    } else if database_url.starts_with("sqlite://") || database_url.starts_with("sqlite:") {
        "sqlite"
    } else if database_url.starts_with("libsql://") {
        "libsql"
    } else {
        "unknown"
    }
}

pub fn parse_libsql_connection(
    database_url: &str,
    fallback_auth_token: Option<&str>,
) -> anyhow::Result<(String, String)> {
    let mut parsed_url = Url::parse(database_url)?;
    let mut auth_token =
        first_query_value(&parsed_url, &["authToken", "token", "auth_token", "jwt"]);

    let retained_query_pairs = parsed_url
        .query_pairs()
        .filter(|(key, _)| !["authToken", "token", "auth_token", "jwt"].contains(&key.as_ref()))
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    parsed_url.set_query(None);
    if !retained_query_pairs.is_empty() {
        let mut query_pairs = parsed_url.query_pairs_mut();
        for (key, value) in retained_query_pairs {
            query_pairs.append_pair(&key, &value);
        }
    }

    if auth_token.is_none() {
        auth_token = fallback_auth_token
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .or_else(|| env_auth_token("DATABASE_AUTH_TOKEN"))
            .or_else(|| env_auth_token("DB_AUTH_TOKEN"))
            .or_else(|| env_auth_token("TURSO_AUTH_TOKEN"))
            .or_else(|| env_auth_token("LIBSQL_AUTH_TOKEN"));
    }

    let auth_token = auth_token.ok_or_else(|| {
        anyhow::anyhow!("libsql database requires authToken in DATABASE_URL or DATABASE_AUTH_TOKEN/TURSO_AUTH_TOKEN")
    })?;

    Ok((parsed_url.to_string(), auth_token))
}

fn first_query_value(parsed_url: &Url, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some((_, value)) = parsed_url
            .query_pairs()
            .find(|(candidate, _)| candidate == *key)
        {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn env_auth_token(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::parse_libsql_connection;

    #[test]
    fn separates_auth_token_from_libsql_url() {
        let (safe_url, auth_token) =
            parse_libsql_connection("libsql://example.turso.io?authToken=secret-jwt&tls=1", None)
                .expect("parse libsql url");

        assert_eq!(auth_token, "secret-jwt");
        assert!(!safe_url.contains("secret-jwt"));
        assert!(!safe_url.contains("authToken"));
        assert!(safe_url.contains("tls=1"));
    }

    #[test]
    fn uses_fallback_auth_token() {
        let (safe_url, auth_token) =
            parse_libsql_connection("libsql://example.turso.io", Some("separate-token"))
                .expect("parse libsql url");

        assert_eq!(auth_token, "separate-token");
        assert!(!safe_url.contains("separate-token"));
    }
}
