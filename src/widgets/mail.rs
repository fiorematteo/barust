use crate::{
    utils::{HookSender, TimedHooks},
    widget_default,
    widgets::{Result, Text, Widget, WidgetConfig},
};
use async_trait::async_trait;
use imap::Session;
use log::debug;
use native_tls::TlsStream;
use std::{fmt::Display, net::TcpStream, time::Duration};
use tokio::time::sleep;

#[derive(Debug)]
pub struct Mail {
    inner: Text,
    session: Session<TlsStream<TcpStream>>,
}

impl Mail {
    pub async fn new(
        domain: impl ToString,
        user: impl ToString,
        password: impl ToString,
        config: &WidgetConfig,
    ) -> Result<Box<Self>> {
        let tls = native_tls::TlsConnector::builder()
            .build()
            .map_err(Error::from)?;

        let domain = domain.to_string();
        let client = imap::connect((domain.clone(), 993), &domain, &tls).map_err(Error::from)?;
        let mut session = client
            .login(user.to_string(), password.to_string())
            .map_err(|e| e.0)
            .map_err(Error::from)?;
        session.select("INBOX").map_err(Error::from)?;

        Ok(Box::new(Self {
            inner: *Text::new("", config).await,
            session,
        }))
    }
}

#[async_trait]
impl Widget for Mail {
    async fn update(&mut self) -> Result<()> {
        debug!("updating wlan");
        let message_count = self.session.search("(UNSEEN)").map_err(Error::from)?.len();
        let new_text = if message_count == 0 {
            "".to_string()
        } else {
            format!("{} 📧", message_count)
        };
        self.inner.set_text(new_text);
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, _pool: &mut TimedHooks) -> Result<()> {
        // 1 min
        tokio::spawn(async move {
            loop {
                if let Err(e) = sender.send().await {
                    debug!("breaking thread loop: {}", e);
                    break;
                }
                sleep(Duration::from_secs(60)).await;
            }
        });
        Ok(())
    }

    widget_default!(draw, size, padding);
}

impl Display for Mail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Mail").fmt(f)
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    Tls(#[from] native_tls::Error),
    Imap(#[from] imap::Error),
}
