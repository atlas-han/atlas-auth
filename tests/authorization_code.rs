use atlas_auth::oauth::authorization_code::{hash_authorization_code, issue_authorization_code};
use chrono::Utc;

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
