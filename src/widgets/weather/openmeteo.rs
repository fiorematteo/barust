use crate::widgets::weather::Error;
use derive_getters::Getters;
use ipgeolocate::{Locator, Service};
use log::debug;
use reqwest::{get, StatusCode};
use serde::Deserialize;
use std::fmt::Debug;

#[derive(Deserialize, Debug, Getters)]
struct CurrentUnits {
    time: String,
    interval: String,
    temperature_2m: String,
    weather_code: String,
}

#[derive(Deserialize, Debug, Getters)]
struct Current {
    time: String,
    interval: i32,
    temperature_2m: f32,
    weather_code: u8,
}

#[derive(Deserialize, Debug, Getters)]
struct DailyUnits {
    time: String,
    temperature_2m_max: String,
    temperature_2m_min: String,
}

#[derive(Deserialize, Debug, Getters)]
struct Daily {
    time: Vec<String>,
    temperature_2m_max: Vec<f32>,
    temperature_2m_min: Vec<f32>,
}
#[derive(Deserialize, Debug, Getters)]
pub struct ApiError {
    error: bool,
    reason: String,
}

#[derive(Deserialize, Debug, Getters)]
struct ApiResponse {
    latitude: f32,
    longitude: f32,
    generationtime_ms: f64,
    utc_offset_seconds: i32,
    timezone: String,
    timezone_abbreviation: String,
    elevation: f32,
    current_units: CurrentUnits,
    current: Current,
    daily_units: DailyUnits,
    daily: Daily,
}

#[derive(Debug, Getters)]
pub struct Meteo {
    code: u8,
    city: String,
    cur: String,
    max: String,
    min: String,
    cur_u: String,
    max_u: String,
    min_u: String,
}

pub async fn get_current_meteo() -> Result<Meteo, Error> {
    let addr = public_ip::addr_v4().await.ok_or(Error::PublicIpNotFound)?;
    debug!("Reading current public ip:{}", addr);
    let loc_info = Locator::get(&addr.to_string(), Service::IpApi).await?;
    debug!(
        "Reading current location info lat:{} lon:{} city:{} country:{}",
        loc_info.latitude, loc_info.longitude, loc_info.city, loc_info.country
    );
    let url = format!("https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,weather_code&daily=temperature_2m_max,temperature_2m_min&forecast_days=1",
    loc_info.latitude,loc_info.longitude);
    debug!("Created open-meteo api call : {}", url);
    let response = get(url).await?;
    let status = response.status();
    let response_text = response.text().await?;
    match status {
        StatusCode::OK => {
            let response: ApiResponse = serde_json::from_str(&response_text)?;
            Ok(Meteo {
                code: response.current.weather_code,
                city: loc_info.city.to_owned(),
                cur: response.current.temperature_2m.to_string(),
                max: response.daily.temperature_2m_max[0].to_string(),
                min: response.daily.temperature_2m_min[0].to_string(),
                cur_u: response.current_units.temperature_2m.to_owned(),
                max_u: response.daily_units.temperature_2m_max.to_owned(),
                min_u: response.daily_units.temperature_2m_min.to_owned(),
            })
        }
        StatusCode::BAD_REQUEST => Err(Error::ApiError(serde_json::from_str(&response_text)?)),
        code => Err(Error::InvalidStatusCode(code)),
    }
}
