#![allow(non_snake_case)]

use crate::atoms_struct_2;
use std::ops::Deref;
use xcb::Connection;

static mut ATOMS: Option<InnerAtoms> = None;

atoms_struct_2!(
     struct InnerAtoms {
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
        MANAGER,
    }
);

#[macro_export]
macro_rules! atoms_struct_2 {
    (
        struct $Atoms:ident {
            $(
                $field:ident,
            )*
        }
    ) => {
        #[derive(Copy, Clone, Debug)]
        pub struct $Atoms {
            $(pub $field: xcb::x::Atom,)*
        }
        impl $Atoms {
            #[allow(dead_code)]
            pub fn intern_all(conn: &xcb::Connection) -> xcb::Result<$Atoms> {
                $(
                    let $field = conn.send_request(&xcb::x::InternAtom {
                        only_if_exists: false, // NOTE: this is important
                        name: stringify!($field).as_bytes(),
                    });
                )*
                Ok($Atoms {
                    $(
                        $field: conn.wait_for_reply($field)?.atom(),
                    )*
                })
            }
        }
    };
}

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
        self.0
    }
}
