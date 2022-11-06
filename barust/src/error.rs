use std::rc::Rc;
use thiserror::Error;

#[derive(Debug, Error)]
#[error(transparent)]
pub enum BarustError {
    Cairo(#[from] cairo::Error),
    #[error("Draw was called without any regions defined")]
    DrawBeforeUpdate,
    Io(#[from] std::io::Error),
    Widget(#[from] crate::widgets::WidgetError),
    Xcb(#[from] xcb::Error),
}

impl From<xcb::ConnError> for BarustError {
    fn from(v: xcb::ConnError) -> Self {
        Self::Xcb(xcb::Error::Connection(v))
    }
}

impl From<xcb::ProtocolError> for BarustError {
    fn from(v: xcb::ProtocolError) -> Self {
        Self::Xcb(xcb::Error::Protocol(v))
    }
}

pub type Result<T> = std::result::Result<T, BarustError>;

/// Rc that implements [std::error::Error]
#[derive(Debug, Error)]
pub struct Erc {
    inner: Rc<dyn std::error::Error>,
}

impl Erc {
    pub fn new<E: std::error::Error + 'static>(error: E) -> Self {
        let inner = Rc::new(error);
        Self { inner }
    }
}

impl std::fmt::Display for Erc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
