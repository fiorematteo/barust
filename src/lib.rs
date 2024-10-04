pub mod statusbar;
pub mod utils;
pub mod widgets;

use std::{fs::create_dir_all, io, path::PathBuf};

use thiserror::Error;
#[derive(Debug, Error)]
#[error(transparent)]
pub enum BarustError {
    Cairo(#[from] cairo::Error),
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

fn xdg_getter(name: &str, default: &str) -> io::Result<PathBuf> {
    let home = PathBuf::from(std::env::var("HOME").expect("HOME not set"));
    let base = std::env::var(name)
        .map(PathBuf::from)
        .unwrap_or(home.join(default));
    let path = base.join("barust");
    create_dir_all(&path)?;
    Ok(path)
}

pub fn xdg_config() -> io::Result<PathBuf> {
    xdg_getter("XDG_CONFIG_HOME", ".config")
}

pub fn xdg_data() -> io::Result<PathBuf> {
    xdg_getter("XDG_DATA_HOME", ".local/share")
}

pub fn xdg_cache() -> io::Result<PathBuf> {
    xdg_getter("XDG_CACHE_HOME", ".cache")
}

pub type Result<T> = std::result::Result<T, BarustError>;
