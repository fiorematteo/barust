use super::{Result, Widget, WidgetConfig, Workspaces};
use crate::corex::{Color, HookSender, TimedHooks};
use std::fmt::Display;

#[derive(Debug)]
pub struct FilteredWorkspaces {
    inner: Workspaces,
    ignored_workspaces: Vec<String>,
}

impl FilteredWorkspaces {
    pub fn new<T: ToString>(
        active_workspace_color: Color,
        internal_padding: f64,
        config: &WidgetConfig,
        ignored_workspaces: &[T],
    ) -> Box<Self> {
        let inner = *Workspaces::new(active_workspace_color, internal_padding, config, None);
        Box::new(Self {
            inner,
            ignored_workspaces: ignored_workspaces.iter().map(|w| w.to_string()).collect(),
        })
    }
}

impl Widget for FilteredWorkspaces {
    fn draw(&self, context: &cairo::Context, rectangle: &cairo::Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn size(&self, context: &cairo::Context) -> Result<f64> {
        self.inner.size(context)
    }

    fn padding(&self) -> f64 {
        self.inner.padding()
    }

    fn update(&mut self) -> Result<()> {
        self.inner.update()?;

        if self.ignored_workspaces.is_empty() {
            return Err(Error::EmptyFilter.into());
        }

        self.inner
            .workspaces
            .retain(|name| !self.ignored_workspaces.contains(&name.0));
        Ok(())
    }

    fn hook(&mut self, sender: HookSender, pool: &mut TimedHooks) -> Result<()> {
        self.inner.hook(sender, pool)
    }
}

impl Display for FilteredWorkspaces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FilteredWorkspace")
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("The filter is empty")]
    EmptyFilter,
}
