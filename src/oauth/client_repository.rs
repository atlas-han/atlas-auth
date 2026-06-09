use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::oauth::client::OAuthClient;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct ClientRecord {
    pub id: Uuid,
    pub public_client_id: String,
    pub client_secret_hash: Option<String>,
    pub client_type: String,
    pub allowed_redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub scopes: Vec<String>,
    pub status: String,
    pub trusted_first_party: bool,
    pub access_token_ttl_seconds: Option<i32>,
    pub refresh_token_ttl_seconds: Option<i32>,
}

impl From<ClientRecord> for OAuthClient {
    fn from(record: ClientRecord) -> Self {
        Self {
            public_client_id: record.public_client_id,
            client_secret_hash: record.client_secret_hash,
            client_type: record.client_type,
            allowed_redirect_uris: record.allowed_redirect_uris,
            grant_types: record.grant_types,
            scopes: record.scopes,
            status: record.status,
        }
    }
}

pub fn client_by_public_id_sql() -> &'static str {
    r#"
        SELECT
            id,
            public_client_id,
            client_secret_hash,
            client_type,
            allowed_redirect_uris,
            grant_types,
            scopes,
            status,
            trusted_first_party,
            access_token_ttl_seconds,
            refresh_token_ttl_seconds
        FROM clients
        WHERE public_client_id = $1
          AND status = 'active'
    "#
}

pub async fn find_active_client_by_public_client_id(
    pool: &PgPool,
    public_client_id: &str,
) -> Result<Option<ClientRecord>, sqlx::Error> {
    sqlx::query_as::<_, ClientRecord>(client_by_public_id_sql())
        .bind(public_client_id)
        .fetch_optional(pool)
        .await
}

#[derive(Clone)]
pub enum OAuthClientRepository {
    Postgres(PgPool),
    InMemory(Vec<ClientRecord>),
}

impl OAuthClientRepository {
    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(pool)
    }

    pub fn in_memory(clients: Vec<ClientRecord>) -> Self {
        Self::InMemory(clients)
    }

    pub async fn find_active_by_public_client_id(
        &self,
        public_client_id: &str,
    ) -> Result<Option<ClientRecord>, sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                find_active_client_by_public_client_id(pool, public_client_id).await
            }
            Self::InMemory(clients) => Ok(clients
                .iter()
                .find(|client| {
                    client.public_client_id == public_client_id && client.status == "active"
                })
                .cloned()),
        }
    }
}
