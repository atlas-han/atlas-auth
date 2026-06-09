use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use actix_web::{web, HttpResponse};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::oauth::{
    client::hash_client_secret,
    client_repository::{ClientRecord, OAuthClientRepository},
};

#[derive(Debug, Deserialize)]
struct CreateClientRequest {
    client_id: String,
    client_type: String,
    redirect_uris: Vec<String>,
    grant_types: Vec<String>,
    scopes: Vec<String>,
    trusted_first_party: bool,
}

#[derive(Debug, Deserialize)]
struct UpdateClientRequest {
    redirect_uris: Vec<String>,
    grant_types: Vec<String>,
    scopes: Vec<String>,
    trusted_first_party: bool,
}

#[derive(Debug, Serialize)]
struct ClientSecretResponse {
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct AdminUser {
    id: Uuid,
    email: String,
    status: String,
    email_verified: bool,
}

#[derive(Debug, Deserialize)]
struct UpsertUserRequest {
    email: String,
    status: String,
    email_verified: bool,
}

#[derive(Clone, Default)]
pub struct AdminUserRepository {
    users: Arc<Mutex<HashMap<Uuid, AdminUser>>>,
}

impl AdminUserRepository {
    pub fn in_memory() -> Self {
        Self::default()
    }

    fn create(&self, request: UpsertUserRequest) -> AdminUser {
        let user = AdminUser {
            id: Uuid::new_v4(),
            email: request.email,
            status: request.status,
            email_verified: request.email_verified,
        };
        self.users
            .lock()
            .expect("admin users lock poisoned")
            .insert(user.id, user.clone());
        user
    }

    fn update(&self, user_id: Uuid, request: UpsertUserRequest) -> Option<AdminUser> {
        let mut users = self.users.lock().expect("admin users lock poisoned");
        let user = users.get_mut(&user_id)?;
        user.email = request.email;
        user.status = request.status;
        user.email_verified = request.email_verified;
        Some(user.clone())
    }

    fn get(&self, user_id: Uuid) -> Option<AdminUser> {
        self.users
            .lock()
            .expect("admin users lock poisoned")
            .get(&user_id)
            .cloned()
    }

    fn delete(&self, user_id: Uuid) -> bool {
        self.users
            .lock()
            .expect("admin users lock poisoned")
            .remove(&user_id)
            .is_some()
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            .route("/clients", web::post().to(create_client))
            .route("/clients/{client_id}", web::put().to(update_client))
            .route("/clients/{client_id}", web::delete().to(delete_client))
            .route(
                "/clients/{client_id}/secret/rotate",
                web::post().to(rotate_client_secret),
            )
            .route("/users", web::post().to(create_user))
            .route("/users/{user_id}", web::get().to(get_user))
            .route("/users/{user_id}", web::put().to(update_user))
            .route("/users/{user_id}", web::delete().to(delete_user)),
    );
}

async fn create_client(
    clients: web::Data<OAuthClientRepository>,
    request: web::Json<CreateClientRequest>,
) -> HttpResponse {
    let secret = generate_client_secret();
    let record = ClientRecord {
        id: Uuid::new_v4(),
        public_client_id: request.client_id.clone(),
        client_secret_hash: (request.client_type == "confidential")
            .then(|| hash_client_secret(&secret)),
        client_type: request.client_type.clone(),
        allowed_redirect_uris: request.redirect_uris.clone(),
        grant_types: request.grant_types.clone(),
        scopes: request.scopes.clone(),
        status: "active".to_string(),
        trusted_first_party: request.trusted_first_party,
        access_token_ttl_seconds: None,
        refresh_token_ttl_seconds: None,
    };

    if clients.upsert_in_memory(record).is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Created().json(ClientSecretResponse {
        client_id: request.client_id.clone(),
        client_secret: secret,
    })
}

async fn update_client(
    clients: web::Data<OAuthClientRepository>,
    client_id: web::Path<String>,
    request: web::Json<UpdateClientRequest>,
) -> HttpResponse {
    match clients.update_in_memory(&client_id, |client| {
        client.allowed_redirect_uris = request.redirect_uris.clone();
        client.grant_types = request.grant_types.clone();
        client.scopes = request.scopes.clone();
        client.trusted_first_party = request.trusted_first_party;
    }) {
        Ok(true) => HttpResponse::Ok().finish(),
        Ok(false) => HttpResponse::NotFound().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn rotate_client_secret(
    clients: web::Data<OAuthClientRepository>,
    client_id: web::Path<String>,
) -> HttpResponse {
    let secret = generate_client_secret();
    match clients.update_in_memory(&client_id, |client| {
        client.client_secret_hash = Some(hash_client_secret(&secret));
    }) {
        Ok(true) => HttpResponse::Ok().json(ClientSecretResponse {
            client_id: client_id.into_inner(),
            client_secret: secret,
        }),
        Ok(false) => HttpResponse::NotFound().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn delete_client(
    clients: web::Data<OAuthClientRepository>,
    client_id: web::Path<String>,
) -> HttpResponse {
    match clients.deactivate_in_memory(&client_id) {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => HttpResponse::NotFound().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn create_user(
    users: web::Data<AdminUserRepository>,
    request: web::Json<UpsertUserRequest>,
) -> HttpResponse {
    HttpResponse::Created().json(users.create(request.into_inner()))
}

async fn get_user(users: web::Data<AdminUserRepository>, user_id: web::Path<Uuid>) -> HttpResponse {
    match users.get(user_id.into_inner()) {
        Some(user) => HttpResponse::Ok().json(user),
        None => HttpResponse::NotFound().finish(),
    }
}

async fn update_user(
    users: web::Data<AdminUserRepository>,
    user_id: web::Path<Uuid>,
    request: web::Json<UpsertUserRequest>,
) -> HttpResponse {
    match users.update(user_id.into_inner(), request.into_inner()) {
        Some(user) => HttpResponse::Ok().json(user),
        None => HttpResponse::NotFound().finish(),
    }
}

async fn delete_user(
    users: web::Data<AdminUserRepository>,
    user_id: web::Path<Uuid>,
) -> HttpResponse {
    if users.delete(user_id.into_inner()) {
        HttpResponse::NoContent().finish()
    } else {
        HttpResponse::NotFound().finish()
    }
}

fn generate_client_secret() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect()
}
