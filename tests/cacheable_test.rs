use mdl::Cache;
use mdl::Model;
use mdl::Continue;

use serde::{Deserialize, Serialize};

use std::fs::remove_dir_all;

static DB: &'static str = "/tmp/test.lmdb";

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
fn basic_struct_test() {
    let db = &format!("{}-basic", DB);
    let cache = Cache::new(db).unwrap();

    let a = A{ p1: "hello".to_string(), p2: 42 };
    let r = a.store(&cache);
    assert!(r.is_ok());

    let a1: A = A::get(&cache, "hello:42").unwrap();
    assert_eq!(a1.p1, a.p1);
    assert_eq!(a1.p2, a.p2);
    let _ = remove_dir_all(db);
}

#[test]
fn delete_test() {
    let db = &format!("{}-delete", DB);
    let cache = Cache::new(db).unwrap();

    let a = A{ p1: "hello".to_string(), p2: 42 };
    let r = a.store(&cache);
    assert!(r.is_ok());

    let r = A::get(&cache, "hello:42");
    assert!(r.is_ok());

    let r = a.delete(&cache);
    assert!(r.is_ok());

    let r = A::get(&cache, "hello:42");
    assert!(r.is_err());
    let _ = remove_dir_all(db);
}

#[test]
fn iterate_test() {
    let db = &format!("{}-it", DB);
    let cache = Cache::new(db).unwrap();

    for i in 1..10 {
        let a = A{ p1: "hello".to_string(), p2: i };
        let r = a.store(&cache);
        assert!(r.is_ok());
    }

    //inserting other objects in cache
    for i in 1..10 {
        let b = B{ id: i, complex: vec![] };
        let r = b.store(&cache);
        assert!(r.is_ok());
    }

    //and now more A objects
    for i in 10..20 {
        let a = A{ p1: "hello".to_string(), p2: i };
        let r = a.store(&cache);
        assert!(r.is_ok());
    }

    let r = A::get(&cache, "hello:1");
    assert!(r.is_ok());
    assert_eq!(r.unwrap().p2, 1);

    let r = B::get(&cache, "b:1");
    assert!(r.is_ok());
    assert_eq!(r.unwrap().id, 1);

    // Iterate over all A elements
    let mut v = A::all(&cache, "hello").unwrap();
    v.sort_by_key(|a| a.p2);
    for (i, a) in v.iter().enumerate() {
        assert_eq!(a.p2, (i+1) as u32);
    }

    // Iterate over all B elements
    let mut v = B::all(&cache, "b").unwrap();
    v.sort_by_key(|b| b.id);
    for (i, b) in v.iter().enumerate() {
        assert_eq!(b.id, (i+1) as u32);
    }

    let _ = remove_dir_all(db);
}

#[test]
fn iterate_write_test() {
    let db = &format!("{}-it2", DB);
    let cache = Cache::new(db).unwrap();

    //inserting other objects in cache
    for i in 1..10 {
        let b = B{ id: i, complex: vec![] };
        let r = b.store(&cache);
        assert!(r.is_ok());
    }

    // Iterate over all B elements
    let all = B::all(&cache, "b").unwrap();

    for mut b in all {
        b.complex.push("UPDATED".to_string());
        b.store(&cache).unwrap();
    }

    // Iterate over all B elements
    B::iter(&cache, "b", |b| {
        assert_eq!(b.complex.len(), 1);
        Continue(true)
    }).unwrap();

    let _ = remove_dir_all(db);
}

#[test]
fn thread_test() {
    use std::thread;

    let db = &format!("{}-thread", DB);
    let cache = Cache::new(db).unwrap();

    let b = B{ id: 1, complex: vec![] };
    let _ = b.store(&cache);

    let join_handle: thread::JoinHandle<_> =
    thread::spawn(move || {
        let db = &format!("{}-thread", DB);
        let cache = Cache::new(db).unwrap();
        let mut b = B::get(&cache, "b:1").unwrap();

        assert_eq!(b.complex.len(), 0);
        b.complex.push("modified".to_string());
        let _ = b.store(&cache);
    });

    // waiting for the thread to finish
    join_handle.join().unwrap();
    let b = B::get(&cache, "b:1").unwrap();
    assert_eq!(b.id, 1);
    assert_eq!(b.complex.len(), 1);
    assert_eq!(&b.complex[0][..], "modified");

    let _ = remove_dir_all(db);
}

