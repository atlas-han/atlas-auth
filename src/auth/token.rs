use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::Settings;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub iat: i64,
    pub exp: i64,
}

pub fn issue_access_token(settings: &Settings, user_id: Uuid) -> anyhow::Result<(String, i64)> {
    let now = Utc::now();
    let exp = now + Duration::seconds(settings.jwt_access_token_ttl_seconds);
    let claims = Claims {
        sub: user_id.to_string(),
        iss: settings.jwt_issuer.clone(),
        aud: settings.jwt_audience.clone(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(settings.jwt_signing_secret.as_bytes()),
    )?;

    Ok((token, settings.jwt_access_token_ttl_seconds))
}

pub fn new_refresh_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(96)
        .map(char::from)
        .collect()
}

pub fn hash_refresh_token(refresh_token: &str) -> String {
    let digest = Sha256::digest(refresh_token.as_bytes());
    hex::encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_token_hash_is_stable_and_not_plaintext() {
        let token = "refresh-token";

        let hash = hash_refresh_token(token);

        assert_eq!(hash, hash_refresh_token(token));
        assert_ne!(hash, token);
    }

    #[test]
    fn generated_refresh_token_has_enough_entropy_surface() {
        let token = new_refresh_token();

        assert!(token.len() >= 96);
    }
}
