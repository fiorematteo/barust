use crate::{widget_default, Rectangle, Result, Text, Widget, WidgetConfig};
use cairo::Context;
use log::{debug, error};
use std::{fmt::Display, sync::Arc, time::Duration};
use tokio::{spawn, task::spawn_blocking, time::sleep};
use utils::{Atoms, HookSender, TimedHooks};
use xcb::{
    x::{ChangeWindowAttributes, Cw, Event, EventMask, Window},
    Connection, XidNew,
};

pub fn get_active_window_name(connection: &Connection, atoms: &Atoms) -> Result<String> {
    let cookie = connection.send_request(&xcb::x::GetProperty {
        delete: false,
        window: connection.get_setup().roots().next().unwrap().root(),
        property: atoms._NET_ACTIVE_WINDOW,
        r#type: xcb::x::ATOM_WINDOW,
        long_offset: 0,
        long_length: u32::MAX,
    });
    let reply = connection.wait_for_reply(cookie).map_err(Error::Xcb)?;
    let active_window_id = reply
        .value::<u32>()
        .first()
        .map(|data| unsafe { Window::new(*data) })
        .ok_or(Error::Ewmh)?;

    let cookie = connection.send_request(&xcb::x::GetProperty {
        delete: false,
        window: active_window_id,
        property: atoms._NET_WM_NAME,
        r#type: atoms.UTF8_STRING,
        long_offset: 0,
        long_length: u32::MAX,
    });
    let reply = connection.wait_for_reply(cookie).map_err(Error::Xcb)?;
    String::from_utf8(reply.value::<u8>().into()).map_err(|_| Error::Ewmh.into())
}

pub struct ActiveWindow {
    inner: Text,
    connection: Connection,
    atoms: Atoms,
}

impl std::fmt::Debug for ActiveWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "inner: {:?}", self.inner)
    }
}

impl ActiveWindow {
    pub async fn new(config: &WidgetConfig) -> Result<Box<Self>> {
        let (connection, _) = Connection::connect(None).map_err(Error::from)?;
        let atoms = Atoms::new(&connection).map_err(Error::from)?;
        Ok(Box::new(Self {
            inner: *Text::new("", config).await,
            connection,
            atoms,
        }))
    }
}

use async_trait::async_trait;
#[async_trait]
impl Widget for ActiveWindow {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating active_window");
        if let Ok(window_name) = get_active_window_name(&self.connection, &self.atoms) {
            self.inner.set_text(window_name);
        }
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, _timed_hooks: &mut TimedHooks) -> Result<()> {
        let (connection, screen_id) = Connection::connect(None).unwrap();
        let root_window = connection
            .get_setup()
            .roots()
            .nth(
                screen_id
                    .try_into()
                    .expect("Screen id should always be positive"),
            )
            .unwrap()
            .root();
        connection
            .send_and_check_request(&ChangeWindowAttributes {
                window: root_window,
                value_list: &[Cw::EventMask(EventMask::PROPERTY_CHANGE)],
            })
            .map_err(Error::from)?;
        connection.flush().map_err(Error::from)?;

        let property_sender = Arc::new(sender);
        let property_connection = Arc::new(connection);
        let name_sender = property_sender.clone();
        let name_connection = property_connection.clone();

        spawn_blocking(move || loop {
            let Ok(xcb::Event::X(Event::PropertyNotify(_))) = property_connection.wait_for_event() else {
                continue
            };
            if property_sender.send_blocking().is_err() {
                error!("breaking active_window hook");
                break;
            }
        });

        let atoms = self.atoms;
        let mut old_name = "".into();
        spawn(async move {
            loop {
                sleep(Duration::from_secs(1)).await;
                let Ok(new_name) = get_active_window_name(&name_connection, &atoms) else {
                    continue
                };

                if old_name == new_name {
                    continue;
                }

                old_name = new_name;
                if name_sender.send().await.is_err() {
                    error!("breaking active_window hook");
                    break;
                }
            }
        });
        Ok(())
    }

    widget_default!(size, padding);
}

impl Display for ActiveWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("ActiveWindow").fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum Error {
    #[error("Ewmh")]
    Ewmh,
    Xcb(#[from] xcb::Error),
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
