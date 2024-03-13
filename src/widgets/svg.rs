use crate::{
    utils::{HookSender, TimedHooks},
    widgets::{Rectangle, Result, Size, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::Context;
use rsvg::{CairoRenderer, SvgHandle};
use std::fmt::{Debug, Display};

pub struct Svg {
    handle: SvgHandle,
    padding: u32,
    width: u32,
}

// I don't like this but I'll use it until it gives me problems
unsafe impl Send for Svg {}

impl Debug for Svg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "padding: {:?}, width: {:?}", self.padding, self.width)
    }
}

impl Svg {
    pub fn new(path: &str, width: u32, config: &WidgetConfig) -> Result<Box<Self>> {
        let handle = rsvg::Loader::new().read_path(path).map_err(Error::from)?;
        Ok(Box::new(Self {
            handle,
            padding: config.padding,
            width,
        }))
    }
}

#[async_trait]
impl Widget for Svg {
    fn draw(&self, context: Context, rectangle: &Rectangle) -> Result<()> {
        let renderer = CairoRenderer::new(&self.handle);
        let cairo_rect = cairo::Rectangle::new(0., 0., self.width as _, rectangle.height as _);
        renderer
            .render_document(&context, &cairo_rect)
            .map_err(Error::from)?;
        Ok(())
    }

    fn size(&self, _context: &Context) -> Result<Size> {
        Ok(Size::Static(self.width))
    }

    fn padding(&self) -> u32 {
        self.padding
    }

    async fn hook(&mut self, sender: HookSender, timed_hooks: &mut TimedHooks) -> Result<()> {
        timed_hooks.subscribe(sender);
        Ok(())
    }
}

impl Display for Svg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt("Svg", f)
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    Loading(#[from] rsvg::LoadingError),
    Rendering(#[from] rsvg::RenderingError),
}
