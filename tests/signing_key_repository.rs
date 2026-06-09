use atlas_auth::auth::signing_key_repository::{NewSigningKey, SigningKeyRepository};

fn active_key(kid: &str) -> NewSigningKey {
    NewSigningKey {
        kid: kid.to_string(),
        public_key: format!("public-{kid}"),
        private_key_ciphertext: format!("encrypted-private-{kid}"),
        algorithm: "RS256".to_string(),
        status: "active".to_string(),
    }
}

#[actix_rt::test]
async fn signing_key_repository_returns_latest_active_key() {
    let repository = SigningKeyRepository::in_memory();

    repository.save(active_key("key-1")).await.unwrap();
    repository.save(active_key("key-2")).await.unwrap();

    let active = repository
        .latest_active()
        .await
        .unwrap()
        .expect("active signing key should exist");

    assert_eq!(active.kid, "key-2");
    assert_eq!(active.algorithm, "RS256");
    assert_eq!(active.private_key_ciphertext, "encrypted-private-key-2");
}

#[actix_rt::test]
async fn signing_key_repository_retires_key_by_kid() {
    let repository = SigningKeyRepository::in_memory();

    repository.save(active_key("key-1")).await.unwrap();
    repository.retire("key-1").await.unwrap();

    assert!(repository.latest_active().await.unwrap().is_none());
}
