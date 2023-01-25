use serde;
use anyhow::Error;
use bincode::{serialize, deserialize};

use crate::store::Store;
use crate::store::Continue;

use crate::signal::Signaler;
use crate::signal::SigType;


/// Trait to implement Cacheable data Model
pub trait Model: serde::Serialize +
                 serde::de::DeserializeOwned {

    /// key to identify the data object, this should be unique
    fn key(&self) -> String;

    /// database name, where to store instances of this struct
    fn db() -> &'static str { "default" }

    /// Data Struct serialization
    fn tob(&self) -> Result<Vec<u8>, Error> {
        let encoded: Vec<u8> = serialize(self)?;
        Ok(encoded)
    }

    /// Data Struct deserialization
    fn fromb(data: &[u8]) -> Result<Self, Error> {
        let decoded: Self = deserialize(data)?;
        Ok(decoded)
    }

    /// Persist the struct in the database
    fn store<S: Store>(&self, store: &S)
        -> Result<(), Error> {
        store.push(Self::db(), &self.key(), self.tob()?)
    }

    /// Persist the struct in the database and emit the signal to the signaler
    fn store_sig<S: Store, G: Signaler>(&self, store: &S, sig: &G)
        -> Result<(), Error> {
        self.store(store)
            .and_then(|out| {
                sig.emit(SigType::Update, &self.key())?;
                Ok(out)
            })
    }

    /// Deletes the object from the database
    fn delete<S: Store>(&self, store: &S)
        -> Result<(), Error> {
        store.rm(Self::db(), &self.key())
    }

    /// Deletes the object from the database and emit the signal to the signaler
    fn delete_sig<S: Store, G: Signaler>(&self, store: &S, sig: &G)
        -> Result<(), Error> {
        self.delete(store)
            .and_then(|out| {
                sig.emit(SigType::Delete, &self.key())?;
                Ok(out)
            })
    }

    /// Loads the struct from the database
    fn get<S: Store>(store: &S, key: &str) -> Result<Self, Error> {
        store.pull(Self::db(), key, Self::fromb)
    }

    /// Get all objects with this prefix
    fn all<S: Store>(store: &S, prefix: &str)
        -> Result<Vec<Self>, Error> {
        store.all(Self::db(), prefix, Self::fromb)
    }

    /// Iterate over all objects with this prefix
    fn iter<S, F>(store: &S, prefix: &str, f: F) -> Result<(), Error>
        where S: Store,
              F: Fn(Self) -> Continue {
        store.iter(Self::db(), prefix, move |data| {
            match Self::fromb(data) {
                Ok(obj) => f(obj),
                _ => Continue(true)
            }
        })
    }
}
