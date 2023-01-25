use anyhow::Error;
use anyhow::anyhow;

use lmdb::Transaction;
use lmdb::Cursor;
use lmdb::Environment;
use lmdb::Database;
use lmdb::DatabaseFlags;
use lmdb::WriteFlags;
use lmdb::RwCursor;
use lmdb::RoCursor;

use std::path::Path;
use std::fs::create_dir_all;
use std::collections::HashMap;

use std::cell::RefCell;

use crate::store::Store;
use crate::store::Continue;

/// LMDB cache. This struct implements the Store trait so it can be used
/// to cache Model structs
pub struct Cache {
    /// LMDB environment
    pub env: Environment,
    /// database path in the filesystem
    pub path: String,
    /// List of LMDB databases
    dbs: RefCell<HashMap<&'static str, Database>>,
}

impl Cache {
    pub fn new(path: &str) -> Result<Cache, Error> {
        let envpath = Path::new(path);
        if !envpath.exists() {
            let _ = create_dir_all(&envpath);
        }

        let env = Environment::new()
                    .set_max_dbs(1024)
                    .set_map_size(256 * 1024 * 1024) /* 256 MB */
                    .open(envpath)?;

        Ok(Cache {
            env: env,
            path: path.to_string(),
            dbs: RefCell::new(HashMap::new()),
        })
    }

    pub fn db(&self, name: &'static str) -> Result<Database, Error> {
        // if the db is created, we return the db stored in cache
        {
            let dbs = self.dbs.borrow();
            if dbs.contains_key(name) {
                return Ok(dbs[name].clone());
            }
        }

        // if the db doesn't exists, we create that db and store for the future
        let db = self.env
            .create_db(Some(name), DatabaseFlags::default())
            .or(Err(anyhow!("error opening the db {}", name)))?;

        self.dbs.borrow_mut().insert(name, db.clone());
        Ok(db)
    }

    pub fn rw<F, T>(&self, db: &'static str, op: F) -> Result<T, Error>
        where F: Fn(RwCursor) -> Result<T, Error> {

        let db = self.db(db)?;
        let mut txn = self.env.begin_rw_txn()?;
        let output;
        {
            let cursor = txn.open_rw_cursor(db)?;
            output = op(cursor);
        }
        txn.commit()?;

        output
    }

    pub fn ro<F, T>(&self, db: &'static str, op: F) -> Result<T, Error>
        where F: Fn(RoCursor) -> Result<T, Error> {

        let db = self.db(db)?;
        let txn = self.env.begin_ro_txn()?;
        let output;
        {
            let cursor = txn.open_ro_cursor(db)?;
            output = op(cursor);
        }
        txn.commit()?;

        output
    }
}

impl Store for Cache {
    fn push(&self, db: &'static str, key: &str, value: Vec<u8>)
        -> Result<(), Error> {
        self.rw(db, move |mut cursor| {
            cursor.put(&key.as_bytes(), &value, WriteFlags::empty())?;
            Ok(())
        })
    }

    fn pull<F, T>(&self, db: &'static str, key: &str, formatter: F)
        -> Result<T, Error>
        where F: Fn(&[u8]) -> Result<T, Error> {
        self.ro(db, move |cursor| {
            let k = Some(key.as_ref());
            let (_rkey, rdata) = cursor.get(k, None, 15)?;
            formatter(rdata)
        })
    }

    fn iter<F>(&self, db: &'static str, prefix: &str, f: F)
        -> Result<(), Error>
        where F: Fn(&[u8]) -> Continue {
        let l = prefix.len();

        self.ro(db, move |mut cursor| {
            let k = Some(prefix.as_ref());
            cursor.get(k, None, 17)?;

            let iter = cursor.iter_from(k.unwrap())
                .filter(|(k, _v)| { k.len() >= l && &k[0..l] == prefix[0..l].as_bytes() });

            for (_, v) in iter {
                if let Continue(false) = f(v) {
                    break;
                }
            };

            Ok(())
        })?;

        Ok(())
    }

    fn rm(&self, db: &'static str, key: &str) -> Result<(), Error> {
        self.rw(db, move |mut cursor| {
            cursor.get(Some(key.as_ref()), None, 15)?;
            cursor.del(WriteFlags::empty())?;
            Ok(())
        })
    }
}

