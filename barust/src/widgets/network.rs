use super::{OnClickCallback, Result, Text, Widget, WidgetConfig};
use crate::corex::{OptionCallback, RawCallback};
use cairo::{Context, Rectangle};
use log::debug;
use std::{
    fmt::Display,
    fs::{metadata, read_to_string},
};

fn get_interface_stats(ifname: &str) -> Result<(bool, bool)> {
    metadata(format!("/sys/class/net/{}", ifname)).map_err(Error::from)?;
    let wireless = metadata(format!("/sys/class/net/{}/wireless", ifname)).is_ok();
    let operstate =
        read_to_string(format!("/sys/class/net/{}/operstate", ifname)).map_err(Error::from)?;
    Ok((wireless, operstate == "up\n"))
}

/// Icons used by [Network]
#[derive(Debug)]
pub struct NetworkIcons {
    ///displayed if the interface is wireless
    pub wireless: String,
    ///displayed if the interface is wired
    pub ethernet: String,
    ///displayed if the interface is connected
    pub online: String,
    ///displayed if the interface is not connected
    pub offline: String,
}

impl Default for NetworkIcons {
    fn default() -> Self {
        Self {
            wireless: String::from("W"),
            ethernet: String::from("E"),
            online: String::from("Connected"),
            offline: String::from("Offline"),
        }
    }
}

/// Displays informations about a network interface
#[derive(Debug)]
pub struct Network {
    format: String,
    interface: String,
    icons: NetworkIcons,
    inner: Text,
    on_click: OnClickCallback,
}

impl Network {
    ///* `format`
    ///  * `%n` will be replaced with the interface name
    ///  * `%s` will be replaced with the interface status
    ///  * `%t` will be replaced with the interface type
    ///* `icons` sets a custom [NetworkIcons]
    ///* `interface` name of the network interface
    ///* `fg_color` foreground color
    ///* `on_click` callback to run on click
    pub fn new(
        format: &str,
        interface: String,
        icons: Option<NetworkIcons>,
        config: &WidgetConfig,
        on_click: Option<&'static RawCallback<(), ()>>,
    ) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            interface,
            inner: *Text::new("Up", config, None),
            on_click: on_click.into(),
            icons: icons.unwrap_or_default(),
        })
    }
}

impl Widget for Network {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating network");
        let text = if let Ok((wireless, online)) = get_interface_stats(&self.interface) {
            self.format
                .replace("%n", &self.interface)
                .replace("%s", {
                    if online {
                        self.icons.online.as_str()
                    } else {
                        self.icons.offline.as_str()
                    }
                })
                .replace("%t", {
                    if wireless {
                        self.icons.wireless.as_str()
                    } else {
                        self.icons.ethernet.as_str()
                    }
                })
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
        if let OptionCallback::Some(cb) = &self.on_click {
            cb.call(());
        }
    }
}

impl Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Network").fmt(f)
    }
}

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
pub enum Error {
    IO(std::io::Error),
}
