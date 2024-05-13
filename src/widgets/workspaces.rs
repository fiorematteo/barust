use crate::{
    utils::{set_source_rgba, Atoms, Color, HookSender, TimedHooks},
    widgets::{Rectangle, Result, Size, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::Context;
use log::debug;
use pango::{FontDescription, Layout};
use pangocairo::functions::{create_context, show_layout};
use std::{collections::HashSet, fmt::Display, thread};
use xcb::Connection;

pub fn get_desktops_names(connection: &Connection) -> Result<Vec<String>> {
    let atoms = Atoms::new(connection).map_err(Error::from)?;
    let cookie = connection.send_request(&xcb::x::GetProperty {
        delete: false,
        window: connection.get_setup().roots().next().unwrap().root(),
        property: atoms._NET_DESKTOP_NAMES,
        r#type: atoms.UTF8_STRING,
        long_offset: 0,
        long_length: u32::MAX,
    });
    let reply = connection.wait_for_reply(cookie).map_err(Error::Xcb)?;
    Ok(reply
        .value::<u8>()
        .split(|c| *c == 0)
        .filter_map(|s| String::from_utf8(s.to_vec()).ok())
        .collect::<Vec<String>>())
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum WorkspaceStatus {
    Active,
    Used,
    Empty,
}

/// Displays informations about the active workspaces
#[derive(Debug)]
pub struct Workspaces {
    padding: u32,
    fg_color: Color,
    font: String,
    font_size: f64,
    internal_padding: u32,
    active_workspace_color: Color,
    policy: Box<dyn WorkspaceHider>,
    status_provider: Box<dyn WorkspaceStatusProvider>,
    workspaces: Vec<(String, WorkspaceStatus)>,
}

impl Workspaces {
    ///* `active_workspace_color` color of the active workspace
    ///* `internal_padding` space to leave between workspaces name
    ///* `config` a [&WidgetConfig]
    pub async fn new(
        active_workspace_color: Color,
        internal_padding: u32,
        config: &WidgetConfig,
        policy: impl WorkspaceHider + 'static,
        status_provider: impl WorkspaceStatusProvider + 'static,
    ) -> Box<Self> {
        Box::new(Self {
            padding: config.padding,
            fg_color: config.fg_color,
            internal_padding,
            active_workspace_color,
            workspaces: Vec::new(),
            font: config.font.to_owned(),
            font_size: config.font_size,
            policy: Box::new(policy),
            status_provider: Box::new(status_provider),
        })
    }

    fn get_layout(&self, context: &Context) -> Result<Layout> {
        let pango_context = create_context(context);
        let layout = Layout::new(&pango_context);
        let mut font = FontDescription::from_string(&self.font);
        font.set_absolute_size(self.font_size * f64::from(pango::SCALE));
        layout.set_font_description(Some(&font));
        Ok(layout)
    }
}

#[async_trait]
impl Widget for Workspaces {
    fn draw(&self, context: Context, rectangle: &Rectangle) -> Result<()> {
        context.move_to(f64::from(self.padding), 0.0);
        let layout = self.get_layout(&context)?;
        let mut first = true;
        for (workspace, active) in &self.workspaces {
            let color = match active {
                WorkspaceStatus::Active => self.active_workspace_color,
                WorkspaceStatus::Used => self.fg_color,
                WorkspaceStatus::Empty => Color::new(0.4, 0.4, 0.4, 1.0),
            };
            if self.policy.should_hide(workspace, active) {
                continue;
            }
            set_source_rgba(&context, color);
            layout.set_text(workspace);
            if first {
                first = false;
                context.rel_move_to(
                    0.0,
                    f64::from((rectangle.height - layout.pixel_size().1 as u32) / 2),
                );
            }
            show_layout(&context, &layout);
            context.rel_move_to(
                f64::from(self.internal_padding) + f64::from(layout.pixel_size().0),
                0.0,
            );
        }
        Ok(())
    }

    async fn update(&mut self) -> Result<()> {
        debug!("updating workspaces");
        let (connection, _) = Connection::connect(None).map_err(Error::from)?;
        let Ok(workspaces) = get_desktops_names(&connection) else {
            return Ok(());
        };

        self.workspaces.clear();

        self.status_provider.update().await?;
        for (i, workspace) in workspaces.into_iter().enumerate() {
            let f = self.status_provider.status(&workspace, i);
            let new_status = f.await;
            self.workspaces.push((workspace, new_status));
        }

        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, _timed_hooks: &mut TimedHooks) -> Result<()> {
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
            if matches!(
                connection.wait_for_event(),
                Ok(xcb::Event::X(xcb::x::Event::PropertyNotify(_)))
            ) && sender.send_blocking().is_err()
            {
                break;
            }
        });
        Ok(())
    }

    fn size(&self, context: &Context) -> Result<Size> {
        let hidden_workspaces: HashSet<_> = self
            .workspaces
            .iter()
            .filter(|(w, status)| self.policy.should_hide(w, status))
            .map(|(w, _)| w)
            .collect();

        let layout = self.get_layout(context)?;
        let big_string = self
            .workspaces
            .iter()
            .filter(|(w, _)| !hidden_workspaces.contains(w))
            .map(|(text, _)| text.clone())
            .collect::<String>();

        layout.set_text(&big_string);
        let text_size: u32 = layout.pixel_size().0 as u32;
        let valid_workspaces = self.workspaces.len() - hidden_workspaces.len();

        Ok(Size::Static(
            text_size + (2 * self.padding) + ((valid_workspaces as u32) * self.internal_padding),
        ))
    }

    fn padding(&self) -> u32 {
        self.padding
    }
}

impl Display for Workspaces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Workspace").fmt(f)
    }
}

pub trait WorkspaceHider: std::fmt::Debug + Send {
    fn should_hide(&self, workspace: &str, status: &WorkspaceStatus) -> bool;
}

#[derive(Debug)]
pub struct NeverHide;

impl WorkspaceHider for NeverHide {
    fn should_hide(&self, _workspace: &str, _status: &WorkspaceStatus) -> bool {
        false
    }
}

#[async_trait]
pub trait WorkspaceStatusProvider: std::fmt::Debug + Send {
    async fn update(&mut self) -> Result<()>;
    async fn status(&self, workspaces: &str, index: usize) -> WorkspaceStatus;
}

pub struct ActiveProvider {
    connection: Connection,
    active_index: usize,
}

impl ActiveProvider {
    pub fn new() -> Result<Self> {
        let (connection, _) = Connection::connect(None).map_err(Error::from)?;
        Ok(Self {
            connection,
            active_index: 0,
        })
    }
}

impl std::fmt::Debug for ActiveProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt("ActiveProvider", f)
    }
}

#[async_trait]
impl WorkspaceStatusProvider for ActiveProvider {
    async fn status(&self, _workspace: &str, index: usize) -> WorkspaceStatus {
        if index == self.active_index {
            WorkspaceStatus::Active
        } else {
            WorkspaceStatus::Used
        }
    }
    async fn update(&mut self) -> Result<()> {
        self.active_index = get_current_desktop(&self.connection)? as usize;
        Ok(())
    }
}

pub fn get_current_desktop(connection: &Connection) -> Result<u32> {
    let atoms = Atoms::new(connection).map_err(Error::from)?;
    let cookie = connection.send_request(&xcb::x::GetProperty {
        delete: false,
        window: connection.get_setup().roots().next().unwrap().root(),
        property: atoms._NET_CURRENT_DESKTOP,
        r#type: xcb::x::ATOM_CARDINAL,
        long_offset: 0,
        long_length: u32::MAX,
    });
    let reply = connection.wait_for_reply(cookie).map_err(Error::Xcb)?;
    reply
        .value::<u32>()
        .first()
        .ok_or_else(|| Error::Ewmh.into())
        .map(|v| *v)
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("Ewmh")]
    Ewmh,
    #[error("Pango")]
    Pango,
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
