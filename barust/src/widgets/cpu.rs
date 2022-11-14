use super::{OnClickCallback, Rectangle, Result, Text, Widget, WidgetConfig};
use crate::{
    corex::{EmptyCallback, HookSender, TimedHooks},
    widget_default,
};
use cairo::Context;
use log::debug;
use psutil::cpu::{CpuPercentCollector, CpuTimesPercentCollector};
use std::{fmt::Display, time::Duration};

/// Displays cpu informations
#[derive(Debug)]
pub struct Cpu {
    format: String,
    per: CpuPercentCollector,
    times: CpuTimesPercentCollector,
    inner: Text,
    on_click: OnClickCallback,
}

impl Cpu {
    ///* `format`
    ///  * *%p* will be replaced with the cpu usage percentage
    ///  * *%u* will be replaced with the time spent in user mode
    ///  * *%s* will be replaced with the time spent in system mode
    ///  * *%i* will be replaced with the time spent idle
    ///  * *%b* will be replaced with the time spent busy
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(
        format: impl ToString,
        config: &WidgetConfig,
        on_click: Option<&'static EmptyCallback>,
    ) -> Result<Box<Self>> {
        Ok(Box::new(Self {
            format: format.to_string(),
            per: CpuPercentCollector::new().map_err(Error::from)?,
            times: CpuTimesPercentCollector::new().map_err(Error::from)?,
            inner: *Text::new("", config, None),
            on_click: on_click.map(|c| c.into()),
        }))
    }
}

impl Widget for Cpu {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating cpu");
        let times = self.times.cpu_times_percent().map_err(Error::from)?;
        let cpu_percent = self.per.cpu_percent().map_err(Error::from)?;
        let text = self
            .format
            .replace("%p", &format!("{: >4.1}", cpu_percent))
            .replace("%u", &format!("{: >4.1}", times.user()))
            .replace("%s", &format!("{: >4.1}", times.system()))
            .replace("%i", &format!("{: >4.1}", times.idle()))
            .replace("%b", &format!("{: >4.1}", times.busy()));
        self.inner.set_text(text);
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

impl Display for Cpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Cpu").fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum Error {
    HookChannel(#[from] crossbeam_channel::SendError<(Duration, HookSender)>),
    Psutil(#[from] psutil::Error),
}
