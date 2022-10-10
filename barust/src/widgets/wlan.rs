use super::{OnClickCallback, Result, Text, Widget, WidgetConfig};
use crate::corex::EmptyCallback;
use cairo::{Context, Rectangle};
use log::debug;
use std::fmt::Display;

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
    ///* `icons` sets a custom [NetworkIcons]
    ///* `interface` name of the network interface
    ///* `fg_color` foreground color
    ///* `on_click` callback to run on click
    pub fn new(
        format: &str,
        interface: String,
        config: &WidgetConfig,
        on_click: Option<&'static EmptyCallback>,
    ) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            interface,
            inner: *Text::new("Up", config, None),
            on_click: on_click.map(|c| c.into()),
        })
    }
}

impl Widget for Wlan {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating wlan");
        let text = if let Some(data) = iwlib::get_wireless_info(self.interface.clone()) {
            self.format
                .replace("%i", &self.interface)
                .replace("%e", &data.wi_essid)
                .replace("%q", &data.wi_quality.to_string())
        } else {
            "No interface".to_string()
        };
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

impl Display for Wlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Network").fmt(f)
    }
}

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
pub enum Error {
    IO(std::io::Error),
}
