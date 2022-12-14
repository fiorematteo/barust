use super::{OnClickCallback, Rectangle, Result, Text, Widget, WidgetConfig};
use crate::{
    utils::{HookSender, TimedHooks},
    widget_default,
};
use cairo::Context;
use log::debug;
use std::{fmt::Display, time::Duration};

/// Displays informations about a network interface
#[derive(Debug)]
pub struct Wlan {
    format: String,
    interface: String,
    inner: Text,
    on_click: OnClickCallback,
}

impl Wlan {
    ///* `format`
    ///  * `%i` will be replaced with the interface name
    ///  * `%e` will be replaced with the essid
    ///  * `%q` will be replaced with the signal quality
    ///* `interface` name of the network interface
    ///* `fg_color` foreground color
    ///* `on_click` callback to run on click
    pub fn new(format: impl ToString, interface: String, config: &WidgetConfig) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            interface,
            inner: *Text::new("", config),
            on_click: config.on_click.map(|cb| cb.into()),
        })
    }

    fn build_string(&self) -> String {
        let Some(data) = iwlib::get_wireless_info(self.interface.clone()) else {
            return String::from("No interface")
        };
        self.format
            .replace("%i", &self.interface)
            .replace("%e", &data.wi_essid)
            .replace("%q", &data.wi_quality.to_string())
    }
}

impl Widget for Wlan {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }
    fn update(&mut self) -> Result<()> {
        debug!("updating wlan");
        let text = self.build_string();
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

impl Display for Wlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Network").fmt(f)
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    HookChannel(#[from] crossbeam_channel::SendError<(Duration, HookSender)>),
}
