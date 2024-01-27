use http::{Request, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll},
    time::{Duration, SystemTime},
};
use tower_cookies::{Cookie, CookieManager, Cookies};
use tower_service::Service;
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
    data: Arc<Mutex<HashMap<String, Value>>>,
    modified: Arc<AtomicBool>,
}

// Expires in one week from now
pub const DEFAULT_EXPIRATION: Duration = Duration::from_secs(60 * 60 * 24 * 7);

impl Session {
    // Creates a new `Session`, providing when the session will expire.
    pub fn new(expires_in: Duration) -> Self {
        Self {
            uid: Uuid::new_v4(),
            expires_at: SystemTime::now() + expires_in,
            data: Arc::new(Mutex::new(HashMap::default())),
            // Creating a new session using `new` makes it unsaved/modified
            modified: Arc::new(AtomicBool::new(true)),
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

    // Returns if the session is modified
    pub fn is_modified(&self) -> bool {
        self.modified.load(Ordering::Acquire)
    }

    // Regenerate a new unique identifier for the session.
    // This can be useful to keep a session while changing it's unique identifier.
    // Returns the replaced Uuid.
    pub fn cycle_uid(&mut self) -> Uuid {
        let old_uid = self.uid;

        self.uid = Uuid::new_v4();
        self.modified.store(true, Ordering::Release);
        old_uid
    }

    // Insert a new data in the session.
    pub fn insert(&self, key: &str, value: impl Serialize) -> Result<()> {
        let mut map = self.data.lock().expect("poisoned mutex");
        map.insert(key.to_string(), serde_json::to_value(value)?);
        self.modified.store(true, Ordering::Release);
        Ok(())
    }

    // Get a value from the data stored in the session.
    // Data stored must be JSON-serializable.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let map = self.data.lock().expect("poisoned mutex");
        map.get(key)
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(Into::into)
    }

    // Removes an item from the data stored in the session, returning the value if any.
    pub fn remove<T: DeserializeOwned>(&mut self, key: &str) -> Result<Option<T>> {
        let mut map = self.data.lock().expect("poisoned mutex");
        let res = map
            .remove(key)
            .map(serde_json::from_value)
            .transpose()
            .map_err(Into::<Error>::into)?;
        if res.is_some() {
            self.modified.store(true, Ordering::Release);
        }
        Ok(res)
    }

    // Clear all data stored
    pub fn clear(&mut self) {
        self.data.lock().expect("poisoned mutex").clear();
        self.modified.store(true, Ordering::Release);
    }
}

// ----------------------------------------------------------------------------

/// Manages sessions and implements Service
#[derive(Debug, Clone)]
pub struct SessionManager<Service, Store>
where
    Store: crate::Store<Object = Session, Id = Uuid>,
{
    inner: Service,
    store: Store,
    cookie_name: &'static str,
}

/// Implement the `Service` trait for `SessionManager`
impl<ReqBody, ResBody, S, Store> Service<Request<ReqBody>> for SessionManager<S, Store>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send,
    ReqBody: Send + 'static,
    ResBody: Default + Send,
    Store: crate::Store<Object = Session, Id = Uuid> + Clone + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future =
        Pin<Box<dyn Future<Output = std::result::Result<Self::Response, Self::Error>> + Send>>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        // https://docs.rs/tower/latest/tower/trait.Service.html#be-careful-when-cloning-inner-services
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let store = self.store.clone();
        let cookie_name = self.cookie_name;

        Box::pin(async move {
            // Start by fetching the cookie storing the session uid.
            let Some(cookies) = req.extensions().get::<Cookies>().cloned() else {
                // this should technically not happen as we wrap this SessionManager
                // with CookieManager in the layer.
                return inner.call(req).await;
            };

            let session_uid = cookies.get(cookie_name).and_then(|cookie| {
                cookie
                    .value()
                    .parse::<Uuid>()
                    .map_err(|err| {
                        tracing::warn!(err = %err, uid = cookie.value(), "possible funny business, unable to parse uid");
                    })
                    .ok()
            });
            // Fetch the session from the uid.
            // Here, multiple scenarios are possible:
            // - We don't have a session uid, might mean a new visitor or invalid uid,
            //   so we generate a new session.
            // - We have a session uid but we cannot fetch a proper session from it,
            //   so, again, we generate a new one
            // - Or we fetch a valid session and everything is fine
            let session = match session_uid {
                Some(suid) => {
                    // Load the session from the store
                    match store.load(&suid).await {
                        // Either the session has been deleted or it expired
                        Ok(None) => Session::new(DEFAULT_EXPIRATION),
                        Ok(Some(session)) => session,
                        Err(err) => {
                            tracing::error!(err = %err, "failed to load session");

                            let mut res = Response::default();
                            *res.status_mut() = http::StatusCode::INTERNAL_SERVER_ERROR;
                            return Ok(res);
                        }
                    }
                }
                None => Session::new(DEFAULT_EXPIRATION),
            };

            tracing::trace!(uid = %session.uid(), "session used");
            req.extensions_mut().insert(session.clone());

            let res = inner.call(req).await?;

            // Save the session if modified
            if session.is_modified() {
                if let Err(err) = store.save(session.uid(), &session).await {
                    tracing::error!(err = %err, "failed to save session");

                    // TODO: return 500
                    let mut res = Response::default();
                    *res.status_mut() = http::StatusCode::INTERNAL_SERVER_ERROR;
                    return Ok(res);
                }

                // Add the cookie to the jar
                cookies.add(Cookie::new(cookie_name, session.uid().to_string()));
            }

            Ok(res)
        })
    }
}

// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SessionManagerLayer<S>
where
    S: crate::Store<Object = Session, Id = Uuid>,
{
    store: S,
    cookie_name: &'static str,
}

impl<Store> SessionManagerLayer<Store>
where
    Store: crate::Store<Object = Session, Id = Uuid>,
{
    pub fn new(store: Store, cookie_name: &'static str) -> Self {
        Self { store, cookie_name }
    }
}

impl<S, Store> tower_layer::Layer<S> for SessionManagerLayer<Store>
where
    Store: crate::Store<Object = Session, Id = Uuid> + Clone,
{
    type Service = CookieManager<SessionManager<S, Store>>;

    fn layer(&self, inner: S) -> Self::Service {
        let manager = SessionManager {
            inner,
            store: self.store.clone(),
            cookie_name: self.cookie_name,
        };

        CookieManager::new(manager)
    }
}
// ----------------------------------------------------------------------------

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

        assert_eq!(1, session.data.lock().expect("poisoned").len());
        session.insert("u64", 42u64)?;
        assert_eq!(2, session.data.lock().expect("poisoned").len());

        // Remove a key
        assert_eq!(Some(42u64), session.remove("u64")?);
        assert_eq!(None, session.remove::<()>("unknown")?);
        assert_eq!(1, session.data.lock().expect("poisoned").len());

        // Clear the store
        session.clear();
        assert_eq!(0, session.data.lock().expect("poisoned").len());

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

    #[test]
    fn is_modified() -> Result<()> {
        let mut session = Session::new(DEFAULT_EXPIRATION);
        assert!(session.is_modified());

        session.modified.store(false, Ordering::Release);

        session.get::<usize>("sdf")?;
        assert!(!session.is_modified());

        session.insert("user_uid", 42)?;
        assert!(session.is_modified());
        session.modified.store(false, Ordering::Release);

        session.remove::<usize>("user_uid")?;
        assert!(session.is_modified());
        session.modified.store(false, Ordering::Release);

        session.remove::<()>("unknown")?;
        assert!(!session.is_modified());

        session.clear();
        assert!(session.is_modified());

        Ok(())
    }
}
