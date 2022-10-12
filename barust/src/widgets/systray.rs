use super::{Result, Widget, WidgetConfig, WidgetError};
use crate::{
    corex::{
        set_source_rgba, Atoms, Color, HookSender, MANAGER, _NET_SYSTEM_TRAY_ORIENTATION,
        _NET_SYSTEM_TRAY_S0, _NET_WM_WINDOW_TYPE, _NET_WM_WINDOW_TYPE_DOCK,
    },
    statusbar::{screen_true_height, Position, StatusBarInfo},
};
use crossbeam_channel::{bounded, Receiver};
use log::{debug, error, warn};
use std::{fmt::Display, sync::Arc, thread};
use xcb::{
    x::{
        ChangeWindowAttributes, ClientMessageData, ConfigWindow, ConfigureWindow, CreateWindow, Cw,
        DestroyWindow, EventMask, MapWindow, Pixmap, ReparentWindow, SendEventDest, UnmapWindow,
        Window, WindowClass,
    },
    Connection, Xid, XidNew,
};

const SYSTEM_TRAY_REQUEST_DOCK: u32 = 0;
const SYSTEM_TRAY_BEGIN_MESSAGE: u32 = 1;
const SYSTEM_TRAY_CANCEL_MESSAGE: u32 = 2;

/// Displays a system tray
pub struct Systray {
    padding: f64,
    icon_size: f64,
    window: Option<Window>,
    connection: Arc<Connection>,
    screen_id: i32,
    children: Vec<Window>,
    event_receiver: Option<Receiver<SystrayEvent>>,
}

impl std::fmt::Debug for Systray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "padding: {:?}, icon_size: {:?}, window: {:?}, screen_id: {:?}, children: {:?}",
            self.padding, self.icon_size, self.window, self.screen_id, self.children,
        )
    }
}

impl Systray {
    ///* `icon_size` width of the icons
    ///* `config` a [WidgetConfig]
    pub fn new(icon_size: f64, config: &WidgetConfig) -> Result<Box<Self>> {
        warn!("Systray is unstable");
        let (connection, screen_id) = Connection::connect(None).map_err(Error::from)?;

        Ok(Box::new(Self {
            padding: config.padding,
            icon_size,
            window: None,
            connection: Arc::new(connection),
            screen_id,
            children: Vec::new(),
            event_receiver: None,
        }))
    }

    fn adopt(&mut self, window: Window) -> Result<()> {
        if self.children.contains(&window) {
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

        self.connection
            .send_and_check_request(&ChangeWindowAttributes {
                window,
                value_list: &[Cw::EventMask(EventMask::STRUCTURE_NOTIFY)],
            })
            .map_err(Error::from)?;

        if self.children.is_empty() {
            self.connection
                .send_and_check_request(&MapWindow {
                    window: self.window.unwrap(),
                })
                .map_err(Error::from)?;
        }

        self.children.push(window);
        self.reposition_children()?;
        self.connection
            .send_and_check_request(&MapWindow { window })
            .map_err(Error::from)?;

        self.connection.flush().map_err(Error::from)?;
        Ok(())
    }

    fn reposition_children(&mut self) -> Result<()> {
        let mut offset = 0.0;
        for window in &self.children {
            offset += self.icon_size;
            self.connection
                .send_and_check_request(
                    &(ReparentWindow {
                        window: *window,
                        parent: self.window.unwrap(),
                        x: offset as i16,
                        y: 0,
                    }),
                )
                .map_err(Error::from)?;
        }
        Ok(())
    }

    fn forget(&mut self, window: Window) -> Result<()> {
        self.children.retain(|child| *child != window);
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
                    //Cw::BackPixel(screen.black_pixel()),
                    Cw::EventMask(EventMask::PROPERTY_CHANGE | EventMask::STRUCTURE_NOTIFY),
                ],
            })
            .map_err(Error::from)?;

        let atoms = Atoms::new(&self.connection);
        self.connection
            .send_and_check_request(&xcb::x::ChangeProperty {
                mode: xcb::x::PropMode::Replace,
                window,
                property: atoms.get(_NET_WM_WINDOW_TYPE),
                r#type: xcb::x::ATOM_ATOM,
                data: &[atoms.get(_NET_WM_WINDOW_TYPE_DOCK)],
            })
            .map_err(Error::from)?;

        self.connection
            .send_and_check_request(&xcb::x::ChangeProperty {
                mode: xcb::x::PropMode::Replace,
                window,
                property: atoms.get(_NET_SYSTEM_TRAY_ORIENTATION),
                r#type: xcb::x::ATOM_CARDINAL,
                data: &[0_u32],
            })
            .map_err(Error::from)?;
        self.connection.flush().map_err(Error::from)?;

        Ok(window)
    }

    fn take_selection(&self, time: u32) -> Result<bool> {
        let atoms = Atoms::new(&self.connection);
        let selection = atoms.get(_NET_SYSTEM_TRAY_S0);
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
            atoms.get(MANAGER),
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
}

impl Widget for Systray {
    fn draw(&self, context: &cairo::Context, rectangle: &cairo::Rectangle) -> Result<()> {
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
                drawable: xcb::x::Drawable::Window(self.window.unwrap()),
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
        self.connection
            .send_and_check_request(&MapWindow {
                window: self.window.unwrap(),
            })
            .map_err(Error::from)?;
        Ok(())
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating systray");
        let event_receiver = self.event_receiver.take().unwrap();
        for event in event_receiver.try_iter() {
            match event {
                SystrayEvent::ClientMessage(event) => {
                    if let ClientMessageData::Data32(data) = event.data() {
                        let opcode = data[1];
                        let window = data[2];
                        match opcode {
                            SYSTEM_TRAY_REQUEST_DOCK => {
                                if let Err(e) = self.adopt(unsafe { Window::new(window) }) {
                                    if let WidgetError::Systray(Error::Xcb(xcb::Error::Protocol(
                                        xcb::ProtocolError::X(xcb::x::Error::Window(ref e), _),
                                    ))) = e
                                    {
                                        println!("possible bad window error: {:?}", e);
                                    } else {
                                        return Err(e);
                                    }
                                }
                            }
                            SYSTEM_TRAY_BEGIN_MESSAGE => {}
                            SYSTEM_TRAY_CANCEL_MESSAGE => {}
                            _ => {
                                unreachable!("Invalid opcode")
                            }
                        };
                    }
                }
                SystrayEvent::DestroyNotify(event) => self.forget(event.window())?,
                SystrayEvent::PropertyNotify(event) => {
                    if !self.take_selection(event.time())? {
                        return Err(Error::NoSelection)?;
                    }
                }
                SystrayEvent::ReparentNotify(event) => {
                    if event.parent() != self.window.unwrap() {
                        self.forget(event.window())?;
                    }
                }
                SystrayEvent::SelectionClear => self.last_update()?,
            }
        }
        self.event_receiver.replace(event_receiver);
        Ok(())
    }

    fn last_update(&mut self) -> Result<()> {
        let setup = self.connection.get_setup();
        let screen = setup.roots().nth(self.screen_id as _).unwrap();
        let root = screen.root();

        for child in &self.children {
            let window = *child;
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

    fn hook(&mut self, sender: HookSender) -> Result<()> {
        let connection = self.connection.clone();
        let (tx, rx) = bounded::<SystrayEvent>(10);
        self.event_receiver = Some(rx);
        thread::spawn(move || loop {
            if let Ok(xcb::Event::X(event)) = connection.wait_for_event() {
                let to_send = match event {
                    xcb::x::Event::ClientMessage(e) => Some(SystrayEvent::ClientMessage(e)),
                    xcb::x::Event::DestroyNotify(e) => Some(SystrayEvent::DestroyNotify(e)),
                    xcb::x::Event::PropertyNotify(e) => Some(SystrayEvent::PropertyNotify(e)),
                    xcb::x::Event::ReparentNotify(e) => Some(SystrayEvent::ReparentNotify(e)),
                    xcb::x::Event::SelectionClear(_) => Some(SystrayEvent::SelectionClear),
                    _ => {
                        //error!("{:#?}", event);
                        None
                    }
                };
                if let Some(to_send) = to_send {
                    if tx.send(to_send).is_err() || sender.send().is_err() {
                        error!("breaking systray hook loop");
                        break;
                    }
                }
            }
        });
        Ok(())
    }

    fn size(&self, _context: &cairo::Context) -> Result<f64> {
        if self.children.is_empty() {
            return Ok(1.0);
        }
        Ok(self.icon_size * self.children.len() as f64 + 2.0 * self.padding)
    }

    fn padding(&self) -> f64 {
        self.padding
    }
}

impl Display for Systray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Systray").fmt(f)
    }
}

#[derive(Debug)]
enum SystrayEvent {
    ClientMessage(xcb::x::ClientMessageEvent),
    DestroyNotify(xcb::x::DestroyNotifyEvent),
    PropertyNotify(xcb::x::PropertyNotifyEvent),
    ReparentNotify(xcb::x::ReparentNotifyEvent),
    SelectionClear,
}

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
pub enum Error {
    Xcb(xcb::Error),
    Cairo(cairo::Error),
    MissingWindow,
    NoSelection,
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
