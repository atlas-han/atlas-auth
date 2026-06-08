#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthClient {
    pub public_client_id: String,
    pub client_type: String,
    pub allowed_redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub scopes: Vec<String>,
    pub status: String,
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
