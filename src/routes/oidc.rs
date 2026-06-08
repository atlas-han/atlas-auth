use actix_web::{get, web, HttpResponse};
use serde::Serialize;

use crate::app::AppState;

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

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(openid_configuration);
}
