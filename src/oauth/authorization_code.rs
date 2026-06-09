use chrono::{DateTime, Duration, Utc};
use rand::{distributions::Alphanumeric, Rng};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use std::sync::{Arc, Mutex};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAuthorizationCode {
    pub id: Uuid,
    pub code_hash: String,
    pub client_id: Uuid,
    pub user_id: Uuid,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub scope: Vec<String>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
struct AuthorizationCodeRow {
    code_hash: String,
    client_id: Uuid,
    user_id: Uuid,
    redirect_uri: String,
    code_challenge: String,
    code_challenge_method: String,
    scope: Vec<String>,
    expires_at: DateTime<Utc>,
    consumed: bool,
}

impl From<AuthorizationCodeRow> for StoredAuthorizationCode {
    fn from(row: AuthorizationCodeRow) -> Self {
        Self {
            code_hash: row.code_hash,
            client_id: row.client_id,
            user_id: row.user_id,
            redirect_uri: row.redirect_uri,
            code_challenge: row.code_challenge,
            code_challenge_method: row.code_challenge_method,
            scope: row.scope,
            expires_at: row.expires_at,
            consumed: row.consumed,
        }
    }
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

#[derive(Clone)]
pub enum AuthorizationCodeRepository {
    Postgres(PgPool),
    InMemory(Arc<Mutex<Vec<StoredAuthorizationCode>>>),
}

impl AuthorizationCodeRepository {
    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(pool)
    }

    pub fn in_memory() -> Self {
        Self::InMemory(Arc::new(Mutex::new(Vec::new())))
    }

    pub async fn save(&self, code: NewAuthorizationCode) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                sqlx::query(
                    r#"
                        INSERT INTO authorization_codes (
                            id,
                            code_hash,
                            client_id,
                            user_id,
                            redirect_uri,
                            code_challenge,
                            code_challenge_method,
                            scope,
                            expires_at
                        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    "#,
                )
                .bind(code.id)
                .bind(code.code_hash)
                .bind(code.client_id)
                .bind(code.user_id)
                .bind(code.redirect_uri)
                .bind(code.code_challenge)
                .bind(code.code_challenge_method)
                .bind(code.scope)
                .bind(code.expires_at)
                .execute(pool)
                .await?;
                Ok(())
            }
            Self::InMemory(codes) => {
                codes
                    .lock()
                    .expect("authorization code store poisoned")
                    .push(StoredAuthorizationCode {
                        code_hash: code.code_hash,
                        client_id: code.client_id,
                        user_id: code.user_id,
                        redirect_uri: code.redirect_uri,
                        code_challenge: code.code_challenge,
                        code_challenge_method: code.code_challenge_method,
                        scope: code.scope,
                        expires_at: code.expires_at,
                        consumed: false,
                    });
                Ok(())
            }
        }
    }

    pub async fn find_unconsumed_by_hash(
        &self,
        code_hash: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<StoredAuthorizationCode>, sqlx::Error> {
        match self {
            Self::Postgres(pool) => sqlx::query_as::<_, AuthorizationCodeRow>(
                r#"
                        SELECT
                            code_hash,
                            client_id,
                            user_id,
                            redirect_uri,
                            code_challenge,
                            code_challenge_method,
                            scope,
                            expires_at,
                            consumed_at IS NOT NULL AS consumed
                        FROM authorization_codes
                        WHERE code_hash = $1
                          AND consumed_at IS NULL
                          AND expires_at > $2
                    "#,
            )
            .bind(code_hash)
            .bind(now)
            .fetch_optional(pool)
            .await
            .map(|row| row.map(Into::into)),
            Self::InMemory(codes) => Ok(codes
                .lock()
                .expect("authorization code store poisoned")
                .iter()
                .find(|code| code.code_hash == code_hash && !code.consumed && code.expires_at > now)
                .cloned()),
        }
    }

    pub async fn consume(&self, code_hash: &str) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                sqlx::query(
                    r#"
                        UPDATE authorization_codes
                        SET consumed_at = now()
                        WHERE code_hash = $1
                          AND consumed_at IS NULL
                    "#,
                )
                .bind(code_hash)
                .execute(pool)
                .await?;
                Ok(())
            }
            Self::InMemory(codes) => {
                // Mirror Postgres `WHERE code_hash = $1 AND consumed_at IS NULL`.
                if let Some(code) = codes
                    .lock()
                    .expect("authorization code store poisoned")
                    .iter_mut()
                    .find(|code| code.code_hash == code_hash && !code.consumed)
                {
                    code.consumed = true;
                }
                Ok(())
            }
        }
    }
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
