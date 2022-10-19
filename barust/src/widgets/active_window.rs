use super::{OnClickCallback, Result, Text, Widget, WidgetConfig};
use crate::corex::{
    Atoms, EmptyCallback, HookSender, TimedHooks, UTF8_STRING, _NET_ACTIVE_WINDOW, _NET_WM_NAME,
};
use log::debug;
use std::{fmt::Display, thread};
use xcb::XidNew;
use xcb::{x::Window, Connection};

pub fn get_active_window_name(connection: &Connection) -> Result<String> {
    let atoms = Atoms::new(connection);
    let cookie = connection.send_request(&xcb::x::GetProperty {
        delete: false,
        window: connection.get_setup().roots().next().unwrap().root(),
        property: atoms.get(_NET_ACTIVE_WINDOW),
        r#type: xcb::x::ATOM_WINDOW,
        long_offset: 0,
        long_length: u32::MAX,
    });
    let reply = connection.wait_for_reply(cookie).map_err(Error::Xcb)?;
    let active_window_id = if let Some(data) = reply.value::<u32>().get(0) {
        unsafe { Window::new(*data) }
    } else {
        return Err(Error::Ewmh.into());
    };
    let cookie = connection.send_request(&xcb::x::GetProperty {
        delete: false,
        window: active_window_id,
        property: atoms.get(_NET_WM_NAME),
        r#type: atoms.get(UTF8_STRING),
        long_offset: 0,
        long_length: u32::MAX,
    });
    let reply = connection.wait_for_reply(cookie).map_err(Error::Xcb)?;
    String::from_utf8(reply.value::<u8>().into()).map_err(|_| Error::Ewmh.into())
}

#[derive(Debug)]
pub struct ActiveWindow {
    inner: Text,
    on_click: OnClickCallback,
}

impl ActiveWindow {
    pub fn new(config: &WidgetConfig, on_click: Option<&'static EmptyCallback>) -> Box<Self> {
        Box::new(Self {
            inner: *Text::new("", config, None),
            on_click: on_click.map(|c| c.into()),
        })
    }
}

impl Widget for ActiveWindow {
    fn draw(&self, context: &cairo::Context, rectangle: &cairo::Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating active_window");
        let (connection, _) = Connection::connect(None).map_err(Error::from)?;
        if let Ok(window_name) = get_active_window_name(&connection) {
            self.inner.set_text(window_name);
        }
        Ok(())
    }

    fn hook(&mut self, sender: HookSender, _timed_hooks: &mut TimedHooks) -> Result<()> {
        let (connection, screen_id) = Connection::connect(None).unwrap();
        let root_window = connection
            .get_setup()
            .roots()
            .nth(screen_id as usize)
            .unwrap()
            .root();
        connection
            .send_and_check_request(&xcb::x::ChangeWindowAttributes {
                window: root_window,
                value_list: &[xcb::x::Cw::EventMask(xcb::x::EventMask::PROPERTY_CHANGE)],
            })
            .map_err(Error::from)?;
        connection.flush().map_err(Error::from)?;
        thread::spawn(move || loop {
            if let Ok(xcb::Event::X(xcb::x::Event::PropertyNotify(_))) = connection.wait_for_event()
            {
                if sender.send().is_err() {
                    break;
                }
            }
        });
        Ok(())
    }

    fn size(&self, context: &cairo::Context) -> Result<f64> {
        self.inner.size(context)
    }

    fn padding(&self) -> f64 {
        self.inner.padding()
    }

    fn on_click(&self) {
        if let Some(cb) = &self.on_click {
            cb.call(());
        }
    }
}

impl Display for ActiveWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("ActiveWindow").fmt(f)
    }
}

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
pub enum Error {
    Ewmh,
    Xcb(xcb::Error),
}

impl From<xcb::ConnError> for Error {
    fn from(e: xcb::ConnError) -> Self {
        Error::Xcb(xcb::Error::Connection(e))
    }
}

impl From<xcb::ProtocolError> for Error {
    fn from(e: xcb::ProtocolError) -> Self {
        Error::Xcb(xcb::Error::Protocol(e))
    }
}
