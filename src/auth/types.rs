use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserData {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub user_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionInfo {
    pub id: Option<String>,
    pub user_id: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionData {
    pub user_id: String,
    pub email: String,
    pub name: Option<String>,
    pub user_type: Option<String>,
    pub organization_id: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub validated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BridgeValidationResponse {
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub valid: bool,
    pub user: Option<UserData>,
    pub session: Option<SessionInfo>,
    pub error: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationOptions {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationMode {
    Disabled,
    #[default]
    Standard,
    Advanced,
    Strict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationModeSettings {
    pub enabled: bool,
    pub ip_enabled: bool,
    pub user_agent_enabled: bool,
    pub device_enabled: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationSignals {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub device_id: Option<String>,
}

impl std::str::FromStr for ValidationMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_uppercase().as_str() {
            "DISABLED" => Ok(Self::Disabled),
            "STANDARD" => Ok(Self::Standard),
            "ADVANCED" => Ok(Self::Advanced),
            "STRICT" => Ok(Self::Strict),
            other => Err(format!("invalid validation mode: {other}")),
        }
    }
}

impl ValidationMode {
    pub fn build_options(
        self,
        settings: ValidationModeSettings,
        signals: ValidationSignals,
    ) -> ValidationOptions {
        if !settings.enabled || self == Self::Disabled {
            return ValidationOptions::default();
        }

        ValidationOptions {
            ip: (settings.ip_enabled
                && matches!(self, Self::Standard | Self::Advanced | Self::Strict))
            .then_some(signals.ip)
            .flatten(),
            user_agent: (settings.user_agent_enabled
                && matches!(self, Self::Advanced | Self::Strict))
            .then_some(signals.user_agent)
            .flatten(),
            device_id: (settings.device_enabled && self == Self::Strict)
                .then_some(signals.device_id)
                .flatten(),
        }
    }
}
