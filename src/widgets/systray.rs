use crate::{
    statusbar::set_window_title,
    utils::{screen_true_height, Atoms, HookSender, Position, StatusBarInfo, TimedHooks},
    widgets::{Rectangle, Result, Size, Widget, WidgetConfig},
};
use async_channel::{bounded, Receiver};
use async_trait::async_trait;
use cairo::Context;
use log::{debug, error};
use std::{fmt::Display, sync::Arc, thread};
use xcb::{
    x::{
        ChangeProperty, ChangeWindowAttributes, ClientMessageData, ClientMessageEvent, Colormap,
        ColormapAlloc, ConfigWindow, ConfigureWindow, CreateColormap, CreateWindow, Cw,
        DestroyWindow, Drawable, EventMask, Gcontext, MapWindow, Pixmap, PropMode, ReparentWindow,
        SendEvent, SendEventDest, StackMode, UnmapWindow, VisualClass, Window, WindowClass,
        CURRENT_TIME,
    },
    Connection, Xid, XidNew,
};

const SYSTEM_TRAY_REQUEST_DOCK: u32 = 0;

/// Displays a system tray
pub struct Systray {
    padding: u32,
    internal_padding: u32,
    window: Option<Window>,
    connection: Arc<Connection>,
    screen_id: i32,
    children: Vec<Window>,
    event_receiver: Option<Receiver<SystrayEvent>>,
    icon_size: u32,
    context: Option<Gcontext>,
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
            icon_size: 0,
            context: None,
        }))
    }

    fn adopt(&mut self, window: Window) -> Result<()> {
        if self.children.contains(&window) {
            return Ok(());
        }

        self.connection
            .send_and_check_request(&ReparentWindow {
                window,
                parent: self.window.unwrap(),
                x: 0,
                y: 0,
            })
            .map_err(Error::from)?;

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
            .send_and_check_request(&ConfigureWindow {
                window,
                value_list: &[
                    ConfigWindow::Sibling(self.window.unwrap()),
                    ConfigWindow::StackMode(StackMode::Above),
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

        self.children.push(window);
        self.connection.flush().map_err(Error::from)?;
        Ok(())
    }

    fn forget(&mut self, window: Window) -> Result<()> {
        if !self.children.contains(&window) {
            return Ok(());
        }
        self.children.retain(|child| *child != window);

        self.connection.send_request(&ChangeWindowAttributes {
            window,
            value_list: &[
                Cw::OverrideRedirect(false),
                Cw::EventMask(EventMask::NO_EVENT),
            ],
        });
        self.connection.send_request(&UnmapWindow { window });
        self.connection.send_request(
            &(ReparentWindow {
                window,
                parent: self.connection.get_setup().roots().next().unwrap().root(),
                x: 0,
                y: 0,
            }),
        );
        self.connection.flush().map_err(Error::from)?;

        if self.children.is_empty() {
            self.connection
                .send_and_check_request(&UnmapWindow {
                    window: self.window.unwrap(),
                })
                .map_err(Error::from)?;
        }
        Ok(())
    }

    fn create_tray_window(&mut self, y: i16, height: u16) -> Result<()> {
        let window: Window = self.connection.generate_id();
        let colormap: Colormap = self.connection.generate_id();

        let screen = self
            .connection
            .get_setup()
            .roots()
            .next()
            .unwrap_or_else(|| panic!("cannot find screen:{}", 0));

        let depth = screen
            .allowed_depths()
            .find(|d| d.depth() == 32)
            .expect("cannot find valid depth");

        let visual_type = depth
            .visuals()
            .iter()
            .find(|v| v.class() == VisualClass::TrueColor)
            .expect("cannot find valid visual type")
            .to_owned();

        self.connection
            .send_and_check_request(&CreateColormap {
                alloc: ColormapAlloc::None,
                mid: colormap,
                window: screen.root(),
                visual: visual_type.visual_id(),
            })
            .map_err(Error::from)?;

        self.connection
            .send_and_check_request(&CreateWindow {
                depth: depth.depth(),
                wid: window,
                parent: screen.root(),
                x: 0,
                y,
                width: 1,
                height,
                border_width: 0,
                class: WindowClass::InputOutput,
                visual: visual_type.visual_id(),
                value_list: &[
                    Cw::BackPixmap(Pixmap::none()),
                    Cw::BorderPixel(screen.black_pixel()),
                    Cw::EventMask(EventMask::PROPERTY_CHANGE | EventMask::STRUCTURE_NOTIFY),
                    Cw::Colormap(colormap),
                ],
            })
            .map_err(Error::from)?;

        let atoms = Atoms::new(&self.connection).map_err(Error::from)?;
        self.connection
            .send_and_check_request(&ChangeProperty {
                mode: PropMode::Replace,
                window,
                property: atoms._NET_SYSTEM_TRAY_VISUAL,
                r#type: xcb::x::ATOM_VISUALID,
                data: &[visual_type.visual_id()],
            })
            .map_err(Error::from)?;

        self.connection
            .send_and_check_request(&ChangeProperty {
                mode: xcb::x::PropMode::Replace,
                window,
                property: atoms._NET_WM_WINDOW_TYPE,
                r#type: xcb::x::ATOM_ATOM,
                data: &[atoms._NET_WM_WINDOW_TYPE_DOCK],
            })
            .map_err(Error::from)?;

        self.connection
            .send_and_check_request(&ChangeProperty {
                mode: xcb::x::PropMode::Replace,
                window,
                property: atoms._NET_SYSTEM_TRAY_ORIENTATION,
                r#type: xcb::x::ATOM_CARDINAL,
                data: &[0_u32],
            })
            .map_err(Error::from)?;

        set_window_title(self.connection.clone(), window, "systray").map_err(Error::from)?;

        self.connection.flush().map_err(Error::from)?;

        // get context
        // can't use cairo because it's not Send
        let cid = self.connection.generate_id();
        self.connection
            .send_and_check_request(&xcb::x::CreateGc {
                cid,
                drawable: Drawable::Window(window),
                value_list: &[],
            })
            .map_err(Error::from)?;

        self.window = Some(window);
        self.context = Some(cid);
        Ok(())
    }

    fn take_selection(&self) -> Result<()> {
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
            return Ok(());
        }

        if !owner.is_none() {
            return Err(Error::NoSelection.into());
        }

        self.connection
            .send_and_check_request(&xcb::x::SetSelectionOwner {
                owner: window,
                selection,
                time: xcb::x::CURRENT_TIME,
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
            return Err(Error::NoSelection.into());
        }

        let setup = self.connection.get_setup();
        let screen = setup.roots().next().unwrap();
        let client_event = ClientMessageEvent::new(
            screen.root(),
            atoms.MANAGER,
            xcb::x::ClientMessageData::Data32([
                xcb::x::CURRENT_TIME,
                selection.resource_id(),
                window.resource_id(),
                0,
                0,
            ]),
        );
        self.connection
            .send_and_check_request(&SendEvent {
                propagate: false,
                destination: SendEventDest::Window(screen.root()),
                event_mask: EventMask::STRUCTURE_NOTIFY,
                event: &client_event,
            })
            .map_err(Error::from)?;
        self.connection.flush().map_err(Error::from)?;
        Ok(())
    }

    fn handle_client_message(&mut self, event: ClientMessageEvent) -> Result<()> {
        let ClientMessageData::Data32(data) = event.data() else {
            return Ok(());
        };
        let opcode = data[1];
        let window = data[2];
        if SYSTEM_TRAY_REQUEST_DOCK == opcode {
            debug!("systray request dock message");

            let window = unsafe { Window::new(window) };

            if self.adopt(window).is_err() {
                self.forget(window)?;
            }
        };
        Ok(())
    }

    fn handle_event(&mut self, event: SystrayEvent) -> Result<()> {
        match event {
            SystrayEvent::ClientMessage(event) => {
                self.handle_client_message(event)?;
            }
            SystrayEvent::DestroyNotify(window) => self.forget(window)?,
            SystrayEvent::PropertyNotify => {}
            SystrayEvent::ReparentNotify((parent, window)) => {
                if parent != self.window.unwrap() {
                    self.forget(window)?;
                }
            }
            _ => (),
        }
        Ok(())
    }
}

#[async_trait]
impl Widget for Systray {
    fn draw(&self, _: Context, rectangle: &Rectangle) -> Result<()> {
        // fit to rectangle
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

        // clear surface
        self.connection
            .send_and_check_request(&xcb::x::PolyFillRectangle {
                drawable: Drawable::Window(self.window.unwrap()),
                gc: self.context.unwrap(),
                rectangles: &[xcb::x::Rectangle {
                    x: 0,
                    y: 0,
                    width: rectangle.width as _,
                    height: rectangle.height as _,
                }],
            })
            .map_err(Error::from)?;

        // paint children
        let mut offset = 1;
        for child in &self.children {
            let atoms = Atoms::new(&self.connection).map_err(Error::from)?;
            let data = ClientMessageData::Data32([
                CURRENT_TIME,
                atoms._XEMBED_EMBEDDED_NOTIFY.resource_id(),
                0,
                self.window.unwrap().resource_id(),
                0,
            ]);
            // don't trust child windows
            let event = &ClientMessageEvent::new(self.window.unwrap(), atoms._XEMBED, data);
            self.connection
                .send_and_check_request(&SendEvent {
                    propagate: false,
                    destination: SendEventDest::Window(*child),
                    event_mask: EventMask::all(),
                    event,
                })
                .ok();

            self.connection
                .send_and_check_request(&MapWindow { window: *child })
                .ok();
            self.connection
                .send_and_check_request(
                    &(ConfigureWindow {
                        window: *child,
                        value_list: &[
                            ConfigWindow::X(offset as _),
                            ConfigWindow::Y(1),
                            ConfigWindow::Width(self.icon_size as _),
                            ConfigWindow::Height(self.icon_size as _),
                        ],
                    }),
                )
                .ok();
            offset += self.icon_size + self.internal_padding;
        }

        Ok(())
    }

    async fn setup(&mut self, info: &StatusBarInfo) -> Result<()> {
        let y = match info.position {
            Position::Top => 0,
            Position::Bottom => {
                screen_true_height(&self.connection, self.screen_id) - info.height as u16
            }
        };
        self.create_tray_window(y as _, info.height as _)?;
        self.icon_size = info.height - 2;

        // enforce stacking order
        self.connection
            .send_and_check_request(&ConfigureWindow {
                window: self.window.unwrap(),
                value_list: &[
                    ConfigWindow::Sibling(info.window),
                    ConfigWindow::StackMode(StackMode::Above),
                ],
            })
            .map_err(Error::from)?;

        self.take_selection()?;
        Ok(())
    }

    async fn update(&mut self) -> Result<()> {
        debug!("updating systray");
        let Some(events) = self.event_receiver.take() else {
            return Ok(());
        };
        while let Ok(event) = events.try_recv() {
            self.handle_event(event)?;
        }
        self.event_receiver.replace(events);
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, _timed_hooks: &mut TimedHooks) -> Result<()> {
        let connection = self.connection.clone();
        let (tx, rx) = bounded(10);
        self.event_receiver = Some(rx);
        thread::spawn(move || loop {
            let event = if let Ok(xcb::Event::X(event)) = connection.wait_for_event() {
                let event: xcb::x::Event = event;
                Some(SystrayEvent::from(event))
            } else {
                None
            };
            if let Some(event) = event {
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
        let children_len = self.children.len() as u32;
        Ok(Size::Static(
            children_len * self.icon_size + (children_len - 1) * self.internal_padding + 2,
        ))
    }

    fn padding(&self) -> u32 {
        self.padding
    }
}

impl Drop for Systray {
    fn drop(&mut self) {
        let setup = self.connection.get_setup();
        let screen = setup.roots().nth(self.screen_id as _).unwrap();
        let root = screen.root();

        for window in &self.children {
            let window = *window;
            self.connection
                .send_and_check_request(&ChangeWindowAttributes {
                    window,
                    value_list: &[Cw::EventMask(EventMask::NO_EVENT)],
                })
                .ok();
            self.connection
                .send_and_check_request(&UnmapWindow { window })
                .ok();
            self.connection
                .send_and_check_request(
                    &(ReparentWindow {
                        window,
                        parent: root,
                        x: 0,
                        y: 0,
                    }),
                )
                .ok();
        }

        if let Some(window) = self.window {
            self.connection
                .send_and_check_request(&ChangeWindowAttributes {
                    window,
                    value_list: &[Cw::EventMask(EventMask::STRUCTURE_NOTIFY)],
                })
                .ok();
            self.connection
                .send_and_check_request(&DestroyWindow { window })
                .ok();
        }
        self.connection.flush().ok();
    }
}

impl Display for Systray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Systray").fmt(f)
    }
}

enum SystrayEvent {
    ClientMessage(ClientMessageEvent),
    DestroyNotify(Window),
    PropertyNotify,
    ReparentNotify((Window, Window)),
    SelectionClear,
    Unknown,
}

impl From<xcb::x::Event> for SystrayEvent {
    fn from(value: xcb::x::Event) -> Self {
        match value {
            xcb::x::Event::ClientMessage(event) => Self::ClientMessage(event),
            xcb::x::Event::DestroyNotify(event) => Self::DestroyNotify(event.window()),
            xcb::x::Event::PropertyNotify(_) => Self::PropertyNotify,
            xcb::x::Event::ReparentNotify(event) => {
                Self::ReparentNotify((event.parent(), event.window()))
            }
            xcb::x::Event::SelectionClear(_) => Self::SelectionClear,
            _ => Self::Unknown,
        }
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
