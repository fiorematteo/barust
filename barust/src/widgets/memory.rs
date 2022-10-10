use super::{OnClickCallback, Result, Text, Widget, WidgetConfig};
use crate::corex::EmptyCallback;
use cairo::{Context, Rectangle};
use log::debug;
use psutil::{memory::virtual_memory, Bytes};
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
        format: &str,
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

fn bytes_to_closest(value: Bytes) -> String {
    if value == 0 {
        return "0B".to_string();
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut selected_unit: usize = 0;
    let mut value = value as f64;
    while value > 1024.0 {
        if selected_unit == 4 {
            break;
        }
        value /= 1024.0;
        selected_unit += 1;
    }
    format!("{:.1}{}", value, units[selected_unit])
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
