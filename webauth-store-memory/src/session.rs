use std::{
    any::TypeId,
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex},
    time::SystemTime,
};
use webauth::session::Session;
use webauth::store::{Error, Store as StoreTrait};

#[derive(Default, Clone, Debug)]
pub struct Store<Uid, Object> {
    objects: Arc<Mutex<HashMap<Uid, Object>>>,
}

impl<Uid, Object> Store<Uid, Object> {
    pub fn new() -> Self {
        Self {
            objects: Default::default(),
        }
    }
}

impl<Uid, Object> StoreTrait for Store<Uid, Object>
where
    Uid: Hash + Eq + Copy,
    Object: Clone + Send + 'static,
{
    type Object = Object;
    type Id = Uid;

    fn load(
        &self,
        id: &Self::Id,
    ) -> impl std::future::Future<Output = Result<Option<Self::Object>, Error>> + Send {
        let map = self.objects.lock().expect("poisoned mutex");
        let mut obj = map.get(id);
        if TypeId::of::<Object>() == TypeId::of::<Session>() {
            // Specific case for sessions which can expire, so we must check
            // the expiration. This is a bit ugly but we don't have a ton of solutions
            // to runtime cast from generic type.
            if let Some(sess) = obj {
                let sess: &Session = unsafe { std::mem::transmute::<&Object, &Session>(sess) };
                if sess.expires_at() < &SystemTime::now() {
                    // Session is expired
                    obj = None;
                }
            }
        }
        let obj = obj.cloned();
        async move { Ok(obj) }
    }

    fn save(
        &self,
        id: &Self::Id,
        obj: &Self::Object,
    ) -> impl std::future::Future<Output = Result<(), Error>> + Send {
        let mut map = self.objects.lock().expect("poisoned mutex");
        map.insert(*id, obj.clone());
        async move { Ok(()) }
    }

    fn delete(&self, id: &Self::Id) -> impl std::future::Future<Output = Result<(), Error>> + Send {
        let mut map = self.objects.lock().expect("poisoned mutex");
        map.remove(id);
        async move { Ok(()) }
    }
}
