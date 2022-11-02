use crate::{error::BarustError, statusbar::RightLeft};
use cairo::Context;
pub use cairo::{FontSlant, FontWeight};
use crossbeam_channel::{bounded, Receiver, SendError, Sender};
use log::error;
use psutil::Bytes;
use signal_hook::iterator::Signals;
use std::{
    ffi::c_int,
    fmt::Debug,
    thread,
    time::{Duration, Instant},
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
    ( $struct_name:ident, $( $x:ident ),* ) => {
        #[allow(non_snake_case)]
        #[derive(Debug, Clone, Copy)]
        pub struct $struct_name{
            $(pub $x: Atom,)*
        }

        impl $struct_name {
            pub fn new(connection: &Connection) -> Result<Self, xcb::Error>{
                Ok(Self {
                    $($x: Self::intern(connection, stringify!($x))?,)*
                })
            }
            fn intern(connection: &Connection, name: &str) -> Result<Atom, xcb::Error> {
                Ok(connection
                    .wait_for_reply(connection.send_request(&InternAtom {
                        only_if_exists: false,
                        name: name.as_bytes(),
                    }))
                    .unwrap()
                    .atom())
            }
        }
    }
}

atoms!(
    Atoms,
    UTF8_STRING,
    _NET_ACTIVE_WINDOW,
    _NET_CURRENT_DESKTOP,
    _NET_DESKTOP_NAMES,
    _NET_SYSTEM_TRAY_OPCODE,
    _NET_SYSTEM_TRAY_ORIENTATION,
    _NET_SYSTEM_TRAY_S0,
    _NET_WM_NAME,
    _NET_WM_WINDOW_TYPE,
    _NET_WM_WINDOW_TYPE_DOCK,
    MANAGER
);

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

#[derive(Debug)]
pub struct TimedHooks {
    thread: Sender<(Duration, HookSender)>,
}

impl Default for TimedHooks {
    fn default() -> Self {
        let (thread, rx) = bounded::<(Duration, HookSender)>(10);
        let mut senders = vec![];
        thread::spawn(move || loop {
            while let Ok(id) = rx.try_recv() {
                senders.push((Instant::now(), id.0, id.1));
            }
            for (time, duration, sender) in &mut senders {
                if time.elapsed() > *duration {
                    *time = Instant::now();
                    if sender.send().is_err() {
                        error!("breaking thread loop")
                    }
                }
            }

            let smallest_time = senders
                .iter()
                .map(|(t, d, _)| (d.saturating_sub(t.elapsed())))
                .min()
                .unwrap_or_else(|| Duration::from_secs(1));
            thread::sleep(smallest_time);
        });
        Self { thread }
    }
}

impl TimedHooks {
    pub fn subscribe(
        &mut self,
        duration: Duration,
        sender: HookSender,
    ) -> Result<(), SendError<(Duration, HookSender)>> {
        self.thread.send((duration, sender))?;
        Ok(())
    }
}
