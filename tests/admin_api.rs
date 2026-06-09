use actix_web::{test, web, App};
use atlas_auth::{oauth::client_repository::OAuthClientRepository, routes};
use serde_json::{json, Value};

#[actix_rt::test]
async fn admin_clients_crud_and_secret_rotation() {
    let clients = OAuthClientRepository::in_memory(vec![]);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(clients.clone()))
            .configure(routes::admin::configure),
    )
    .await;

    let create = test::TestRequest::post()
        .uri("/admin/clients")
        .set_json(json!({
            "client_id": "client-admin-1",
            "name": "Admin Client",
            "client_type": "confidential",
            "redirect_uris": ["https://app.example.test/callback"],
            "grant_types": ["authorization_code", "client_credentials"],
            "scopes": ["openid", "email"],
            "trusted_first_party": false
        }))
        .to_request();
    let created = test::call_service(&app, create).await;
    assert_eq!(created.status(), actix_web::http::StatusCode::CREATED);
    let created_body: Value = test::read_body_json(created).await;
    let first_secret = created_body["client_secret"].as_str().unwrap();
    assert!(first_secret.len() >= 32);

    let stored = clients
        .find_active_by_public_client_id("client-admin-1")
        .await
        .unwrap()
        .expect("client should be stored");
    assert_ne!(stored.client_secret_hash.as_deref(), Some(first_secret));

    let rotate = test::TestRequest::post()
        .uri("/admin/clients/client-admin-1/secret/rotate")
        .to_request();
    let rotated = test::call_service(&app, rotate).await;
    assert_eq!(rotated.status(), actix_web::http::StatusCode::OK);
    let rotated_body: Value = test::read_body_json(rotated).await;
    let second_secret = rotated_body["client_secret"].as_str().unwrap();
    assert_ne!(first_secret, second_secret);

    let update = test::TestRequest::put()
        .uri("/admin/clients/client-admin-1")
        .set_json(json!({
            "redirect_uris": ["https://app.example.test/new-callback"],
            "grant_types": ["authorization_code"],
            "scopes": ["openid"],
            "trusted_first_party": true
        }))
        .to_request();
    assert_eq!(
        test::call_service(&app, update).await.status(),
        actix_web::http::StatusCode::OK
    );

    let delete = test::TestRequest::delete()
        .uri("/admin/clients/client-admin-1")
        .to_request();
    assert_eq!(
        test::call_service(&app, delete).await.status(),
        actix_web::http::StatusCode::NO_CONTENT
    );
    assert!(clients
        .find_active_by_public_client_id("client-admin-1")
        .await
        .unwrap()
        .is_none());
}

#[actix_rt::test]
async fn admin_users_crud() {
    let users = routes::admin::AdminUserRepository::in_memory();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(users))
            .configure(routes::admin::configure),
    )
    .await;

    let create = test::TestRequest::post()
        .uri("/admin/users")
        .set_json(
            json!({"email": "user@example.test", "status": "active", "email_verified": false}),
        )
        .to_request();
    let created = test::call_service(&app, create).await;
    assert_eq!(created.status(), actix_web::http::StatusCode::CREATED);
    let created_body: Value = test::read_body_json(created).await;
    let user_id = created_body["id"].as_str().unwrap();

    let update = test::TestRequest::put()
        .uri(&format!("/admin/users/{user_id}"))
        .set_json(
            json!({"email": "updated@example.test", "status": "locked", "email_verified": true}),
        )
        .to_request();
    let updated = test::call_service(&app, update).await;
    assert_eq!(updated.status(), actix_web::http::StatusCode::OK);
    let updated_body: Value = test::read_body_json(updated).await;
    assert_eq!(updated_body["email"], "updated@example.test");
    assert_eq!(updated_body["status"], "locked");
    assert_eq!(updated_body["email_verified"], true);

    let delete = test::TestRequest::delete()
        .uri(&format!("/admin/users/{user_id}"))
        .to_request();
    assert_eq!(
        test::call_service(&app, delete).await.status(),
        actix_web::http::StatusCode::NO_CONTENT
    );

    let get_deleted = test::TestRequest::get()
        .uri(&format!("/admin/users/{user_id}"))
        .to_request();
    assert_eq!(
        test::call_service(&app, get_deleted).await.status(),
        actix_web::http::StatusCode::NOT_FOUND
    );
}
