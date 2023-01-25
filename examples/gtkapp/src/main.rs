use gtk::prelude::*;

use mdl::Cache;
use mdl::Model;
use mdl::Signal;
use mdl::SignalerSync;
use mdl::SigType;
use mdl::Continue;

use serde::{Deserialize, Serialize};

use gtk::{Window, WindowType};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Serialize, Deserialize, Debug)]
struct TodoApp {
    number: u32,
    text: String,
}

impl Model for TodoApp {
    fn key(&self) -> String { "app".to_string() }
}

#[derive(Serialize, Deserialize, Debug)]
struct TodoRow {
    index: u32,
    text: String,
    done: bool,
}

impl Model for TodoRow {
    fn key(&self) -> String {
        format!("todo:{}", self.index)
    }
}

static DB: &'static str = "todo.lmdb";
static TEXTS: [&'static str; 4] = [
    "This is a TODO app",
    "Written in Rust",
    "Using mdl lib for the data model",
    "Caching data using lmdb",
];

#[derive(Clone)]
struct St {
    pub cache: Arc<Mutex<Cache>>,
    pub sig: SignalerSync,
}

impl St {
    pub fn new() -> St {
        let cache = Arc::new(Mutex::new(Cache::new(DB).unwrap()));
        let sig = SignalerSync::new();
        St { cache, sig }
    }
    pub fn c(&self) -> MutexGuard<Cache> {
        self.cache.lock().unwrap()
    }
}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let st = St::new();

    // populating model
    let t;
    {
        let c = &*st.c();
        t = match TodoApp::get(c, "app") {
            Ok(t) => t,
            Err(_) => {
                let t = TodoApp { number: 1, text: "Initial App".to_string() };
                let _ = t.store(c);
                t
            }
        };
    }

    let window = Window::new(WindowType::Toplevel);
    window.set_title("TODO list");
    window.set_default_size(400, 600);

    // Building the interface
    //
    // +----------------------+
    // | Text entry           |
    // +----------------------+
    // | todo entry           |
    // | todo entry           |
    // | ...                  |
    // +----------------------+
    let bx = gtk::Box::new(gtk::Orientation::Vertical, 6);

    let entry = gtk::Entry::new();
    bx.pack_start(&entry, false, false, 0);

    let scroll = gtk::ScrolledWindow::new(None, None);
    let listbox = gtk::ListBox::new();
    scroll.add(&listbox);
    bx.pack_start(&scroll, true, true, 0);

    let label = gtk::Label::new(&t.text[..]);
    bx.add(&label);

    window.add(&bx);
    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    // populating all todos
    {
        let list = listbox.clone();
        let st1 = st.clone();
        let _ = TodoRow::iter(&*st.c(), "todo", |row| {
            add_row(&list, &row, &st1);
            Continue(true)
        });
    }

    // Connecting all events
    let st1 = st.clone();
    entry.connect_activate(move |e| {
        e.get_text()
         .map(|t| {
            let c = &*st1.c();
            let n = TodoApp::get(c, "app").map(|a| a.number).unwrap_or_default();
            let r = TodoRow{ index: n, text: t, done: false };
            let _ = r.store_sig(c, &st1.sig);
            println!("STORED");
         });
        e.set_text("");
    });

    let l = listbox.clone();
    let st1 = st.clone();
    st.sig.subscribe("todo", Box::new(
        move |Signal{type_: t, name: n}| {
            let c = &*st1.c();
            let s = &st1.sig;
            match t {
                SigType::Update => {
                    // Add row;
                    println!("update: {}", n);
                    let _ = TodoRow::get(c, &n[..])
                        .map(|ref t| add_row(&l, &t, &st1));
                    let app = TodoApp::get(c, "app");
                    if let Ok(mut a) = app {
                        a.number += 1;
                        let _ = a.store_sig(c, s);
                    }
                }
                SigType::Delete => {
                    // Remove row;
                    println!("delete: {}", n);
                }
            };
        }
    )).unwrap();

    let st1 = st.clone();
    st.sig.subscribe("app", Box::new(
        move |_| {
            let c = &*st1.c();
            TodoApp::get(c, "app")
                .map(|ref mut a| {
                    let msg = format!("{}: {}", a.text, a.number);
                    label.set_text(&msg[..]);
                }).unwrap();
        }
    )).unwrap();

    // signal loop
    let st1 = st.clone();
    gtk::timeout_add(50, move || {
        gtk::Continue(st1.sig.signal_loop_sync())
    });

    let index = Arc::new(Mutex::new(0));
    gtk::timeout_add(1000, move || {
        let c = &*st.c();
        let _ = TodoApp::get(c, "app")
            .map(|ref mut a| {
                let idx = *index.lock().unwrap() % TEXTS.len();
                *index.lock().unwrap() += 1;
                a.text = TEXTS[idx].to_string();
                a.store_sig(c, &st.sig)
            });
        gtk::Continue(true)
    });

    gtk::main();
}

fn add_row(list: &gtk::ListBox, row: &TodoRow, st: &St) {
    let w = gtk::Box::new(gtk::Orientation::Horizontal, 3);
    let l = gtk::Label::new(&row.text[..]);
    let done = gtk::Button::new();
    let rm = gtk::Button::new();

    l.set_xalign(0.0);
    l.set_yalign(0.5);
    done.set_label("done");
    rm.set_label("rm");

    w.pack_start(&l, true, true, 0);
    w.pack_start(&done, false, false, 0);
    w.pack_start(&rm, false, false, 0);

    if row.done {
        l.set_markup(&format!("<s>{}</s>", row.text));
    }

    let l = l.clone();
    let key = row.key();
    let st1 = st.clone();
    done.connect_clicked(move |_| {
        let c = &*st1.c();
        let _ = TodoRow::get(c, &key[..])
            .map(|mut r| {
                r.done = true;
                if let Ok(_) = r.store(c) {
                    l.set_markup(&format!("<s>{}</s>", r.text));
                }
            });
    });

    let b = w.clone();
    let key = row.key();
    let st = st.clone();
    rm.connect_clicked(move |_| {
        let c = &*st.c();
        let _ = TodoRow::get(c, &key[..])
            .and_then(|r| {
                r.delete_sig(c, &st.sig)
            })
            .map(|_| {
                b.get_parent().map(|p| p.destroy());
            });
    });

    w.show_all();
    list.add(&w);
}

