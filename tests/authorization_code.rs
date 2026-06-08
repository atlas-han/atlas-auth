use atlas_auth::oauth::authorization_code::{
    exchange_authorization_code, hash_authorization_code, issue_authorization_code,
    StoredAuthorizationCode,
};
use atlas_auth::oauth::pkce::s256_code_challenge;
use chrono::Utc;
use uuid::Uuid;

#[test]
fn issued_authorization_code_is_opaque_and_short_lived() {
    let issued = issue_authorization_code();
    let now = Utc::now();

    assert!(issued.plaintext.len() >= 96);
    assert_ne!(issued.plaintext, issued.code_hash);
    assert!(issued.expires_at > now);
    assert!(issued.expires_at <= now + chrono::Duration::seconds(60));
}

#[test]
fn authorization_code_hash_is_stable_and_not_plaintext() {
    let code = "authorization-code-value";

    let first = hash_authorization_code(code);
    let second = hash_authorization_code(code);

    assert_eq!(first, second);
    assert_ne!(first, code);
    assert_eq!(first.len(), 64);
}

fn stored_code(verifier: &str) -> StoredAuthorizationCode {
    let plaintext = "stored-code";
    StoredAuthorizationCode {
        code_hash: hash_authorization_code(plaintext),
        client_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        redirect_uri: "https://app.example.test/callback".to_string(),
        code_challenge: s256_code_challenge(verifier),
        code_challenge_method: "S256".to_string(),
        scope: vec!["openid".to_string(), "email".to_string()],
        expires_at: Utc::now() + chrono::Duration::seconds(60),
        consumed: false,
    }
}

#[test]
fn exchange_authorization_code_accepts_matching_code_and_pkce_verifier() {
    let verifier = "verifier-with-enough-length-1234567890";
    let stored = stored_code(verifier);

    let exchanged = exchange_authorization_code(
        &stored,
        "stored-code",
        verifier,
        &stored.redirect_uri,
        stored.client_id,
        Utc::now(),
    );

    assert!(exchanged.is_ok());
}

#[test]
fn exchange_authorization_code_rejects_wrong_pkce_verifier() {
    let verifier = "verifier-with-enough-length-1234567890";
    let stored = stored_code(verifier);

    let exchanged = exchange_authorization_code(
        &stored,
        "stored-code",
        "wrong-verifier",
        &stored.redirect_uri,
        stored.client_id,
        Utc::now(),
    );

    assert_eq!(exchanged, Err("invalid code_verifier"));
}

#[test]
fn exchange_authorization_code_rejects_consumed_code() {
    let verifier = "verifier-with-enough-length-1234567890";
    let mut stored = stored_code(verifier);
    stored.consumed = true;

    let exchanged = exchange_authorization_code(
        &stored,
        "stored-code",
        verifier,
        &stored.redirect_uri,
        stored.client_id,
        Utc::now(),
    );

    assert_eq!(
        exchanged,
        Err("authorization code has already been consumed")
    );
}
