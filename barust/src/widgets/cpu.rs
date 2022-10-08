use super::{OnClickCallback, Result, Text, Widget, WidgetConfig};
use crate::corex::RawCallback;
use cairo::{Context, Rectangle};
use log::debug;
use psutil::cpu::{CpuPercentCollector, CpuTimesPercentCollector};
use std::{
    fmt::Display,
    time::{Duration, SystemTime, SystemTimeError},
};

/// Displays cpu informations
#[derive(Debug)]
pub struct Cpu {
    format: String,
    per: CpuPercentCollector,
    times: CpuTimesPercentCollector,
    last_update: SystemTime,
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
        format: &str,
        config: &WidgetConfig,
        on_click: Option<&'static RawCallback<(), ()>>,
    ) -> Result<Box<Self>> {
        Ok(Box::new(Self {
            format: format.to_string(),
            per: CpuPercentCollector::new().map_err(Error::from)?,
            times: CpuTimesPercentCollector::new().map_err(Error::from)?,
            last_update: SystemTime::now(),
            inner: *Text::new("CPU", config, None),
            on_click: on_click.map(|c| c.into()),
        }))
    }
}

impl Widget for Cpu {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn first_update(&mut self) -> Result<()> {
        debug!("updating cpu");
        let times = self.times.cpu_times_percent().map_err(Error::from)?;
        let text = self
            .format
            .replace(
                "%p",
                &format!("{:.1}", self.per.cpu_percent().map_err(Error::from)?),
            )
            .replace("%u", &format!("{:.1}", times.user()))
            .replace("%s", &format!("{:.1}", times.system()))
            .replace("%i", &format!("{:.1}", times.idle()))
            .replace("%b", &format!("{:.1}", times.busy()));
        self.inner.set_text(text);
        Ok(())
    }

    fn update(&mut self) -> Result<()> {
        if self.last_update.elapsed().map_err(Error::from)? < Duration::from_secs(1) {
            return Ok(());
        }
        self.last_update = SystemTime::now();
        self.first_update()
    }

    fn size(&self, context: &Context) -> Result<f64> {
        self.inner.size(context)
    }

    fn padding(&self) -> f64 {
        self.inner.padding()
    }

    fn on_click(&self) {
        if let Some(cb) = &self.on_click {
            cb.call(());
        }
    }
}

impl Display for Cpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Cpu").fmt(f)
    }
}

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
pub enum Error {
    Psutil(psutil::Error),
    SystemTime(SystemTimeError),
}
