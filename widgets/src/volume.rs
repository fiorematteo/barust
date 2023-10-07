use crate::{widget_default, Rectangle, Result, Text, Widget, WidgetConfig};
use async_trait::async_trait;
use libpulse_binding::{
    callbacks::ListResult,
    context::{self, introspect::Introspector, Context, FlagSet},
    mainloop::threaded::Mainloop,
    operation::{Operation, State},
    volume::{ChannelVolumes, Volume as PaVolume},
};
use log::debug;
use std::{
    fmt::Display,
    mem::forget,
    sync::{Arc, Mutex},
};
use tokio::task::yield_now;
use utils::{percentage_to_index, HookSender, ResettableTimer, TimedHooks};

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
    provider: Box<dyn VolumeProvider + Send>,
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
    ///* `on_click` callback to run on click
    pub async fn new(
        format: impl ToString,
        provider: Box<impl VolumeProvider + 'static + std::marker::Send>,
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
}

#[async_trait]
impl Widget for Volume {
    fn draw(&self, context: &cairo::Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    async fn update(&mut self) -> Result<()> {
        debug!("updating volume");
        let muted = self.provider.muted();
        let muted = muted.await.unwrap_or(false);
        let volume = self.provider.volume();
        let volume = volume.await.unwrap_or(0.0);

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

    widget_default!(size, padding);
}

impl Volume {
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

impl Display for Volume {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Volume").fmt(f)
    }
}

#[async_trait]
pub trait VolumeProvider: std::fmt::Debug {
    async fn volume(&self) -> Option<f64>;
    async fn muted(&self) -> Option<bool>;
}

#[async_trait]
trait WaitOp<T: ?Sized> {
    async fn wait(&self);
}

#[async_trait]
impl<T: ?Sized> WaitOp<T> for Operation<T> {
    async fn wait(&self) {
        while self.get_state() == State::Running {
            yield_now().await;
        }
    }
}

async fn get_default_sink(introspector: &Introspector) -> Option<String> {
    let cell = Arc::new(Mutex::new(None));
    let cell2 = cell.clone();
    introspector
        .get_server_info(move |s| {
            *cell2.lock().unwrap() = Some(s.default_sink_name.as_ref().unwrap().to_string());
        })
        .wait()
        .await;
    let mut lock = cell.lock().ok()?;
    lock.take()
}

async fn get_default_volume(introspector: &Introspector) -> Option<ChannelVolumes> {
    let name = get_default_sink(introspector).await.unwrap();

    let cell = Arc::new(Mutex::new(None));
    let cell2 = cell.clone();
    introspector
        .get_sink_info_by_name(&name, move |r| {
            let ListResult::Item(info) = r else {return};
            *cell2.lock().unwrap() = Some(info.volume);
        })
        .wait()
        .await;
    let mut lock = cell.lock().ok()?;
    lock.take()
}

async fn get_default_mute(introspector: &Introspector) -> Option<bool> {
    let name = get_default_sink(introspector).await.unwrap();

    let cell = Arc::new(Mutex::new(None));
    let cell2 = cell.clone();
    introspector
        .get_sink_info_by_name(&name, move |r| {
            let ListResult::Item(info) = r else {return};
            *cell2.lock().unwrap() = Some(info.mute);
        })
        .wait()
        .await;
    let mut lock = cell.lock().ok()?;
    lock.take()
}

async fn setup_pulseaudio() -> Result<Introspector> {
    let mut pulseloop = Mainloop::new().unwrap();
    let mut context = Context::new(&pulseloop, "barust").unwrap();

    context.connect(None, FlagSet::NOFLAGS, None).unwrap();
    pulseloop.start().unwrap();
    loop {
        match context.get_state() {
            context::State::Ready => {
                break;
            }
            context::State::Failed | context::State::Terminated => {
                pulseloop.unlock();
                pulseloop.stop();
                return Err(Error::PulseAudio("Failed to connect".into()).into());
            }
            _ => {
                pulseloop.wait();
            }
        }
    }

    let intro = context.introspect();
    forget(pulseloop);
    forget(context);

    Ok(intro)
}

fn volume_to_percent(volume: ChannelVolumes) -> f64 {
    let avg = volume.avg().0;

    let base_delta = (PaVolume::NORMAL.0 as f64 - PaVolume::MUTED.0 as f64) / 100.0;

    (avg - PaVolume::MUTED.0) as f64 / base_delta
}

pub struct PulseaudioProvider(Introspector);

impl PulseaudioProvider {
    pub async fn new() -> Result<Self> {
        Ok(Self(setup_pulseaudio().await?))
    }
}

impl From<Introspector> for PulseaudioProvider {
    fn from(value: Introspector) -> Self {
        Self(value)
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
        get_default_volume(&self.0)
            .await
            .map(volume_to_percent)
            .map(Into::into)
    }
    async fn muted(&self) -> Option<bool> {
        get_default_mute(&self.0).await
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    Psutil(#[from] psutil::Error),
    #[error("PulseAudio error: {0}")]
    PulseAudio(String),
}
