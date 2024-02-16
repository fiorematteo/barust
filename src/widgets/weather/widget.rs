use crate::utils::{HookSender, ResettableTimer, TimedHooks};
use crate::widgets::weather::openmeteo::get_current_meteo;
use crate::{
    widget_default,
    widgets::{Rectangle, Result, Text, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::Context;
use log::debug;
use std::fmt::Display;

/// A set of strings used as icons in the Weather widget
#[derive(Debug)]
pub struct MeteoIcons {
    unknown: String,
    clear: String,
    cloudy: String,
    thunderstorm: String,
    lighting: String,
    fog: String,
    light_rain: String,
    heavy_rain: String,
    freezing_rain: String,
    hail: String,
    light_snow: String,
    heavy_snow: String,
    duststorm: String,
}

impl Default for MeteoIcons {
    fn default() -> Self {
        Self {
            unknown: "".to_string(),
            clear: "󰖙".to_string(),
            cloudy: "󰖐".to_string(),
            thunderstorm: "".to_string(),
            lighting: "󰖓".to_string(),
            fog: "󰖑".to_string(),
            light_rain: "󰖗".to_string(),
            heavy_rain: "󰖖".to_string(),
            freezing_rain: "󰙿".to_string(),
            hail: "󰖒".to_string(),
            light_snow: "󰖘".to_string(),
            heavy_snow: "󰼶".to_string(),
            duststorm: "".to_string(),
        }
    }
}

impl MeteoIcons {
    /// Convert meteo code to icon
    fn translate_code(&self, value: u8) -> &String {
        match value {
            1 | 2 => &self.clear,
            3..=8 | 10..=12 | 14..=16 | 18 | 19 => &self.cloudy,
            13 => &self.lighting,
            17 | 29 | 95..=99 => &self.thunderstorm,
            20 | 21 | 50..=53 | 60..=63 | 80 | 83 | 85 | 91 => &self.light_rain,
            22 | 36 | 38 | 70..=73 | 76..=78 => &self.light_snow,
            26 | 37 | 39 | 74 | 75 => &self.heavy_snow,
            23 | 25 | 54 | 55 | 64 | 65 | 81 | 82 | 84 | 86 | 92 => &self.heavy_rain,
            24 | 87..=90 | 93 | 94 | 66..=69 | 56..=59 => &self.freezing_rain,
            27 | 79 => &self.hail,
            28 | 40..=49 => &self.fog,
            9 | 30..=35 => &self.duststorm,
            _ => &self.unknown,
        }
    }
}

/// Fetches and Displays the meteo at the current position using the machine public ip
#[derive(Debug)]
pub struct Weather {
    icons: MeteoIcons,
    format: String,
    inner: Text,
    init: bool,
    update_timer: ResettableTimer,
}

impl Weather {
    ///* `format`
    ///  * `%cit` will be replaced with the current city used as reference for the meteo
    ///  * `%cod` will be replaced with the current symbol for the weather
    ///  * `%cur` will be replaced with the current temperature
    ///  * `%max` will be replaced with the max temperature
    ///  * `%min` will be replaced with the min temperature
    ///  * `%cur-u` will be replaced with the current temperature unit
    ///  * `%max-u` will be replaced with the max temperature unit
    ///  * `%min-u` will be replaced with the min temperature unit
    ///* `icons` a [&MeteoIcons]
    ///* `config` a [&WidgetConfig]
    pub async fn new(
        format: &impl ToString,
        icons: MeteoIcons,
        config: &WidgetConfig,
    ) -> Box<Self> {
        Box::new(Self {
            icons,
            format: format.to_string(),
            inner: *Text::new("Loading...", config).await,
            init: false,
            update_timer: ResettableTimer::new(config.hide_timeout),
        })
    }

    async fn update_inner_text(&mut self) -> Result<()> {
        let meteo = get_current_meteo().await?;
        let text_str = self
            .format
            .replace("%cur-u", meteo.cur_u())
            .replace("%max-u", meteo.max_u())
            .replace("%min-u", meteo.min_u())
            .replace("%cit", meteo.city())
            .replace("%cod", self.icons.translate_code(*meteo.code()))
            .replace("%cur", meteo.cur())
            .replace("%max", meteo.max())
            .replace("%min", meteo.min());
        self.inner.set_text(text_str);
        Ok(())
    }
}

#[async_trait]
impl Widget for Weather {
    async fn update(&mut self) -> Result<()> {
        debug!("updating meteo");
        if self.update_timer.is_done() || !self.init {
            self.update_timer.reset();
            self.init = true;
            self.update_inner_text().await?;
        }
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, pool: &mut TimedHooks) -> Result<()> {
        pool.subscribe(sender);
        Ok(())
    }

    widget_default!(draw, size, padding);
}

impl Display for Weather {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Weather").fmt(f)
    }
}
