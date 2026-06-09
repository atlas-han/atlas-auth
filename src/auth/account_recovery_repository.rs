use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountTokenPurpose {
    EmailVerification,
    PasswordReset,
}

impl AccountTokenPurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmailVerification => "email_verification",
            Self::PasswordReset => "password_reset",
        }
    }
}

impl From<String> for AccountTokenPurpose {
    fn from(value: String) -> Self {
        match value.as_str() {
            "email_verification" => Self::EmailVerification,
            "password_reset" => Self::PasswordReset,
            _ => Self::PasswordReset,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAccountRecoveryToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub purpose: AccountTokenPurpose,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAccountRecoveryToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub purpose: AccountTokenPurpose,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct AccountRecoveryTokenRow {
    id: Uuid,
    user_id: Uuid,
    token_hash: String,
    purpose: String,
    expires_at: DateTime<Utc>,
    consumed_at: Option<DateTime<Utc>>,
}

impl From<AccountRecoveryTokenRow> for StoredAccountRecoveryToken {
    fn from(row: AccountRecoveryTokenRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            token_hash: row.token_hash,
            purpose: AccountTokenPurpose::from(row.purpose),
            expires_at: row.expires_at,
            consumed_at: row.consumed_at,
        }
    }
}

/// A minimal [`sqlx::error::DatabaseError`] reporting a unique-constraint
/// violation. The in-memory repository variant returns this so it rejects a
/// duplicate key exactly like the Postgres `UNIQUE` constraint does:
/// `AppError`'s `From<sqlx::Error>` maps any `is_unique_violation()` error to
/// `AppError::Conflict`, keeping the two backends behaviorally interchangeable
/// (Liskov) on duplicate inserts instead of silently accepting them.
#[derive(Debug)]
struct InMemoryUniqueViolation {
    constraint: &'static str,
}

impl InMemoryUniqueViolation {
    fn new(constraint: &'static str) -> Self {
        Self { constraint }
    }
}

impl std::fmt::Display for InMemoryUniqueViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "duplicate key value violates unique constraint \"{}\"",
            self.constraint
        )
    }
}

impl std::error::Error for InMemoryUniqueViolation {}

impl sqlx::error::DatabaseError for InMemoryUniqueViolation {
    fn message(&self) -> &str {
        "duplicate key value violates unique constraint"
    }

    fn kind(&self) -> sqlx::error::ErrorKind {
        sqlx::error::ErrorKind::UniqueViolation
    }

    fn constraint(&self) -> Option<&str> {
        Some(self.constraint)
    }

    fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        self
    }

    fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
        self
    }

    fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
        self
    }
}

#[derive(Clone)]
pub enum AccountRecoveryRepository {
    Postgres(PgPool),
    InMemory(Arc<Mutex<Vec<StoredAccountRecoveryToken>>>),
}

impl AccountRecoveryRepository {
    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(pool)
    }

    pub fn in_memory() -> Self {
        Self::InMemory(Arc::new(Mutex::new(Vec::new())))
    }

    pub async fn save(&self, token: NewAccountRecoveryToken) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                sqlx::query(
                    r#"
                        INSERT INTO account_recovery_tokens (
                            id,
                            user_id,
                            token_hash,
                            purpose,
                            expires_at
                        ) VALUES ($1, $2, $3, $4, $5)
                    "#,
                )
                .bind(token.id)
                .bind(token.user_id)
                .bind(token.token_hash)
                .bind(token.purpose.as_str())
                .bind(token.expires_at)
                .execute(pool)
                .await?;
                Ok(())
            }
            Self::InMemory(tokens) => {
                let mut guard = tokens
                    .lock()
                    .expect("account recovery token store poisoned");
                if guard
                    .iter()
                    .any(|existing| existing.token_hash == token.token_hash)
                {
                    // Postgres enforces `token_hash TEXT NOT NULL UNIQUE`; mirror
                    // that here so the in-memory backend rejects duplicates too.
                    return Err(sqlx::Error::Database(Box::new(
                        InMemoryUniqueViolation::new("account_recovery_tokens_token_hash_key"),
                    )));
                }
                guard.push(StoredAccountRecoveryToken {
                    id: token.id,
                    user_id: token.user_id,
                    token_hash: token.token_hash,
                    purpose: token.purpose,
                    expires_at: token.expires_at,
                    consumed_at: None,
                });
                Ok(())
            }
        }
    }

    pub async fn find_active_by_hash(
        &self,
        token_hash: &str,
        purpose: AccountTokenPurpose,
        now: DateTime<Utc>,
    ) -> Result<Option<StoredAccountRecoveryToken>, sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                let row = sqlx::query_as::<_, AccountRecoveryTokenRow>(
                    r#"
                        SELECT id, user_id, token_hash, purpose, expires_at, consumed_at
                        FROM account_recovery_tokens
                        WHERE token_hash = $1
                          AND purpose = $2
                          AND consumed_at IS NULL
                          AND expires_at > $3
                    "#,
                )
                .bind(token_hash)
                .bind(purpose.as_str())
                .bind(now)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(StoredAccountRecoveryToken::from))
            }
            Self::InMemory(tokens) => Ok(tokens
                .lock()
                .expect("account recovery token store poisoned")
                .iter()
                .find(|token| {
                    token.token_hash == token_hash
                        && token.purpose == purpose
                        && token.consumed_at.is_none()
                        && token.expires_at > now
                })
                .cloned()),
        }
    }

    pub async fn consume(
        &self,
        token_hash: &str,
        purpose: AccountTokenPurpose,
        now: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                sqlx::query(
                    r#"
                        UPDATE account_recovery_tokens
                        SET consumed_at = $3
                        WHERE token_hash = $1
                          AND purpose = $2
                          AND consumed_at IS NULL
                          AND expires_at > $3
                    "#,
                )
                .bind(token_hash)
                .bind(purpose.as_str())
                .bind(now)
                .execute(pool)
                .await?;
                Ok(())
            }
            Self::InMemory(tokens) => {
                if let Some(token) = tokens
                    .lock()
                    .expect("account recovery token store poisoned")
                    .iter_mut()
                    .find(|token| {
                        token.token_hash == token_hash
                            && token.purpose == purpose
                            && token.consumed_at.is_none()
                            && token.expires_at > now
                    })
                {
                    token.consumed_at = Some(now);
                }
                Ok(())
            }
        }
    }
}
