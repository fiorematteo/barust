use xcb::{
    x::{Atom, InternAtom},
    Connection,
};

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
