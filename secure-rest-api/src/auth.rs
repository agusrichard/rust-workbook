use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::Error;

pub fn validate_password(password: &str) -> Result<(), Error> {
    if password.len() < 8 {
        return Err(Error::new("Password must be at least 8 characters"));
    }

    Ok(())
}

pub fn hash_password(password: &str) -> Result<String, Error> {
    let hash = Argon2::default()
        .hash_password(password.as_bytes())
        .map_err(|_| Error::new("Please try again"))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    match PasswordHash::new(stored_hash) {
        Ok(hash) => Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok(),
        Err(_) => false,
    }
}
