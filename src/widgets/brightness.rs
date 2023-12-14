use crate::{
    utils::{percentage_to_index, HookSender, ResettableTimer, TimedHooks},
    widget_default,
    widgets::{Rectangle, Result, Text, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::Context;
use std::{fmt::Display, fs, io::SeekFrom, ops::DerefMut, path::PathBuf, process::Command};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
    sync::Mutex,
};

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
    brightness_provider: Box<dyn BrightnessProvider>,
    previous_brightness: f64,
    show_counter: ResettableTimer,
    inner: Text,
    icons: BrightnessIcons,
}

impl Brightness {
    ///* `format`
    ///  * *%p* will be replaced with the brightness percentage
    ///  * *%i* will be replaced with the correct icon
    ///* `brightness_command` a function that returns the brightness in a range from 0 to 100
    ///* `icons` sets a custom [VolumeIcons]
    ///* `config` a [&WidgetConfig]
    pub async fn new(
        format: impl ToString,
        brightness_provider: Box<impl BrightnessProvider + 'static>,
        icons: Option<BrightnessIcons>,
        config: &WidgetConfig,
    ) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            previous_brightness: 0.0,
            brightness_provider,
            show_counter: ResettableTimer::new(config.hide_timeout),
            inner: *Text::new("", config).await,
            icons: icons.unwrap_or_default(),
        })
    }

    fn build_string(&self, current_brightness: f64) -> String {
        if self.show_counter.is_done() {
            return String::from("");
        }
        let percentages_len = self.icons.percentages.len();
        let index = percentage_to_index(current_brightness, (0, percentages_len - 1));
        self.format
            .replace("%p", &format!("{:.0}", current_brightness))
            .replace("%i", &self.icons.percentages[index].to_string())
    }
}

#[async_trait]
impl Widget for Brightness {
    async fn update(&mut self) -> Result<()> {
        let f = self.brightness_provider.brightness();
        let current_brightness = f.await.ok_or(Error::Command)?;

        if current_brightness != self.previous_brightness {
            self.previous_brightness = current_brightness;
            self.show_counter.reset();
        }
        let text = self.build_string(current_brightness);
        self.inner.set_text(text);

        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, timed_hooks: &mut TimedHooks) -> Result<()> {
        timed_hooks.subscribe(sender);
        Ok(())
    }

    widget_default!(draw, size, padding);
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
    Command,
    Io(#[from] std::io::Error),
    #[error("Failed to find a valid sysfs folder")]
    NoBrightnessFile,
}

#[async_trait]
pub trait BrightnessProvider: std::fmt::Debug + Send {
    async fn brightness(&self) -> Option<f64>;
}

#[derive(Debug, Default)]
pub struct LightProvider;

impl LightProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BrightnessProvider for LightProvider {
    async fn brightness(&self) -> Option<f64> {
        String::from_utf8(Command::new("light").output().ok()?.stdout)
            .ok()?
            .trim()
            .parse::<f64>()
            .ok()
    }
}

#[derive(Debug)]
pub struct SysfsProvider {
    brightness_file: Mutex<File>,
    max_brightness_file: Mutex<File>,
}

impl SysfsProvider {
    pub async fn new() -> Result<Self> {
        let mut folder = PathBuf::from("/sys/class/backlight");
        let mut d = fs::read_dir(&folder).map_err(Error::from)?;
        let device = d
            .next()
            .ok_or(Error::NoBrightnessFile)?
            .map_err(Error::from)?;
        folder.push(device.file_name());

        let mut brightness = None;
        let mut max_brightness = None;
        let mut d = fs::read_dir(folder).map_err(Error::from)?;
        while let Some(Ok(file)) = d.next() {
            match file.file_name().to_str() {
                Some("brightness") => {
                    let mut path = device.path();
                    path.push("brightness");
                    brightness = Some(path)
                }
                Some("max_brightness") => {
                    let mut path = device.path();
                    path.push("max_brightness");
                    max_brightness = Some(path)
                }
                _ => (),
            }
        }
        let brightness_path = brightness.ok_or(Error::NoBrightnessFile)?;
        let max_brightness_path = max_brightness.ok_or(Error::NoBrightnessFile)?;
        let brightness_file = File::open(&brightness_path).await.map_err(Error::from)?;
        let max_brightness_file = File::open(&max_brightness_path)
            .await
            .map_err(Error::from)?;
        Ok(Self {
            brightness_file: Mutex::new(brightness_file),
            max_brightness_file: Mutex::new(max_brightness_file),
        })
    }

    async fn read_brightness_raw(&self) -> Option<f64> {
        read_file_from_start(self.brightness_file.lock().await)
            .await?
            .trim()
            .parse::<f64>()
            .ok()
    }

    async fn read_max_brightness_raw(&self) -> Option<f64> {
        read_file_from_start(self.max_brightness_file.lock().await)
            .await?
            .trim()
            .parse::<f64>()
            .ok()
    }
}

async fn read_file_from_start<T: AsyncReadExt + AsyncSeekExt + Unpin>(
    mut f: impl DerefMut<Target = T>,
) -> Option<String> {
    f.seek(SeekFrom::Start(0)).await.ok()?;
    let mut buf = String::new();
    f.read_to_string(&mut buf).await.ok()?;
    Some(buf)
}

#[async_trait]
impl BrightnessProvider for SysfsProvider {
    async fn brightness(&self) -> Option<f64> {
        Some(self.read_brightness_raw().await? / self.read_max_brightness_raw().await? * 100.0)
    }
}
