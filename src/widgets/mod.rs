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
mod clock;
mod cpu;
mod disk;
mod memory;
mod network;
mod spacer;
mod systray;
mod temp;
mod text;
mod update;
mod volume;
//mod wlan;
mod weather;

mod wlan;
mod workspaces;

pub use active_window::ActiveWindow;
pub use bat::{Battery, BatteryIcons, LowBatteryWarner, NotifySend};
pub use brightness::{Brightness, BrightnessProvider, LightProvider, SysfsProvider};
pub use clock::Clock;
pub use cpu::Cpu;
pub use disk::Disk;
pub use memory::Memory;
pub use network::{Network, NetworkIcons};
pub use spacer::Spacer;
pub use systray::Systray;
pub use temp::Temperatures;
pub use text::Text;
pub use update::{Apt, Update, UpdateSource};
pub use volume::{PulseaudioProvider, Volume, VolumeIcons, VolumeProvider};
pub use weather::{MeteoIcons, Weather};
pub use wlan::Wlan;
//pub use wlan::Wlan;
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
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()>;
    fn setup(&mut self, _info: &StatusBarInfo) -> Result<()> {
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
    Clock(#[from] clock::Error),
    Cpu(#[from] cpu::Error),
    Disk(#[from] disk::Error),
    Memory(#[from] memory::Error),
    Network(#[from] network::Error),
    #[error("Spacer")]
    Spacer,
    Systray(#[from] systray::Error),
    Temperatures(#[from] temp::Error),
    Text(#[from] text::Error),
    Update(#[from] update::Error),
    Volume(#[from] volume::Error),
    Wlan(#[from] wlan::Error),
    Weather(#[from] weather::Error),
    Workspaces(#[from] workspaces::Error),
    CustomWidget(#[from] Box<dyn std::error::Error>),
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
        fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
            self.inner.draw(context, rectangle)
        }
    };
    ($a:ident, $($b:tt)*) => {
        widget_default!($a);
        widget_default!($($b)*);
    }
}
