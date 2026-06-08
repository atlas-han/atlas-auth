use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;

pub fn hash_password(password: &str, pepper: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let peppered = pepper_password(password, pepper);
    Ok(argon2
        .hash_password(peppered.as_bytes(), &salt)?
        .to_string())
}

pub fn verify_password(password: &str, pepper: &str, password_hash: &str) -> anyhow::Result<bool> {
    let parsed_hash = PasswordHash::new(password_hash)?;
    let peppered = pepper_password(password, pepper);
    Ok(Argon2::default()
        .verify_password(peppered.as_bytes(), &parsed_hash)
        .is_ok())
}

fn pepper_password(password: &str, pepper: &str) -> String {
    format!("{password}{pepper}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_original_password() {
        let hash = hash_password("correct horse battery staple", "test-pepper").unwrap();

        let verified =
            verify_password("correct horse battery staple", "test-pepper", &hash).unwrap();

        assert!(verified);
        assert_ne!(hash, "correct horse battery staple");
    }

    #[test]
    fn rejects_wrong_password() {
        let hash = hash_password("correct horse battery staple", "test-pepper").unwrap();

        let verified = verify_password("wrong password", "test-pepper", &hash).unwrap();

        assert!(!verified);
    }
}
