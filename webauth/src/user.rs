use crate::session::Session;
use http::{Request, Response};
use serde::Deserialize;
use std::{fmt::Debug, future::Future, pin::Pin};
use tower_service::Service;

/// A User which can be authenticated and identified.
pub trait AuthUser: Debug + Clone {
    type Id;

    fn id(&self) -> Self::Id;
}

impl<U> AuthUser for &U
where
    U: AuthUser,
{
    type Id = U::Id;

    fn id(&self) -> Self::Id {
        (*self).id()
    }
}

// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct UserManager<Service, User, Store>
where
    Store: crate::store::Store<Object = User, Id = <User as AuthUser>::Id>,
    User: AuthUser,
{
    inner: Service,
    store: Store,
}

impl<ReqBody, ResBody, S, User, Store> Service<Request<ReqBody>> for UserManager<S, User, Store>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send,
    ReqBody: Send + 'static,
    ResBody: Default,
    User: AuthUser + Send + Sync + 'static,
    for<'de> <User as AuthUser>::Id: Send + std::fmt::Debug + Deserialize<'de>,
    Store: crate::store::Store<Object = User, Id = <User as AuthUser>::Id> + Clone + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future =
        Pin<Box<dyn Future<Output = std::result::Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        // https://docs.rs/tower/latest/tower/trait.Service.html#be-careful-when-cloning-inner-services
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let store = self.store.clone();

        fn return_error<ResBody: Default, Error>(
            code: http::StatusCode,
        ) -> std::result::Result<Response<ResBody>, Error> {
            let mut res = Response::default();
            // TODO: headers
            *res.status_mut() = code;
            Ok(res)
        }

        Box::pin(async move {
            // Start by getting the session
            let Some(session) = req.extensions().get::<Session>() else {
                // this should not be possible but we are in a protected space,
                // redirect
                tracing::warn!("not session found");
                return return_error(http::StatusCode::UNAUTHORIZED);
            };

            // Get the user_uid from the session
            let user_uid = match session.get::<<User as AuthUser>::Id>("user_uid") {
                Ok(Some(user_uid)) => user_uid,
                Ok(None) => {
                    // Session not authenticated
                    tracing::warn!(suid = %session.uid(), "no user_uid found in session");
                    return return_error(http::StatusCode::UNAUTHORIZED);
                }
                Err(err) => {
                    // Unable to get the user_uid from the session
                    tracing::warn!(err = %err, suid = %session.uid(), "unable to get user_uid from session");
                    return return_error(http::StatusCode::INTERNAL_SERVER_ERROR);
                }
            };

            // Get the user
            let user = match store.load(&user_uid).await {
                Ok(Some(user)) => user,
                Ok(None) => {
                    // We have a valid session, with a user_uid that does not
                    // resolve to a valid user. This should not happen.
                    tracing::warn!(uid = %session.uid(), user_uid = ?user_uid, "unable to resolve a valid user");
                    return return_error(http::StatusCode::UNAUTHORIZED);
                }
                Err(err) => {
                    // Unable to load user
                    tracing::warn!(err = %err, uid = %session.uid(), user_uid = ?user_uid, "unable to resolve a valid user");
                    return return_error(http::StatusCode::INTERNAL_SERVER_ERROR);
                }
            };

            tracing::trace!(uid = ?user_uid, "user used");
            req.extensions_mut().insert(user.clone());

            let res = inner.call(req).await?;

            Ok(res)
        })
    }
}

// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct UserManagerLayer<Store, User>
where
    Store: crate::store::Store<Object = User, Id = <User as AuthUser>::Id>,
    User: AuthUser,
{
    store: Store,
}

impl<Store, User> UserManagerLayer<Store, User>
where
    Store: crate::store::Store<Object = User, Id = <User as AuthUser>::Id>,
    User: AuthUser,
{
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl<S, Store, User> tower_layer::Layer<S> for UserManagerLayer<Store, User>
where
    Store: crate::store::Store<Object = User, Id = <User as AuthUser>::Id> + Clone,
    User: AuthUser,
{
    type Service = UserManager<S, User, Store>;

    fn layer(&self, inner: S) -> Self::Service {
        UserManager {
            inner,
            store: self.store.clone(),
        }
    }
}
