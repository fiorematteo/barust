use super::{OnClickCallback, Result, Text, Widget, WidgetConfig};
use crate::corex::{bytes_to_closest, EmptyCallback};
use cairo::{Context, Rectangle};
use log::debug;
use psutil::memory::virtual_memory;
use std::fmt::Display;

/// Displays memory informations
#[derive(Debug)]
pub struct Memory {
    format: String,
    inner: Text,
    on_click: OnClickCallback,
}

impl Memory {
    ///* `format`
    ///  * *%p* will be replaced with the usage percentage
    ///  * *%t* will be replaced with the total ram
    ///  * *%a* will be replaced with the available ram
    ///  * *%u* will be replaced with the used ram
    ///  * *%f* will be replaced with the free ram
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(
        format: impl ToString,
        config: &WidgetConfig,
        on_click: Option<&'static EmptyCallback>,
    ) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            inner: *Text::new("Memory", config, None),
            on_click: on_click.map(|c| c.into()),
        })
    }
}

impl Widget for Memory {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating memory");
        let ram = virtual_memory().map_err(Error::from)?;
        let text = self
            .format
            .replace("%p", &format!("{:.2}", ram.percent()))
            .replace("%t", &bytes_to_closest(ram.total()))
            .replace("%a", &bytes_to_closest(ram.available()))
            .replace("%u", &bytes_to_closest(ram.used()))
            .replace("%f", &bytes_to_closest(ram.free()));
        self.inner.set_text(text);
        Ok(())
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

impl Display for Memory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Memory").fmt(f)
    }
}

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
pub enum Error {
    Cairo(cairo::Error),
    Psutil(psutil::Error),
}
