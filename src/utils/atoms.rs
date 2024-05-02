#![allow(non_snake_case)]

use crate::atoms;
use std::sync::OnceLock;
use xcb::Connection;

static ATOMS: OnceLock<Atoms> = OnceLock::new();

atoms!(
     struct Atoms {
        MANAGER,
        UTF8_STRING,
        WM_NAME,
        _NET_ACTIVE_WINDOW,
        _NET_CURRENT_DESKTOP,
        _NET_DESKTOP_NAMES,
        _NET_SYSTEM_TRAY_OPCODE,
        _NET_SYSTEM_TRAY_ORIENTATION,
        _NET_SYSTEM_TRAY_S0,
        _NET_SYSTEM_TRAY_VISUAL,
        _NET_WM_NAME,
        _NET_WM_WINDOW_TYPE,
        _NET_WM_WINDOW_TYPE_DOCK,
        _XEMBED,
        _XEMBED_EMBEDDED_NOTIFY,
    }
);

#[macro_export]
macro_rules! atoms {
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
            fn intern_all(conn: &xcb::Connection) -> xcb::Result<$Atoms> {
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

impl Atoms {
    pub fn new(connection: &Connection) -> xcb::Result<&'static Atoms> {
        if ATOMS.get().is_none() {
            let inner = Atoms::intern_all(connection)?;
            ATOMS.set(inner).unwrap();
        }
        Ok(ATOMS.get().unwrap())
    }
}
