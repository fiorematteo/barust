use crate::{
    utils::{percentage_to_index, HookSender, ResettableTimer, TimedHooks},
    widget_default,
    widgets::{Result, Text, Widget, WidgetConfig},
};
use async_trait::async_trait;
use futures::StreamExt;
use inotify::Inotify;
use log::{debug, error};
use std::{fmt::Display, fs, io::SeekFrom, path::PathBuf};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
    spawn,
    sync::Mutex,
    time::sleep,
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
    previous_brightness: f64,
    show_counter: ResettableTimer,
    inner: Text,
    icons: BrightnessIcons,
    brightness_file: Mutex<File>,
    max_brightness_file: Mutex<File>,
    device: Option<String>,
}

impl Brightness {
    ///* `format`
    ///  * *%p* will be replaced with the brightness percentage
    ///  * *%i* will be replaced with the correct icon
    ///* `icons` sets a custom [VolumeIcons]
    ///* `config` a [&WidgetConfig]
    pub async fn new(
        format: impl ToString,
        icons: Option<BrightnessIcons>,
        device: Option<String>,
        config: &WidgetConfig,
    ) -> Result<Box<Self>> {
        let (brightness_path, max_brightness_path) = Self::brightness_file_path(&device)?;
        let brightness_file = File::open(&brightness_path).await.map_err(Error::from)?;
        let max_brightness_file = File::open(&max_brightness_path)
            .await
            .map_err(Error::from)?;

        Ok(Box::new(Self {
            format: format.to_string(),
            previous_brightness: -1.0,
            show_counter: ResettableTimer::new(config.hide_timeout),
            inner: *Text::new("", config).await,
            icons: icons.unwrap_or_default(),
            brightness_file: Mutex::new(brightness_file),
            max_brightness_file: Mutex::new(max_brightness_file),
            device,
        }))
    }

    fn build_string(&self, current_brightness: f64) -> String {
        let percentages_len = self.icons.percentages.len();
        let index = percentage_to_index(current_brightness, (0, percentages_len - 1));
        self.format
            .replace("%p", &format!("{:.0}", current_brightness))
            .replace("%i", &self.icons.percentages[index].to_string())
    }

    async fn read_brightness_raw(&self) -> Result<f64> {
        Self::fetch_from_file(&self.brightness_file).await
    }

    async fn read_max_brightness_raw(&self) -> Result<f64> {
        Self::fetch_from_file(&self.max_brightness_file).await
    }

    async fn fetch_from_file(file: &Mutex<File>) -> Result<f64> {
        let mut file = file.lock().await;
        file.seek(SeekFrom::Start(0)).await.map_err(Error::from)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf).await.map_err(Error::from)?;
        Ok(buf.trim().parse::<f64>().map_err(Error::from)?)
    }

    async fn brightness(&self) -> Result<f64> {
        Ok(self.read_brightness_raw().await? / self.read_max_brightness_raw().await? * 100.0)
    }

    fn brightness_file_path(
        device_name: &Option<String>,
    ) -> std::result::Result<(PathBuf, PathBuf), Error> {
        let mut folder = PathBuf::from("/sys/class/backlight");
        let mut d = fs::read_dir(&folder).map_err(Error::from)?;

        if let Some(device_name) = device_name {
            folder.push(device_name);
        } else {
            folder = d
                .next()
                .ok_or(Error::NoBrightnessFile)?
                .map_err(Error::from)?
                .path();
        }

        let mut brightness = None;
        let mut max_brightness = None;
        let mut d = fs::read_dir(&folder).map_err(Error::from)?;
        while let Some(Ok(file)) = d.next() {
            match file.file_name().to_str() {
                Some("brightness") => {
                    let mut path = folder.clone();
                    path.push("brightness");
                    brightness = Some(path);
                }
                Some("max_brightness") => {
                    let mut path = folder.clone();
                    path.push("max_brightness");
                    max_brightness = Some(path);
                }
                _ => (),
            }
        }
        Ok((
            brightness.ok_or(Error::NoBrightnessFile)?,
            max_brightness.ok_or(Error::NoBrightnessFile)?,
        ))
    }
}

#[async_trait]
impl Widget for Brightness {
    async fn update(&mut self) -> Result<()> {
        let current_brightness = self.brightness().await?;
        if self.previous_brightness == -1.0 {
            // first_update
            self.previous_brightness = current_brightness;
            self.inner.clear();
            return Ok(());
        }
        if current_brightness != self.previous_brightness {
            self.previous_brightness = current_brightness;
            self.show_counter.reset();
        }
        if self.show_counter.is_done() {
            self.inner.clear();
        } else {
            let text = self.build_string(current_brightness);
            self.inner.set_text(text);
        }
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, _timed_hooks: &mut TimedHooks) -> Result<()> {
        let (path, _) = Self::brightness_file_path(&self.device)?;

        let events = Inotify::init().unwrap();
        events
            .watches()
            .add(path, inotify::WatchMask::MODIFY)
            .map_err(Error::from)?;
        let show_counter_duration = self.show_counter.duration;
        spawn(async move {
            let mut buffer = [0; 1024];
            let mut event_stream = events.into_event_stream(&mut buffer).unwrap();
            loop {
                match event_stream.next().await {
                    Some(Ok(_event)) => {
                        if let Err(e) = sender.send().await {
                            debug!("breaking thread loop: {}", e);
                            return;
                        }
                        let c_sender = sender.clone();
                        spawn(async move {
                            // hide after some time
                            sleep(show_counter_duration).await;
                            let _ = c_sender.send().await;
                        });
                    }
                    Some(Err(e)) => {
                        debug!("breaking thread loop: {}", e);
                        return;
                    }
                    None => {}
                }
            }
        });
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
    Io(#[from] std::io::Error),
    #[error("Failed to find a valid sysfs folder")]
    NoBrightnessFile,
    #[error("Failed to parse brightness file")]
    Parse(#[from] std::num::ParseFloatError),
}
