use argon2::password_hash::{
    rand_core::OsRng, Error, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;

/// Hash the given password
pub fn hash(password: &[u8]) -> Result<String, Error> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password, &salt)?
        .to_string())
}

/// Verify that the given password matches the given hash (hash must be
/// generated using `hash`)
pub fn verify(password: &[u8], password_hash: &str) -> Result<bool, Error> {
    let parsed_hash = PasswordHash::new(password_hash)?;
    Ok(Argon2::default()
        .verify_password(password, &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password() -> Result<(), Error> {
        let passwd = "thisisafakepassword";

        let hashed = hash(passwd.as_ref())?;
        assert_ne!(passwd.to_string(), hashed);
        assert!(hashed.starts_with("$argon2id$"), "{}", hashed);
        assert_eq!(hashed.len(), 97);

        let passwd = "anotherfakepasswordbutdifferent";

        let hashed = hash(passwd.as_ref())?;
        assert_ne!(passwd.to_string(), hashed);
        assert!(hashed.starts_with("$argon2id$"), "{}", hashed);
        assert_eq!(hashed.len(), 97);

        // Verify now
        assert!(verify(passwd.as_ref(), hashed.as_ref())?);
        assert!(!verify(b"thisisnotright", hashed.as_ref())?);

        Ok(())
    }
}
