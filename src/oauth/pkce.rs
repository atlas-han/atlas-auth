use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};

pub fn s256_code_challenge(code_verifier: &str) -> String {
    let digest = Sha256::digest(code_verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

pub fn verify_s256_code_challenge(code_verifier: &str, expected_code_challenge: &str) -> bool {
    s256_code_challenge(code_verifier) == expected_code_challenge
}
