use super::{OnClickCallback, Rectangle, Result, Text, Widget, WidgetConfig};
use crate::utils::{bytes_to_closest, HookSender, OnClickRaw, TimedHooks};
use crate::widget_default;
use cairo::Context;
use std::{fmt::Display, time::Duration};

#[derive(Debug)]
pub struct Disk {
    format: String,
    path: String,
    inner: Text,
    on_click: OnClickCallback,
}

impl Disk {
    ///* `format`
    ///  * *%p* will be replaced with the disk used percent
    ///  * *%u* will be replaced with the used disk
    ///  * *%f* will be replaced with the free disk
    ///  * *%t* will be replaced with the total disk
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(
        format: impl ToString,
        path: impl ToString,
        config: &WidgetConfig,
        on_click: Option<&'static OnClickRaw>,
    ) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            path: path.to_string(),
            inner: *Text::new("", config, None),
            on_click: OnClickCallback::new(on_click),
        })
    }
}

impl Widget for Disk {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        let disk_usage = psutil::disk::disk_usage(self.path.clone()).map_err(Error::from)?;
        let text = self
            .format
            .replace("%p", &disk_usage.percent().to_string())
            .replace("%u", &bytes_to_closest(disk_usage.used()))
            .replace("%f", &bytes_to_closest(disk_usage.free()))
            .replace("%t", &bytes_to_closest(disk_usage.total()));
        self.inner.set_text(text);
        Ok(())
    }

    fn hook(&mut self, sender: HookSender, timed_hooks: &mut TimedHooks) -> Result<()> {
        timed_hooks
            .subscribe(Duration::from_secs(5), sender)
            .map_err(Error::from)?;
        Ok(())
    }

    widget_default!(size);
    widget_default!(padding);
    widget_default!(on_click);
}

impl Display for Disk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Disk").fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum Error {
    HookChannel(#[from] crossbeam_channel::SendError<(Duration, HookSender)>),
    Psutil(#[from] psutil::Error),
}
