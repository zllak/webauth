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

/// Trait to load, save and delete arbitrary types.
/// This will be used to manipulate Sessions, and all other types that
/// could be stored in a store.
pub trait Store {
    /// The type of the resource itself
    type Object;
    /// The unique identifier of the resource
    type Id;

    /// Load the resource `Object` using the `Id`.
    /// Method should be idempotent, and return Ok(None) if
    /// the given `Id` does not resolve to a valid resource
    /// (an expired session should return Ok(None) for example).
    fn load(
        &self,
        id: &Self::Id,
    ) -> impl Future<Output = Result<Option<Self::Object>, Error>> + Send;
    /// Commit the resource `Object` to the underlying store.
    /// This method should behave like an upsert.
    fn save(
        &self,
        id: &Self::Id,
        obj: &Self::Object,
    ) -> impl Future<Output = Result<(), Error>> + Send;
    /// Deletes a resource `Object` by its `Id`.
    /// Method should be idempotent and return Ok(()) if the
    /// resource has already been deleted.
    fn delete(&self, id: &Self::Id) -> impl Future<Output = Result<(), Error>> + Send;
}
