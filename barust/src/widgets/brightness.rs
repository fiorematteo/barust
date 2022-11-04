use super::{OnClickCallback, Result, Text, Widget, WidgetConfig};
use crate::{
    corex::{Callback, EmptyCallback, HookSender, RawCallback, ResettableCounter, TimedHooks},
    forward_to_inner,
};
use std::{fmt::Display, time::Duration};

#[derive(Debug)]
pub struct Brightness {
    format: String,
    brightness_command: Callback<(), Option<i32>>,
    previous_brightness: i32,
    show_counter: ResettableCounter,
    inner: Text,
    on_click: OnClickCallback,
}

impl Brightness {
    pub fn new(
        format: impl ToString,
        brightness_command: &'static RawCallback<(), Option<i32>>,
        config: &WidgetConfig,
        on_click: Option<&'static EmptyCallback>,
    ) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            inner: *Text::new("", config, None),
            previous_brightness: 0,
            brightness_command: brightness_command.into(),
            on_click: on_click.map(|c| c.into()),
            show_counter: ResettableCounter::new(5),
        })
    }
}

impl Widget for Brightness {
    fn draw(&self, context: &cairo::Context, rectangle: &cairo::Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        let current_brightness = self
            .brightness_command
            .call(())
            .ok_or(Error::CommandError)?;

        if current_brightness == self.previous_brightness {
            self.show_counter.tick();
        } else {
            self.previous_brightness = current_brightness;
            self.show_counter.reset();
        }

        let text = if self.show_counter.is_done() {
            String::from("")
        } else {
            self.format
                .replace("%b", &format!("{:.0}", current_brightness))
        };
        self.inner.set_text(text);

        Ok(())
    }

    fn hook(&mut self, sender: HookSender, timed_hook: &mut TimedHooks) -> Result<()> {
        timed_hook
            .subscribe(Duration::from_secs(1), sender)
            .map_err(Error::from)?;
        Ok(())
    }

    fn on_click(&self) {
        if let Some(cb) = &self.on_click {
            cb.call(());
        }
    }

    forward_to_inner!(size);
    forward_to_inner!(padding);
}

impl Display for Brightness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Brightness").fmt(f)
    }
}

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
pub enum Error {
    CommandError,
    HookChannel(crossbeam_channel::SendError<(Duration, HookSender)>),
}
