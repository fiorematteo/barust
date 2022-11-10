use super::{OnClickCallback, Rectangle, Result, Text, Widget, WidgetConfig};
use crate::{
    corex::{EmptyCallback, HookSender, TimedHooks},
    forward_to_inner,
};
use cairo::Context;
use log::debug;
use std::{cmp::min, fmt::Display, fs::read_dir, time::Duration};

/// Icons used by [Battery]
#[derive(Debug)]
pub struct BatteryIcons {
    pub percentages: Vec<String>,
    ///displayed if the device is charging
    pub percentages_charging: Vec<String>,
}

impl Default for BatteryIcons {
    fn default() -> Self {
        let percentages = ['', '', '', '', '', '', '', '', '', '']
            .map(String::from)
            .to_vec();
        let percentages_charging = ['', '', '', '', '', '', '']
            .map(String::from)
            .to_vec();
        Self {
            percentages,
            percentages_charging,
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
    on_click: OnClickCallback,
}

impl Battery {
    ///* `format`
    ///  * `%c` will be replaced with the charge percentage
    ///  * `%i` will be replaced with the correct icon from `icons`
    ///* `icons` sets a custom [BatteryIcons]
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(
        format: impl ToString,
        icons: Option<BatteryIcons>,
        config: &WidgetConfig,
        on_click: Option<&'static EmptyCallback>,
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
            inner: *Text::new("", config, None),
            root_path,
            icons: icons.unwrap_or_default(),
            on_click: on_click.map(|c| c.into()),
        }))
    }

    fn read_os_file(&self, filename: &str) -> Option<String> {
        let path = format!("{}/{}", self.root_path, filename);
        let value = std::fs::read_to_string(path).ok()?;
        Some(value.trim().into())
    }

    fn get_charge(&self) -> Option<f64> {
        self.get_from_files("charge_now", "charge_full")
    }

    fn get_energy(&self) -> Option<f64> {
        self.get_from_files("energy_now", "energy_full")
    }

    fn get_from_files(&self, f1: &str, f2: &str) -> Option<f64> {
        let Some(Ok(v1)) = self.read_os_file(f1).map(|s| s.parse::<f64>()) else {return None;};
        let Some(Ok(v2)) = self.read_os_file(f2).map(|s| s.parse::<f64>()) else {return None;};
        Some(v1 / v2 * 100.0)
    }
}

impl Widget for Battery {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating battery");
        let percent = match (self.get_charge(), self.get_energy()) {
            (Some(c), Some(_)) => c,
            (Some(c), None) => c,
            (None, Some(e)) => e,
            (None, None) => return Ok(()),
        };

        let percentages = if self.read_os_file("status") == Some("Charging".into()) {
            &self.icons.percentages_charging
        } else {
            &self.icons.percentages
        };

        let icon = {
            let percentages_len = percentages.len();
            let index = min(
                (percent / percentages_len as f64).floor() as usize,
                percentages_len - 1,
            );
            &percentages[index]
        };

        let text = self
            .format
            .replace("%i", icon)
            .replace("%c", &percent.round().to_string());
        self.inner.set_text(text);
        Ok(())
    }

    fn hook(&mut self, sender: HookSender, timed_hooks: &mut TimedHooks) -> Result<()> {
        timed_hooks
            .subscribe(Duration::from_secs(5), sender)
            .map_err(Error::from)?;
        Ok(())
    }

    forward_to_inner!(size);
    forward_to_inner!(padding);
    forward_to_inner!(on_click);
}

impl Display for Battery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Battery").fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum Error {
    IO(#[from] std::io::Error),
    HookChannel(#[from] crossbeam_channel::SendError<(Duration, HookSender)>),
    #[error("No battery found")]
    NoBattery,
}
