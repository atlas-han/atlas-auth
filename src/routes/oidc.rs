use actix_web::{get, web, HttpRequest, HttpResponse};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Serialize;

use crate::{
    app::AppState,
    auth::{
        signing_key_repository::SigningKeyRepository,
        token::{public_jwks, public_jwks_from_public_key_pem, Claims},
    },
};

#[derive(Debug, Serialize)]
struct OpenIdConfiguration {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    token_endpoint_auth_methods_supported: Vec<&'static str>,
    userinfo_endpoint: String,
    jwks_uri: String,
    response_types_supported: Vec<&'static str>,
    grant_types_supported: Vec<&'static str>,
    subject_types_supported: Vec<&'static str>,
    id_token_signing_alg_values_supported: Vec<&'static str>,
    scopes_supported: Vec<&'static str>,
    code_challenge_methods_supported: Vec<&'static str>,
}

#[get("/.well-known/openid-configuration")]
async fn openid_configuration(state: web::Data<AppState>) -> HttpResponse {
    let issuer = state.settings.jwt_issuer.trim_end_matches('/').to_string();

    HttpResponse::Ok().json(OpenIdConfiguration {
        authorization_endpoint: format!("{issuer}/oauth/authorize"),
        token_endpoint: format!("{issuer}/oauth/token"),
        token_endpoint_auth_methods_supported: vec![
            "client_secret_basic",
            "client_secret_post",
            "none",
        ],
        userinfo_endpoint: format!("{issuer}/userinfo"),
        jwks_uri: format!("{issuer}/.well-known/jwks.json"),
        response_types_supported: vec!["code"],
        grant_types_supported: vec!["authorization_code", "refresh_token", "client_credentials"],
        subject_types_supported: vec!["public"],
        id_token_signing_alg_values_supported: vec!["RS256"],
        scopes_supported: vec!["openid", "profile", "email"],
        code_challenge_methods_supported: vec!["S256"],
        issuer,
    })
}

#[get("/.well-known/jwks.json")]
async fn jwks(
    state: web::Data<AppState>,
    signing_key_repository: Option<web::Data<SigningKeyRepository>>,
) -> HttpResponse {
    if let Some(signing_key_repository) = signing_key_repository.as_ref() {
        match signing_key_repository.latest_active().await {
            Ok(Some(signing_key)) => {
                return match public_jwks_from_public_key_pem(
                    &signing_key.kid,
                    &signing_key.public_key,
                ) {
                    Ok(jwks) => HttpResponse::Ok().json(jwks),
                    Err(error) => {
                        tracing::error!(%error, "failed to build repository-backed JWKS");
                        HttpResponse::InternalServerError().finish()
                    }
                };
            }
            Ok(None) => {}
            Err(error) => {
                tracing::error!(%error, "failed to load active signing key");
                return HttpResponse::InternalServerError().finish();
            }
        }
    }

    match public_jwks(&state.settings) {
        Ok(jwks) => HttpResponse::Ok().json(jwks),
        Err(error) => {
            tracing::error!(%error, "failed to build JWKS");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[derive(Debug, Serialize)]
struct UserInfoResponse {
    sub: String,
    scope: String,
}

#[get("/userinfo")]
async fn userinfo(request: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    let Some(access_token) = bearer_token(&request) else {
        return HttpResponse::Unauthorized().finish();
    };

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[state.settings.jwt_audience.as_str()]);
    validation.set_issuer(&[state.settings.jwt_issuer.as_str()]);

    let decoding_key = match DecodingKey::from_rsa_pem(state.settings.jwt_public_key_pem.as_bytes())
    {
        Ok(key) => key,
        Err(error) => {
            tracing::error!(%error, "failed to parse JWT public key for userinfo");
            return HttpResponse::InternalServerError().finish();
        }
    };
    let decoded = decode::<Claims>(access_token, &decoding_key, &validation);

    match decoded {
        Ok(token) => HttpResponse::Ok().json(UserInfoResponse {
            sub: token.claims.sub,
            scope: token.claims.scope,
        }),
        Err(_) => HttpResponse::Unauthorized().finish(),
    }
}

fn bearer_token(request: &HttpRequest) -> Option<&str> {
    request
        .headers()
        .get(actix_web::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|token| !token.trim().is_empty())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(openid_configuration)
        .service(jwks)
        .service(userinfo);
}
