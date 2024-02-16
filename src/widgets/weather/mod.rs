mod openmeteo;
mod widget;
use ipgeolocate::GeoError;
use reqwest::StatusCode;
use std::fmt::{Display, Formatter};

pub use widget::{MeteoIcons, Weather};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    PublicIpNotFound,
    InvalidStatusCode(StatusCode),
    ApiError(crate::widgets::weather::openmeteo::ApiError),
    GeoError(#[from] GeoError),
    RequestError(#[from] reqwest::Error),
    JsonError(#[from] serde_json::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::PublicIpNotFound => f.write_str("Public ip not found"),
            Error::InvalidStatusCode(s) => (s as &dyn std::fmt::Debug).fmt(f),
            Error::ApiError(e) => (e as &dyn std::fmt::Debug).fmt(f),
            Error::GeoError(e) => (e as &dyn std::fmt::Debug).fmt(f),
            Error::RequestError(e) => (e as &dyn std::fmt::Debug).fmt(f),
            Error::JsonError(e) => (e as &dyn std::fmt::Debug).fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::widgets::weather::openmeteo::get_current_meteo;
    use log::debug;
    use std::io::stdout;

    #[tokio::test]
    pub async fn test_meteo_api() {
        simple_logging::log_to(stdout(), log::LevelFilter::Debug);
        let x = get_current_meteo().await;
        debug!("Meteo info = {:?}", x.unwrap());
    }
}
