use mdl::SigType;
use mdl::SignalerAsync;
use mdl::Cache;
use mdl::Model;

use serde::{Deserialize, Serialize};

use std::fs::remove_dir_all;

use std::sync::{Arc, Mutex};
use std::{thread, time};


static DB: &'static str = "/tmp/test.lmdb";


#[derive(Serialize, Deserialize, Debug)]
struct B {
    pub id: u32,
    pub complex: Vec<String>,
}
impl Model for B {
    fn key(&self) -> String {
        format!("b:{}", self.id)
    }
}


#[test]
fn basic_signal_test() {
    let db = &format!("{}-basic", DB);
    let cache = Cache::new(db).unwrap();
    let sig = SignalerAsync::new();
    sig.signal_loop();

    let up_c = Arc::new(Mutex::new(0));
    let rm_c = Arc::new(Mutex::new(0));
    let counter = Arc::new(Mutex::new(0));

    let c1 = up_c.clone();
    let c2 = rm_c.clone();
    let c3 = counter.clone();
    let _id = sig.subscribe("b", Box::new(move |sig| {
        match sig.type_ {
            SigType::Update => *c1.lock().unwrap() += 1,
            SigType::Delete => *c2.lock().unwrap() += 1,
        };

        *c3.lock().unwrap() += 1;
    }));

    let b = B{ id: 1, complex: vec![] };
    let r = b.store_sig(&cache, &sig);
    assert!(r.is_ok());

    let b = B{ id: 2, complex: vec![] };
    let r = b.store_sig(&cache, &sig);
    assert!(r.is_ok());

    let r = b.delete_sig(&cache, &sig);
    assert!(r.is_ok());

    let _ = remove_dir_all(db);

    // waiting for signal to come
    let ten_millis = time::Duration::from_millis(10);
    thread::sleep(ten_millis);

    assert_eq!(*up_c.lock().unwrap(), 2);
    assert_eq!(*rm_c.lock().unwrap(), 1);
    assert_eq!(*counter.lock().unwrap(), 3);
}

