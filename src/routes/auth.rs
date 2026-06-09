use actix_web::{web, HttpResponse};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::{
    app::AppState,
    auth::{
        account_recovery_repository::{AccountRecoveryRepository, AccountTokenPurpose},
        federated_identity_repository::{FederatedIdentityRepository, NewFederatedIdentity},
        login_attempt_repository::LoginAttemptRepository,
        password::{hash_password, verify_password},
        token::{hash_refresh_token, issue_access_token, new_refresh_token},
    },
    config::Settings,
    error::AppError,
};

#[derive(Debug, Deserialize, Validate)]
pub struct PasswordAuthRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 12, max = 256))]
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct TokenOnlyRequest {
    #[validate(length(min = 1, max = 512))]
    pub token: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct PasswordResetRequest {
    #[validate(length(min = 1, max = 512))]
    pub token: String,
    #[validate(length(min = 12, max = 256))]
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct SocialCallbackQuery {
    pub provider_user_id: String,
    pub email: Option<String>,
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct SocialCallbackResponse {
    pub provider: String,
    pub provider_user_id: String,
    pub user_id: Uuid,
    pub linked_existing: bool,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub user_id: Uuid,
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
}

#[derive(sqlx::FromRow)]
struct LoginRow {
    user_id: Uuid,
    password_hash: String,
}

#[derive(sqlx::FromRow)]
struct RefreshTokenRow {
    id: Uuid,
    user_id: Uuid,
    family_id: Uuid,
    revoked_at: Option<chrono::DateTime<Utc>>,
    expires_at: chrono::DateTime<Utc>,
}

pub async fn register(
    state: web::Data<AppState>,
    request: web::Json<PasswordAuthRequest>,
) -> Result<HttpResponse, AppError> {
    request.validate().map_err(|_| AppError::Validation)?;

    let user_id = Uuid::new_v4();
    let normalized_email = request.email.trim().to_lowercase();
    let password_hash = hash_password(&request.password, &state.settings.password_pepper)
        .map_err(AppError::Internal)?;

    sqlx::query("INSERT INTO users (id, email) VALUES ($1, $2)")
        .bind(user_id)
        .bind(&normalized_email)
        .execute(&state.pool)
        .await?;

    sqlx::query("INSERT INTO user_credentials (user_id, password_hash) VALUES ($1, $2)")
        .bind(user_id)
        .bind(password_hash)
        .execute(&state.pool)
        .await?;

    write_audit_event(&state.pool, Some(user_id), "user_registered").await?;
    let response = issue_token_pair(&state.pool, &state.settings, user_id, None).await?;

    Ok(HttpResponse::Created().json(response))
}

pub async fn login(
    state: Option<web::Data<AppState>>,
    login_attempts: Option<web::Data<LoginAttemptRepository>>,
    request: web::Json<PasswordAuthRequest>,
) -> Result<HttpResponse, AppError> {
    request.validate().map_err(|_| AppError::Validation)?;
    let normalized_email = request.email.trim().to_lowercase();

    if let Some(login_attempts) = login_attempts.as_ref() {
        let status = login_attempts.status(&normalized_email, Utc::now()).await?;
        if status.locked {
            return Err(AppError::AccountLocked);
        }
    }

    let state = state.ok_or_else(|| AppError::Internal(anyhow::anyhow!("app state missing")))?;

    let row = sqlx::query_as::<_, LoginRow>(
        r#"
        SELECT users.id AS user_id, user_credentials.password_hash
        FROM users
        JOIN user_credentials ON user_credentials.user_id = users.id
        WHERE users.email = $1 AND users.status = 'active'
        "#,
    )
    .bind(&normalized_email)
    .fetch_optional(&state.pool)
    .await?;

    let Some(row) = row else {
        if let Some(login_attempts) = login_attempts.as_ref() {
            login_attempts
                .record_failure(&normalized_email, 5, Duration::minutes(15), Utc::now())
                .await?;
        }
        write_audit_event(&state.pool, None, "login_failed").await?;
        return Err(AppError::InvalidCredentials);
    };

    let verified = verify_password(
        &request.password,
        &state.settings.password_pepper,
        &row.password_hash,
    )
    .map_err(AppError::Internal)?;

    if !verified {
        if let Some(login_attempts) = login_attempts.as_ref() {
            login_attempts
                .record_failure(&normalized_email, 5, Duration::minutes(15), Utc::now())
                .await?;
        }
        write_audit_event(&state.pool, Some(row.user_id), "login_failed").await?;
        return Err(AppError::InvalidCredentials);
    }

    if let Some(login_attempts) = login_attempts.as_ref() {
        login_attempts.clear(&normalized_email).await?;
    }
    write_audit_event(&state.pool, Some(row.user_id), "login_succeeded").await?;
    let response = issue_token_pair(&state.pool, &state.settings, row.user_id, None).await?;

    Ok(HttpResponse::Ok().json(response))
}

pub async fn refresh(
    state: web::Data<AppState>,
    request: web::Json<RefreshTokenRequest>,
) -> Result<HttpResponse, AppError> {
    let token_hash = hash_refresh_token(&request.refresh_token);
    let existing = sqlx::query_as::<_, RefreshTokenRow>(
        r#"
        SELECT id, user_id, family_id, revoked_at, expires_at
        FROM refresh_tokens
        WHERE token_hash = $1
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(&state.pool)
    .await?;

    let Some(existing) = existing else {
        return Err(AppError::Unauthorized);
    };

    if existing.revoked_at.is_some() {
        revoke_refresh_family(&state.pool, existing.family_id).await?;
        write_audit_event(
            &state.pool,
            Some(existing.user_id),
            "refresh_reuse_detected",
        )
        .await?;
        return Err(AppError::Unauthorized);
    }

    if existing.expires_at <= Utc::now() {
        return Err(AppError::Unauthorized);
    }

    let response = issue_token_pair(
        &state.pool,
        &state.settings,
        existing.user_id,
        Some(existing.family_id),
    )
    .await?;

    let new_hash = hash_refresh_token(&response.refresh_token);
    let replacement_id =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM refresh_tokens WHERE token_hash = $1")
            .bind(&new_hash)
            .fetch_one(&state.pool)
            .await?;

    sqlx::query("UPDATE refresh_tokens SET revoked_at = now(), replaced_by = $1 WHERE id = $2")
        .bind(replacement_id)
        .bind(existing.id)
        .execute(&state.pool)
        .await?;

    write_audit_event(&state.pool, Some(existing.user_id), "refresh_rotated").await?;
    Ok(HttpResponse::Ok().json(response))
}

pub async fn logout(
    state: web::Data<AppState>,
    request: web::Json<RefreshTokenRequest>,
) -> Result<HttpResponse, AppError> {
    let token_hash = hash_refresh_token(&request.refresh_token);
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE token_hash = $1 AND revoked_at IS NULL",
    )
    .bind(token_hash)
    .execute(&state.pool)
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

pub async fn verify_email(
    state: Option<web::Data<AppState>>,
    account_recovery: web::Data<AccountRecoveryRepository>,
    request: web::Json<TokenOnlyRequest>,
) -> Result<HttpResponse, AppError> {
    request.validate().map_err(|_| AppError::Validation)?;
    let token_hash = hash_refresh_token(&request.token);
    let now = Utc::now();
    let Some(token) = account_recovery
        .find_active_by_hash(&token_hash, AccountTokenPurpose::EmailVerification, now)
        .await?
    else {
        return Err(AppError::Unauthorized);
    };

    if let Some(state) = state.as_ref() {
        sqlx::query("UPDATE users SET email_verified_at = $1, updated_at = $1 WHERE id = $2")
            .bind(now)
            .bind(token.user_id)
            .execute(&state.pool)
            .await?;
        write_audit_event(&state.pool, Some(token.user_id), "email_verified").await?;
    }

    account_recovery
        .consume(&token_hash, AccountTokenPurpose::EmailVerification, now)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn reset_password(
    state: Option<web::Data<AppState>>,
    account_recovery: web::Data<AccountRecoveryRepository>,
    request: web::Json<PasswordResetRequest>,
) -> Result<HttpResponse, AppError> {
    request.validate().map_err(|_| AppError::Validation)?;
    let token_hash = hash_refresh_token(&request.token);
    let now = Utc::now();
    let Some(token) = account_recovery
        .find_active_by_hash(&token_hash, AccountTokenPurpose::PasswordReset, now)
        .await?
    else {
        return Err(AppError::Unauthorized);
    };

    if let Some(state) = state.as_ref() {
        let password_hash = hash_password(&request.new_password, &state.settings.password_pepper)
            .map_err(AppError::Internal)?;
        sqlx::query(
            "UPDATE user_credentials SET password_hash = $1, password_changed_at = $2, updated_at = $2 WHERE user_id = $3",
        )
        .bind(password_hash)
        .bind(now)
        .bind(token.user_id)
        .execute(&state.pool)
        .await?;
        write_audit_event(&state.pool, Some(token.user_id), "password_reset_completed").await?;
    }

    account_recovery
        .consume(&token_hash, AccountTokenPurpose::PasswordReset, now)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn social_callback(
    provider: web::Path<String>,
    identities: web::Data<FederatedIdentityRepository>,
    query: web::Query<SocialCallbackQuery>,
) -> Result<HttpResponse, AppError> {
    let provider = provider.into_inner().to_lowercase();
    if provider != "google" && provider != "facebook" {
        return Err(AppError::Validation);
    }
    if query.provider_user_id.trim().is_empty() {
        return Err(AppError::Validation);
    }

    if let Some(existing) = identities
        .find_by_provider_user_id(&provider, &query.provider_user_id)
        .await?
    {
        return Ok(HttpResponse::Ok().json(SocialCallbackResponse {
            provider,
            provider_user_id: query.provider_user_id.clone(),
            user_id: existing.user_id,
            linked_existing: true,
        }));
    }

    let user_id = query.user_id.unwrap_or_else(Uuid::new_v4);
    identities
        .link(NewFederatedIdentity {
            id: Uuid::new_v4(),
            user_id,
            provider: provider.clone(),
            provider_user_id: query.provider_user_id.clone(),
            email: query.email.clone(),
            profile: serde_json::json!({
                "provider": provider,
                "provider_user_id": query.provider_user_id,
                "email": query.email,
            }),
        })
        .await?;

    Ok(HttpResponse::Ok().json(SocialCallbackResponse {
        provider,
        provider_user_id: query.provider_user_id.clone(),
        user_id,
        linked_existing: false,
    }))
}

async fn issue_token_pair(
    pool: &PgPool,
    settings: &Settings,
    user_id: Uuid,
    family_id: Option<Uuid>,
) -> Result<TokenResponse, AppError> {
    let (access_token, expires_in) = issue_access_token(settings, user_id, "openid profile email")
        .map_err(AppError::Internal)?;
    let refresh_token = new_refresh_token();
    let refresh_token_hash = hash_refresh_token(&refresh_token);
    let refresh_token_id = Uuid::new_v4();
    let family_id = family_id.unwrap_or_else(Uuid::new_v4);
    let expires_at = Utc::now() + Duration::seconds(settings.jwt_refresh_token_ttl_seconds);

    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (id, user_id, token_hash, family_id, expires_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(refresh_token_id)
    .bind(user_id)
    .bind(refresh_token_hash)
    .bind(family_id)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(TokenResponse {
        user_id,
        access_token,
        refresh_token,
        token_type: "Bearer",
        expires_in,
    })
}

async fn revoke_refresh_family(pool: &PgPool, family_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE family_id = $1 AND revoked_at IS NULL",
    )
    .bind(family_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn write_audit_event(
    pool: &PgPool,
    user_id: Option<Uuid>,
    event_type: &str,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO audit_events (id, user_id, event_type) VALUES ($1, $2, $3)")
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(event_type)
        .execute(pool)
        .await?;
    Ok(())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/auth")
            .route("/password/register", web::post().to(register))
            .route("/password/login", web::post().to(login))
            .route("/token/refresh", web::post().to(refresh))
            .route("/logout", web::post().to(logout))
            .route("/email/verify", web::post().to(verify_email))
            .route("/password/reset", web::post().to(reset_password))
            .route(
                "/social/{provider}/callback",
                web::get().to(social_callback),
            ),
    );
}
