use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Error;
use anyhow::anyhow;

pub struct Continue(pub bool);

/// Trait that defines a Store that can be implemented to save Model objects
/// in memory, filesystem or the network
pub trait Store {
    /// Stores the value in the database with the corresponding key
    fn push(&self, db: &'static str, key: &str, value: Vec<u8>)
        -> Result<(), Error>;

    /// Retrieves the value in the database with the corresponding key
    /// Returns an error if the key doesn't exists
    fn pull<F, T>(&self, db: &'static str, key: &str, formatter: F)
        -> Result<T, Error>
        where F: Fn(&[u8]) -> Result<T, Error>;

    /// Iterates over all objects that starts with the prefix and run
    /// the function f. If f returns Continue(false) the iteration stops
    fn iter<F>(&self, db: &'static str, prefix: &str, f: F)
        -> Result<(), Error>
        where F: Fn(&[u8]) -> Continue;

    /// Retrieves all items in the database that starts with the prefix key
    fn all<F, T>(&self, db: &'static str, prefix: &str, formatter: F)
        -> Result<Vec<T>, Error>
        where F: Fn(&[u8]) -> Result<T, Error> {

        let output: Rc<RefCell<Vec<T>>> = Rc::new(RefCell::new(vec![]));
        let out = output.clone();

        self.iter(db, prefix, move |data| {
            if let Ok(obj) = formatter(data) {
                out.borrow_mut().push(obj);
            }
            Continue(true)
        })?;

        Ok(Rc::try_unwrap(output)
            .map_err(|_| anyhow!("error reading from db"))?
            .into_inner())
    }

    /// Remove the corresponding data in the database by key
    fn rm(&self, db: &'static str, key: &str) -> Result<(), Error>;
}

