use psutil::Bytes;
use std::fmt::Debug;

pub mod atoms;
pub mod callback;
pub mod color;
pub mod hook_sender;
pub mod resettable_timer;
pub mod timed_hooks;

pub use atoms::Atoms;
pub use callback::{Callback, EmptyCallback, OnClickCallback, OnClickRaw, RawCallback};
pub use color::{set_source_rgba, Color};
pub use hook_sender::{HookSender, WidgetID};
pub use resettable_timer::ResettableTimer;
pub use timed_hooks::TimedHooks;

pub enum StatusBarEvent {
    Wake,
    Click(i16, i16),
}

pub fn percentage_to_index(v: f64, out_range: (usize, usize)) -> usize {
    let scale = (out_range.1 - out_range.0) as f64 / 100.0;
    (v * scale + out_range.0 as f64) as _
}

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
