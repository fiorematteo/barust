use crate::corex::{OptionCallback, SelfCallback};

use super::{Result, Text, Widget, WidgetConfig};
use cairo::{Context, Rectangle};
use log::debug;
use std::{fmt::Display, fs::read_dir, cmp::min};

/// Icons used by [Battery]
#[derive(Debug)]
pub struct BatteryIcons {
    pub percentages: [String; 10],
    ///displayed if the device is charging
    pub charging: String,
}

impl Default for BatteryIcons {
    fn default() -> Self {
        let percentages = ['', '', '', '', '', '', '', '', '', ''];
        Self {
            percentages: percentages.map(String::from),
            charging: String::from('🗲'),
        }
    }
}
/// Displays status and charge of the battery
#[derive(Debug)]
pub struct Battery<'a> {
    format: String,
    inner: Text<'a>,
    root_path: String,
    icons: BatteryIcons,
    on_click: OptionCallback<'a, Self>,
}

impl<'a> Battery<'a> {
    ///* `format`
    ///  * `%c` will be replaced with the charge percentage
    ///  * `%i` will be replaced with the correct icon from `icons`
    ///* `icons` sets a custom [BatteryIcons]
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(
        format: &str,
        icons: Option<BatteryIcons>,
        config: &WidgetConfig,
        on_click: Option<&'a SelfCallback<Self>>,
    ) -> Result<Box<Self>> {
        let mut root_path = String::default();
        for path in read_dir("/sys/class/power_supply")
            .map_err(Error::from)?
            .flatten()
        {
            let name = String::from(path.path().to_str().unwrap());
            if name.contains("BAT") {
                root_path.clone_from(&name);
                break;
            }
        }
        if root_path.is_empty() {
            return Err(Error::NoBattery.into());
        }

        Ok(Box::new(Self {
            format: format.to_string(),
            inner: *Text::new("CPU", config, None),
            root_path,
            icons: icons.unwrap_or_default(),
            on_click: on_click.into(),
        }))
    }

    #[inline(always)]
    fn read_os_file(&self, filename: &str) -> Option<String> {
        let path = format!("{}/{}", self.root_path, filename);
        let mut value: String = std::fs::read_to_string(path).ok()?;
        value.pop(); //removes /n
        Some(value)
    }
}

impl Widget for Battery<'_> {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating battery");
        let status = &self.read_os_file("status");
        let charge = (|| -> Option<f64> {
            Some(
                self.read_os_file("charge_now")?.parse::<f64>().ok()?
                    / self.read_os_file("charge_full")?.parse::<f64>().ok()?
                    * 100.0,
            )
        })();

        let energy = (|| -> Option<f64> {
            Some(
                self.read_os_file("energy_now")?.parse::<f64>().ok()?
                    / self.read_os_file("energy_full")?.parse::<f64>().ok()?
                    * 100.0,
            )
        })();

        let percent = match (charge, energy) {
            (Some(c), Some(_)) => c,
            (Some(c), None) => c,
            (None, Some(e)) => e,
            (None, None) => return Ok(()),
        };

        let index = min((percent / 10.0).floor() as usize, 9);
        let text = self
            .format
            .replace(
                "%i",
                if status.as_ref().map(|s| s == "Charging").unwrap_or(false) {
                    &self.icons.charging
                } else {
                    &self.icons.percentages[index]
                },
            )
            .replace("%c", &percent.round().to_string());
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
        if let OptionCallback::Some(cb) = self.on_click {
            cb(self);
        }
    }
}

impl Display for Battery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Battery").fmt(f)
    }
}

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
pub enum Error {
    IO(std::io::Error),
    NoBattery,
}
