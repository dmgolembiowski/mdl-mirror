use anyhow::Error;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::collections::HashMap;
use std::fmt;
use std::sync::mpsc::TryRecvError;


macro_rules! subscribe {
    ($self: expr, $CallBack: ident, $signal: expr, $f: expr) => {{
        let id = *$self.base.id.lock().unwrap();
        *$self.base.id.lock().unwrap() += 1;

        let c = $CallBack { id: id, callback: $f };

        let mut guard = $self.callbacks.lock().unwrap();
        if guard.contains_key($signal) {
            guard.get_mut($signal).map(|v| v.push(c));
        } else {
            guard.insert($signal.to_string(), vec![c]);
        }

        Ok(id)
    }}
}

macro_rules! unsubscribe {
    ($self: expr, $CallBack: ident, $id: expr) => {{
        let mut guard = $self.callbacks.lock().unwrap();
        for (_, ref mut v) in guard.iter_mut() {
            let idx = v.iter().position(|cb: &$CallBack| cb.id() == $id);
            if let Some(i) = idx {
                v.remove(i);
                break;
            }
        }
    }}
}

/// signal -> [cb1, cb2, cb3, ...]
type CBs = Arc<Mutex< HashMap<String, Vec<CallBack>> >>;
type CBsSync = Arc<Mutex< HashMap<String, Vec<CallBackSync>> >>;

// Custom types

#[derive(Clone, Debug)]
pub enum SigType {
    Update,
    Delete,
}

#[derive(Clone, Debug)]
pub struct Signal {
    pub type_: SigType,
    pub name: String,
}

pub struct CallBack {
    pub id: u32,
    pub callback: Box<dyn Fn(Signal) + Send + 'static>,
}

pub struct CallBackSync {
    pub id: u32,
    pub callback: Box<dyn Fn(Signal) + 'static>,
}

#[derive(Clone, Debug)]
pub struct SigBase {
    id: Arc<Mutex<u32>>,
    recv: Arc<Mutex<Receiver<Signal>>>,
    main: Option<Sender<Signal>>,
}

#[derive(Clone, Debug)]
pub struct SignalerAsync {
    base: SigBase,
    callbacks: CBs,
}

#[derive(Clone, Debug)]
pub struct SignalerSync {
    base: SigBase,
    callbacks: CBsSync,
}

// Traits

trait CB {
    fn id(&self) -> u32;
    fn call(&self, sig: Signal);
}

pub trait Signaler {
    fn base<'a>(&'a self) -> &'a SigBase;

    /// emit a signal that trigger all callbacks subscribed to this signal
    fn emit(&self, t: SigType, signal: &str) -> Result<(), Error> {
        if let Some(ref tx) = self.base().main {
            let tx = tx.clone();
            let n = signal.to_string();
            thread::spawn(move || {
                let _ = tx.send(Signal{ type_: t, name: n });
            });
        }
        Ok(())
    }
}

// struct methods

impl SigBase {
    pub fn new() -> SigBase {
        let (tx, rv) = channel::<Signal>();
        let main = Some(tx);
        let recv = Arc::new(Mutex::new(rv));
        let id = Arc::new(Mutex::new(1));
        SigBase{id, recv, main}
    }
}

impl SignalerAsync {
    pub fn new() -> SignalerAsync {
        let callbacks = Arc::new(Mutex::new(HashMap::new()));
        let base = SigBase::new();
        SignalerAsync { base, callbacks }
    }

    pub fn stop(&mut self) {
        self.base.main = None;
    }

    /// subscribe a callback to a signal
    /// This callback will be called with all signals that starts with the
    /// `signal` string, for example, if you subscribe a callback to the signal
    /// "custom-signal", this callback will have the following behaviour:
    ///
    ///   signal                         | f is called
    ///   -------------------------------+-------------
    ///   "custom-signal"                | true
    ///   "custom-signal2"               | true
    ///   "custom-signal-with more text" | true
    ///   "custom"                       | false
    ///   "custom-signa"                 | false
    ///   "other signal"                 | false
    ///
    /// This method returns the callback id that can be used to unsubscribe
    pub fn subscribe(&self, signal: &str, f: Box<dyn Fn(Signal) + Send + 'static>)
        -> Result<u32, Error> {

        subscribe!(self, CallBack, signal, f)
    }

    /// Unsubscribe a callback by id. Use the id returned in the subscribe
    /// method to remove this signal
    pub fn unsubscribe(&self, id: u32) {
        unsubscribe!(self, CallBack, id);
    }

    pub fn clear_signal(&self, signal: &str) {
        let mut guard = self.callbacks.lock().unwrap();
        guard.remove(signal);
    }

    pub fn signal_loop(&self) {
        let cbs = self.callbacks.clone();
        let recv = self.base.recv.clone();
        thread::spawn(move || {
            event_loop(&*recv.lock().unwrap(), cbs);
        });
    }
}

impl SignalerSync {
    pub fn new() -> SignalerSync {
        let callbacks = Arc::new(Mutex::new(HashMap::new()));
        let base = SigBase::new();
        SignalerSync { base, callbacks }
    }

    pub fn stop(&mut self) {
        self.base.main = None;
    }

    /// subscribe a callback to a signal
    /// This callback will be called with all signals that starts with the
    /// `signal` string, for example, if you subscribe a callback to the signal
    /// "custom-signal", this callback will have the following behaviour:
    ///
    ///   signal                         | f is called
    ///   -------------------------------+-------------
    ///   "custom-signal"                | true
    ///   "custom-signal2"               | true
    ///   "custom-signal-with more text" | true
    ///   "custom"                       | false
    ///   "custom-signa"                 | false
    ///   "other signal"                 | false
    ///
    /// This method returns the callback id that can be used to unsubscribe
    pub fn subscribe(&self, signal: &str, f: Box<dyn Fn(Signal) + 'static>)
        -> Result<u32, Error> {

        subscribe!(self, CallBackSync, signal, f)
    }

    /// Unsubscribe a callback by id. Use the id returned in the subscribe
    /// method to remove this signal
    pub fn unsubscribe(&self, id: u32) {
        unsubscribe!(self, CallBackSync, id);
    }

    pub fn clear_signal(&self, signal: &str) {
        let mut guard = self.callbacks.lock().unwrap();
        guard.remove(signal);
    }

    pub fn signal_loop_sync(&self) -> bool {
        let recv = self.base.recv.lock().unwrap();
        match recv.try_recv() {
            Ok(ref signal) => {
                let mut cbs = self.callbacks.lock().unwrap();
                signal_recv(signal, &mut *cbs);
                true
            }
            Err(TryRecvError::Empty) => {
                true
            }
            Err(_) => {
                let mut cbs = self.callbacks.lock().unwrap();
                clear(&mut *cbs);
                false
            }
        }
    }
}

// Trait implementation

impl fmt::Debug for CallBack {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "callback: {}", self.id)
    }
}

impl fmt::Debug for CallBackSync {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "callback-sync: {}", self.id)
    }
}

impl CB for CallBack {
    fn id(&self) -> u32 { self.id }
    fn call(&self, sig: Signal) { (*self.callback)(sig); }
}

impl CB for CallBackSync {
    fn id(&self) -> u32 { self.id }
    fn call(&self, sig: Signal) { (*self.callback)(sig); }
}

impl Signaler for SignalerAsync {
    fn base<'a>(&'a self) -> &'a SigBase { &self.base }
}

impl Signaler for SignalerSync {
    fn base<'a>(&'a self) -> &'a SigBase { &self.base }
}

// static functions

fn event_loop<T: CB>(receiver: &Receiver<Signal>,
                     cbs: Arc<Mutex< HashMap<String, Vec<T>> >>) {
    loop {
        match receiver.recv() {
            Ok(ref signal) => {
                let mut cbs = cbs.lock().unwrap();
                signal_recv(signal, &mut *cbs);
            }
            Err(_) => {
                let mut cbs = cbs.lock().unwrap();
                clear(&mut *cbs);
                break;
            }
        };
    }
}

fn signal_recv<T: CB>(signal: &Signal, cbs: &mut HashMap<String, Vec<T>>) {
    for (ref k, ref v) in cbs.iter() {
        if !&signal.name[..].starts_with(&k[..]) {
            continue;
        }

        for c in v.iter() {
            c.call(signal.clone());
        }
    }
}

fn clear<T: CB>(cbs: &mut HashMap<String, Vec<T>>) {
    for (_, v) in cbs.iter_mut() {
        v.clear();
    }
}

