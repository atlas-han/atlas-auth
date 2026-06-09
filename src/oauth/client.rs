use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthClient {
    pub public_client_id: String,
    pub client_secret_hash: Option<String>,
    pub client_type: String,
    pub allowed_redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub scopes: Vec<String>,
    pub status: String,
}

pub fn hash_client_secret(client_secret: &str) -> String {
    let digest = Sha256::digest(client_secret.as_bytes());
    hex::encode(digest)
}

pub fn verify_client_secret(client: &OAuthClient, client_secret: &str) -> bool {
    client
        .client_secret_hash
        .as_ref()
        .is_some_and(|stored_hash| stored_hash == &hash_client_secret(client_secret))
}

pub fn validate_client_credentials_client(
    client: &OAuthClient,
    client_secret: &str,
    requested_scopes: &[String],
) -> Result<(), &'static str> {
    if client.status != "active" {
        return Err("Client is not active");
    }

    if client.client_type != "confidential" {
        return Err("Client credentials grant requires a confidential client");
    }

    if !client
        .grant_types
        .iter()
        .any(|grant_type| grant_type == "client_credentials")
    {
        return Err("Client is not allowed to use client_credentials grant");
    }

    if !verify_client_secret(client, client_secret) {
        return Err("Client authentication failed");
    }

    let all_scopes_allowed = requested_scopes.iter().all(|requested_scope| {
        client
            .scopes
            .iter()
            .any(|allowed_scope| allowed_scope == requested_scope)
    });
    if !all_scopes_allowed {
        return Err("Requested scope is not allowed for this client");
    }

    Ok(())
}

pub fn validate_authorize_client(
    client: &OAuthClient,
    redirect_uri: &str,
    requested_scopes: &[String],
) -> Result<(), &'static str> {
    if client.status != "active" {
        return Err("Client is not active");
    }

    if !client
        .grant_types
        .iter()
        .any(|grant_type| grant_type == "authorization_code")
    {
        return Err("Client is not allowed to use authorization_code grant");
    }

    if !client
        .allowed_redirect_uris
        .iter()
        .any(|registered_redirect_uri| registered_redirect_uri == redirect_uri)
    {
        return Err("redirect_uri is not registered for this client");
    }

    let all_scopes_allowed = requested_scopes.iter().all(|requested_scope| {
        client
            .scopes
            .iter()
            .any(|allowed_scope| allowed_scope == requested_scope)
    });
    if !all_scopes_allowed {
        return Err("Requested scope is not allowed for this client");
    }

    Ok(())
}

pub fn parse_scope(scope: Option<&str>) -> Vec<String> {
    scope
        .unwrap_or_default()
        .split_ascii_whitespace()
        .map(str::to_string)
        .collect()
}
