use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub client_id: Uuid,
    pub token_hash: String,
    pub family_id: Uuid,
    pub scope: Vec<String>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct StoredRefreshTokenRecord {
    pub token_hash: String,
    pub user_id: Uuid,
    pub client_id: Uuid,
    pub family_id: Uuid,
    pub scope: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
}

#[derive(Clone)]
pub enum RefreshTokenRepository {
    Postgres(PgPool),
    InMemory(Arc<Mutex<Vec<StoredRefreshTokenRecord>>>),
}

impl RefreshTokenRepository {
    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(pool)
    }

    pub fn in_memory() -> Self {
        Self::InMemory(Arc::new(Mutex::new(Vec::new())))
    }

    pub async fn save(&self, token: NewRefreshToken) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                sqlx::query(
                    r#"
                        INSERT INTO refresh_tokens (
                            id,
                            user_id,
                            client_id,
                            token_hash,
                            family_id,
                            scope,
                            expires_at
                        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                    "#,
                )
                .bind(token.id)
                .bind(token.user_id)
                .bind(token.client_id)
                .bind(token.token_hash)
                .bind(token.family_id)
                .bind(token.scope)
                .bind(token.expires_at)
                .execute(pool)
                .await?;
                Ok(())
            }
            Self::InMemory(tokens) => {
                tokens.lock().expect("refresh token store poisoned").push(
                    StoredRefreshTokenRecord {
                        token_hash: token.token_hash,
                        user_id: token.user_id,
                        client_id: token.client_id,
                        family_id: token.family_id,
                        scope: token.scope,
                        expires_at: token.expires_at,
                        revoked: false,
                    },
                );
                Ok(())
            }
        }
    }

    pub async fn find_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<StoredRefreshTokenRecord>, sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                sqlx::query_as::<_, StoredRefreshTokenRecord>(
                    r#"
                    SELECT
                        token_hash,
                        user_id,
                        client_id,
                        family_id,
                        scope,
                        expires_at,
                        revoked_at IS NOT NULL AS revoked
                    FROM refresh_tokens
                    WHERE token_hash = $1
                "#,
                )
                .bind(token_hash)
                .fetch_optional(pool)
                .await
            }
            Self::InMemory(tokens) => Ok(tokens
                .lock()
                .expect("refresh token store poisoned")
                .iter()
                .find(|token| token.token_hash == token_hash)
                .cloned()),
        }
    }
}
