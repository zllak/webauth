use crate::session::Session;
use axum_core::extract::FromRequestParts;
use http::{request::Parts, StatusCode};

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
