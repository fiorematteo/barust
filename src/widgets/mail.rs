use crate::{
    utils::{HookSender, TimedHooks},
    widget_default,
    widgets::{Result, Text, Widget, WidgetConfig},
};
use async_trait::async_trait;
use futures::Future;
use imap::Session;
use log::{debug, error, warn};
use native_tls::TlsStream;
use std::{env, fmt::Display, fs, net::TcpStream, path::PathBuf, pin::Pin, time::Duration};
use tokio::time::sleep;
use yup_oauth2::{
    authenticator_delegate::{DefaultInstalledFlowDelegate, InstalledFlowDelegate},
    InstalledFlowAuthenticator, InstalledFlowReturnMethod,
};

#[derive(Debug)]
pub struct Mail {
    inner: Text,
    format: String,
    folder_name: String,
    filter: String,
    authenticator: Box<dyn ImapLogin>,
}

#[async_trait]
pub trait ImapLogin: std::fmt::Debug + Send {
    async fn login(&self) -> Result<Session<TlsStream<TcpStream>>>;
}

/// mail and password login
#[derive(Debug)]
pub struct PasswordLogin {
    domain: String,
    user: String,
    password: String,
}

impl PasswordLogin {
    pub fn new(domain: impl ToString, user: impl ToString, password: impl ToString) -> Box<Self> {
        Box::new(Self {
            domain: domain.to_string(),
            user: user.to_string(),
            password: password.to_string(),
        })
    }
}

#[async_trait]
impl ImapLogin for PasswordLogin {
    async fn login(&self) -> Result<Session<TlsStream<TcpStream>>> {
        let tls = native_tls::TlsConnector::builder()
            .build()
            .map_err(Error::from)?;

        let client =
            imap::connect((self.domain.clone(), 993), &self.domain, &tls).map_err(Error::from)?;
        let session = client
            .login(&self.user, &self.password)
            .map_err(|e| e.0)
            .map_err(Error::from)?;
        Ok(session)
    }
}

/// https://github.com/jonhoo/rust-imap/blob/345bd644877f22d845b7a5ae657e6db2aa04dcab/examples/gmail_oauth2.rs
struct GmailOAuth2 {
    user: String,
    access_token: String,
}

impl imap::Authenticator for GmailOAuth2 {
    type Response = String;
    #[allow(unused_variables)]
    fn process(&self, data: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

/// google oauth2 login
#[derive(Debug)]
pub struct GmailLogin {
    user: String,
    client_secret_path: PathBuf,
}

impl GmailLogin {
    pub fn new(user: impl ToString, client_secret_path: impl Into<PathBuf>) -> Box<Self> {
        Box::new(Self {
            user: user.to_string(),
            client_secret_path: client_secret_path.into(),
        })
    }
}

#[async_trait]
impl ImapLogin for GmailLogin {
    async fn login(&self) -> Result<Session<TlsStream<TcpStream>>> {
        let xdg_cache = env::var("XDG_CACHE_HOME")
            .unwrap_or_else(|_| format!("{}/.cache", env::var("HOME").expect("HOME not set")));
        let cache_path = format!("{}/barust/{}", xdg_cache, &self.user);
        fs::create_dir_all(&cache_path).map_err(Error::from)?;

        let secret = yup_oauth2::read_application_secret(&self.client_secret_path)
            .await
            .map_err(Error::from)?;

        let auth =
            InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
                .persist_tokens_to_disk(&format!("{}/tokencache.json", cache_path))
                .flow_delegate(Box::new(InstalledFlowBrowserDelegate::new(&self.user)))
                .build()
                .await
                .map_err(Error::from)?;

        let scopes = &["https://mail.google.com/"];

        let token = auth.token(scopes).await.map_err(Error::from)?;
        let token = token.token().unwrap();

        let gmail_auth = GmailOAuth2 {
            user: self.user.clone(),
            access_token: token.to_string(),
        };

        let tls = native_tls::TlsConnector::builder()
            .build()
            .map_err(Error::from)?;

        let client =
            imap::connect(("imap.gmail.com", 993), "imap.gmail.com", &tls).map_err(Error::from)?;
        let imap_session = client
            .authenticate("XOAUTH2", &gmail_auth)
            .map_err(|e| e.0)
            .map_err(Error::from)?;
        Ok(imap_session)
    }
}

/// https://github.com/dermesser/yup-oauth2/blob/master/examples/custom_flow.rs
#[derive(Clone)]
struct InstalledFlowBrowserDelegate {
    user: String,
}

impl InstalledFlowBrowserDelegate {
    fn new(user: &str) -> Self {
        Self {
            user: user.to_string(),
        }
    }
}

impl InstalledFlowDelegate for InstalledFlowBrowserDelegate {
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        need_code: bool,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<String, String>> + Send + 'a>> {
        async fn browser_user_url(
            url: &str,
            need_code: bool,
        ) -> std::result::Result<String, String> {
            webbrowser::open(url).map_err(|e| format!("{}", e))?;

            DefaultInstalledFlowDelegate
                .present_user_url(url, need_code)
                .await
        }
        warn!("opening browser for oauth2");
        let n = libnotify::Notification::new(
            "Login gmail",
            format!("Login to {} account", self.user).as_str(),
            None,
        );
        n.set_urgency(libnotify::Urgency::Normal);
        n.show().ok();

        Box::pin(browser_user_url(url, need_code))
    }
}

impl Mail {
    ///* `format`
    ///  * *%c* will be replaced with the unread mail count
    ///* `domain` domain of the mail server
    ///* `authenticator` implements `ImapLogin`
    ///* `folder_name` folder to check for mail (defaults to "INBOX")
    ///* `filter` filter for the mail (defaults to "(UNSEEN)")
    ///* `config` a [&WidgetConfig]
    pub async fn new(
        format: impl ToString,
        authenticator: Box<dyn ImapLogin>,
        folder_name: impl Into<Option<&str>>,
        filter: impl Into<Option<&str>>,
        config: &WidgetConfig,
    ) -> Result<Box<Self>> {
        Ok(Box::new(Self {
            inner: *Text::new("", config).await,
            authenticator,
            folder_name: folder_name.into().unwrap_or("INBOX").to_string(),
            filter: filter.into().unwrap_or("(UNSEEN)").to_string(),
            format: format.to_string(),
        }))
    }
}

#[async_trait]
impl Widget for Mail {
    async fn update(&mut self) -> Result<()> {
        debug!("updating mail");
        let mut session = self.authenticator.login().await?;
        session.select(&self.folder_name).map_err(Error::from)?;
        let message_count = match session.search(&self.filter).map(|ids| ids.len()) {
            Ok(c) => c,
            Err(e) => {
                // TODO: some error should be non-recoverable
                // right now we just log and continue
                error!("error getting mail count: {}", e);
                return Ok(());
            }
        };
        if message_count == 0 {
            self.inner.clear();
        } else {
            let new_text = self
                .format
                .replace("%c", message_count.to_string().as_str());
            self.inner.set_text(new_text);
        };
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, _pool: &mut TimedHooks) -> Result<()> {
        // 5 min
        tokio::spawn(async move {
            loop {
                if let Err(e) = sender.send().await {
                    debug!("breaking thread loop: {}", e);
                    break;
                }
                sleep(Duration::from_secs(60 * 5)).await;
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
    Io(#[from] std::io::Error),
    YupOauth2(#[from] yup_oauth2::Error),
}
