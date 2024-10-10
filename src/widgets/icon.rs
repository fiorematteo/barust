use crate::{
    utils::{HookSender, Rectangle, StatusBarInfo, TimedHooks},
    widgets::{Png, Result, Size, Svg, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::Context;
use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

#[derive(Debug)]
pub enum Icon {
    Svg(Svg),
    Png(Png),
}

impl Icon {
    pub fn new(path: impl Into<PathBuf>, width: u32, config: &WidgetConfig) -> Result<Box<Self>> {
        let path: PathBuf = path.into();
        if !path.is_file() {
            return Err(
                Error::UnsupportedFileType(format!("{} is not a file", path.display())).into(),
            );
        }

        // if the extension is missing, assume it's a png
        if path.extension().map(|ext| ext == "png").unwrap_or(true) {
            Png::new(path, width, config)
                .map(|w| Icon::Png(*w))
                .map(Box::new)
        } else if path.extension().map(|ext| ext == "svg").unwrap_or(true) {
            Svg::new(path, width, config)
                .map(|w| Icon::Svg(*w))
                .map(Box::new)
        } else {
            Err(Error::UnsupportedFileType(path.display().to_string()).into())
        }
    }
}

impl Deref for Icon {
    type Target = dyn Widget;
    fn deref(&self) -> &Self::Target {
        match self {
            Icon::Svg(svg) => svg,
            Icon::Png(png) => png,
        }
    }
}

impl DerefMut for Icon {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Icon::Svg(svg) => svg,
            Icon::Png(png) => png,
        }
    }
}

#[async_trait]
impl Widget for Icon {
    fn draw(&self, context: Context, rectangle: &Rectangle) -> Result<()> {
        self.deref().draw(context, rectangle)
    }

    async fn setup(&mut self, info: &StatusBarInfo) -> Result<()> {
        self.deref_mut().setup(info).await
    }

    async fn update(&mut self) -> Result<()> {
        self.deref_mut().update().await
    }

    async fn hook(&mut self, sender: HookSender, pool: &mut TimedHooks) -> Result<()> {
        self.deref_mut().hook(sender, pool).await
    }

    fn size(&self, context: &Context) -> Result<Size> {
        self.deref().size(context)
    }

    fn padding(&self) -> u32 {
        self.deref().padding()
    }
}

impl Display for Icon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt("Icon", f)
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("unsupported file type: {0}")]
    UnsupportedFileType(String),
}
