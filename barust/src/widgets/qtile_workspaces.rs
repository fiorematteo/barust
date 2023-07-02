use super::{OnClickCallback, Rectangle, Result, Widget, WidgetConfig, Workspaces};
use crate::{
    utils::{Color, HookSender, TimedHooks},
    widget_default,
    widgets::workspaces::WorkspaceStatus,
};
use cairo::Context;
use log::{debug, error};
use pyo3::prelude::*;
use pyo3::{types::PyModule, Python};
use std::{collections::HashMap, fmt::Display};

#[derive(Debug)]
pub struct QtileWorkspaces {
    on_click: OnClickCallback,
    inner: Workspaces,
    python_module: Py<PyModule>,
}

impl QtileWorkspaces {
    ///* `active_workspace_color` color of the active workspace
    ///* `internal_padding` space to leave between workspaces name
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(
        active_workspace_color: Color,
        internal_padding: u32,
        config: &WidgetConfig,
        ignored_workspaces: &[impl ToString],
    ) -> Box<Self> {
        let inner = Workspaces::new(
            active_workspace_color,
            internal_padding,
            config,
            ignored_workspaces,
        );
        let on_click = config.on_click.map(|cb| cb.into());
        let python_module = Python::with_gil(|py| -> PyResult<Py<PyModule>> {
            Ok(PyModule::from_code(
                py,
                r#"from collections import Counter
from libqtile.command.client import CommandClient

c = CommandClient()
def windows():
    windows = c.call("windows")
    return dict(Counter([(w["group"]) for w in windows if w["group"]]))"#,
                "",
                "",
            )?
            .into())
        })
        .unwrap();
        Box::new(Self {
            on_click,
            python_module,
            inner: *inner,
        })
    }
}

impl Widget for QtileWorkspaces {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating qtile workspaces");
        self.inner.update()?;
        let group_count = Python::with_gil(|py| -> PyResult<HashMap<String, usize>> {
            self.python_module
                .getattr(py, "windows")?
                .call0(py)?
                .extract::<HashMap<String, usize>>(py)
        })
        .unwrap();
        for (workspace, status) in self.inner.workspaces.iter_mut() {
            if *status == WorkspaceStatus::Active {
                continue;
            }
            if let Some(&count) = group_count.get(workspace) {
                assert!(count > 0);
                *status = WorkspaceStatus::Used;
            } else {
                *status = WorkspaceStatus::Empty;
            }
        }
        Ok(())
    }

    fn hook(&mut self, sender: HookSender, _timed_hooks: &mut TimedHooks) -> Result<()> {
        self.inner.hook(sender, _timed_hooks)
    }

    widget_default!(size, padding, on_click);
}

impl Display for QtileWorkspaces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("QtileWorkspace").fmt(f)
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
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
