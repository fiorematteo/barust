use super::{OptionCallback, Result, Widget, WidgetConfig};
use crate::corex::{set_source_rgba, Color};
use cairo::{Context, Rectangle};
use log::debug;
use pango::{FontDescription, Layout};
use pangocairo::{create_context, show_layout};
use std::{cell::RefCell, thread};
use xcb::Connection;

pub fn get_desktops_names(connection: &Connection) -> Result<Vec<String>> {
    let ewmh_connection = xcb_wm::ewmh::Connection::connect(connection);
    let cookie = ewmh_connection.send_request(&xcb_wm::ewmh::proto::GetDesktopNames);
    let reply = ewmh_connection.wait_for_reply(cookie).map_err(Error::Xcb)?;
    Ok(reply.names)
}

pub fn get_current_desktop(connection: &Connection) -> Result<u32> {
    let ewmh_connection = xcb_wm::ewmh::Connection::connect(connection);
    let cookie = ewmh_connection.send_request(&xcb_wm::ewmh::proto::GetCurrentDesktop);
    let reply = ewmh_connection.wait_for_reply(cookie).map_err(Error::Xcb)?;
    Ok(reply.desktop)
}

/// Displays informations about the active workspaces
#[derive(Debug)]
pub struct Workspace {
    padding: f64,
    fg_color: Color,
    font: String,
    font_size: f64,
    on_click: OptionCallback<Self>,
    internal_padding: f64,
    active_workspace_color: Color,
    pub workspaces: Vec<(String, bool)>,
    cached_layout: RefCell<Option<Layout>>,
}

impl Workspace {
    ///* `active_workspace_color` color of the active workspace
    ///* `internal_padding` space to leave between workspaces name
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(
        active_workspace_color: Color,
        internal_padding: f64,
        config: &WidgetConfig,
        on_click: Option<fn(&mut Self)>,
    ) -> Box<Self> {
        Box::new(Self {
            padding: config.padding,
            fg_color: config.fg_color,
            on_click: on_click.into(),
            internal_padding,
            active_workspace_color,
            workspaces: Vec::new(),
            font: config.font.into(),
            font_size: config.font_size,
            cached_layout: RefCell::default(),
        })
    }

    fn get_layout(&self, context: &Context) -> Result<Layout> {
        if let Some(layout) = self.cached_layout.take() {
            self.cached_layout.replace(Some(layout.clone()));
            Ok(layout)
        } else {
            let pango_context = create_context(context).ok_or(Error::PangoError)?;
            let layout = Layout::new(&pango_context);
            let mut font = FontDescription::from_string(&self.font);
            font.set_absolute_size(self.font_size * pango::SCALE as f64);
            layout.set_font_description(Some(&font));
            self.cached_layout.replace(Some(layout.clone()));
            Ok(layout)
        }
    }
}

impl Widget for Workspace {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        context.move_to(self.padding, 0.0);
        let layout = self.get_layout(context)?;
        let mut first = true;
        for (workspace, active) in &self.workspaces {
            if *active {
                set_source_rgba(context, self.active_workspace_color);
            } else {
                set_source_rgba(context, self.fg_color);
            }
            layout.set_text(workspace);
            if first {
                first = false;
                context.rel_move_to(0.0, (rectangle.height - layout.pixel_size().1 as f64) / 2.0);
            }
            show_layout(context, &layout);
            context.rel_move_to(self.internal_padding + layout.pixel_size().0 as f64, 0.0);
        }
        Ok(())
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating workspaces");
        let (connection, _) = Connection::connect(None).map_err(Error::from)?;
        let workspace = get_desktops_names(&connection)?;
        let index = get_current_desktop(&connection)?;
        self.workspaces = workspace.iter().map(|w| (w.to_owned(), false)).collect();
        if let Some(active_workspace) = self.workspaces.get_mut(index as usize) {
            active_workspace.1 = true;
        }
        Ok(())
    }

    fn size(&self, context: &Context) -> Result<f64> {
        let layout = self.get_layout(context)?;
        let big_string = self
            .workspaces
            .iter()
            .map(|(text, _)| text.clone())
            .collect::<Vec<_>>()
            .join("");
        layout.set_text(&big_string);
        let text_size = layout.pixel_size().0 as f64;
        Ok(text_size
            + (2.0 * self.padding)
            + (self.workspaces.len() as f64 * self.internal_padding))
    }

    fn padding(&self) -> f64 {
        self.padding
    }

    fn on_click(&mut self) {
        if let OptionCallback::Some(cb) = &self.on_click {
            cb(self);
        }
    }

    fn hook(&mut self, sender: chan::Sender<()>) -> Result<()> {
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
            if let Ok(xcb::Event::X(_)) = connection.wait_for_event() {
                sender.send(());
            }
        });
        Ok(())
    }
}

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
pub enum Error {
    PangoError,
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
