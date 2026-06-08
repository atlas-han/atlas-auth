use atlas_auth::oauth::pkce::{s256_code_challenge, verify_s256_code_challenge};

#[test]
fn s256_code_challenge_matches_rfc7636_example() {
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

    let challenge = s256_code_challenge(verifier);

    assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
}

#[test]
fn verify_s256_code_challenge_accepts_matching_verifier() {
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

    assert!(verify_s256_code_challenge(verifier, challenge));
}

#[test]
fn verify_s256_code_challenge_rejects_reused_or_wrong_verifier() {
    let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

    assert!(!verify_s256_code_challenge("wrong-verifier", challenge));
}
