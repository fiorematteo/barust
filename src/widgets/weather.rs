use crate::utils::{HookSender, ResettableTimer, TimedHooks};
use crate::{
    widget_default,
    widgets::{Rectangle, Result, Text, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::Context;
use ipgeolocate::{GeoError, Locator, Service};
use log::debug;
use open_meteo_api::models::OpenMeteoData;

#[derive(Debug)]
pub struct Meteo {
    pub code: i32,
    pub city: String,
    pub current_temperature: String,
    pub max: String,
    pub min: String,
    pub current_temperature_unit: String,
    pub max_unit: String,
    pub min_unit: String,
}

pub async fn get_current_meteo() -> Result<Meteo> {
    let addr = public_ip::addr_v4().await.ok_or(Error::PublicIpNotFound)?;
    debug!("Reading current public ip:{}", addr);
    let loc_info = Locator::get(&addr.to_string(), Service::IpApi)
        .await
        .map_err(Error::from)?;

    let data: OpenMeteoData = open_meteo_api::query::OpenMeteo::new()
        .coordinates(
            loc_info.latitude.parse::<f32>().unwrap(),
            loc_info.longitude.parse::<f32>().unwrap(),
        )
        .map_err(|e| Error::OpenMeteoRequest(e.to_string()))?
        .current_weather()
        .map_err(|e| Error::OpenMeteoRequest(e.to_string()))?
        .query()
        .await
        .map_err(|e| Error::OpenMeteoRequest(e.to_string()))?;

    let current_weather = data.current_weather.ok_or(Error::MissingData)?;
    let daily = data.daily.ok_or(Error::MissingData)?;
    let daily_units = data.daily_units.ok_or(Error::MissingData)?;

    let out = Meteo {
        code: current_weather.weathercode as _,
        city: loc_info.city,
        current_temperature: current_weather.temperature.to_string(),
        max: daily
            .temperature_2m_max
            .first()
            .ok_or(Error::MissingData)?
            .ok_or(Error::MissingData)?
            .to_string(),
        min: daily
            .temperature_2m_min
            .first()
            .ok_or(Error::MissingData)?
            .ok_or(Error::MissingData)?
            .to_string(),
        current_temperature_unit: daily_units.temperature_2m_min.clone(),
        max_unit: daily_units.temperature_2m_max,
        min_unit: daily_units.temperature_2m_min,
    };
    Ok(out)
}

/// A set of strings used as icons in the Weather widget
#[derive(Debug)]
pub struct MeteoIcons {
    pub clear: String,
    pub cloudy: String,
    pub fog: String,
    pub freezing_rain: String,
    pub freezing_drizzle: String,
    pub hail: String,
    pub rain: String,
    pub snow: String,
    pub drizzle: String,
    pub light_snow: String,
    pub thunderstorm: String,
    pub unknown: String,
}

impl Default for MeteoIcons {
    fn default() -> Self {
        Self {
            clear: "󰖙".to_string(),
            cloudy: "󰖐".to_string(),
            drizzle: "󰖗".to_string(),
            fog: "󰖑".to_string(),
            freezing_drizzle: "󰖘".to_string(),
            freezing_rain: "󰙿".to_string(),
            hail: "󰖒".to_string(),
            light_snow: "󰖘".to_string(),
            rain: "󰖖".to_string(),
            snow: "󰼶".to_string(),
            thunderstorm: "".to_string(),
            unknown: "".to_string(),
        }
    }
}

impl MeteoIcons {
    /// Convert meteo code to icon
    fn translate_code(&self, value: u8) -> &str {
        match value {
            0 => &self.clear,
            1..=3 => &self.cloudy,
            45 | 48 => &self.fog,
            51 | 53 | 55 => &self.drizzle,
            56 | 57 => &self.freezing_drizzle,
            61 | 63 | 65 => &self.rain,
            66 | 67 => &self.freezing_rain,
            71 | 73 | 75 => &self.snow,
            77 => &self.light_snow,
            85 | 86 => &self.snow,
            95 => &self.thunderstorm,
            96 | 99 => &self.hail,
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
            .replace("%cur-u", &meteo.current_temperature_unit.to_string())
            .replace("%max-u", &meteo.max_unit.to_string())
            .replace("%min-u", &meteo.min_unit.to_string())
            .replace("%cit", &meteo.city.to_string())
            .replace("%cod", self.icons.translate_code(meteo.code as _))
            .replace("%cur", &meteo.current_temperature.to_string())
            .replace("%max", &meteo.max.to_string())
            .replace("%min", &meteo.min.to_string());
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

impl std::fmt::Display for Weather {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Weather").fmt(f)
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("Ip address not found")]
    PublicIpNotFound,
    Geo(#[from] GeoError),
    Request(#[from] reqwest::Error),
    SerdeJson(#[from] serde_json::Error),
    #[error("OpenMeteo request error: {0}")]
    OpenMeteoRequest(String),
    #[error("Missing data from weather provider")]
    MissingData,
}
