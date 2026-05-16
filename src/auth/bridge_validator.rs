use std::time::Duration;

use chrono::Utc;
use reqwest::Url;
use serde::Serialize;
use tokio::time::sleep;
use tracing::{debug, warn};

use crate::{
    auth::{BridgeValidationResponse, SessionData, ValidationOptions},
    config::Settings,
};

#[derive(Debug, Clone)]
pub struct BridgeValidator {
    client: reqwest::Client,
    flowless_url: Url,
    endpoint: String,
    bridge_secret: String,
    secret_in_body: bool,
    retry_attempts: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum BridgeValidationError {
    #[error("session_id is required")]
    MissingSession,
    #[error("bridge request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("invalid bridge response: {0}")]
    InvalidResponse(String),
    #[error("session validation failed: {0}")]
    InvalidSession(String),
}

impl BridgeValidator {
    pub fn new(settings: &Settings) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(settings.bridge_timeout())
            .build()?;

        Ok(Self {
            client,
            flowless_url: Url::parse(&settings.flowless_api_url)?,
            endpoint: settings.bridge_validation_endpoint.clone(),
            bridge_secret: settings.bridge_validation_secret.clone(),
            secret_in_body: settings.bridge_secret_in_body,
            retry_attempts: settings.bridge_retry_attempts.max(1),
        })
    }

    pub fn for_tests(flowless_url: &str, bridge_secret: &str) -> anyhow::Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()?,
            flowless_url: Url::parse(flowless_url)?,
            endpoint: "/auth/bridge/validate".to_string(),
            bridge_secret: bridge_secret.to_string(),
            secret_in_body: false,
            retry_attempts: 1,
        })
    }

    pub async fn validate_session(
        &self,
        session_id: &str,
        options: ValidationOptions,
    ) -> Result<SessionData, BridgeValidationError> {
        if session_id.is_empty() {
            return Err(BridgeValidationError::MissingSession);
        }

        let mut last_error = None;
        for attempt in 0..self.retry_attempts {
            match self.validate_once(session_id, options.clone()).await {
                Ok(session) => return Ok(session),
                Err(error) => {
                    warn!(attempt = attempt + 1, error = %error, "bridge validation attempt failed");
                    last_error = Some(error);
                }
            }

            if attempt + 1 < self.retry_attempts {
                sleep(Duration::from_millis(
                    100 * 2_u64.saturating_pow(attempt as u32),
                ))
                .await;
            }
        }

        Err(last_error.unwrap_or_else(|| {
            BridgeValidationError::InvalidSession("validation failed".to_string())
        }))
    }

    async fn validate_once(
        &self,
        session_id: &str,
        options: ValidationOptions,
    ) -> Result<SessionData, BridgeValidationError> {
        #[derive(Serialize)]
        struct RequestBody<'a> {
            session_id: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            bridge_secret: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            ip: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            user_agent: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            device_id: Option<String>,
        }

        let url = self
            .flowless_url
            .join(self.endpoint.trim_start_matches('/'))
            .map_err(|err| BridgeValidationError::InvalidResponse(err.to_string()))?;
        debug!(url = %url, "validating session with Flowless");

        let response = self
            .client
            .post(url)
            .header("X-Bridge-Secret", &self.bridge_secret)
            .json(&RequestBody {
                session_id,
                bridge_secret: self.secret_in_body.then_some(self.bridge_secret.as_str()),
                ip: options.ip,
                user_agent: options.user_agent,
                device_id: options.device_id,
            })
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(BridgeValidationError::InvalidSession(format!(
                "status {status}: {body}"
            )));
        }

        let parsed: BridgeValidationResponse = serde_json::from_str(&body)
            .map_err(|err| BridgeValidationError::InvalidResponse(err.to_string()))?;
        if !parsed.success && !parsed.valid {
            return Err(BridgeValidationError::InvalidSession(
                parsed
                    .error
                    .or(parsed.message)
                    .unwrap_or_else(|| "rejected".to_string()),
            ));
        }

        let user = parsed
            .user
            .ok_or_else(|| BridgeValidationError::InvalidResponse("missing user".to_string()))?;

        Ok(SessionData {
            user_id: user.id,
            email: user.email,
            name: user.name,
            user_type: user.user_type,
            organization_id: None,
            permissions: Vec::new(),
            expires_at: parsed.session.and_then(|session| session.expires_at),
            validated_at: Utc::now(),
        })
    }
}
