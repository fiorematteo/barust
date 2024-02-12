use crate::{
    utils::{HookSender, StatusBarInfo, TimedHooks},
    widget_default,
    widgets::{Rectangle, Result, Text, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::Context;
use log::error;
use std::{fmt::Display, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    task,
    time::sleep,
};

#[derive(Debug)]
pub struct Update {
    inner: Text,
    sources: Vec<Box<dyn UpdateSource>>,
}

impl Update {
    pub async fn new(config: &WidgetConfig, sources: Vec<Box<dyn UpdateSource>>) -> Box<Self> {
        Box::new(Self {
            inner: *Text::new("", config).await,
            sources,
        })
    }
}

#[async_trait]
impl Widget for Update {
    fn setup(&mut self, _info: &StatusBarInfo) -> Result<()> {
        Ok(())
    }

    async fn update(&mut self) -> Result<()> {
        let mut all_updates = Vec::new();
        for source in &mut self.sources {
            if source.update_available().await? {
                all_updates.push(source.message());
            }
        }
        self.inner.set_text(all_updates.join(" - "));
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, _pool: &mut TimedHooks) -> Result<()> {
        task::spawn(async move {
            loop {
                if let Err(e) = sender.send().await {
                    error!("error sending update hook: {}", e);
                }
                sleep(Duration::from_secs(60)).await;
            }
        });
        Ok(())
    }

    widget_default!(draw, size, padding);
}

impl Display for Update {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Update").fmt(f)
    }
}

#[async_trait]
pub trait UpdateSource: std::fmt::Debug + Send {
    async fn update_available(&mut self) -> Result<bool>;
    fn message(&self) -> String;
}

#[derive(Debug)]
pub struct Apt {}

impl Apt {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

#[async_trait]
impl UpdateSource for Apt {
    async fn update_available(&mut self) -> Result<bool> {
        let mut child = Command::new("apt")
            .args(["list", "--upgradable"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(Error::from)?;

        child.wait().await.unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut lines = BufReader::new(stdout).lines();
        let _ = lines.next_line().await;
        let line = lines.next_line().await.map_err(Error::from)?;

        Ok(line.is_some())
    }

    fn message(&self) -> String {
        "apt".to_string()
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum Error {
    Io(#[from] std::io::Error),
}
