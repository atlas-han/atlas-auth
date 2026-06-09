use actix_web::{get, post, web, HttpResponse};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    auth::token::{
        hash_refresh_token, issue_access_token, new_refresh_token, rotate_refresh_token,
        StoredRefreshToken,
    },
    oauth::{
        authorization_code::{
            exchange_authorization_code, hash_authorization_code, AuthorizationCodeRepository,
        },
        client::{parse_scope, validate_authorize_client},
        client_repository::OAuthClientRepository,
        refresh_token_repository::{NewRefreshToken, RefreshTokenRepository},
    },
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

#[derive(Debug, Serialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    token_type: &'static str,
    expires_in: i64,
    scope: String,
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
async fn token(
    form: web::Form<TokenForm>,
    state: Option<web::Data<AppState>>,
    client_repository: Option<web::Data<OAuthClientRepository>>,
    authorization_code_repository: Option<web::Data<AuthorizationCodeRepository>>,
    refresh_token_repository: Option<web::Data<RefreshTokenRepository>>,
) -> HttpResponse {
    if let Err((error, message)) = validate_token_form(&form) {
        return HttpResponse::BadRequest().json(OAuthErrorResponse { error, message });
    }

    if form.grant_type == "authorization_code" {
        if let (
            Some(state),
            Some(client_repository),
            Some(authorization_code_repository),
            Some(refresh_token_repository),
        ) = (
            state.as_ref(),
            client_repository.as_ref(),
            authorization_code_repository.as_ref(),
            refresh_token_repository.as_ref(),
        ) {
            return exchange_authorization_code_grant(
                &form,
                state,
                client_repository,
                authorization_code_repository,
                refresh_token_repository,
            )
            .await;
        }
    }

    if form.grant_type == "refresh_token" {
        if let (Some(state), Some(client_repository), Some(refresh_token_repository)) = (
            state.as_ref(),
            client_repository.as_ref(),
            refresh_token_repository.as_ref(),
        ) {
            return exchange_refresh_token_grant(
                &form,
                state,
                client_repository,
                refresh_token_repository,
            )
            .await;
        }
    }

    HttpResponse::Ok().json(TokenValidationResponse {
        status: "validated",
        grant_type: form.grant_type.clone(),
        client_id: form.client_id.clone().unwrap_or_default(),
    })
}

async fn exchange_authorization_code_grant(
    form: &TokenForm,
    state: &AppState,
    client_repository: &OAuthClientRepository,
    authorization_code_repository: &AuthorizationCodeRepository,
    refresh_token_repository: &RefreshTokenRepository,
) -> HttpResponse {
    let client_id = form.client_id.as_deref().unwrap_or_default();
    let client_record = match client_repository
        .find_active_by_public_client_id(client_id)
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

    let code = form.code.as_deref().unwrap_or_default();
    let code_hash = hash_authorization_code(code);
    let stored_code = match authorization_code_repository
        .find_unconsumed_by_hash(&code_hash, Utc::now())
        .await
    {
        Ok(Some(stored_code)) => stored_code,
        Ok(None) => {
            return HttpResponse::BadRequest().json(OAuthErrorResponse {
                error: "invalid_grant",
                message: "authorization code is invalid, expired, or already consumed",
            })
        }
        Err(_) => {
            return HttpResponse::InternalServerError().json(OAuthErrorResponse {
                error: "server_error",
                message: "Authorization code lookup failed",
            })
        }
    };

    if let Err(message) = exchange_authorization_code(
        &stored_code,
        code,
        form.code_verifier.as_deref().unwrap_or_default(),
        form.redirect_uri.as_deref().unwrap_or_default(),
        client_record.id,
        Utc::now(),
    ) {
        return HttpResponse::BadRequest().json(OAuthErrorResponse {
            error: "invalid_grant",
            message,
        });
    }

    let scope = stored_code.scope.join(" ");
    let (access_token, expires_in) =
        match issue_access_token(&state.settings, stored_code.user_id, &scope) {
            Ok(issued_token) => issued_token,
            Err(_) => {
                return HttpResponse::InternalServerError().json(OAuthErrorResponse {
                    error: "server_error",
                    message: "Access token issuance failed",
                })
            }
        };

    let refresh_token = new_refresh_token();
    let refresh_token_hash = hash_refresh_token(&refresh_token);
    if refresh_token_repository
        .save(NewRefreshToken {
            id: uuid::Uuid::new_v4(),
            user_id: stored_code.user_id,
            client_id: client_record.id,
            token_hash: refresh_token_hash,
            family_id: uuid::Uuid::new_v4(),
            scope: stored_code.scope.clone(),
            expires_at: Utc::now()
                + Duration::seconds(state.settings.jwt_refresh_token_ttl_seconds),
        })
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().json(OAuthErrorResponse {
            error: "server_error",
            message: "Refresh token persistence failed",
        });
    }

    if authorization_code_repository
        .consume(&code_hash)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().json(OAuthErrorResponse {
            error: "server_error",
            message: "Authorization code consumption failed",
        });
    }

    HttpResponse::Ok().json(TokenResponse {
        access_token,
        refresh_token,
        token_type: "Bearer",
        expires_in,
        scope,
    })
}

async fn exchange_refresh_token_grant(
    form: &TokenForm,
    state: &AppState,
    client_repository: &OAuthClientRepository,
    refresh_token_repository: &RefreshTokenRepository,
) -> HttpResponse {
    let client_id = match form.client_id.as_deref() {
        Some(client_id) if !client_id.trim().is_empty() => client_id,
        _ => {
            return HttpResponse::BadRequest().json(OAuthErrorResponse {
                error: "invalid_request",
                message: "client_id is required",
            })
        }
    };
    let client_record = match client_repository
        .find_active_by_public_client_id(client_id)
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

    let presented_refresh_token = form.refresh_token.as_deref().unwrap_or_default();
    let presented_hash = hash_refresh_token(presented_refresh_token);
    let stored_record = match refresh_token_repository.find_by_hash(&presented_hash).await {
        Ok(Some(stored_record)) => stored_record,
        Ok(None) => {
            return HttpResponse::BadRequest().json(OAuthErrorResponse {
                error: "invalid_grant",
                message: "refresh token is invalid",
            })
        }
        Err(_) => {
            return HttpResponse::InternalServerError().json(OAuthErrorResponse {
                error: "server_error",
                message: "Refresh token lookup failed",
            })
        }
    };

    if stored_record.client_id != client_record.id {
        return HttpResponse::BadRequest().json(OAuthErrorResponse {
            error: "invalid_grant",
            message: "refresh token does not belong to this client",
        });
    }

    let stored_token = StoredRefreshToken {
        token_hash: stored_record.token_hash.clone(),
        family_id: stored_record.family_id,
        revoked: stored_record.revoked,
        expires_at: stored_record.expires_at,
    };
    let rotated = match rotate_refresh_token(&stored_token, presented_refresh_token, Utc::now()) {
        Ok(rotated) => rotated,
        Err(_) => {
            return HttpResponse::BadRequest().json(OAuthErrorResponse {
                error: "invalid_grant",
                message: "refresh token is invalid, expired, or already used",
            })
        }
    };

    let scope = stored_record.scope.join(" ");
    let (access_token, expires_in) =
        match issue_access_token(&state.settings, stored_record.user_id, &scope) {
            Ok(issued_token) => issued_token,
            Err(_) => {
                return HttpResponse::InternalServerError().json(OAuthErrorResponse {
                    error: "server_error",
                    message: "Access token issuance failed",
                })
            }
        };

    if refresh_token_repository
        .save(NewRefreshToken {
            id: uuid::Uuid::new_v4(),
            user_id: stored_record.user_id,
            client_id: stored_record.client_id,
            token_hash: rotated.token_hash,
            family_id: rotated.family_id,
            scope: stored_record.scope.clone(),
            expires_at: Utc::now()
                + Duration::seconds(state.settings.jwt_refresh_token_ttl_seconds),
        })
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().json(OAuthErrorResponse {
            error: "server_error",
            message: "Refresh token persistence failed",
        });
    }

    if refresh_token_repository
        .revoke_by_hash(&presented_hash)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().json(OAuthErrorResponse {
            error: "server_error",
            message: "Refresh token revocation failed",
        });
    }

    HttpResponse::Ok().json(TokenResponse {
        access_token,
        refresh_token: rotated.plaintext,
        token_type: "Bearer",
        expires_in,
        scope,
    })
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
