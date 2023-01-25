use anyhow::Error;
use anyhow::anyhow;

use std::collections::BTreeMap;
use std::ops::Bound::{Included, Unbounded};

use std::sync::{Arc, RwLock};

use crate::store::Store;
use crate::store::Continue;

/// BTreeMap cache. This struct implements the Store trait so it can be used
/// to cache Model structs
/// A BTreeMap is used to store the data in memory. This struct implements clone
/// so it can be shared between threads safely creating a clone
#[derive(Clone)]
pub struct Cache {
    db: Arc<RwLock<BTreeMap<String, Vec<u8>> >>,
}

impl Cache {
    pub fn new() -> Result<Cache, Error> {
        Ok(Cache {
            db: Arc::new(RwLock::new(BTreeMap::new())),
        })
    }
}

impl Store for Cache {
    fn push(&self, db: &'static str, key: &str, value: Vec<u8>)
        -> Result<(), Error> {
        let newk = format!("{}:{}", db, key);
        match self.db.write() {
            Ok(ref mut map) => {
                map.insert(newk, value);
                Ok(())
            },
            Err(_err) => Err(anyhow!("DB ERROR")),
        }
    }

    fn pull<F, T>(&self, db: &'static str, key: &str, formatter: F)
        -> Result<T, Error>
        where F: Fn(&[u8]) -> Result<T, Error> {

        let newk = format!("{}:{}", db, key);
        match self.db.read() {
            Ok(map) => {
                let rdata = map.get(&newk).ok_or(anyhow!("Not found, pull {}", newk))?;
                formatter(rdata)
            },
            Err(_err) => Err(anyhow!("DB ERROR")),
        }
    }

    fn iter<F>(&self, db: &'static str, prefix: &str, f: F)
        -> Result<(), Error>
        where F: Fn(&[u8]) -> Continue {
        let newk = format!("{}:{}", db, prefix);
        let l = newk.len();

        match self.db.read() {
            Ok(map) => {
                let range = map.range::<String, _>((Included(&newk), Unbounded))
                    .filter(|(k, _v)| { k.len() >= l && &k[..l] == &newk });
                for (_, v) in range {
                    if let Continue(false) = f(v) {
                        break;
                    }
                };

                Ok(())
            },
            Err(_err) => Err(anyhow!("DB ERROR")),
        }
    }

    fn rm(&self, db: &'static str, key: &str) -> Result<(), Error> {
        let newk = format!("{}:{}", db, key);
        match self.db.write() {
            Ok(ref mut map) => {
                map.remove(&newk).ok_or(anyhow!("Not found, rm {}", newk))?;
                Ok(())
            },
            Err(_err) => Err(anyhow!("DB ERROR")),
        }
    }
}
