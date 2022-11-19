use super::{OnClickCallback, Rectangle, Result, Text, Widget, WidgetConfig};
use crate::{
    utils::{HookSender, TimedHooks},
    widget_default,
};
use cairo::Context;
use chrono::Local;
use log::debug;
use std::{
    fmt::{Debug, Display},
    time::Duration,
};

/// Displays a datetime
pub struct Clock {
    format: String,
    inner: Text,
    on_click: OnClickCallback,
}

impl Debug for Clock {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Clock(format: {}, padding: {})",
            self.format,
            self.inner.padding(),
        )
    }
}

impl Clock {
    ///* `format` describes how to display the time following [chrono format rules](chrono::format::strftime)
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(format: impl ToString, config: &WidgetConfig) -> Box<Self> {
        let format = format.to_string();
        Box::new(Self {
            inner: *Text::new("", config),
            format,
            on_click: config.on_click.map(|cb| cb.into()),
        })
    }

    #[inline(always)]
    fn current_time_str(format: &str) -> String {
        Local::now().format(format).to_string()
    }
}

impl Widget for Clock {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating clock");
        self.inner.set_text(Self::current_time_str(&self.format));
        Ok(())
    }

    fn hook(&mut self, sender: HookSender, timed_hooks: &mut TimedHooks) -> Result<()> {
        timed_hooks
            .subscribe(Duration::from_secs(1), sender)
            .map_err(Error::from)?;
        Ok(())
    }

    widget_default!(size);
    widget_default!(padding);
    widget_default!(on_click);
}

impl Display for Clock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from("Clock"))
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum Error {
    HookChannel(#[from] crossbeam_channel::SendError<(Duration, HookSender)>),
}
