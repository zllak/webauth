#[cfg(feature = "axum-core")]
pub mod axum;

#[path = "./store.rs"]
mod _store;
pub mod store {
    pub use super::_store::{Error, Identifiable, Store};
}

#[path = "./user.rs"]
mod _user;
pub mod user {
    pub use super::_user::{UserManager, UserManagerLayer};
}

#[path = "./session.rs"]
mod _session;
pub mod session {
    pub use super::_session::{Session, SessionManager, SessionManagerLayer, DEFAULT_EXPIRATION};
    // Re-exports the Uuid we use
    pub use uuid::Uuid;
}

#[cfg(feature = "password")]
#[path = "./password/mod.rs"]
mod _password;
#[cfg(feature = "password")]
pub mod password {
    pub use super::_password::{hash, verify};
}
