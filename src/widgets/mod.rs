use crate::utils::{Color, HookSender, Rectangle, StatusBarInfo, TimedHooks};
use async_trait::async_trait;
use cairo::Context;
use std::{fmt::Display, time::Duration};
use thiserror::Error;

mod replaceable;

pub use replaceable::ReplaceableWidget;

mod active_window;
mod bat;
mod brightness;
#[cfg(feature = "clock")]
mod clock;
#[cfg(feature = "cpu")]
mod cpu;
#[cfg(feature = "disk")]
mod disk;
mod icon;
mod mail;
#[cfg(feature = "memory")]
mod memory;
mod network;
mod png;
mod spacer;
mod svg;
mod systray;
#[cfg(feature = "temp")]
mod temp;
mod text;
mod update;
mod volume;
mod weather;
#[cfg(feature = "wlan")]
mod wlan;
mod workspaces;

pub use active_window::ActiveWindow;
pub use bat::{Battery, BatteryIcons, LowBatteryWarner, NotifySend};
pub use brightness::{Brightness, BrightnessProvider, LightProvider, SysfsProvider};
#[cfg(feature = "clock")]
pub use clock::Clock;
#[cfg(feature = "cpu")]
pub use cpu::Cpu;
#[cfg(feature = "disk")]
pub use disk::Disk;
pub use icon::Icon;
pub use mail::{GmailLogin, ImapLogin, Mail, PasswordLogin};
#[cfg(feature = "memory")]
pub use memory::Memory;
pub use network::{Network, NetworkIcons};
pub use png::Png;
pub use spacer::Spacer;
pub use svg::Svg;
pub use systray::Systray;
#[cfg(feature = "temp")]
pub use temp::Temperatures;
pub use text::Text;
pub use update::{Apt, Update, UpdateSource};
#[cfg(feature = "pulseaudio")]
pub use volume::pulseaudio::PulseaudioProvider;
pub use volume::{Volume, VolumeIcons, VolumeProvider};
#[cfg(feature = "openmeteo")]
pub use weather::openmeteo::OpenMeteoProvider;
pub use weather::{MeteoIcons, Weather, WeatherProvider};
#[cfg(feature = "wlan")]
pub use wlan::Wlan;
pub use workspaces::{
    ActiveProvider, NeverHide, WorkspaceHider, WorkspaceStatus, WorkspaceStatusProvider, Workspaces,
};

pub enum Size {
    Flex,
    Static(u32),
}

impl Size {
    pub fn is_flex(&self) -> bool {
        matches!(self, Size::Flex)
    }

    pub fn unwrap_or(&self, s: u32) -> u32 {
        match self {
            Size::Flex => s,
            Size::Static(s) => *s,
        }
    }
}

pub type Result<T> = std::result::Result<T, WidgetError>;

#[async_trait]
pub trait Widget: std::fmt::Debug + Display + Send {
    fn draw(&self, context: Context, rectangle: &Rectangle) -> Result<()>;
    async fn setup(&mut self, _info: &StatusBarInfo) -> Result<()> {
        Ok(())
    }
    async fn update(&mut self) -> Result<()> {
        Ok(())
    }
    async fn hook(&mut self, _sender: HookSender, _pool: &mut TimedHooks) -> Result<()> {
        Ok(())
    }
    fn size(&self, context: &Context) -> Result<Size>;
    fn padding(&self) -> u32;
}

#[derive(Debug, Clone)]
pub struct WidgetConfig {
    pub font: String,
    pub font_size: f64,
    pub padding: u32,
    pub fg_color: Color,
    pub hide_timeout: Duration,
    pub flex: bool,
}

impl WidgetConfig {
    pub fn new(
        font: impl ToString,
        font_size: f64,
        padding: u32,
        fg_color: Color,
        hide_timeout: Duration,
        flex: bool,
    ) -> WidgetConfig {
        Self {
            font: font.to_string(),
            font_size,
            padding,
            fg_color,
            hide_timeout,
            flex,
        }
    }
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            font: "DejaVu Sans".to_string(),
            font_size: 15.0,
            padding: 10,
            fg_color: Color::new(1.0, 1.0, 1.0, 1.0),
            hide_timeout: Duration::from_secs(1),
            flex: false,
        }
    }
}

#[derive(Debug, Error)]
#[error(transparent)]
pub enum WidgetError {
    ActiveWindow(#[from] active_window::Error),
    Battery(#[from] bat::Error),
    Brightness(#[from] brightness::Error),
    #[cfg(feature = "clock")]
    Clock(#[from] clock::Error),
    #[cfg(feature = "cpu")]
    Cpu(#[from] cpu::Error),
    #[cfg(feature = "disk")]
    Disk(#[from] disk::Error),
    Icon(#[from] icon::Error),
    Mail(#[from] mail::Error),
    #[cfg(feature = "memory")]
    Memory(#[from] memory::Error),
    Network(#[from] network::Error),
    Png(#[from] png::Error),
    #[error("Spacer")]
    Spacer,
    Svg(#[from] svg::Error),
    Systray(#[from] systray::Error),
    #[cfg(feature = "temp")]
    Temperatures(#[from] temp::Error),
    Text(#[from] text::Error),
    Update(#[from] update::Error),
    Volume(#[from] volume::Error),
    #[cfg(feature = "wlan")]
    Wlan(#[from] wlan::Error),
    Weather(#[from] weather::Error),
    Workspaces(#[from] workspaces::Error),
    CustomWidget(#[from] Box<dyn std::error::Error + Send>),
}

impl WidgetError {
    pub fn custom<E: std::error::Error + Send + 'static>(error: E) -> Self {
        Self::CustomWidget(Box::new(error))
    }
}

#[macro_export]
macro_rules! widget_default {
    (size) => {
        fn size(&self, context: &cairo::Context) -> Result<$crate::widgets::Size> {
            self.inner.size(context)
        }
    };
    (padding) => {
        fn padding(&self) -> u32 {
            self.inner.padding()
        }
    };
    (draw) => {
        fn draw(&self, context: cairo::Context, rectangle: &$crate::utils::Rectangle) -> Result<()> {
            self.inner.draw(context, rectangle)
        }
    };
    ($a:ident, $($b:tt)*) => {
        widget_default!($a);
        widget_default!($($b)*);
    }
}
