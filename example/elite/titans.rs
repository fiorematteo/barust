use std::{cmp::Ordering, time::Duration};

use async_trait::async_trait;
use barust::{
    utils::{HookSender, TimedHooks},
    widget_default,
    widgets::{Result, Text, Widget, WidgetConfig, WidgetError},
};
use log::{debug, error};
use serde::Deserialize;
use tokio::time::sleep;

#[derive(Debug)]
pub struct Titans {
    inner: Text,
    titan: Option<Titan>,
}

impl Titans {
    pub async fn new(config: &WidgetConfig) -> Box<Self> {
        Box::new(Self {
            inner: *Text::new("Titans", config).await,
            titan: None,
        })
    }

    async fn update_data(&mut self) -> Result<()> {
        let mut data = reqwest::get("https://dcoh.watch/api/v1/Overwatch/Titans")
            .await
            .map_err(Error::from)?
            .json::<TitanList>()
            .await
            .map_err(Error::from)?
            .maelstroms;
        data.retain(|t| t.total_progress != 1.);
        self.titan = data.into_iter().max();
        Ok(())
    }
}

#[async_trait]
impl Widget for Titans {
    async fn update(&mut self) -> Result<()> {
        if let Err(e) = self.update_data().await {
            error!("Failed to update titans: {}", e);
            self.titan = None;
        }
        let text = if let Some(titan) = &self.titan {
            format!(
                "🌸 {}: {:.2}% ({}💚)",
                titan.name,
                titan.heart_progress * 100.,
                titan.hearts_remaining
            )
        } else {
            String::from("No active titans")
        };
        self.inner.set_text(text);
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, _pool: &mut TimedHooks) -> Result<()> {
        // 10 mins
        tokio::spawn(async move {
            loop {
                if let Err(e) = sender.send().await {
                    debug!("breaking thread loop: {}", e);
                    break;
                }
                sleep(Duration::from_secs(600)).await;
            }
        });
        Ok(())
    }

    widget_default!(draw, size, padding);
}

impl std::fmt::Display for Titans {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&String::from("Titans"), f)
    }
}

impl Eq for Titan {}

impl Ord for Titan {
    fn cmp(&self, other: &Self) -> Ordering {
        self.total_progress
            .partial_cmp(&other.total_progress)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for Titan {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Deserialize)]
struct TitanList {
    maelstroms: Vec<Titan>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, PartialEq)]
struct Titan {
    name: String,
    #[serde(rename = "systemName")]
    system_name: String,
    #[serde(rename = "heartsRemaining")]
    hearts_remaining: u64,
    #[serde(rename = "heartProgress")]
    heart_progress: f64,
    #[serde(rename = "totalProgress")]
    total_progress: f64,
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    Reqwest(#[from] reqwest::Error),
}

impl From<Error> for WidgetError {
    fn from(value: Error) -> Self {
        WidgetError::custom(value)
    }
}
