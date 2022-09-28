use super::{OptionCallback, Result, Text, Widget, WidgetConfig};
use cairo::{Context, Rectangle};
use log::debug;
use psutil::cpu::{CpuPercentCollector, CpuTimesPercentCollector};

/// Displays cpu informations
#[derive(Debug)]
pub struct Cpu {
    format: String,
    per: CpuPercentCollector,
    times: CpuTimesPercentCollector,
    inner: Text,
    on_click: OptionCallback<Self>,
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
        on_click: Option<fn(&mut Self)>,
    ) -> Result<Box<Self>> {
        Ok(Box::new(Self {
            format: format.to_string(),
            per: CpuPercentCollector::new().map_err(Error::from)?,
            times: CpuTimesPercentCollector::new().map_err(Error::from)?,
            inner: *Text::new("CPU", config, None),
            on_click: on_click.into(),
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

    fn size(&self, context: &Context) -> Result<f64> {
        self.inner.size(context)
    }

    fn padding(&self) -> f64 {
        self.inner.padding()
    }

    fn on_click(&mut self) {
        if let OptionCallback::Some(cb) = &self.on_click {
            cb(self);
        }
    }
}

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
pub enum Error {
    Psutil(psutil::Error),
}
