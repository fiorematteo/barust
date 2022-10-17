use super::{OnClickCallback, Result, Text, Widget, WidgetConfig};
use crate::corex::{Callback, EmptyCallback, HookSender, RawCallback, TimedHooks};
use log::debug;
use std::{cmp::min, fmt::Display, time::Duration};

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
#[derive(Debug)]
pub struct Volume {
    format: String,
    inner: Text,
    volume_command: Callback<(), Option<f64>>,
    muted_command: Callback<(), Option<bool>>,
    icons: VolumeIcons,
    on_click: OnClickCallback,
}

impl Volume {
    ///* `format`
    ///  * *%p* will be replaced with the volume percentage
    ///  * *%i* will be replaced with the correct icon
    ///* `volume_command` a function that returns the volume in a range from 0 to 100
    ///* `muted_command` a function that returns true if the volume is muted
    ///* `icons` sets a custom [VolumeIcons]
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(
        format: impl ToString,
        volume_command: &'static RawCallback<(), Option<f64>>,
        muted_command: &'static RawCallback<(), Option<bool>>,
        icons: Option<VolumeIcons>,
        config: &WidgetConfig,
        on_click: Option<&'static EmptyCallback>,
    ) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            volume_command: volume_command.into(),
            muted_command: muted_command.into(),
            icons: icons.unwrap_or_default(),
            inner: *Text::new("VOLUME", config, None),
            on_click: on_click.map(|c| c.into()),
        })
    }
}

impl Widget for Volume {
    fn draw(&self, context: &cairo::Context, rectangle: &cairo::Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }
    fn update(&mut self) -> Result<()> {
        debug!("updating volume");
        let text = if self.muted_command.call(()).unwrap_or_default() {
            self.icons.muted.clone()
        } else {
            let volume = self.volume_command.call(()).unwrap_or_default();
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

    fn hook(&mut self, sender: HookSender, timed_hooks: &mut TimedHooks) -> Result<()> {
        timed_hooks
            .subscribe(Duration::from_secs(1), sender)
            .map_err(Error::from)?;
        Ok(())
    }

    fn size(&self, context: &cairo::Context) -> Result<f64> {
        self.inner.size(context)
    }

    fn padding(&self) -> f64 {
        self.inner.padding()
    }

    fn on_click(&self) {
        if let Some(cb) = &self.on_click {
            cb.call(());
        }
    }
}

impl Display for Volume {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Volume").fmt(f)
    }
}

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
pub enum Error {
    HookChannel(crossbeam_channel::SendError<HookSender>),
    Psutil(psutil::Error),
}
