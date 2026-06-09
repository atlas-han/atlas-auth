use actix_web::{test, web, App};
use atlas_auth::{auth::login_attempt_repository::LoginAttemptRepository, routes};
use chrono::{Duration, Utc};
use serde_json::Value;

#[actix_rt::test]
async fn password_login_rejects_locked_subject_before_database_lookup() {
    let login_attempts = LoginAttemptRepository::in_memory();
    login_attempts
        .record_failure("locked@example.test", 1, Duration::minutes(15), Utc::now())
        .await
        .unwrap();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(login_attempts))
            .configure(routes::auth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/v1/auth/password/login")
        .set_json(serde_json::json!({
            "email": "LOCKED@example.test",
            "password": "correct horse battery staple"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["error"], "account_locked");
}
