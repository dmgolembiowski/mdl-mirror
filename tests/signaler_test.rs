use mdl::Signaler;
use mdl::SignalerAsync;
use mdl::SigType;

use std::sync::{Arc, Mutex};
use std::{thread, time};

#[test]
fn one_signal_test() {
    let sig = SignalerAsync::new();
    sig.signal_loop();
    let counter = Arc::new(Mutex::new(0));

    // one thread for receive signals
    let sig1 = sig.clone();
    let c1 = counter.clone();
    let t1: thread::JoinHandle<_> =
    thread::spawn(move || {
        let _ = sig1.subscribe("signal", Box::new(move |_sig| {
            *c1.lock().unwrap() += 1;
        }));
    });

    // waiting for threads to finish
    t1.join().unwrap();

    // one thread for emit signals
    let sig2 = sig.clone();
    let t2: thread::JoinHandle<_> =
    thread::spawn(move || {
        sig2.emit(SigType::Update, "signal").unwrap();
        sig2.emit(SigType::Update, "signal:2").unwrap();
        sig2.emit(SigType::Update, "signal:2:3").unwrap();
    });

    // waiting for threads to finish
    t2.join().unwrap();

    let ten_millis = time::Duration::from_millis(10);
    thread::sleep(ten_millis);

    assert_eq!(*counter.lock().unwrap(), 3);
}

#[test]
fn two_signal_test() {
    let sig = SignalerAsync::new();
    sig.signal_loop();
    let counter = Arc::new(Mutex::new(0));
    let counter2 = Arc::new(Mutex::new(0));

    // one thread for receive signals
    let sig1 = sig.clone();
    let c1 = counter.clone();
    let c2 = counter2.clone();
    let t1: thread::JoinHandle<_> =
    thread::spawn(move || {
        let _ = sig1.subscribe("signal", Box::new(move |_sig| {
            *c1.lock().unwrap() += 1;
        }));

        let _ = sig1.subscribe("others", Box::new(move |_sig| {
            *c2.lock().unwrap() += 1;
        }));
    });

    // waiting for threads to finish
    t1.join().unwrap();

    // one thread for emit signals
    let sig2 = sig.clone();
    let t2: thread::JoinHandle<_> =
    thread::spawn(move || {
        sig2.emit(SigType::Update, "signal").unwrap();
        sig2.emit(SigType::Update, "others:2:3").unwrap();
        sig2.emit(SigType::Update, "signal:2").unwrap();
        sig2.emit(SigType::Update, "signal:2:3").unwrap();
    });

    // waiting for threads to finish
    t2.join().unwrap();

    let ten_millis = time::Duration::from_millis(10);
    thread::sleep(ten_millis);

    assert_eq!(*counter.lock().unwrap(), 3);
    assert_eq!(*counter2.lock().unwrap(), 1);

}

#[test]
fn unsubscribe_test() {
    let sig = SignalerAsync::new();
    sig.signal_loop();
    let counter = Arc::new(Mutex::new(0));

    // one thread for receive signals
    let sig1 = sig.clone();
    let c1 = counter.clone();
    let t1: thread::JoinHandle<_> =
    thread::spawn(move || {
        let sig2 = sig1.clone();
        let _ = sig1.subscribe("unsub", Box::new(move |_sig| {
            *c1.lock().unwrap() += 1;
            sig2.unsubscribe(1);
        }));
    });

    // waiting for threads to finish
    t1.join().unwrap();

    // one thread for emit signals
    let sig2 = sig.clone();
    let t2: thread::JoinHandle<_> =
    thread::spawn(move || {
        sig2.emit(SigType::Update, "unsub").unwrap();
        sig2.emit(SigType::Update, "unsub:2").unwrap();
        sig2.emit(SigType::Update, "unsub:2:3").unwrap();
    });

    // waiting for threads to finish
    t2.join().unwrap();

    let ten_millis = time::Duration::from_millis(10);
    thread::sleep(ten_millis);

    assert_eq!(*counter.lock().unwrap(), 1);
}
