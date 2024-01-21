use std::future::Future;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("encode")]
    Encode,
    #[error("decode")]
    Decode,
    #[error("backend")]
    Backend,
}

pub type Result<T> = std::result::Result<T, Error>;

/// Trait to load, save and delete arbitrary types.
/// This will be used to manipulate Sessions, and all other types that
/// could be stored in a store.
pub trait Store {
    type Object;
    type Id;

    fn load(&self, id: &Self::Id) -> impl Future<Output = Result<Option<Self::Object>>> + Send;
    fn save(&self, id: &Self::Id, obj: &Self::Object) -> impl Future<Output = Result<()>> + Send;
    fn delete(&self, id: &Self::Id) -> impl Future<Output = Result<()>> + Send;
}
