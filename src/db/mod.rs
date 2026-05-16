use std::time::Duration;

use sqlx::{AnyPool, any::AnyPoolOptions};

use crate::config::Settings;

pub async fn connect(settings: &Settings) -> anyhow::Result<AnyPool> {
    sqlx::any::install_default_drivers();

    let pool = AnyPoolOptions::new()
        .max_connections(settings.database_max_connections)
        .min_connections(settings.database_min_connections)
        .acquire_timeout(Duration::from_secs(
            settings.database_connect_timeout_seconds,
        ))
        .connect(&settings.database_url)
        .await?;
    Ok(pool)
}

pub async fn ping(pool: &AnyPool) -> anyhow::Result<()> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}

pub fn database_type(database_url: &str) -> &'static str {
    if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        "postgres"
    } else if database_url.starts_with("mysql://") {
        "mysql"
    } else if database_url.starts_with("sqlite://") || database_url.starts_with("sqlite:") {
        "sqlite"
    } else {
        "unknown"
    }
}
