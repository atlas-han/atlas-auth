use actix_web::{get, post, web, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::oauth::{
    client::{parse_scope, validate_authorize_client},
    client_repository::OAuthClientRepository,
};

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

#[derive(Debug, Deserialize)]
struct TokenForm {
    grant_type: String,
    client_id: Option<String>,
    code: Option<String>,
    code_verifier: Option<String>,
    redirect_uri: Option<String>,
    refresh_token: Option<String>,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RevokeForm {
    token: Option<String>,
    token_type_hint: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IntrospectForm {
    token: Option<String>,
    token_type_hint: Option<String>,
}

#[derive(Debug, Serialize)]
struct IntrospectionResponse {
    active: bool,
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

#[derive(Debug, Serialize)]
struct TokenValidationResponse {
    status: &'static str,
    grant_type: String,
    client_id: String,
}

#[get("/oauth/authorize")]
async fn authorize(
    query: web::Query<AuthorizeQuery>,
    client_repository: Option<web::Data<OAuthClientRepository>>,
) -> HttpResponse {
    if let Err(message) = validate_authorize_query(&query) {
        return HttpResponse::BadRequest().json(OAuthErrorResponse {
            error: "invalid_request",
            message,
        });
    }

    if let Some(client_repository) = client_repository {
        let client_record = match client_repository
            .find_active_by_public_client_id(&query.client_id)
            .await
        {
            Ok(Some(client_record)) => client_record,
            Ok(None) => {
                return HttpResponse::BadRequest().json(OAuthErrorResponse {
                    error: "invalid_client",
                    message: "Client is not registered or active",
                })
            }
            Err(_) => {
                return HttpResponse::InternalServerError().json(OAuthErrorResponse {
                    error: "server_error",
                    message: "Client lookup failed",
                })
            }
        };

        let client = client_record.into();
        let requested_scopes = parse_scope(query.scope.as_deref());
        if let Err(message) =
            validate_authorize_client(&client, &query.redirect_uri, &requested_scopes)
        {
            return HttpResponse::BadRequest().json(OAuthErrorResponse {
                error: "invalid_request",
                message,
            });
        }
    }

    HttpResponse::Ok().json(AuthorizeValidationResponse {
        status: "validated",
        response_type: query.response_type.clone(),
        client_id: query.client_id.clone(),
        redirect_uri: query.redirect_uri.clone(),
        scope: query.scope.clone().unwrap_or_default(),
        state: query.state.clone(),
        code_challenge_method: "S256".to_string(),
    })
}

#[post("/oauth/token")]
async fn token(form: web::Form<TokenForm>) -> HttpResponse {
    match validate_token_form(&form) {
        Ok(()) => HttpResponse::Ok().json(TokenValidationResponse {
            status: "validated",
            grant_type: form.grant_type.clone(),
            client_id: form.client_id.clone().unwrap_or_default(),
        }),
        Err((error, message)) => {
            HttpResponse::BadRequest().json(OAuthErrorResponse { error, message })
        }
    }
}

#[post("/oauth/revoke")]
async fn revoke(form: web::Form<RevokeForm>) -> HttpResponse {
    match validate_revoke_form(&form) {
        Ok(()) => HttpResponse::Ok().finish(),
        Err((error, message)) => {
            HttpResponse::BadRequest().json(OAuthErrorResponse { error, message })
        }
    }
}

#[post("/oauth/introspect")]
async fn introspect(form: web::Form<IntrospectForm>) -> HttpResponse {
    match validate_introspect_form(&form) {
        Ok(()) => HttpResponse::Ok().json(IntrospectionResponse { active: false }),
        Err((error, message)) => {
            HttpResponse::BadRequest().json(OAuthErrorResponse { error, message })
        }
    }
}

fn validate_introspect_form(form: &IntrospectForm) -> Result<(), (&'static str, &'static str)> {
    let has_token = form
        .token
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    if !has_token {
        return Err(("invalid_request", "token is required"));
    }

    match form.token_type_hint.as_deref() {
        None | Some("") | Some("access_token") | Some("refresh_token") => Ok(()),
        Some(_) => Err((
            "unsupported_token_type",
            "token_type_hint must be access_token or refresh_token",
        )),
    }
}

fn validate_revoke_form(form: &RevokeForm) -> Result<(), (&'static str, &'static str)> {
    let has_token = form
        .token
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    if !has_token {
        return Err(("invalid_request", "token is required"));
    }

    match form.token_type_hint.as_deref() {
        None | Some("") | Some("access_token") | Some("refresh_token") => Ok(()),
        Some(_) => Err((
            "unsupported_token_type",
            "token_type_hint must be access_token or refresh_token",
        )),
    }
}

fn validate_token_form(form: &TokenForm) -> Result<(), (&'static str, &'static str)> {
    match form.grant_type.as_str() {
        "authorization_code" => {
            let has_code = form
                .code
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty());
            let has_verifier = form
                .code_verifier
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty());
            let has_client = form
                .client_id
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty());
            let has_redirect_uri = form
                .redirect_uri
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty());

            if !has_code || !has_verifier {
                return Err(("invalid_request", "code and code_verifier are required"));
            }
            if !has_client {
                return Err(("invalid_request", "client_id is required"));
            }
            if !has_redirect_uri {
                return Err(("invalid_request", "redirect_uri is required"));
            }
            Ok(())
        }
        "refresh_token" => {
            let has_refresh_token = form
                .refresh_token
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty());
            if has_refresh_token {
                Ok(())
            } else {
                Err(("invalid_request", "refresh_token is required"))
            }
        }
        "client_credentials" => {
            if form.scope.as_deref().unwrap_or_default().contains("openid") {
                Err((
                    "invalid_scope",
                    "client_credentials cannot request openid scope",
                ))
            } else {
                Ok(())
            }
        }
        _ => Err(("unsupported_grant_type", "Grant type is not supported")),
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
    cfg.service(authorize)
        .service(token)
        .service(revoke)
        .service(introspect);
}
