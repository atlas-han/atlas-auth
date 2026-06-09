use actix_web::{test, web, App};
use atlas_auth::{
    auth::{
        account_recovery_repository::{
            AccountRecoveryRepository, AccountTokenPurpose, NewAccountRecoveryToken,
        },
        token::hash_refresh_token,
    },
    routes,
};
use chrono::{Duration, Utc};
use uuid::Uuid;

#[actix_rt::test]
async fn email_verify_endpoint_consumes_active_verification_token() {
    let repository = AccountRecoveryRepository::in_memory();
    let token = "email-verification-token";
    repository
        .save(NewAccountRecoveryToken {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            token_hash: hash_refresh_token(token),
            purpose: AccountTokenPurpose::EmailVerification,
            expires_at: Utc::now() + Duration::minutes(30),
        })
        .await
        .unwrap();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(repository.clone()))
            .configure(routes::auth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/v1/auth/email/verify")
        .set_json(serde_json::json!({ "token": token }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), actix_web::http::StatusCode::NO_CONTENT);
    let stored = repository
        .find_active_by_hash(
            &hash_refresh_token(token),
            AccountTokenPurpose::EmailVerification,
            Utc::now(),
        )
        .await
        .unwrap();
    assert!(stored.is_none(), "verification token should be consumed");
}

#[actix_rt::test]
async fn password_reset_endpoint_consumes_active_reset_token() {
    let repository = AccountRecoveryRepository::in_memory();
    let token = "password-reset-token";
    repository
        .save(NewAccountRecoveryToken {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            token_hash: hash_refresh_token(token),
            purpose: AccountTokenPurpose::PasswordReset,
            expires_at: Utc::now() + Duration::minutes(30),
        })
        .await
        .unwrap();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(repository.clone()))
            .configure(routes::auth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/v1/auth/password/reset")
        .set_json(serde_json::json!({
            "token": token,
            "new_password": "correct horse battery staple reset"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), actix_web::http::StatusCode::NO_CONTENT);
    let stored = repository
        .find_active_by_hash(
            &hash_refresh_token(token),
            AccountTokenPurpose::PasswordReset,
            Utc::now(),
        )
        .await
        .unwrap();
    assert!(stored.is_none(), "reset token should be consumed");
}

#[actix_rt::test]
async fn password_reset_endpoint_rejects_wrong_purpose_token() {
    let repository = AccountRecoveryRepository::in_memory();
    let token = "email-token-not-reset";
    repository
        .save(NewAccountRecoveryToken {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            token_hash: hash_refresh_token(token),
            purpose: AccountTokenPurpose::EmailVerification,
            expires_at: Utc::now() + Duration::minutes(30),
        })
        .await
        .unwrap();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(repository))
            .configure(routes::auth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/v1/auth/password/reset")
        .set_json(serde_json::json!({
            "token": token,
            "new_password": "correct horse battery staple reset"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
}
