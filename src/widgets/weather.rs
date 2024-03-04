use crate::{
    utils::{HookSender, TimedHooks},
    widget_default,
    widgets::{Result, Text, Widget, WidgetConfig},
};
use async_trait::async_trait;
use log::{debug, warn};
use std::{fmt::Debug, time::Duration};
use tokio::time::sleep;

#[derive(Debug)]
pub struct Meteo {
    pub code: f32,
    pub city: String,
    pub current: String,
    pub max: String,
    pub min: String,
}

#[cfg(feature = "openmeteo")]
pub mod openmeteo {
    use super::{Error, Meteo, Result, WeatherProvider};
    use async_trait::async_trait;
    use ipgeolocate::{Locator, Service};
    use log::debug;
    use open_meteo_api::models::TimeZone;

    #[derive(Debug)]
    pub struct OpenMeteoProvider;

    impl OpenMeteoProvider {
        pub fn new() -> Box<Self> {
            Box::new(Self)
        }
    }

    #[async_trait]
    impl WeatherProvider for OpenMeteoProvider {
        async fn get_current_meteo(&self) -> Result<Meteo> {
            let addr = public_ip::addr_v4()
                .await
                .ok_or(Error::MissingData("public ip"))?;
            debug!("Reading current public ip:{}", addr);
            let loc_info = Locator::get(&addr.to_string(), Service::IpApi)
                .await
                .map_err(Box::new)
                .map_err(|e| Error::ProviderError(e))?;

            let data = open_meteo_api::query::OpenMeteo::new()
                .coordinates(
                    loc_info.latitude.parse::<f32>().unwrap(),
                    loc_info.longitude.parse::<f32>().unwrap(),
                )
                .expect("why is this error not Send???")
                .current_weather()
                .expect("why is this error not Send???")
                .time_zone(TimeZone::Auto)
                .expect("why is this error not Send???")
                .daily()
                .expect("why is this error not Send???")
                .query()
                .await
                .expect("why is this error not Send???");

            let current_weather = data
                .current_weather
                .ok_or(Error::MissingData("current_weather"))?;
            let daily = data.daily.ok_or(Error::MissingData("daily"))?;
            let daily_units = data.daily_units.ok_or(Error::MissingData("daily_units"))?;

            let max = format!(
                "{}{}",
                daily
                    .temperature_2m_max
                    .first()
                    .ok_or(Error::MissingData("max_temperature"))?
                    .ok_or(Error::MissingData("max_temperature"))?,
                daily_units.temperature_2m_max
            );
            let min = format!(
                "{}{}",
                daily
                    .temperature_2m_min
                    .first()
                    .ok_or(Error::MissingData("min_temperature"))?
                    .ok_or(Error::MissingData("min_temperature"))?,
                daily_units.temperature_2m_min
            );
            let current = format!(
                "{}{}",
                current_weather.temperature, daily_units.temperature_2m_min
            );

            let out = Meteo {
                code: current_weather.weathercode,
                city: loc_info.city,
                current,
                max,
                min,
            };
            Ok(out)
        }
    }
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
            0 => &self.clear,                  // Clear sky
            1..=3 => &self.cloudy,             // Mainly clear, partly cloudy, and overcast
            45 | 48 => &self.fog,              // Fog and depositing rime fog
            51 | 53 | 55 => &self.drizzle,     // Drizzle: Light, moderate, and dense intensity
            56 | 57 => &self.freezing_drizzle, // Freezing Drizzle: Light and dense intensity
            61 | 63 | 65 => &self.rain,        // Rain: Slight, moderate and heavy intensity
            66 | 67 => &self.freezing_rain,    // Freezing Rain: Light and heavy intensity
            71 | 73 | 75 => &self.snow,        // Snow fall: Slight, moderate, and heavy intensity
            77 => &self.light_snow,            // Snow grains
            80..=82 => &self.rain,             // Rain showers: Slight, moderate, and violent
            85 | 86 => &self.snow,             // Snow showers slight and heavy
            95 => &self.thunderstorm,          // Thunderstorm: Slight or moderate
            96 | 99 => &self.hail,             // Thunderstorm with slight and heavy hail
            _ => {
                warn!("Unknown meteo code: {}", value);
                &self.unknown
            }
        }
    }
}

#[async_trait]
pub trait WeatherProvider: Send + std::fmt::Debug {
    async fn get_current_meteo(&self) -> Result<Meteo>;
}

/// Fetches and Displays the meteo at the current position using the machine public ip
#[derive(Debug)]
pub struct Weather {
    icons: MeteoIcons,
    format: String,
    inner: Text,
    provider: Box<dyn WeatherProvider>,
}

impl Weather {
    ///* `format`
    ///  * `%city` will be replaced with the current city used as reference for the meteo
    ///  * `%icon` will be replaced with the current symbol for the weather
    ///  * `%cur` will be replaced with the current temperature
    ///  * `%max` will be replaced with the max temperature
    ///  * `%min` will be replaced with the min temperature
    ///* `icons` a [&MeteoIcons]
    ///* `config` a [&WidgetConfig]
    pub async fn new(
        format: &impl ToString,
        icons: MeteoIcons,
        config: &WidgetConfig,
        provider: Box<impl WeatherProvider + 'static>,
    ) -> Box<Self> {
        Box::new(Self {
            icons,
            format: format.to_string(),
            inner: *Text::new("Loading...", config).await,
            provider,
        })
    }
}

#[async_trait]
impl Widget for Weather {
    async fn update(&mut self) -> Result<()> {
        debug!("updating meteo");
        let meteo = self.provider.get_current_meteo().await?;
        let text_str = self
            .format
            .replace("%city", &meteo.city.to_string())
            .replace("%icon", self.icons.translate_code(meteo.code as _))
            .replace("%cur", &meteo.current)
            .replace("%max", &meteo.max)
            .replace("%min", &meteo.min);
        self.inner.set_text(text_str);
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, _pool: &mut TimedHooks) -> Result<()> {
        // 1 hour
        tokio::spawn(async move {
            loop {
                if let Err(e) = sender.send().await {
                    debug!("breaking thread loop: {}", e);
                    break;
                }
                sleep(Duration::from_secs(3600)).await;
            }
        });
        Ok(())
    }

    widget_default!(draw, size, padding);
}

impl std::fmt::Display for Weather {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&String::from("Weather"), f)
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("Missing data: {0}")]
    MissingData(&'static str),
    ProviderError(#[from] Box<dyn std::error::Error + Send>),
}
