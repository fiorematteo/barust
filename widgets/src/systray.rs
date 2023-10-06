use crate::{Rectangle, Result, Size, Widget, WidgetConfig, WidgetError};
use async_channel::{Receiver, bounded};
use cairo::Context;
use log::{debug, error, warn};
use std::{fmt::Display, sync::Arc};
use tokio::task::spawn_blocking;
use utils::{
    screen_true_height, set_source_rgba, Atoms, Color, HookSender, Position, StatusBarInfo,
    TimedHooks,
};
use xcb::{
    x::{
        ChangeWindowAttributes, ClientMessageData, ClientMessageEvent, ConfigWindow,
        ConfigureWindow, CreateWindow, Cw, DestroyWindow, Drawable, EventMask, GetGeometry,
        MapWindow, Pixmap, ReparentWindow, SendEventDest, UnmapWindow, Window, WindowClass,
    },
    Connection, Xid, XidNew,
};

const SYSTEM_TRAY_REQUEST_DOCK: u32 = 0;
const SYSTEM_TRAY_BEGIN_MESSAGE: u32 = 1;
const SYSTEM_TRAY_CANCEL_MESSAGE: u32 = 2;

/// Displays a system tray
pub struct Systray {
    padding: u32,
    internal_padding: u32,
    window: Option<Window>,
    connection: Arc<Connection>,
    screen_id: i32,
    children: Vec<(Window, u16)>,
    event_receiver: Option<Receiver<xcb::x::Event>>,
}

impl std::fmt::Debug for Systray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "padding: {:?}, window: {:?}, screen_id: {:?}, children: {:?}",
            self.padding, self.window, self.screen_id, self.children,
        )
    }
}

impl Systray {
    ///* `icon_size` width of the icons
    ///* `config` a [&WidgetConfig]
    pub async fn new(internal_padding: u32, config: &WidgetConfig) -> Result<Box<Self>> {
        let (connection, screen_id) = Connection::connect(None).map_err(Error::from)?;

        Ok(Box::new(Self {
            padding: config.padding,
            window: None,
            connection: Arc::new(connection),
            screen_id,
            children: Vec::new(),
            event_receiver: None,
            internal_padding,
        }))
    }

    fn adopt(&mut self, window: Window) -> Result<()> {
        if self.children.iter().any(|(c, _)| *c == window) {
            return Ok(());
        }

        self.connection
            .send_and_check_request(&ChangeWindowAttributes {
                window,
                value_list: &[
                    Cw::OverrideRedirect(true),
                    Cw::EventMask(EventMask::STRUCTURE_NOTIFY),
                ],
            })
            .map_err(Error::from)?;

        if self.children.is_empty() {
            self.connection
                .send_and_check_request(&MapWindow {
                    window: self.window.unwrap(),
                })
                .map_err(Error::from)?;
        }

        let window_width = self
            .connection
            .wait_for_reply(self.connection.send_request(&GetGeometry {
                drawable: Drawable::Window(window),
            }))
            .map_err(Error::from)?
            .width();

        self.children.push((window, window_width));
        self.reposition_children()?;
        self.connection
            .send_and_check_request(&MapWindow { window })
            .map_err(Error::from)?;
        self.connection.flush().map_err(Error::from)?;
        Ok(())
    }

    fn reposition_children(&mut self) -> Result<()> {
        let mut offset = 0;
        for (window, width) in &self.children {
            offset += u32::from(*width) + self.internal_padding;
            self.connection.send_request(
                &(ReparentWindow {
                    window: *window,
                    parent: self.window.unwrap(),
                    x: offset.try_into().unwrap(),
                    y: 0,
                }),
            );
        }
        // Since there are no ways to ping a window
        // destroyed windows can sometimes still be in self.children
        self.connection.flush().ok();
        Ok(())
    }

    fn forget(&mut self, window: Window) -> Result<()> {
        self.children.retain(|(child, _)| *child != window);
        self.reposition_children()?;
        if self.children.is_empty() {
            self.connection
                .send_and_check_request(&UnmapWindow {
                    window: self.window.unwrap(),
                })
                .map_err(Error::from)?;
        }
        Ok(())
    }

    fn create_tray_window(&self, y: i16) -> Result<Window> {
        let window: Window = self.connection.generate_id();
        let screen = self
            .connection
            .get_setup()
            .roots()
            .nth(self.screen_id as _)
            .unwrap_or_else(|| panic!("cannot find screen:{}", self.screen_id));

        self.connection
            .send_and_check_request(&CreateWindow {
                depth: xcb::x::COPY_FROM_PARENT as _,
                wid: window,
                parent: screen.root(),
                x: 0,
                y,
                width: 1,
                height: 1,
                border_width: 0,
                class: WindowClass::InputOutput,
                visual: xcb::x::COPY_FROM_PARENT,
                value_list: &[
                    Cw::BackPixmap(Pixmap::none()),
                    Cw::EventMask(EventMask::PROPERTY_CHANGE | EventMask::STRUCTURE_NOTIFY),
                ],
            })
            .map_err(Error::from)?;

        let atoms = Atoms::new(&self.connection).map_err(Error::from)?;
        self.connection
            .send_and_check_request(&xcb::x::ChangeProperty {
                mode: xcb::x::PropMode::Replace,
                window,
                property: atoms._NET_WM_WINDOW_TYPE,
                r#type: xcb::x::ATOM_ATOM,
                data: &[atoms._NET_WM_WINDOW_TYPE_DOCK],
            })
            .map_err(Error::from)?;

        self.connection
            .send_and_check_request(&xcb::x::ChangeProperty {
                mode: xcb::x::PropMode::Replace,
                window,
                property: atoms._NET_SYSTEM_TRAY_ORIENTATION,
                r#type: xcb::x::ATOM_CARDINAL,
                data: &[0_u32],
            })
            .map_err(Error::from)?;
        self.connection.flush().map_err(Error::from)?;

        Ok(window)
    }

    fn take_selection(&self, time: u32) -> Result<bool> {
        let atoms = Atoms::new(&self.connection).map_err(Error::from)?;
        let selection = atoms._NET_SYSTEM_TRAY_S0;
        let window = self.window.ok_or(Error::MissingWindow)?;

        let owner = self
            .connection
            .wait_for_reply(
                self.connection
                    .send_request(&xcb::x::GetSelectionOwner { selection }),
            )
            .map_err(Error::from)?
            .owner();

        if owner == window {
            return Ok(true);
        }

        if !owner.is_none() {
            return Ok(false);
        }

        self.connection
            .send_and_check_request(&xcb::x::SetSelectionOwner {
                owner: window,
                selection,
                time,
            })
            .map_err(Error::from)?;

        let owner = self
            .connection
            .wait_for_reply(
                self.connection
                    .send_request(&xcb::x::GetSelectionOwner { selection }),
            )
            .map_err(Error::from)?
            .owner();

        if owner != window {
            return Ok(false);
        }

        let setup = self.connection.get_setup();
        let screen = setup.roots().next().unwrap();
        let client_event = xcb::x::ClientMessageEvent::new(
            screen.root(),
            atoms.MANAGER,
            xcb::x::ClientMessageData::Data32([
                time,
                selection.resource_id(),
                window.resource_id(),
                0,
                0,
            ]),
        );
        self.connection
            .send_and_check_request(&xcb::x::SendEvent {
                propagate: false,
                destination: SendEventDest::Window(screen.root()),
                event_mask: EventMask::STRUCTURE_NOTIFY,
                event: &client_event,
            })
            .map_err(Error::from)?;
        self.connection.flush().map_err(Error::from)?;
        Ok(true)
    }

    fn handle_client_message(&mut self, event: ClientMessageEvent) -> Result<()> {
        let ClientMessageData::Data32(data) = event.data() else {
            return Ok(());
        };
        let opcode = data[1];
        let window = data[2];
        match opcode {
            SYSTEM_TRAY_REQUEST_DOCK => {
                debug!("systray request dock message");

                let window = unsafe { Window::new(window) };

                if let Err(e) = self.adopt(window) {
                    if let WidgetError::Systray(Error::Xcb(xcb::Error::Protocol(
                        xcb::ProtocolError::X(xcb::x::Error::Window(ref e), _),
                    ))) = e
                    {
                        warn!("possible bad window error: {:?}", e);
                    } else {
                        return Err(e);
                    }
                }
            }
            SYSTEM_TRAY_BEGIN_MESSAGE => {
                debug!("systray begin message");
            }
            SYSTEM_TRAY_CANCEL_MESSAGE => {
                debug!("systray cancel message");
            }
            _ => {
                unreachable!("Invalid opcode")
            }
        };
        Ok(())
    }

    fn handle_event(&mut self, event: xcb::x::Event) -> Result<()> {
        match event {
            xcb::x::Event::ClientMessage(event) => {
                self.handle_client_message(event)?;
            }
            xcb::x::Event::DestroyNotify(event) => self.forget(event.window())?,
            xcb::x::Event::PropertyNotify(event) => {
                if !self.take_selection(event.time())? {
                    return Err(Error::NoSelection.into());
                }
            }
            xcb::x::Event::ReparentNotify(event) => {
                if event.parent() != self.window.unwrap() {
                    self.forget(event.window())?;
                }
            }
            xcb::x::Event::SelectionClear(_) => self.last_update()?,
            _ => (),
        }
        Ok(())
    }
}

use async_trait::async_trait;
#[async_trait]
impl Widget for Systray {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        set_source_rgba(
            context,
            Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
        );
        context.fill().map_err(Error::from)?;
        let geometry = self
            .connection
            .wait_for_reply(self.connection.send_request(&xcb::x::GetGeometry {
                drawable: Drawable::Window(self.window.unwrap()),
            }))
            .map_err(Error::from)?;

        if geometry.x() != rectangle.x as i16 || geometry.width() != rectangle.width as u16 {
            self.connection
                .send_and_check_request(&ConfigureWindow {
                    window: self.window.unwrap(),
                    value_list: &[
                        ConfigWindow::X(rectangle.x as _),
                        ConfigWindow::Width(rectangle.width as _),
                        ConfigWindow::Height(rectangle.height as _),
                    ],
                })
                .map_err(Error::from)?;
        }
        Ok(())
    }

    fn setup(&mut self, info: &StatusBarInfo) -> Result<()> {
        let y = match info.position {
            Position::Top => 0,
            Position::Bottom => {
                screen_true_height(&self.connection, self.screen_id) - info.height as u16
            }
        };
        self.window = Some(self.create_tray_window(y as _)?);
        Ok(())
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating systray");
        //NOTE xcb::x::Event doesn't implement copy :(
        let event_receiver = self.event_receiver.take();
        let Some(events) = event_receiver else {
            self.event_receiver = event_receiver;
            return Ok(());
        };
        while let Ok(event) = events.try_recv() {
            self.handle_event(event)?;
        }
        self.event_receiver.replace(events);
        Ok(())
    }

    fn last_update(&mut self) -> Result<()> {
        let setup = self.connection.get_setup();
        let screen = setup.roots().nth(self.screen_id as _).unwrap();
        let root = screen.root();

        for (window, _) in &self.children {
            let window = *window;
            self.connection
                .send_and_check_request(&ChangeWindowAttributes {
                    window,
                    value_list: &[Cw::EventMask(EventMask::NO_EVENT)],
                })
                .map_err(Error::from)?;
            self.connection
                .send_and_check_request(&UnmapWindow { window })
                .map_err(Error::from)?;
            self.connection
                .send_and_check_request(
                    &(ReparentWindow {
                        window,
                        parent: root,
                        x: 0,
                        y: 0,
                    }),
                )
                .map_err(Error::from)?;
        }
        self.connection
            .send_and_check_request(&ChangeWindowAttributes {
                window: self.window.unwrap(),
                value_list: &[Cw::EventMask(EventMask::STRUCTURE_NOTIFY)],
            })
            .map_err(Error::from)?;
        self.connection
            .send_and_check_request(&DestroyWindow {
                window: self.window.unwrap(),
            })
            .map_err(Error::from)?;
        self.connection.flush().map_err(Error::from)?;
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, _timed_hooks: &mut TimedHooks) -> Result<()> {
        let connection = self.connection.clone();
        let (tx, rx) = bounded(10);
        self.event_receiver = Some(rx);
        spawn_blocking(move || loop {
            if let Ok(xcb::Event::X(event)) = connection.wait_for_event() {
                if tx.send_blocking(event).is_err() || sender.send_blocking().is_err() {
                    error!("breaking systray hook loop");
                    break;
                }
            }
        });
        Ok(())
    }

    fn size(&self, _context: &Context) -> Result<Size> {
        if self.children.is_empty() {
            return Ok(Size::Static(1));
        }
        Ok(Size::Static(
            self.children
                .iter()
                .map(|(_, width)| u32::from(*width) + self.internal_padding)
                .sum::<u32>()
                + 2 * self.padding,
        ))
    }

    fn padding(&self) -> u32 {
        self.padding
    }
}

impl Display for Systray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Systray").fmt(f)
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    Xcb(#[from] xcb::Error),
    Cairo(#[from] cairo::Error),
    #[error("Missing window")]
    MissingWindow,
    #[error("No selection")]
    NoSelection,
    #[error("Mutex error")]
    Mutex,
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
