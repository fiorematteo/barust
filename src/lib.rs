pub mod utils;
pub mod widgets;

pub mod statusbar;

use thiserror::Error;
#[derive(Debug, Error)]
#[error(transparent)]
pub enum BarustError {
    Cairo(#[from] cairo::Error),
    #[error("Draw was called without any regions defined")]
    DrawBeforeUpdate,
    Io(#[from] std::io::Error),
    Widget(#[from] widgets::WidgetError),
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
