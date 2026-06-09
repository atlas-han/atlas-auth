use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rand::{distributions::Alphanumeric, Rng};
use rsa::{pkcs8::DecodePublicKey, traits::PublicKeyParts, RsaPublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::Settings;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub scope: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdTokenClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
}

#[derive(Debug, Serialize)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

#[derive(Debug, Serialize)]
pub struct Jwk {
    pub kid: String,
    pub kty: &'static str,
    pub alg: &'static str,
    #[serde(rename = "use")]
    pub key_use: &'static str,
    pub n: String,
    pub e: String,
}

pub fn issue_access_token(
    settings: &Settings,
    user_id: Uuid,
    scope: &str,
) -> anyhow::Result<(String, i64)> {
    issue_access_token_for_subject(settings, &user_id.to_string(), scope)
}

pub fn issue_access_token_for_subject(
    settings: &Settings,
    subject: &str,
    scope: &str,
) -> anyhow::Result<(String, i64)> {
    let now = Utc::now();
    let exp = now + Duration::seconds(settings.jwt_access_token_ttl_seconds);
    let claims = Claims {
        sub: subject.to_string(),
        iss: settings.jwt_issuer.clone(),
        aud: settings.jwt_audience.clone(),
        scope: scope.to_string(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
        jti: Uuid::new_v4().to_string(),
    };

    let token = encode_rs256(settings, &claims)?;

    Ok((token, settings.jwt_access_token_ttl_seconds))
}

pub fn issue_id_token(
    settings: &Settings,
    user_id: Uuid,
    audience: &str,
) -> anyhow::Result<(String, i64)> {
    let now = Utc::now();
    let exp = now + Duration::seconds(settings.jwt_access_token_ttl_seconds);
    let claims = IdTokenClaims {
        sub: user_id.to_string(),
        iss: settings.jwt_issuer.clone(),
        aud: audience.to_string(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
        jti: Uuid::new_v4().to_string(),
    };

    let token = encode_rs256(settings, &claims)?;

    Ok((token, settings.jwt_access_token_ttl_seconds))
}

fn encode_rs256<T: Serialize>(settings: &Settings, claims: &T) -> anyhow::Result<String> {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(settings.jwt_signing_key_id.clone());
    Ok(encode(
        &header,
        claims,
        &EncodingKey::from_rsa_pem(settings.jwt_private_key_pem.as_bytes())?,
    )?)
}

pub fn public_jwks(settings: &Settings) -> anyhow::Result<Jwks> {
    let public_key = RsaPublicKey::from_public_key_pem(&settings.jwt_public_key_pem)?;
    let key = Jwk {
        kid: settings.jwt_signing_key_id.clone(),
        kty: "RSA",
        alg: "RS256",
        key_use: "sig",
        n: URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be()),
        e: URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be()),
    };

    Ok(Jwks { keys: vec![key] })
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRefreshToken {
    pub token_hash: String,
    pub family_id: Uuid,
    pub revoked: bool,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RotatedRefreshToken {
    pub plaintext: String,
    pub token_hash: String,
    pub family_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshTokenRotationError {
    InvalidToken,
    Expired,
    ReuseDetected,
}

pub fn rotate_refresh_token(
    current: &StoredRefreshToken,
    presented_plaintext: &str,
    now: DateTime<Utc>,
) -> Result<RotatedRefreshToken, RefreshTokenRotationError> {
    if current.token_hash != hash_refresh_token(presented_plaintext) {
        return Err(RefreshTokenRotationError::InvalidToken);
    }

    if current.revoked {
        return Err(RefreshTokenRotationError::ReuseDetected);
    }

    if current.expires_at <= now {
        return Err(RefreshTokenRotationError::Expired);
    }

    let plaintext = new_refresh_token();
    let token_hash = hash_refresh_token(&plaintext);

    Ok(RotatedRefreshToken {
        plaintext,
        token_hash,
        family_id: current.family_id,
    })
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

    #[test]
    fn refresh_token_rotation_allows_active_token_and_preserves_family() {
        let family_id = Uuid::new_v4();
        let current = StoredRefreshToken {
            token_hash: hash_refresh_token("current-refresh-token"),
            family_id,
            revoked: false,
            expires_at: Utc::now() + Duration::days(14),
        };

        let rotated = rotate_refresh_token(&current, "current-refresh-token", Utc::now())
            .expect("active refresh token should rotate");

        assert_eq!(rotated.family_id, family_id);
        assert_ne!(rotated.plaintext, "current-refresh-token");
        assert_ne!(rotated.token_hash, current.token_hash);
    }

    #[test]
    fn refresh_token_rotation_detects_reuse_of_revoked_token() {
        let current = StoredRefreshToken {
            token_hash: hash_refresh_token("current-refresh-token"),
            family_id: Uuid::new_v4(),
            revoked: true,
            expires_at: Utc::now() + Duration::days(14),
        };

        let result = rotate_refresh_token(&current, "current-refresh-token", Utc::now());

        assert_eq!(result, Err(RefreshTokenRotationError::ReuseDetected));
    }

    #[test]
    fn refresh_token_rotation_rejects_expired_token() {
        let current = StoredRefreshToken {
            token_hash: hash_refresh_token("current-refresh-token"),
            family_id: Uuid::new_v4(),
            revoked: false,
            expires_at: Utc::now() - Duration::seconds(1),
        };

        let result = rotate_refresh_token(&current, "current-refresh-token", Utc::now());

        assert_eq!(result, Err(RefreshTokenRotationError::Expired));
    }

    #[test]
    fn access_token_uses_rs256_kid_and_prd_claims() {
        use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
        use rand_core::OsRng;
        use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
        use rsa::RsaPrivateKey;

        let private_key = RsaPrivateKey::new(&mut OsRng, 2048).expect("test key should generate");
        let public_key = private_key.to_public_key();
        let private_pem = private_key
            .to_pkcs8_pem(LineEnding::LF)
            .expect("private key should encode")
            .to_string();
        let public_pem = public_key
            .to_public_key_pem(LineEnding::LF)
            .expect("public key should encode");
        let settings = Settings {
            app_env: "test".to_string(),
            server_host: "127.0.0.1".to_string(),
            server_port: 0,
            database_url: "postgres://localhost/atlas_auth_test".to_string(),
            jwt_issuer: "https://auth.example.test".to_string(),
            jwt_audience: "atlas-services".to_string(),
            jwt_access_token_ttl_seconds: 900,
            jwt_refresh_token_ttl_seconds: 2_592_000,
            jwt_signing_key_id: "test-key-1".to_string(),
            jwt_private_key_pem: private_pem,
            jwt_public_key_pem: public_pem.clone(),
            password_pepper: "test-pepper".to_string(),
        };
        let user_id = Uuid::new_v4();

        let (token, expires_in) =
            issue_access_token(&settings, user_id, "openid email").expect("token should issue");
        let header = decode_header(&token).expect("header should decode");
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&["atlas-services"]);
        validation.set_issuer(&["https://auth.example.test"]);
        let decoded = decode::<Claims>(
            &token,
            &DecodingKey::from_rsa_pem(public_pem.as_bytes()).expect("public key should parse"),
            &validation,
        )
        .expect("token should verify with public key");

        assert_eq!(expires_in, 900);
        assert_eq!(header.alg, Algorithm::RS256);
        assert_eq!(header.kid.as_deref(), Some("test-key-1"));
        assert_eq!(decoded.claims.sub, user_id.to_string());
        assert_eq!(decoded.claims.iss, "https://auth.example.test");
        assert_eq!(decoded.claims.aud, "atlas-services");
        assert_eq!(decoded.claims.scope, "openid email");
        assert!(!decoded.claims.jti.is_empty());
    }
}
