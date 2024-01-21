use std::fmt::Debug;
use std::future::Future;

/// A User which can be authenticated and identified.
pub trait AuthUser: Debug + Clone {
    type Id;

    fn id(&self) -> Self::Id;
}

/// An authentication backend
pub trait AuthBackend {
    type User: AuthUser;
    type Credentials;
    type Error: std::error::Error;

    fn authenticate(
        &self,
        credentials: Self::Credentials,
    ) -> impl Future<Output = Result<Option<Self::User>, Self::Error>> + Send;
    fn get_user(
        &self,
        id: &<Self::User as AuthUser>::Id,
    ) -> impl Future<Output = Result<Option<Self::User>, Self::Error>> + Send;
}
