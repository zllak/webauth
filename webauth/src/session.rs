use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};
use uuid::Uuid;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Error while serializing/deserializing
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

type Result<T> = std::result::Result<T, Error>;

// Session with a UUIDv4 identifier
// and a "generic" hash map to store data.
// (like the user_uid of the session, ...)
#[derive(Debug, Clone)]
pub struct Session {
    uid: Uuid,
    expires_at: SystemTime,
    data: HashMap<String, Value>,
}

// Expires in one week from now
pub const DEFAULT_EXPIRATION: Duration = Duration::from_secs(60 * 60 * 24 * 7);

impl Session {
    // Creates a new `Session`, providing when the session will expire.
    pub fn new(expires_in: Duration) -> Self {
        Self {
            uid: Uuid::new_v4(),
            expires_at: SystemTime::now() + expires_in,
            data: HashMap::default(),
        }
    }

    // Returns the uniquer identifier of this `Session`.
    pub const fn uid(&self) -> &Uuid {
        &self.uid
    }

    // Returns when the `Session` expires.
    pub const fn expires_at(&self) -> &SystemTime {
        &self.expires_at
    }

    // Regenerate a new unique identifier for the session.
    // This can be useful to keep a session while changing it's unique identifier.
    // Returns the replaced Uuid.
    pub fn cycle_uid(&mut self) -> Uuid {
        let old_uid = self.uid;

        self.uid = Uuid::new_v4();
        old_uid
    }

    // Insert a new data in the session.
    pub fn insert(&mut self, key: &str, value: impl Serialize) -> Result<()> {
        self.data
            .insert(key.to_string(), serde_json::to_value(value)?);
        Ok(())
    }

    // Get a value from the data stored in the session.
    // Data stored must be JSON-serializable.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        self.data
            .get(key)
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(Into::into)
    }

    // Removes an item from the data stored in the session, returning the value if any.
    pub fn remove<T: DeserializeOwned>(&mut self, key: &str) -> Result<Option<T>> {
        self.data
            .remove(key)
            .map(serde_json::from_value)
            .transpose()
            .map_err(Into::into)
    }

    // Clear all data stored
    pub fn clear(&mut self) {
        self.data.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store() -> Result<()> {
        let mut session = Session::new(DEFAULT_EXPIRATION);
        let uid = Uuid::new_v4();

        // Insert and get to make sure we stored properly
        session.insert("user_uid", uid)?;
        assert_eq!(Some(uid), session.get("user_uid")?);

        // Generate a new uid, insert with the same key and make sure
        // we now get the new value.
        let new_uid = Uuid::new_v4();
        assert_ne!(uid, new_uid);
        session.insert("user_uid", new_uid)?;
        assert_eq!(Some(new_uid), session.get("user_uid")?);

        assert_eq!(1, session.data.len());
        session.insert("u64", 42u64)?;
        assert_eq!(2, session.data.len());

        // Remove a key
        assert_eq!(Some(42u64), session.remove("u64")?);
        assert_eq!(None, session.remove::<()>("unknown")?);
        assert_eq!(1, session.data.len());

        // Clear the store
        session.clear();
        assert_eq!(0, session.data.len());

        Ok(())
    }

    #[test]
    fn cycle_uid() {
        let mut session = Session::new(DEFAULT_EXPIRATION);

        let uid = *session.uid();
        let old_uid = session.cycle_uid();

        assert_eq!(uid, old_uid);
        assert_ne!(old_uid, *session.uid());
    }
}
