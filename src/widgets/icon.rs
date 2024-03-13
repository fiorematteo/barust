use crate::{
    utils::{HookSender, Rectangle, StatusBarInfo, TimedHooks},
    widgets::{Png, Result, Size, Svg, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::Context;
use std::fmt::Display;

#[derive(Debug)]
pub enum Icon {
    Svg(Svg),
    Png(Png),
}

impl Icon {
    pub fn new(path: &str, width: u32, config: &WidgetConfig) -> Result<Box<Self>> {
        if path.ends_with(".svg") {
            Svg::new(path, width, config)
                .map(|w| Icon::Svg(*w))
                .map(Box::new)
        } else if path.ends_with(".png") {
            Png::new(path, width, config)
                .map(|w| Icon::Png(*w))
                .map(Box::new)
        } else {
            Err(Error::UnsupportedFileType(path.to_string()).into())
        }
    }
}

#[async_trait]
impl Widget for Icon {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        match self {
            Icon::Svg(svg) => svg.draw(context, rectangle),
            Icon::Png(png) => png.draw(context, rectangle),
        }
    }

    async fn setup(&mut self, info: &StatusBarInfo) -> Result<()> {
        match self {
            Icon::Svg(svg) => svg.setup(info).await,
            Icon::Png(png) => png.setup(info).await,
        }
    }

    async fn update(&mut self) -> Result<()> {
        match self {
            Icon::Svg(svg) => svg.update().await,
            Icon::Png(png) => png.update().await,
        }
    }

    async fn hook(&mut self, sender: HookSender, pool: &mut TimedHooks) -> Result<()> {
        match self {
            Icon::Svg(svg) => svg.hook(sender, pool).await,
            Icon::Png(png) => png.hook(sender, pool).await,
        }
    }

    fn size(&self, context: &Context) -> Result<Size> {
        match self {
            Icon::Svg(svg) => svg.size(context),
            Icon::Png(png) => png.size(context),
        }
    }

    fn padding(&self) -> u32 {
        match self {
            Icon::Svg(svg) => svg.padding(),
            Icon::Png(png) => png.padding(),
        }
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
