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
    fn draw(&self, context: Context, rectangle: &Rectangle) -> Result<()> {
        icon!(self.draw(context, rectangle))
    }

    async fn setup(&mut self, info: &StatusBarInfo) -> Result<()> {
        icon!(self.setup(info).await)
    }

    async fn update(&mut self) -> Result<()> {
        icon!(self.update().await)
    }

    async fn hook(&mut self, sender: HookSender, pool: &mut TimedHooks) -> Result<()> {
        icon!(self.hook(sender, pool).await)
    }

    fn size(&self, context: &Context) -> Result<Size> {
        icon!(self.size(context))
    }

    fn padding(&self) -> u32 {
        icon!(self.padding())
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

macro_rules! icon {
    (
        $self:ident.$fn_name:ident($($args:ident),*)
    ) => {
        match $self {
            Icon::Svg(svg) => svg.$fn_name($($args,)*),
            Icon::Png(png) => png.$fn_name($($args,)*),
        }
    };

    (
        $self:ident.$fn_name:ident($($args:ident),*).await
    ) => {
        match $self {
            Icon::Svg(svg) => svg.$fn_name($($args,)*).await,
            Icon::Png(png) => png.$fn_name($($args,)*).await,
        }
    };
}
use icon;
