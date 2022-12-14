use super::{OnClickCallback, Rectangle, Result, Size, Widget, WidgetConfig};
use crate::{
    utils::{set_source_rgba, Atoms, Color, HookSender, TimedHooks},
    widget_default,
};
use cairo::Context;
use log::debug;
use pango::{FontDescription, Layout};
use pangocairo::{create_context, show_layout};
use std::{fmt::Display, thread};
use xcb::Connection;

pub fn get_desktops_names(connection: &Connection, atoms: &Atoms) -> Result<Vec<String>> {
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

pub fn get_current_desktop(connection: &Connection, atoms: &Atoms) -> Result<u32> {
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

/// Displays informations about the active workspaces
#[derive(Debug)]
pub struct Workspaces {
    padding: u32,
    fg_color: Color,
    font: String,
    font_size: f64,
    on_click: OnClickCallback,
    internal_padding: u32,
    active_workspace_color: Color,
    pub workspaces: Vec<(String, bool)>,
}

impl Workspaces {
    ///* `active_workspace_color` color of the active workspace
    ///* `internal_padding` space to leave between workspaces name
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(
        active_workspace_color: Color,
        internal_padding: u32,
        config: &WidgetConfig,
    ) -> Box<Self> {
        Box::new(Self {
            padding: config.padding,
            fg_color: config.fg_color,
            on_click: config.on_click.map(|cb| cb.into()),
            internal_padding,
            active_workspace_color,
            workspaces: Vec::new(),
            font: config.font.into(),
            font_size: config.font_size,
        })
    }

    fn get_layout(&self, context: &Context) -> Result<Layout> {
        let pango_context = create_context(context).ok_or(Error::Pango)?;
        let layout = Layout::new(&pango_context);
        let mut font = FontDescription::from_string(&self.font);
        font.set_absolute_size(self.font_size * f64::from(pango::SCALE));
        layout.set_font_description(Some(&font));
        Ok(layout)
    }
}

impl Widget for Workspaces {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        context.move_to(f64::from(self.padding), 0.0);
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
                context.rel_move_to(
                    0.0,
                    f64::from((rectangle.height - layout.pixel_size().1 as u32) / 2),
                );
            }
            show_layout(context, &layout);
            context.rel_move_to(
                f64::from(self.internal_padding) + f64::from(layout.pixel_size().0),
                0.0,
            );
        }
        Ok(())
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating workspaces");
        let (connection, _) = Connection::connect(None).map_err(Error::from)?;
        let atoms = Atoms::new(&connection).map_err(Error::from)?;

        let Ok(workspace) = get_desktops_names(&connection, &atoms) else {
            return Ok(())
        };
        let Ok(index) = get_current_desktop(&connection, &atoms) else {
            return Ok(())
        };
        self.workspaces = workspace.iter().map(|w| (w.to_owned(), false)).collect();
        if let Some(active_workspace) = self.workspaces.get_mut(index as usize) {
            active_workspace.1 = true;
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

    fn size(&self, context: &Context) -> Result<Size> {
        let layout = self.get_layout(context)?;
        let big_string = self
            .workspaces
            .iter()
            .map(|(text, _)| text.clone())
            .collect::<Vec<_>>()
            .join("");
        layout.set_text(&big_string);
        let text_size: u32 = layout.pixel_size().0 as u32;
        Ok(Size::Static(
            text_size + (2 * self.padding) + (self.workspaces.len() as u32 * self.internal_padding),
        ))
    }

    fn padding(&self) -> u32 {
        self.padding
    }

    widget_default!(on_click);
}

impl Display for Workspaces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Workspace").fmt(f)
    }
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
