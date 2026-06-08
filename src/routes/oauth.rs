use actix_web::{get, web, HttpResponse};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct AuthorizeQuery {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    scope: Option<String>,
    state: String,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
}

#[derive(Debug, Serialize)]
struct OAuthErrorResponse {
    error: &'static str,
    message: &'static str,
}

#[derive(Debug, Serialize)]
struct AuthorizeValidationResponse {
    status: &'static str,
    response_type: String,
    client_id: String,
    redirect_uri: String,
    scope: String,
    state: String,
    code_challenge_method: String,
}

#[get("/oauth/authorize")]
async fn authorize(query: web::Query<AuthorizeQuery>) -> HttpResponse {
    match validate_authorize_query(&query) {
        Ok(()) => HttpResponse::Ok().json(AuthorizeValidationResponse {
            status: "validated",
            response_type: query.response_type.clone(),
            client_id: query.client_id.clone(),
            redirect_uri: query.redirect_uri.clone(),
            scope: query.scope.clone().unwrap_or_default(),
            state: query.state.clone(),
            code_challenge_method: "S256".to_string(),
        }),
        Err(message) => HttpResponse::BadRequest().json(OAuthErrorResponse {
            error: "invalid_request",
            message,
        }),
    }
}

fn validate_authorize_query(query: &AuthorizeQuery) -> Result<(), &'static str> {
    if query.response_type != "code" {
        return Err("Only Authorization Code flow is supported");
    }

    if query.client_id.trim().is_empty() {
        return Err("client_id is required");
    }

    if query.redirect_uri.trim().is_empty() {
        return Err("redirect_uri is required");
    }

    if query.state.trim().is_empty() {
        return Err("state is required");
    }

    let has_challenge = query
        .code_challenge
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    let uses_s256 = query.code_challenge_method.as_deref() == Some("S256");
    if !has_challenge || !uses_s256 {
        return Err("S256 PKCE code_challenge is required");
    }

    Ok(())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(authorize);
}
