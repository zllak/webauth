use crate::session::Session;
use crate::store::Identifiable;
use axum_core::extract::FromRequestParts;
use http::{request::Parts, StatusCode};

// ----------------------------------------------------------------------------

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for Session
where
    S: Sync + Send,
{
    type Rejection = (http::StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Session>().cloned().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "No Session found, is the layer installed?",
        ))
    }
}

// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default)]
pub struct ProtectedUser<U>(pub U);

// Implement FromRequestParts for any type that implements Identifiable
#[async_trait::async_trait]
impl<S, U> FromRequestParts<S> for ProtectedUser<U>
where
    S: Sync + Send,
    U: Identifiable + Clone + Sync + Send + 'static,
{
    type Rejection = (http::StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<U>()
            .cloned()
            .ok_or((
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "No Identifiable found, is the layer installed?",
            ))
            .map(|user| ProtectedUser(user))
    }
}
