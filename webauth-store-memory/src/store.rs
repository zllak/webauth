use std::{
    any::TypeId,
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex},
    time::SystemTime,
};
use webauth::session::Session;
use webauth::store::{Error, Identifiable, Store as StoreTrait};

#[derive(Default, Clone)]
pub struct Store<Object>
where
    Object: Identifiable,
    <Object as Identifiable>::Uid: Hash + Eq + Copy,
{
    objects: Arc<Mutex<HashMap<<Object as Identifiable>::Uid, Object>>>,
}

impl<Object> Store<Object>
where
    Object: Identifiable,
    <Object as Identifiable>::Uid: Hash + Eq + Copy,
{
    pub fn new() -> Self {
        Self {
            objects: Default::default(),
        }
    }
}

impl<Object> StoreTrait for Store<Object>
where
    Object: Identifiable + Clone + Send + 'static,
    <Object as Identifiable>::Uid: Hash + Eq + Copy,
{
    type Object = Object;

    fn load(
        &self,
        id: &<Self::Object as Identifiable>::Uid,
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
        obj: &Self::Object,
    ) -> impl std::future::Future<Output = Result<(), Error>> + Send {
        let mut map = self.objects.lock().expect("poisoned mutex");
        map.insert(obj.uid(), obj.clone());
        async move { Ok(()) }
    }

    fn delete(
        &self,
        id: &<Self::Object as Identifiable>::Uid,
    ) -> impl std::future::Future<Output = Result<(), Error>> + Send {
        let mut map = self.objects.lock().expect("poisoned mutex");
        map.remove(id);
        async move { Ok(()) }
    }
}
