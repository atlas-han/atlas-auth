use sqlx::{FromRow, PgPool};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewConsent {
    pub id: Uuid,
    pub user_id: Uuid,
    pub client_id: Uuid,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct StoredConsentRecord {
    pub user_id: Uuid,
    pub client_id: Uuid,
    pub scopes: Vec<String>,
}

#[derive(Clone)]
pub enum ConsentRepository {
    Postgres(PgPool),
    InMemory(Arc<Mutex<Vec<StoredConsentRecord>>>),
}

impl ConsentRepository {
    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(pool)
    }

    pub fn in_memory() -> Self {
        Self::InMemory(Arc::new(Mutex::new(Vec::new())))
    }

    pub async fn save(&self, consent: NewConsent) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                sqlx::query(
                    r#"
                        INSERT INTO consents (id, user_id, client_id, scopes)
                        VALUES ($1, $2, $3, $4)
                        ON CONFLICT (user_id, client_id)
                        DO UPDATE SET scopes = EXCLUDED.scopes,
                                      granted_at = now()
                    "#,
                )
                .bind(consent.id)
                .bind(consent.user_id)
                .bind(consent.client_id)
                .bind(consent.scopes)
                .execute(pool)
                .await?;
                Ok(())
            }
            Self::InMemory(consents) => {
                let mut consents = consents.lock().expect("consent store poisoned");
                if let Some(stored) = consents.iter_mut().find(|stored| {
                    stored.user_id == consent.user_id && stored.client_id == consent.client_id
                }) {
                    stored.scopes = consent.scopes;
                } else {
                    consents.push(StoredConsentRecord {
                        user_id: consent.user_id,
                        client_id: consent.client_id,
                        scopes: consent.scopes,
                    });
                }
                Ok(())
            }
        }
    }

    pub async fn has_granted_scopes(
        &self,
        user_id: Uuid,
        client_id: Uuid,
        requested_scopes: &[String],
    ) -> Result<bool, sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                let stored = sqlx::query_as::<_, StoredConsentRecord>(
                    r#"
                        SELECT user_id, client_id, scopes
                        FROM consents
                        WHERE user_id = $1 AND client_id = $2
                    "#,
                )
                .bind(user_id)
                .bind(client_id)
                .fetch_optional(pool)
                .await?;
                Ok(stored.is_some_and(|stored| scopes_cover(&stored.scopes, requested_scopes)))
            }
            Self::InMemory(consents) => Ok(consents
                .lock()
                .expect("consent store poisoned")
                .iter()
                .find(|stored| stored.user_id == user_id && stored.client_id == client_id)
                .is_some_and(|stored| scopes_cover(&stored.scopes, requested_scopes))),
        }
    }
}

fn scopes_cover(granted_scopes: &[String], requested_scopes: &[String]) -> bool {
    requested_scopes.iter().all(|requested_scope| {
        granted_scopes
            .iter()
            .any(|granted_scope| granted_scope == requested_scope)
    })
}
