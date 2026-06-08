use chrono::{DateTime, Duration, Utc};
use rand::{distributions::Alphanumeric, Rng};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::oauth::pkce::verify_s256_code_challenge;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedAuthorizationCode {
    pub plaintext: String,
    pub code_hash: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAuthorizationCode {
    pub code_hash: String,
    pub client_id: Uuid,
    pub user_id: Uuid,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub scope: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub consumed: bool,
}

pub fn issue_authorization_code() -> IssuedAuthorizationCode {
    let plaintext: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(96)
        .map(char::from)
        .collect();
    let code_hash = hash_authorization_code(&plaintext);
    let expires_at = Utc::now() + Duration::seconds(60);

    IssuedAuthorizationCode {
        plaintext,
        code_hash,
        expires_at,
    }
}

pub fn hash_authorization_code(code: &str) -> String {
    let digest = Sha256::digest(code.as_bytes());
    hex::encode(digest)
}

pub fn exchange_authorization_code(
    stored: &StoredAuthorizationCode,
    plaintext_code: &str,
    code_verifier: &str,
    redirect_uri: &str,
    client_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), &'static str> {
    if stored.consumed {
        return Err("authorization code has already been consumed");
    }

    if stored.expires_at <= now {
        return Err("authorization code has expired");
    }

    if stored.client_id != client_id {
        return Err("authorization code does not belong to this client");
    }

    if stored.redirect_uri != redirect_uri {
        return Err("redirect_uri does not match authorization request");
    }

    if stored.code_hash != hash_authorization_code(plaintext_code) {
        return Err("invalid authorization code");
    }

    if stored.code_challenge_method != "S256" {
        return Err("unsupported code_challenge_method");
    }

    if !verify_s256_code_challenge(code_verifier, &stored.code_challenge) {
        return Err("invalid code_verifier");
    }

    Ok(())
}
