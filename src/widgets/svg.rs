use crate::{
    utils::{set_source_rgba, Color, HookSender, TimedHooks},
    widgets::{Rectangle, Result, Size, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::Context;
use rsvg::{CairoRenderer, SvgHandle};
use std::fmt::{Debug, Display};

pub struct Svg {
    handle: SvgHandle,
    padding: u32,
    fg_color: Color,
    width: u32,
}

// I don't like this but I'll use it until it gives me problems
unsafe impl Send for Svg {}

impl Debug for Svg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "padding: {:?}, fg_color: {:?}, width: {:?}",
            self.padding, self.fg_color, self.width
        )
    }
}

impl Svg {
    pub fn new(path: &str, width: u32, config: &WidgetConfig) -> Result<Box<Self>> {
        let handle = rsvg::Loader::new().read_path(path).map_err(Error::from)?;
        Ok(Box::new(Self {
            handle,
            padding: config.padding,
            fg_color: config.fg_color,
            width,
        }))
    }
}

#[async_trait]
impl Widget for Svg {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        set_source_rgba(context, self.fg_color);
        let renderer = CairoRenderer::new(&self.handle);
        let cairo_rect = cairo::Rectangle::new(0., 0., self.width as _, rectangle.height as _);
        renderer
            .render_document(context, &cairo_rect)
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
pub enum Error {
    #[error("Error loading svg: {0}")]
    Loading(#[from] rsvg::LoadingError),
    #[error("Error rendering svg: {0}")]
    Rendering(#[from] rsvg::RenderingError),
}
