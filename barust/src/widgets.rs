use crate::{corex::Color, error::Erc};
use cairo::{Context, Rectangle};
use std::fmt::Display;

mod active_window;
mod bat;
mod clock;
mod cpu;
mod memory;
mod network;
mod spacer;
mod systray;
mod temp;
mod text;
mod volume;
mod workspaces;

pub use active_window::ActiveWindow;
pub use bat::{Battery, BatteryIcons};
pub use clock::Clock;
pub use cpu::Cpu;
pub use memory::Memory;
pub use network::{Network, NetworkIcons};
pub use spacer::Spacer;
pub use systray::Systray;
pub use temp::Temperatures;
pub use text::Text;
pub use volume::{Volume, VolumeIcons};
pub use workspaces::Workspace;

pub type Result<T> = std::result::Result<T, WidgetError>;

pub trait Widget: std::fmt::Debug + Display + Send {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()>;
    fn first_update(&mut self) -> Result<()> {
        Ok(())
    }
    fn update(&mut self) -> Result<()> {
        Ok(())
    }
    fn last_update(&mut self) -> Result<()> {
        Ok(())
    }
    fn hook(&mut self, _sender: chan::Sender<()>) -> Result<()> {
        Ok(())
    }
    fn size(&self, context: &Context) -> Result<f64>;
    fn padding(&self) -> f64;
    fn on_click(&mut self) {}
}

pub struct WidgetConfig<'a> {
    pub font: &'a str,
    pub font_size: f64,
    pub padding: f64,
    pub fg_color: Color,
}

impl<'a> Default for WidgetConfig<'a> {
    fn default() -> Self {
        Self {
            font: "DejaVu Sans",
            font_size: 15.0,
            padding: 10.0,
            fg_color: Color::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}

#[derive(Debug, derive_more::Error, derive_more::Display, derive_more::From)]
pub enum WidgetError {
    ActiveWindow(active_window::Error),
    Battery(bat::Error),
    Clock,
    Cpu(cpu::Error),
    Memory(memory::Error),
    Network(network::Error),
    Spacer,
    Systray(systray::Error),
    Temperatures,
    Text(text::Error),
    Workspace(workspaces::Error),
    CustomWidget(Erc),
}
