use cairo::Context;
pub use cairo::{FontSlant, FontWeight};
use std::{cell::RefCell, collections::HashMap, fmt::Debug, time::Duration};
use xcb::{
    x::{Atom, InternAtom},
    Connection,
};

pub(crate) enum BarustEvent {
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

pub fn debug_times(name: &str, times: Vec<Duration>) {
    let total = times.iter().sum::<std::time::Duration>();
    println!("{} avg: {:?}", name, total / times.len() as u32);
    println!("{} max: {:?}", name, times.iter().max());
    println!("{} min: {:?}", name, times.iter().min());
}

pub type Callback<T> = dyn Fn() -> T + Send + Sync;

pub type SelfCallback<T> = dyn Fn(&mut T) + Send + Sync;

pub enum OptionCallback<'a, T> {
    Some(&'a SelfCallback<T>),
    None,
}

impl<'a, T> From<Option<&'a SelfCallback<T>>> for OptionCallback<'a, T> {
    fn from(cb: Option<&'a SelfCallback<T>>) -> Self {
        match cb {
            Some(cb) => Self::Some(cb),
            None => Self::None,
        }
    }
}

impl<T> std::fmt::Debug for OptionCallback<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Some(_) => "Some callback",
                Self::None => "None",
            }
        )
    }
}
