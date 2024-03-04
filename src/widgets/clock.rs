use crate::utils::{HookSender, TimedHooks};
use crate::{
    widget_default,
    widgets::{Result, Text, Widget, WidgetConfig},
};
use async_trait::async_trait;
use chrono::Local;
use log::debug;
use std::fmt::{Debug, Display};

/// Displays a datetime
pub struct Clock {
    format: String,
    inner: Text,
}

impl Debug for Clock {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Clock(format: {}, padding: {})",
            self.format,
            self.inner.padding(),
        )
    }
}

impl Clock {
    ///* `format` describes how to display the time following [chrono format rules](chrono::format::strftime)
    ///* `config` a [&WidgetConfig]
    pub async fn new(format: impl ToString, config: &WidgetConfig) -> Box<Self> {
        let format = format.to_string();
        Box::new(Self {
            inner: *Text::new("", config).await,
            format,
        })
    }
}

#[async_trait]
impl Widget for Clock {
    async fn update(&mut self) -> Result<()> {
        debug!("updating clock");
        let text = Local::now().format(&self.format);
        self.inner.set_text(text);
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, timed_hooks: &mut TimedHooks) -> Result<()> {
        timed_hooks.subscribe(sender);
        Ok(())
    }

    widget_default!(draw, size, padding);
}

impl Display for Clock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from("Clock"))
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum Error {}
