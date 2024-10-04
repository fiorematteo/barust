use std::{cmp::Ordering, time::Duration};

use async_channel::SendError;
use async_trait::async_trait;
use barust::{
    utils::{HookSender, TimedHooks, WidgetIndex},
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
    hook: Option<HookSender>,
}

impl Titans {
    pub async fn new(config: &WidgetConfig) -> Box<Self> {
        Box::new(Self {
            inner: *Text::new("Titans", config).await,
            titan: None,
            hook: None,
        })
    }

    async fn update_data(&mut self) -> Result<()> {
        let data = reqwest::get("https://dcoh.watch/api/v1/Overwatch/Titans")
            .await
            .map_err(Error::from)?
            .json::<TitanList>()
            .await
            .map_err(Error::from)?
            .maelstroms;
        self.titan = data.into_iter().filter(|t| t.total_progress != 1.).max();
        Ok(())
    }
}

#[async_trait]
impl Widget for Titans {
    async fn update(&mut self) -> Result<()> {
        debug!("updating titans");
        if let Err(e) = self.update_data().await {
            error!("Failed to update titans: {}", e);
            self.titan = None;
            if let Some(hook) = self.hook.clone() {
                tokio::spawn(async move {
                    sleep(Duration::from_secs(5)).await;
                    hook.send().await
                });
            }
        }
        let text = if let Some(titan) = &self.titan {
            format!(
                "🌸 {} ({:.2}󱉸 💚) {} to go",
                titan.name,
                titan.total_progress * 100.,
                titan.systems_thargoid_controlled
            )
        } else {
            String::from("No active titans")
        };
        self.inner.set_text(text);
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, _pool: &mut TimedHooks) -> Result<()> {
        self.hook = Some(sender.clone());
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

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct TitanList {
    pub maelstroms: Vec<Titan>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct Titan {
    #[serde(rename = "systemsInAlert")]
    pub systems_in_alert: i64,
    #[serde(rename = "systemsInInvasion")]
    pub systems_in_invasion: i64,
    #[serde(rename = "systemsThargoidControlled")]
    pub systems_thargoid_controlled: i64,
    #[serde(rename = "systemsInRecovery")]
    pub systems_in_recovery: i64,
    #[serde(rename = "defenseRate")]
    pub defense_rate: f64,
    #[serde(rename = "damageResistance")]
    pub damage_resistance: DamageResistance,
    #[serde(rename = "heartsRemaining")]
    pub hearts_remaining: i64,
    #[serde(rename = "heartProgress")]
    pub heart_progress: f64,
    #[serde(rename = "totalProgress")]
    pub total_progress: f64,
    pub state: String,
    #[serde(rename = "meltdownTimeEstimate")]
    pub meltdown_time_estimate: Option<String>,
    #[serde(rename = "completionTimeEstimate")]
    pub completion_time_estimate: Option<String>,
    #[serde(rename = "causticLevel")]
    pub caustic_level: String,
    pub name: String,
    #[serde(rename = "systemName")]
    pub system_name: String,
    #[serde(rename = "systemAddress")]
    pub system_address: i64,
    #[serde(rename = "ingameNumber")]
    pub ingame_number: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct DamageResistance {
    pub name: String,
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    Reqwest(#[from] reqwest::Error),
    HookSender(#[from] SendError<WidgetIndex>),
}

impl From<Error> for WidgetError {
    fn from(value: Error) -> Self {
        WidgetError::custom(value)
    }
}
