use argon2::password_hash::PasswordHashString;
use argon2::password_hash::{
    rand_core::OsRng, Error, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;

/// Represents a plain password.
#[derive(Debug, Clone)]
pub struct PlainPassword(String);

impl From<String> for PlainPassword {
    fn from(value: String) -> Self {
        PlainPassword(value)
    }
}

impl PlainPassword {
    /// Ciphers the plain password
    pub fn cipher(self) -> Result<CipheredPassword, Error> {
        self.try_into()
    }
}

// ----------------------------------------------------------------------------

/// Represents a ciphered password.;
#[derive(Debug, Clone)]
pub struct CipheredPassword(PasswordHashString);

impl TryFrom<PlainPassword> for CipheredPassword {
    type Error = Error;

    fn try_from(value: PlainPassword) -> Result<Self, Self::Error> {
        Ok(Self(hash(value.0.as_bytes())?))
    }
}

impl TryFrom<&str> for CipheredPassword {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self(PasswordHash::new(value)?.serialize()))
    }
}

impl CipheredPassword {
    pub fn verify(&self, password: &[u8]) -> Result<bool, Error> {
        verify(password, &self.0.password_hash())
    }
}

// ----------------------------------------------------------------------------

/// Hash the given password
pub fn hash(password: &[u8]) -> Result<PasswordHashString, Error> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password, &salt)?
        .serialize())
}

/// Verify that the given password matches the given hash (hash must be
/// generated using `hash`)
pub fn verify(password: &[u8], password_hash: &PasswordHash<'_>) -> Result<bool, Error> {
    Ok(Argon2::default()
        .verify_password(password, password_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password() -> Result<(), Error> {
        let passwd = "thisisafakepassword";

        let hashed = hash(passwd.as_ref())?;
        assert_ne!(passwd, hashed.as_str());
        assert!(hashed.as_str().starts_with("$argon2id$"), "{}", hashed);
        assert_eq!(hashed.len(), 97);

        let passwd = "anotherfakepasswordbutdifferent";

        let hashed = hash(passwd.as_ref())?;
        assert_ne!(passwd, hashed.as_str());
        assert!(hashed.as_str().starts_with("$argon2id$"), "{}", hashed);
        assert_eq!(hashed.len(), 97);

        // Verify now
        assert!(verify(passwd.as_ref(), &hashed.password_hash())?);
        assert!(!verify(b"thisisnotright", &hashed.password_hash())?);

        Ok(())
    }

    #[test]
    fn types() {
        let plain: PlainPassword = "thisisapassword".to_owned().into();
        let ciphered = plain.cipher().expect("should not fail");

        let ciphered: CipheredPassword = ciphered.0.as_ref().try_into().expect("should not fail");
        assert!(ciphered
            .verify("thisisapassword".as_ref())
            .expect("should not fail"));
        assert!(!ciphered
            .verify("wrongpassword".as_ref())
            .expect("should not fail"));

        let err = std::convert::TryInto::<CipheredPassword>::try_into("notavalidargon");
        assert_eq!(err.unwrap_err(), Error::PhcStringField,);
    }
}
