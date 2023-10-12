#![allow(non_snake_case)]

use std::ops::Deref;
use xcb::{atoms_struct, Connection};

static mut ATOMS: Option<InnerAtoms> = None;

atoms_struct!(
#[derive(Copy, Clone, Debug)]
     pub struct InnerAtoms {
        pub UTF8_STRING => b"UTF8_STRING",
        pub _NET_ACTIVE_WINDOW => b"_NET_ACTIVE_WINDOW",
        pub _NET_CURRENT_DESKTOP => b"_NET_CURRENT_DESKTOP",
        pub _NET_DESKTOP_NAMES => b"_NET_DESKTOP_NAMES",
        pub _NET_SYSTEM_TRAY_OPCODE => b"_NET_SYSTEM_TRAY_OPCODE",
        pub _NET_SYSTEM_TRAY_ORIENTATION => b"_NET_SYSTEM_TRAY_ORIENTATION",
        pub _NET_SYSTEM_TRAY_S0 => b"_NET_SYSTEM_TRAY_S0",
        pub _NET_WM_NAME => b"_NET_WM_NAME",
        pub _NET_WM_WINDOW_TYPE => b"_NET_WM_WINDOW_TYPE",
        pub _NET_WM_WINDOW_TYPE_DOCK => b"_NET_WM_WINDOW_TYPE_DOCK",
        pub MANAGER => b"MANAGER",
    }
);

#[derive(Copy, Clone, Debug)]
pub struct Atoms(&'static InnerAtoms);

impl Atoms {
    pub fn intern_all(connection: &Connection) -> xcb::Result<Self> {
        unsafe {
            if ATOMS.is_none() {
                ATOMS = Some(InnerAtoms::intern_all(connection)?);
            }
            Ok(Self(ATOMS.as_ref().unwrap()))
        }
    }
}

impl Deref for Atoms {
    type Target = InnerAtoms;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
