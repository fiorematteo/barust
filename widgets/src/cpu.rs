use crate::{widget_default, Rectangle, Result, Text, Widget, WidgetConfig};
use async_trait::async_trait;
use cairo::Context;
use log::debug;
use psutil::cpu::{CpuPercentCollector, CpuTimesPercentCollector};
use std::fmt::Display;
use utils::{HookSender, TimedHooks};

/// Displays cpu informations
#[derive(Debug)]
pub struct Cpu {
    format: String,
    per: CpuPercentCollector,
    times: CpuTimesPercentCollector,
    inner: Text,
}

impl Cpu {
    ///* `format`
    ///  * *%p* will be replaced with the cpu usage percentage
    ///  * *%u* will be replaced with the time spent in user mode
    ///  * *%s* will be replaced with the time spent in system mode
    ///  * *%i* will be replaced with the time spent idle
    ///  * *%b* will be replaced with the time spent busy
    ///* `config` a [&WidgetConfig]
    pub async fn new(format: impl ToString, config: &WidgetConfig) -> Result<Box<Self>> {
        Ok(Box::new(Self {
            format: format.to_string(),
            per: CpuPercentCollector::new().map_err(Error::from)?,
            times: CpuTimesPercentCollector::new().map_err(Error::from)?,
            inner: *Text::new("", config).await,
        }))
    }
}

#[async_trait]
impl Widget for Cpu {
    async fn update(&mut self) -> Result<()> {
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

    async fn hook(&mut self, sender: HookSender, timed_hooks: &mut TimedHooks) -> Result<()> {
        timed_hooks.subscribe(sender);
        Ok(())
    }

    widget_default!(draw, size, padding);
}

impl Display for Cpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Cpu").fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum Error {
    Psutil(#[from] psutil::Error),
}
