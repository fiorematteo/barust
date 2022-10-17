use crate::{error::BarustError, statusbar::RightLeft};
use cairo::Context;
pub use cairo::{FontSlant, FontWeight};
use crossbeam_channel::{bounded, Receiver, SendError, Sender};
use log::error;
use psutil::Bytes;
use signal_hook::iterator::Signals;
use std::{
    cell::RefCell,
    collections::{hash_map::Iter, HashMap},
    ffi::c_int,
    fmt::Debug,
    thread,
    time::Duration,
};
use xcb::{
    x::{Atom, InternAtom},
    Connection,
};

pub(crate) enum StatusBarEvent {
    Wake,
    Click(i16, i16),
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }
}

pub fn set_source_rgba(context: &Context, color: Color) {
    context.set_source_rgba(color.r, color.g, color.b, color.a);
}

macro_rules! atoms {
    ( $( $x:ident ),* ) => {
        #[allow(non_snake_case)]
        $(pub(crate) const $x: &'static str = stringify!($x);)*
    }
}

atoms!(
    _NET_SYSTEM_TRAY_S0,
    _NET_SYSTEM_TRAY_ORIENTATION,
    _NET_SYSTEM_TRAY_OPCODE,
    _NET_WM_WINDOW_TYPE,
    _NET_WM_WINDOW_TYPE_DOCK,
    MANAGER
);

pub(crate) struct Atoms<'a> {
    conn: &'a Connection,
    cache: RefCell<HashMap<&'a str, Atom>>,
}

impl<'a> Atoms<'a> {
    pub fn new(conn: &Connection) -> Atoms {
        Atoms {
            conn,
            cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn get(&self, name: &'a str) -> Atom {
        let mut cache = self.cache.borrow_mut();
        if let Some(atom) = cache.get(name) {
            *atom
        } else {
            let atom = self
                .conn
                .wait_for_reply(self.conn.send_request(&InternAtom {
                    only_if_exists: false,
                    name: name.as_bytes(),
                }))
                .unwrap()
                .atom();
            cache.insert(name, atom);
            atom
        }
    }
}

pub fn notify(signals: &[c_int]) -> std::result::Result<Receiver<c_int>, BarustError> {
    let (s, r): _ = bounded(10);
    let mut signals = Signals::new(signals).unwrap();
    thread::spawn(move || {
        for signal in signals.forever() {
            if s.send(signal).is_err() {
                break;
            }
        }
    });
    Ok(r)
}

pub fn bytes_to_closest(value: Bytes) -> String {
    if value == 0 {
        return "0B".to_string();
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut selected_unit: usize = 0;
    let mut value = value as f64;
    while value > 1024.0 {
        if selected_unit == 4 {
            break;
        }
        value /= 1024.0;
        selected_unit += 1;
    }
    format!("{}{}", value as u64, units[selected_unit])
}

pub fn debug_times(name: &str, times: Vec<Duration>) {
    let total = times.iter().sum::<std::time::Duration>();
    println!("{} avg: {:?}", name, total / times.len() as u32);
    println!("{} max: {:?}", name, times.iter().max());
    println!("{} min: {:?}", name, times.iter().min());
}

pub type RawCallback<T, R> = dyn Fn(T) -> R + Send + Sync;
pub type EmptyCallback = dyn Fn() + Send + Sync;

pub struct Callback<T, R> {
    callback: Box<RawCallback<T, R>>,
}

impl<T, R> Callback<T, R> {
    pub fn call(&self, arg: T) -> R {
        (self.callback)(arg)
    }
}

impl<T, R> From<&'static RawCallback<T, R>> for Callback<T, R> {
    fn from(c: &'static RawCallback<T, R>) -> Self {
        Self {
            callback: Box::new(c),
        }
    }
}

impl From<&'static EmptyCallback> for Callback<(), ()> {
    fn from(c: &'static EmptyCallback) -> Self {
        Self {
            callback: Box::new(|()| c()),
        }
    }
}

impl<T, R> std::fmt::Debug for Callback<T, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Callback")
    }
}

pub type WidgetID = (RightLeft, usize);

#[derive(Debug)]
pub struct HookSender {
    sender: Sender<WidgetID>,
    id: WidgetID,
}

impl HookSender {
    pub fn new(sender: Sender<WidgetID>, id: WidgetID) -> Self {
        Self { sender, id }
    }

    pub fn send(&self) -> Result<(), SendError<WidgetID>> {
        self.sender.send(self.id)
    }
}

#[derive(Debug, Default)]
pub struct TimedHooks {
    threads: HashMap<Duration, Sender<HookSender>>,
}

impl TimedHooks {
    pub fn new(threads: HashMap<Duration, Sender<HookSender>>) -> Self {
        Self { threads }
    }

    pub fn subscribe(
        &mut self,
        duration: Duration,
        sender: HookSender,
    ) -> Result<(), SendError<HookSender>> {
        if let Some(interal_sender) = self.threads.get(&duration) {
            interal_sender.send(sender)?;
        } else {
            let (tx, rx) = bounded::<HookSender>(10);
            thread::spawn(move || {
                let mut senders = vec![sender];
                loop {
                    while let Ok(id) = rx.try_recv() {
                        senders.push(id)
                    }
                    for sender in &senders {
                        if sender.send().is_err() {
                            error!("breaking thread loop")
                        }
                        thread::sleep(duration);
                    }
                }
            });
            self.threads.insert(duration, tx);
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.threads.len()
    }

    pub fn capacity(&self) -> usize {
        self.threads.capacity()
    }

    pub fn iter(&self) -> Iter<Duration, Sender<HookSender>> {
        self.threads.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.threads.is_empty()
    }
}
