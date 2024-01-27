#[cfg(feature = "axum-core")]
mod axum;

#[path = "./store.rs"]
mod _store;
pub mod store {
    pub use super::_store::{Error, Store};
}

#[path = "./auth.rs"]
mod _auth;
pub mod auth {
    pub use super::_auth::{AuthBackend, AuthUser};
}

#[path = "./session.rs"]
mod _session;
pub mod session {
    pub use super::_session::{Session, SessionManager, SessionManagerLayer, DEFAULT_EXPIRATION};
}
