use crate::utils::{HookSender, TimedHooks};
use crate::{
    widget_default,
    widgets::{Rectangle, Result, Text, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::Context;
use log::debug;
use psutil::sensors::temperatures;
use std::fmt::Display;

/// Displays the average temperature read by the device sensors
#[derive(Debug)]
pub struct Temperatures {
    format: String,
    inner: Text,
}

impl Temperatures {
    ///* `format`
    ///  * `%t` will be replaced with the temperature in celsius
    ///* `config` a [&WidgetConfig]
    pub async fn new(format: impl ToString, config: &WidgetConfig) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            inner: *Text::new("", config).await,
        })
    }
}

#[async_trait]
impl Widget for Temperatures {
    async fn update(&mut self) -> Result<()> {
        debug!("updating temp");
        let mut temp: f64 = 0.0;
        let mut count: f64 = 0.0;
        for elem in temperatures().iter().flatten() {
            temp += elem.current().celsius();
            count += 1.0;
        }
        let text = self.format.replace("%t", &format!("{:.1}", temp / count));
        self.inner.set_text(text);
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, pool: &mut TimedHooks) -> Result<()> {
        pool.subscribe(sender);
        Ok(())
    }

    widget_default!(draw, size, padding);
}

impl Display for Temperatures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Temperatures").fmt(f)
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {}
