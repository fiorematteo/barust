use super::{Result, Text, Widget, WidgetConfig};
use crate::corex::{Callback, OptionCallback, SelfCallback};
use log::debug;
use std::{cmp::min, fmt::Display};

/// Icons used by [Volume]
#[derive(Debug)]
pub struct VolumeIcons {
    pub percentages: Vec<String>,
    ///displayed if the device is muted
    pub muted: String,
}

impl Default for VolumeIcons {
    fn default() -> Self {
        let percentages = ['奄', '奔', '墳'];
        Self {
            percentages: percentages.map(String::from).to_vec(),
            muted: String::from('ﱝ'),
        }
    }
}
/// Displays status and volume of the audio device
pub struct Volume<'a> {
    format: String,
    inner: Text<'a>,
    volume_command: &'a Callback<f64>,
    muted_command: &'a Callback<bool>,
    icons: VolumeIcons,
    on_click: OptionCallback<'a, Self>,
}

impl<'a> Volume<'a> {
    ///* `format`
    ///  * *%p* will be replaced with the volume percentage
    ///  * *%i* will be replaced with the correct icon
    ///* `volume_command` a function that returns the volume in a range from 0 to 100
    ///* `muted_command` a function that returns true if the volume is muted
    ///* `icons` sets a custom [VolumeIcons]
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(
        format: &str,
        volume_command: &'a Callback<f64>,
        muted_command: &'a Callback<bool>,
        icons: Option<VolumeIcons>,
        config: &WidgetConfig,
        on_click: Option<&'a SelfCallback<Self>>,
    ) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            volume_command: volume_command.into(),
            muted_command: muted_command.into(),
            icons: icons.unwrap_or_default(),
            inner: *Text::new("VOLUME", config, None),
            on_click: on_click.into(),
        })
    }
}

impl Widget for Volume<'_> {
    fn draw(&self, context: &cairo::Context, rectangle: &cairo::Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating volume");
        let text = if (self.muted_command)() {
            self.icons.muted.clone()
        } else {
            let volume = (self.volume_command)();
            let percentages_len = self.icons.percentages.len();
            let index = min(
                (volume / percentages_len as f64).floor() as usize,
                percentages_len - 1,
            );
            self.format
                .replace("%p", &format!("{:.1}", volume))
                .replace("%i", &self.icons.percentages[index].to_string())
        };
        self.inner.set_text(text);
        Ok(())
    }

    fn size(&self, context: &cairo::Context) -> Result<f64> {
        self.inner.size(context)
    }

    fn padding(&self) -> f64 {
        self.inner.padding()
    }

    fn on_click(&mut self) {
        if let OptionCallback::Some(cb) = self.on_click {
            cb(self);
        }
    }
}

impl std::fmt::Debug for Volume<'_> {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Display for Volume<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Volume").fmt(f)
    }
}
