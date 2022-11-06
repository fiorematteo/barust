use crate::{
    corex::{Callback, Color, HookSender, TimedHooks},
    error::Erc,
    statusbar::StatusBarInfo,
};
use cairo::{Context, Rectangle};
use std::{fmt::Display, time::Duration};
use thiserror::Error;

mod active_window;
mod bat;
mod brightness;
mod clock;
mod cpu;
mod disk;
mod filtered_workspaces;
mod memory;
mod network;
mod spacer;
mod systray;
mod temp;
mod text;
mod volume;
mod wlan;
mod workspaces;

pub use active_window::ActiveWindow;
pub use bat::{Battery, BatteryIcons};
pub use brightness::Brightness;
pub use clock::Clock;
pub use cpu::Cpu;
pub use disk::Disk;
pub use filtered_workspaces::FilteredWorkspaces;
pub use memory::Memory;
pub use network::{Network, NetworkIcons};
pub use spacer::Spacer;
pub use systray::Systray;
pub use temp::Temperatures;
pub use text::Text;
pub use volume::{Volume, VolumeIcons};
pub use wlan::Wlan;
pub use workspaces::Workspaces;

pub type Result<T> = std::result::Result<T, WidgetError>;

pub trait Widget: std::fmt::Debug + Display + Send {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()>;
    fn setup(&mut self, _info: &StatusBarInfo) -> Result<()> {
        Ok(())
    }
    fn update(&mut self) -> Result<()> {
        Ok(())
    }
    fn last_update(&mut self) -> Result<()> {
        Ok(())
    }
    fn hook(&mut self, _sender: HookSender, _pool: &mut TimedHooks) -> Result<()> {
        Ok(())
    }
    fn size(&self, context: &Context) -> Result<f64>;
    fn padding(&self) -> f64;
    fn on_click(&self) {}
}

pub struct WidgetConfig<'a> {
    pub font: &'a str,
    pub font_size: f64,
    pub padding: f64,
    pub fg_color: Color,
    pub hide_timeout: Duration,
}

impl<'a> WidgetConfig<'a> {
    pub fn new(
        font: &'a str,
        font_size: f64,
        padding: f64,
        fg_color: Color,
        hide_timeout: Duration,
    ) -> Self {
        Self {
            font,
            font_size,
            padding,
            fg_color,
            hide_timeout,
        }
    }
}

impl Default for WidgetConfig<'_> {
    fn default() -> Self {
        Self {
            font: "DejaVu Sans",
            font_size: 15.0,
            padding: 10.0,
            fg_color: Color::new(1.0, 1.0, 1.0, 1.0),
            hide_timeout: Duration::from_secs(1),
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
    FilteredWorkspaces(#[from] filtered_workspaces::Error),
    Memory(#[from] memory::Error),
    Network(#[from] network::Error),
    #[error("Spacer")]
    Spacer,
    Systray(#[from] systray::Error),
    Temperatures(#[from] temp::Error),
    Text(#[from] text::Error),
    Volume(#[from] volume::Error),
    Wlan(#[from] wlan::Error),
    Workspaces(#[from] workspaces::Error),
    CustomWidget(#[from] Erc),
}

type OnClickCallback = Option<Callback<(), ()>>;

#[macro_export]
macro_rules! forward_to_inner {
    (size) => {
        fn size(&self, context: &cairo::Context) -> Result<f64> {
            self.inner.size(context)
        }
    };
    (padding) => {
        fn padding(&self) -> f64 {
            self.inner.padding()
        }
    };
    (on_click) => {
        fn on_click(&self) {
            if let Some(cb) = &self.on_click {
                cb.call(());
            }
        }
    };
}
