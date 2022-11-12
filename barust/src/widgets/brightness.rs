use super::{OnClickCallback, Rectangle, Result, Text, Widget, WidgetConfig};
use crate::{
    corex::{
        percentage_to_index, Callback, EmptyCallback, HookSender, RawCallback, ResettableTimer,
        TimedHooks,
    },
    forward_to_inner,
};
use cairo::Context;
use std::{fmt::Display, time::Duration};

/// Icons used by [Brightness]
#[derive(Debug)]
pub struct BrightnessIcons {
    pub percentages: Vec<String>,
}

impl Default for BrightnessIcons {
    fn default() -> Self {
        let percentages = ['', '', '', ''];
        Self {
            percentages: percentages.map(String::from).to_vec(),
        }
    }
}

#[derive(Debug)]
pub struct Brightness {
    format: String,
    brightness_command: Callback<(), Option<u32>>,
    previous_brightness: u32,
    show_counter: ResettableTimer,
    inner: Text,
    icons: BrightnessIcons,
    on_click: OnClickCallback,
}

impl Brightness {
    ///* `format`
    ///  * *%p* will be replaced with the brightness percentage
    ///  * *%i* will be replaced with the correct icon
    ///* `brightness_command` a function that returns the brightness in a range from 0 to 100
    ///* `icons` sets a custom [VolumeIcons]
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(
        format: impl ToString,
        brightness_command: &'static RawCallback<(), Option<u32>>,
        icons: Option<BrightnessIcons>,
        config: &WidgetConfig,
        on_click: Option<&'static EmptyCallback>,
    ) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            inner: *Text::new("", config, None),
            previous_brightness: 0,
            brightness_command: brightness_command.into(),
            on_click: on_click.map(|c| c.into()),
            show_counter: ResettableTimer::new(config.hide_timeout),
            icons: icons.unwrap_or_default(),
        })
    }

    fn build_string(&self, current_brightness: u32) -> String {
        if self.show_counter.is_done() {
            return String::from("");
        }
        let percentages_len = self.icons.percentages.len();
        let index = percentage_to_index(current_brightness as f64, (0, percentages_len - 1));
        self.format
            .replace("%p", &format!("{:.0}", current_brightness))
            .replace("%i", &self.icons.percentages[index].to_string())
    }
}

impl Widget for Brightness {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        let current_brightness = self
            .brightness_command
            .call(())
            .ok_or(Error::CommandError)?;

        if current_brightness != self.previous_brightness {
            self.previous_brightness = current_brightness;
            self.show_counter.reset();
        }
        let text = self.build_string(current_brightness);
        self.inner.set_text(text);

        Ok(())
    }

    fn hook(&mut self, sender: HookSender, timed_hooks: &mut TimedHooks) -> Result<()> {
        timed_hooks
            .subscribe(self.show_counter.duration / 10, sender)
            .map_err(Error::from)?;
        Ok(())
    }

    forward_to_inner!(size);
    forward_to_inner!(padding);
    forward_to_inner!(on_click);
}

impl Display for Brightness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Brightness").fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum Error {
    #[error("Failed to execute brightness command")]
    CommandError,
    HookChannel(#[from] crossbeam_channel::SendError<(Duration, HookSender)>),
}
