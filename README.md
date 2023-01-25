Disclaimer:
This repository is a readonly mirror of the source at https://gitlab.gnome.org/danigm/mdl
by Daniel Garcia Moreno and friends of the Gnome project

App state library with cache

This crate provides functionality to store data and persists to filesystem
automatically. The main goal is to have a single object to query for app
state and to be able to modify this state.

It also provides a simple signaler to be able to subscribe to update /delete
signals and perform custom operations on cache model modification.

To store the information we use a key-value storage so each model should
provide a unique key that identify it. Use NoSQL schema techniques to add
relations between models using the key and query easily.

The basic `Cache` object uses LMDB as storage so you can access to the same
cache from different threads or process.

# Basic Usage

The simpler way to use is implementing the `Model` trait for your struct,
so you can `get`, `store` and `delete`.

```rust
use mdl::Cache;
use mdl::Model;
use mdl::Continue;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct A {
    pub p1: String,
    pub p2: u32,
}
impl Model for A {
    fn key(&self) -> String {
        format!("{}:{}", self.p1, self.p2)
    }
}

fn main() {
    // initializing the cache. This str will be the fs persistence path
    let db = "/tmp/mydb.lmdb";
    let cache = Cache::new(db).unwrap();

    // create a new *object* and storing in the cache
    let a = A{ p1: "hello".to_string(), p2: 42 };
    let r = a.store(&cache);
    assert!(r.is_ok());

    // querying the cache by key and getting a new *instance*
    let a1: A = A::get(&cache, "hello:42").unwrap();
    assert_eq!(a1.p1, a.p1);
    assert_eq!(a1.p2, a.p2);
}
```

# Signals

To allow easy notifications of changes in the cache, this crate
provides a signal system and the `Model` trait provides `store_sig`
and `delete_sig` that store or delete and then emit the corresponding
signal.

There's two signalers implemented, one that can be `Send` between
threads and another one that should be in the same thread all the time
this allow us to register callbacks for signals and that callbacks
should implement `Send` for the `SignalerAsync`.

## Example

```rust
use mdl::SigType;
use mdl::SignalerAsync;
use mdl::Cache;
use mdl::Model;

use serde::{Deserialize, Serialize};

use std::sync::{Arc, Mutex};
use std::{thread, time};

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

fn main() {
    let db = "/tmp/test.lmdb";
    let cache = Cache::new(db).unwrap();
    // using the async signaler that run in other thread
    let sig = SignalerAsync::new();
    // starting the signaler loop, this can be stoped
    // calling sig.stop() or when the signaler drops
    sig.signal_loop();

    let up_c = Arc::new(Mutex::new(0));
    let rm_c = Arc::new(Mutex::new(0));
    let counter = Arc::new(Mutex::new(0));

    let c1 = up_c.clone();
    let c2 = rm_c.clone();
    let c3 = counter.clone();

    // Subscribing to the "b" signal, that's emited always
    // that an object which key starting with "b" is modified.
    // We're using the SignalerAsync so this callback will
    // be called in a different thread, for that reason we're
    // pasing Arc<Mutex<T>> to be able to modify the counters
    let _id = sig.subscribe("b", Box::new(move |sig| {
        match sig.type_ {
            SigType::Update => *c1.lock().unwrap() += 1,
            SigType::Delete => *c2.lock().unwrap() += 1,
        };

        *c3.lock().unwrap() += 1;
    }));

    let b = B{ id: 1, complex: vec![] };
    // we use the store_sig instead the store to emit the
    // corresponding signal, if we use the store, the callback
    // wont be called.
    let r = b.store_sig(&cache, &sig);
    assert!(r.is_ok());

    let b = B{ id: 2, complex: vec![] };
    let r = b.store_sig(&cache, &sig);
    assert!(r.is_ok());

    let r = b.delete_sig(&cache, &sig);
    assert!(r.is_ok());

    // waiting for signal to come
    let ten_millis = time::Duration::from_millis(10);
    thread::sleep(ten_millis);

    assert_eq!(*up_c.lock().unwrap(), 2);
    assert_eq!(*rm_c.lock().unwrap(), 1);
    assert_eq!(*counter.lock().unwrap(), 3);
}
```

You can use the `Signaler` without a `Model`, it's possible to emit custom
signals and subscribe to that signals, for example:

```rust
use mdl::SigType;
use mdl::Signaler;
use mdl::SignalerAsync;
use std::{thread, time};

use serde::{Deserialize, Serialize};

fn main() {
    let sig = SignalerAsync::new();
    sig.signal_loop();

    let _id = sig.subscribe("my signal", Box::new(move |sig| {
        println!("my signal is called");
    }));

    let _ = sig.emit(SigType::Update, "my signal");

    // waiting for signal to come
    let ten_millis = time::Duration::from_millis(10);
    thread::sleep(ten_millis);
}
```
