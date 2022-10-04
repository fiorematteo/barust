use super::{OptionCallback, Result, Text, Widget, WidgetConfig};
use cairo::{Context, Rectangle};
use log::debug;
use psutil::sensors::temperatures;
use std::fmt::Display;

/// Displays the average temperature read by the device sensors
#[derive(Debug)]
pub struct Temperatures {
    format: String,
    inner: Text,
    on_click: OptionCallback<Self>,
}

impl Temperatures {
    ///* `format`
    ///  * `%t` will be replaced with the temperature in celsius
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(format: &str, config: &WidgetConfig, on_click: Option<fn(&mut Self)>) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            inner: *Text::new("CPU", config, None),
            on_click: on_click.into(),
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

impl Display for Temperatures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Temperatures").fmt(f)
    }
}
