use chrono::{DateTime, Duration, Utc};
use rand::{distributions::Alphanumeric, Rng};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedAuthorizationCode {
    pub plaintext: String,
    pub code_hash: String,
    pub expires_at: DateTime<Utc>,
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
