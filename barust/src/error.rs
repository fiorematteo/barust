use std::rc::Rc;

#[derive(Debug, derive_more::Error, derive_more::From, derive_more::Display)]
pub enum BarustError {
    Cairo(cairo::Error),
    DrawBeforeUpdate,
    Widget(crate::widgets::WidgetError),
    Xcb(xcb::Error),
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
#[derive(Debug, derive_more::Error)]
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
