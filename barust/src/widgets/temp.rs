use super::{OnClickCallback, Rectangle, Result, Text, Widget, WidgetConfig};
use crate::{utils::HookSender, widget_default};
use cairo::Context;
use log::debug;
use psutil::sensors::temperatures;
use std::{fmt::Display, time::Duration};

/// Displays the average temperature read by the device sensors
#[derive(Debug)]
pub struct Temperatures {
    format: String,
    inner: Text,
    on_click: OnClickCallback,
}

impl Temperatures {
    ///* `format`
    ///  * `%t` will be replaced with the temperature in celsius
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(format: impl ToString, config: &WidgetConfig) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            inner: *Text::new("", config),
            on_click: config.on_click.map(|cb| cb.into()),
        })
    }
}

impl Widget for Temperatures {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating temp");
        let mut temp: f64 = 0.0;
        let mut count: f64 = 0.0;
        for elem in temperatures().iter().flatten() {
            temp += elem.current().celsius();
            count += 1.0;
        }
        let text = self.format.replace("%t", &format!("{:.1}", temp / count));
        self.inner.set_text(text);
        Ok(())
    }

    fn hook(&mut self, sender: HookSender, pool: &mut crate::utils::TimedHooks) -> Result<()> {
        pool.subscribe(Duration::from_secs(5), sender)
            .map_err(Error::from)?;
        Ok(())
    }

    widget_default!(size, padding, on_click);
}

impl Display for Temperatures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Temperatures").fmt(f)
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    HookChannel(#[from] crossbeam_channel::SendError<(Duration, HookSender)>),
}
