use serde_json::Value;
use sqlx::{FromRow, PgPool};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct NewFederatedIdentity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub email: Option<String>,
    pub profile: Value,
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct StoredFederatedIdentity {
    pub user_id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub email: Option<String>,
    pub profile: Value,
}

#[derive(Clone)]
pub enum FederatedIdentityRepository {
    Postgres(PgPool),
    InMemory(Arc<Mutex<Vec<StoredFederatedIdentity>>>),
}

impl FederatedIdentityRepository {
    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(pool)
    }

    pub fn in_memory() -> Self {
        Self::InMemory(Arc::new(Mutex::new(Vec::new())))
    }

    pub async fn link(&self, identity: NewFederatedIdentity) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                sqlx::query(
                    r#"
                        INSERT INTO federated_identities (
                            id,
                            user_id,
                            provider,
                            provider_user_id,
                            email,
                            profile
                        ) VALUES ($1, $2, $3, $4, $5, $6)
                        ON CONFLICT (provider, provider_user_id)
                        DO UPDATE SET user_id = EXCLUDED.user_id,
                                      email = EXCLUDED.email,
                                      profile = EXCLUDED.profile
                    "#,
                )
                .bind(identity.id)
                .bind(identity.user_id)
                .bind(identity.provider)
                .bind(identity.provider_user_id)
                .bind(identity.email)
                .bind(identity.profile)
                .execute(pool)
                .await?;
                Ok(())
            }
            Self::InMemory(identities) => {
                let mut identities = identities
                    .lock()
                    .expect("federated identity store poisoned");
                if let Some(stored) = identities.iter_mut().find(|stored| {
                    stored.provider == identity.provider
                        && stored.provider_user_id == identity.provider_user_id
                }) {
                    stored.user_id = identity.user_id;
                    stored.email = identity.email;
                    stored.profile = identity.profile;
                } else {
                    identities.push(StoredFederatedIdentity {
                        user_id: identity.user_id,
                        provider: identity.provider,
                        provider_user_id: identity.provider_user_id,
                        email: identity.email,
                        profile: identity.profile,
                    });
                }
                Ok(())
            }
        }
    }

    pub async fn find_by_provider_user_id(
        &self,
        provider: &str,
        provider_user_id: &str,
    ) -> Result<Option<StoredFederatedIdentity>, sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                sqlx::query_as::<_, StoredFederatedIdentity>(
                    r#"
                        SELECT user_id, provider, provider_user_id, email, profile
                        FROM federated_identities
                        WHERE provider = $1 AND provider_user_id = $2
                    "#,
                )
                .bind(provider)
                .bind(provider_user_id)
                .fetch_optional(pool)
                .await
            }
            Self::InMemory(identities) => Ok(identities
                .lock()
                .expect("federated identity store poisoned")
                .iter()
                .find(|identity| {
                    identity.provider == provider && identity.provider_user_id == provider_user_id
                })
                .cloned()),
        }
    }
}
