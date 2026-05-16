use std::{convert::TryFrom, time::Duration as StdDuration};

use chrono::{DateTime, Duration, Utc};
use pasetors::{
    Public,
    claims::{Claims, ClaimsValidationRules},
    errors::{ClaimValidationError, Error as PasetoError},
    keys::{AsymmetricKeyPair, AsymmetricPublicKey, AsymmetricSecretKey, Generate},
    public,
    token::UntrustedToken,
    version4::V4,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenClaims {
    pub user_id: String,
    pub email: String,
    pub token_type: String,
    pub iat: DateTime<Utc>,
    pub exp: DateTime<Utc>,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct TrustTokenManager {
    secret_key: AsymmetricSecretKey<V4>,
    public_key: AsymmetricPublicKey<V4>,
}

#[derive(Debug, thiserror::Error)]
pub enum TrustTokenError {
    #[error("invalid private key")]
    InvalidPrivateKey,
    #[error("invalid token format")]
    InvalidFormat,
    #[error("token signature verification failed")]
    InvalidSignature,
    #[error("token expired")]
    Expired,
    #[error("missing token claim: {0}")]
    MissingClaim(&'static str),
    #[error("invalid token claim: {0}")]
    InvalidClaim(&'static str),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("base64 error: {0}")]
    Base64(#[from] base64::DecodeError),
}

impl TrustTokenManager {
    pub fn from_private_key_hex(private_key_hex: &str) -> Result<Self, TrustTokenError> {
        let bytes = hex::decode(private_key_hex).map_err(|_| TrustTokenError::InvalidPrivateKey)?;
        let secret_key = AsymmetricSecretKey::<V4>::from(&bytes)
            .map_err(|_| TrustTokenError::InvalidPrivateKey)?;
        let public_key = AsymmetricPublicKey::<V4>::try_from(&secret_key)
            .map_err(|_| TrustTokenError::InvalidPrivateKey)?;
        Ok(Self {
            secret_key,
            public_key,
        })
    }

    pub fn generate() -> Self {
        let keypair = AsymmetricKeyPair::<V4>::generate().expect("PASETO v4 key generation");
        Self {
            secret_key: keypair.secret,
            public_key: keypair.public,
        }
    }

    pub fn private_key_hex(&self) -> String {
        hex::encode(self.secret_key.as_bytes())
    }

    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key.as_bytes())
    }

    pub fn create_token(
        &self,
        user_id: impl Into<String>,
        email: impl Into<String>,
        token_type: impl Into<String>,
        ttl: Duration,
        data: Option<serde_json::Value>,
    ) -> Result<String, TrustTokenError> {
        let mut claims = claims_for_ttl(ttl)?;
        claims
            .add_additional("user_id", user_id.into())
            .map_err(map_paseto_error)?;
        claims
            .add_additional("email", email.into())
            .map_err(map_paseto_error)?;
        claims
            .add_additional("token_type", token_type.into())
            .map_err(map_paseto_error)?;

        if let Some(data) = data {
            claims
                .add_additional("data", data)
                .map_err(map_paseto_error)?;
        }

        public::sign(&self.secret_key, &claims, None, None).map_err(map_paseto_error)
    }

    pub fn verify_token(&self, token: &str) -> Result<TokenClaims, TrustTokenError> {
        let untrusted = UntrustedToken::<Public, V4>::try_from(token)
            .map_err(|_| TrustTokenError::InvalidFormat)?;
        let validation_rules = ClaimsValidationRules::new();
        let trusted = public::verify(&self.public_key, &untrusted, &validation_rules, None, None)
            .map_err(map_paseto_error)?;
        let claims = trusted
            .payload_claims()
            .ok_or(TrustTokenError::InvalidFormat)?;

        Ok(TokenClaims {
            user_id: string_claim(claims, "user_id")?,
            email: string_claim(claims, "email")?,
            token_type: string_claim(claims, "token_type")?,
            iat: datetime_claim(claims, "iat")?,
            exp: datetime_claim(claims, "exp")?,
            data: claims.get_claim("data").cloned(),
        })
    }
}

pub fn generate_keypair() -> (String, String) {
    let manager = TrustTokenManager::generate();
    (manager.public_key_hex(), manager.private_key_hex())
}

fn claims_for_ttl(ttl: Duration) -> Result<Claims, TrustTokenError> {
    if ttl > Duration::zero() {
        let duration = StdDuration::from_secs(ttl.num_seconds() as u64);
        Claims::new_expires_in(&duration).map_err(map_paseto_error)
    } else {
        let mut claims = Claims::new().map_err(map_paseto_error)?;
        claims
            .expiration(&(Utc::now() + ttl).to_rfc3339())
            .map_err(map_paseto_error)?;
        Ok(claims)
    }
}

fn string_claim(claims: &Claims, name: &'static str) -> Result<String, TrustTokenError> {
    claims
        .get_claim(name)
        .ok_or(TrustTokenError::MissingClaim(name))?
        .as_str()
        .map(str::to_string)
        .ok_or(TrustTokenError::InvalidClaim(name))
}

fn datetime_claim(claims: &Claims, name: &'static str) -> Result<DateTime<Utc>, TrustTokenError> {
    let value = string_claim(claims, name)?;
    DateTime::parse_from_rfc3339(&value)
        .map(|datetime| datetime.with_timezone(&Utc))
        .map_err(|_| TrustTokenError::InvalidClaim(name))
}

fn map_paseto_error(error: PasetoError) -> TrustTokenError {
    match error {
        PasetoError::ClaimValidation(ClaimValidationError::Exp) => TrustTokenError::Expired,
        PasetoError::TokenFormat => TrustTokenError::InvalidFormat,
        PasetoError::TokenValidation | PasetoError::Signing => TrustTokenError::InvalidSignature,
        PasetoError::InvalidClaim => TrustTokenError::InvalidClaim("paseto"),
        _ => TrustTokenError::InvalidFormat,
    }
}
