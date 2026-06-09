use sqlx::{FromRow, PgPool};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSigningKey {
    pub kid: String,
    pub public_key: String,
    pub private_key_ciphertext: String,
    pub algorithm: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct StoredSigningKey {
    pub kid: String,
    pub public_key: String,
    pub private_key_ciphertext: String,
    pub algorithm: String,
    pub status: String,
}

#[derive(Clone)]
pub enum SigningKeyRepository {
    Postgres(PgPool),
    InMemory(Arc<Mutex<Vec<StoredSigningKey>>>),
}

impl SigningKeyRepository {
    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(pool)
    }

    pub fn in_memory() -> Self {
        Self::InMemory(Arc::new(Mutex::new(Vec::new())))
    }

    pub async fn save(&self, key: NewSigningKey) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                sqlx::query(
                    r#"
                        INSERT INTO signing_keys (
                            kid,
                            public_key,
                            private_key_ciphertext,
                            algorithm,
                            status
                        ) VALUES ($1, $2, $3, $4, $5)
                    "#,
                )
                .bind(key.kid)
                .bind(key.public_key)
                .bind(key.private_key_ciphertext)
                .bind(key.algorithm)
                .bind(key.status)
                .execute(pool)
                .await?;
                Ok(())
            }
            Self::InMemory(keys) => {
                keys.lock()
                    .expect("signing key store poisoned")
                    .push(StoredSigningKey {
                        kid: key.kid,
                        public_key: key.public_key,
                        private_key_ciphertext: key.private_key_ciphertext,
                        algorithm: key.algorithm,
                        status: key.status,
                    });
                Ok(())
            }
        }
    }

    pub async fn latest_active(&self) -> Result<Option<StoredSigningKey>, sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                sqlx::query_as::<_, StoredSigningKey>(
                    r#"
                        SELECT kid, public_key, private_key_ciphertext, algorithm, status
                        FROM signing_keys
                        WHERE status = 'active'
                        ORDER BY created_at DESC
                        LIMIT 1
                    "#,
                )
                .fetch_optional(pool)
                .await
            }
            Self::InMemory(keys) => Ok(keys
                .lock()
                .expect("signing key store poisoned")
                .iter()
                .rev()
                .find(|key| key.status == "active")
                .cloned()),
        }
    }

    pub async fn retire(&self, kid: &str) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                sqlx::query(
                    r#"
                        UPDATE signing_keys
                        SET status = 'retired', retired_at = now()
                        WHERE kid = $1 AND status = 'active'
                    "#,
                )
                .bind(kid)
                .execute(pool)
                .await?;
                Ok(())
            }
            Self::InMemory(keys) => {
                // Postgres retires only an *active* key (`AND status = 'active'`);
                // match that guard so an already-retired key is a no-op here too.
                if let Some(key) = keys
                    .lock()
                    .expect("signing key store poisoned")
                    .iter_mut()
                    .find(|key| key.kid == kid && key.status == "active")
                {
                    key.status = "retired".to_string();
                }
                Ok(())
            }
        }
    }
}
