use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use base64::Engine;
use rand::Rng;

/// Generate a cryptographically secure random token
/// Returns a base64-encoded 32-byte token
pub fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    let token_bytes: [u8; 32] = rng.r#gen();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(token_bytes)
}

/// Hash a token using Argon2id
/// This should be called before storing the token in the database
pub fn hash_token(token: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(token.as_bytes(), &salt)?;
    Ok(password_hash.to_string())
}

/// Verify a token against its hash
/// Returns true if the token matches the hash
pub fn verify_token(token: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(token.as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token() {
        let token1 = generate_token();
        let token2 = generate_token();

        // Tokens should be different
        assert_ne!(token1, token2);

        // Tokens should be base64 encoded
        assert!(
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(&token1)
                .is_ok()
        );
    }

    #[test]
    fn test_hash_and_verify_token() {
        let token = generate_token();
        let hash = hash_token(&token).expect("Failed to hash token");

        // Verify correct token
        assert!(verify_token(&token, &hash));

        // Verify incorrect token
        let wrong_token = generate_token();
        assert!(!verify_token(&wrong_token, &hash));
    }

    #[test]
    fn test_verify_invalid_hash() {
        let token = generate_token();
        let invalid_hash = "not_a_valid_hash";

        assert!(!verify_token(&token, invalid_hash));
    }
}
