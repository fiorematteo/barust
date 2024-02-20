#[cfg(feature = "psutil")]
use psutil::Bytes;
use std::fmt::Debug;
use xcb::Connection;

pub mod atoms;
pub mod color;
pub mod hook_sender;
pub mod resettable_timer;
pub mod timed_hooks;

pub use atoms::Atoms;
pub use color::{set_source_rgba, Color};
pub use hook_sender::{HookSender, WidgetID};
pub use resettable_timer::ResettableTimer;
pub use timed_hooks::TimedHooks;

#[derive(Debug)]
pub struct StatusBarInfo {
    pub background: Color,
    pub left_regions: Vec<Rectangle>,
    pub right_regions: Vec<Rectangle>,
    pub height: u32,
    pub width: u32,
    pub position: Position,
    pub window: xcb::x::Window,
}

#[derive(Clone, Copy, Debug)]
pub enum Position {
    Top,
    Bottom,
}

pub fn screen_true_width(connection: &Connection, screen_id: i32) -> u16 {
    connection
        .get_setup()
        .roots()
        .nth(screen_id as _)
        .unwrap_or_else(|| panic!("cannot find screen:{}", screen_id))
        .width_in_pixels()
}

pub fn screen_true_height(connection: &Connection, screen_id: i32) -> u16 {
    connection
        .get_setup()
        .roots()
        .nth(screen_id as _)
        .unwrap_or_else(|| panic!("cannot find screen:{}", screen_id))
        .height_in_pixels()
}

pub fn percentage_to_index(v: f64, out_range: (usize, usize)) -> usize {
    let scale = (out_range.1 - out_range.0) as f64 / 100.0;
    (v * scale + out_range.0 as f64) as _
}

#[cfg(feature = "psutil")]
pub fn bytes_to_closest(value: Bytes) -> String {
    if value == 0 {
        return "0B".to_string();
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut selected_unit: usize = 0;
    let mut value = value;
    while value > 1024 {
        if selected_unit == 4 {
            break;
        }
        value /= 1024;
        selected_unit += 1;
    }
    format!("{}{}", value, units[selected_unit])
}

#[derive(Debug, Clone, Copy)]
pub struct Rectangle {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl From<Rectangle> for cairo::Rectangle {
    fn from(r: Rectangle) -> Self {
        cairo::Rectangle {
            x: r.x.into(),
            y: r.y.into(),
            width: r.width.into(),
            height: r.height.into(),
        }
    }
}
