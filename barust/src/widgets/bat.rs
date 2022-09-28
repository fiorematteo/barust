use super::{OptionCallback, Result, Text, Widget, WidgetConfig};
use cairo::{Context, Rectangle};
use log::debug;
use std::{
    fs::{read_dir, File},
    io::Read,
};

/// Icons used by [Battery]
#[derive(Debug)]
pub struct BatteryIcons {
    ///displayed if `charge > 90%`
    pub full: String,
    ///displayed if `70% < charge < 90%`
    pub most: String,
    ///displayed if `40% < charge < 70%`
    pub half: String,
    ///displayed if `10% < charge < 40%`
    pub few: String,
    ///displayed if `charge < 10%`
    pub empty: String,
    ///displayed if the device is charging
    pub charging: String,
}

impl Default for BatteryIcons {
    fn default() -> Self {
        Self {
            full: String::from("▇"),
            most: String::from("▆"),
            half: String::from("▅"),
            few: String::from("▃"),
            empty: String::from("▂"),
            charging: String::from("🗲"),
        }
    }
}
/// Displays status and charge of the battery
#[derive(Debug)]
pub struct Battery {
    format: String,
    inner: Text,
    root_path: String,
    icons: BatteryIcons,
    on_click: OptionCallback<Self>,
}

impl Battery {
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
        on_click: Option<fn(&mut Self)>,
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
        let mut value = String::default();
        let mut file = File::open(format!("{}/{}", self.root_path, filename)).ok()?;
        file.read_to_string(&mut value).ok()?;
        value.pop(); //removes /n
        Some(value)
    }
}

impl Widget for Battery {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating battery");
        let status = &self.read_os_file("status");
        let status = if let Some(status) = status {
            status
        } else {
            return Ok(());
        };

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

        let text = self
            .format
            .replace(
                "%i",
                if status == "Charging" {
                    &self.icons.charging
                } else if percent > 90.0 {
                    &self.icons.full
                } else if percent > 70.0 {
                    &self.icons.most
                } else if percent > 40.0 {
                    &self.icons.half
                } else if percent > 10.0 {
                    &self.icons.few
                } else {
                    &self.icons.empty
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

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
pub enum Error {
    IO(std::io::Error),
    NoBattery,
}
