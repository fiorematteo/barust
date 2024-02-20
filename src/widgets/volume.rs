use crate::{
    utils::{percentage_to_index, HookSender, ResettableTimer, TimedHooks},
    widget_default,
    widgets::{Rectangle, Result, Text, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::Context;
use log::debug;
use std::{fmt::Display, marker::Send};

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
    provider: Box<dyn VolumeProvider>,
    icons: VolumeIcons,
    previous_volume: f64,
    previous_muted: bool,
    show_counter: ResettableTimer,
}

impl Volume {
    ///* `format`
    ///  * *%p* will be replaced with the volume percentage
    ///  * *%i* will be replaced with the correct icon
    ///* `volume_command` a function that returns the volume in a range from 0 to 100
    ///* `muted_command` a function that returns true if the volume is muted
    ///* `icons` sets a custom [VolumeIcons]
    ///* `config` a [&WidgetConfig]
    pub async fn new(
        format: impl ToString,
        provider: Box<impl VolumeProvider + 'static>,
        icons: Option<VolumeIcons>,
        config: &WidgetConfig,
    ) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            provider,
            icons: icons.unwrap_or_default(),
            previous_volume: 0.0,
            previous_muted: false,
            show_counter: ResettableTimer::new(config.hide_timeout),
            inner: *Text::new("", config).await,
        })
    }

    fn build_string(&mut self, volume: f64, muted: bool) -> String {
        if self.show_counter.is_done() {
            return String::from("");
        }
        if muted {
            return self.icons.muted.clone();
        }
        let percentages_len = self.icons.percentages.len();
        let index = percentage_to_index(volume, (0, percentages_len - 1));
        self.format
            .replace("%p", &format!("{:.1}", volume))
            .replace("%i", &self.icons.percentages[index].to_string())
    }
}

#[async_trait]
impl Widget for Volume {
    async fn update(&mut self) -> Result<()> {
        debug!("updating volume");
        let f = self.provider.volume_and_muted();
        let (volume, muted) = f.await.unwrap_or((0.0, false));

        if self.previous_muted != muted || self.previous_volume != volume {
            self.previous_muted = muted;
            self.previous_volume = volume;
            self.show_counter.reset();
        }
        let text = self.build_string(volume, muted);

        self.inner.set_text(text);
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, timed_hooks: &mut TimedHooks) -> Result<()> {
        timed_hooks.subscribe(sender);
        Ok(())
    }

    widget_default!(draw, size, padding);
}

impl Display for Volume {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Volume").fmt(f)
    }
}

#[async_trait]
pub trait VolumeProvider: std::fmt::Debug + Send {
    async fn volume(&self) -> Option<f64>;
    async fn muted(&self) -> Option<bool>;
    async fn volume_and_muted(&self) -> Option<(f64, bool)>;
}

#[cfg(feature = "pulseaudio")]
pub mod pulseaudio {
    use std::{fmt::Display, thread};

    use super::{Result, VolumeProvider};
    use async_channel::{bounded, Receiver, Sender};
    use async_trait::async_trait;
    use libpulse_binding::volume::{ChannelVolumes, Volume as PaVolume};
    use pulsectl::controllers::DeviceControl;

    fn volume_to_percent(volume: ChannelVolumes) -> f64 {
        let avg = volume.avg().0;

        let base_delta = (PaVolume::NORMAL.0 as f64 - PaVolume::MUTED.0 as f64) / 100.0;

        (avg - PaVolume::MUTED.0) as f64 / base_delta
    }

    pub struct PulseaudioProvider {
        request: Sender<()>,
        data: Receiver<Option<(f64, bool)>>,
    }

    impl PulseaudioProvider {
        pub async fn new() -> Result<Self> {
            let (request_tx, request_rx) = bounded(10);
            let (data_tx, data_rx) = bounded(10);
            thread::spawn(move || {
                let mut controller = pulsectl::controllers::SinkController::create().unwrap();
                while request_rx.recv_blocking().is_ok() {
                    let data = if let Ok(default_device) = controller.get_default_device() {
                        Some((
                            volume_to_percent(default_device.volume),
                            default_device.mute,
                        ))
                    } else {
                        None
                    };

                    data_tx.send_blocking(data).unwrap();
                }
            });
            Ok(Self {
                request: request_tx,
                data: data_rx,
            })
        }
    }

    impl std::fmt::Debug for PulseaudioProvider {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            Display::fmt(&"PulseAudio Provider", f)
        }
    }

    #[async_trait]
    impl VolumeProvider for PulseaudioProvider {
        async fn volume(&self) -> Option<f64> {
            self.request.send(()).await.ok()?;
            self.data.recv().await.ok()?.map(|(v, _)| v)
        }

        async fn muted(&self) -> Option<bool> {
            self.request.send(()).await.ok()?;
            self.data.recv().await.ok()?.map(|(_, m)| m)
        }

        async fn volume_and_muted(&self) -> Option<(f64, bool)> {
            self.request.send(()).await.ok()?;
            self.data.recv().await.ok()?
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {}
