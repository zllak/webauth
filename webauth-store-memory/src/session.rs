use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;
use webauth::{Session, Store, StoreError};

#[derive(Default, Clone, Debug)]
pub struct SessionStore {
    sessions: Arc<Mutex<HashMap<Uuid, Session>>>,
}

impl Store for SessionStore {
    type Object = Session;
    type Id = Uuid;

    fn load(
        &self,
        id: &Self::Id,
    ) -> impl std::future::Future<Output = Result<Option<Self::Object>, StoreError>> + Send {
        let map = self.sessions.lock().expect("poisoned mutex");
        let sess = map.get(id).cloned();
        async move { Ok(sess) }
    }

    fn save(
        &self,
        id: &Self::Id,
        obj: &Self::Object,
    ) -> impl std::future::Future<Output = Result<(), StoreError>> + Send {
        let mut map = self.sessions.lock().expect("poisoned mutex");
        map.insert(*id, obj.clone());
        async move { Ok(()) }
    }

    fn delete(
        &self,
        id: &Self::Id,
    ) -> impl std::future::Future<Output = Result<(), StoreError>> + Send {
        let mut map = self.sessions.lock().expect("poisoned mutex");
        map.remove(id);
        async move { Ok(()) }
    }
}
