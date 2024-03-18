use crate::{
    utils::{HookSender, OwnedImageSurface, TimedHooks},
    widgets::{Rectangle, Result, Size, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::{Context, Format, ImageSurface};
use rsvg::CairoRenderer;
use std::fmt::{Debug, Display};

pub struct Svg {
    surface: OwnedImageSurface,
    padding: u32,
    width: u32,
}

impl Debug for Svg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "padding: {:?}, width: {:?}", self.padding, self.width)
    }
}

impl Svg {
    pub fn new(path: &str, width: u32, config: &WidgetConfig) -> Result<Box<Self>> {
        let handle = rsvg::Loader::new().read_path(path).map_err(Error::from)?;

        let surface =
            ImageSurface::create(Format::ARgb32, width as _, width as _).map_err(Error::from)?;
        let context = Context::new(&surface).unwrap();
        let renderer = CairoRenderer::new(&handle);
        let cairo_rect = cairo::Rectangle::new(0., 0., width as _, width as _);
        renderer
            .render_document(&context, &cairo_rect)
            .map_err(Error::from)?;
        drop(context);

        Ok(Box::new(Self {
            surface: OwnedImageSurface::new(surface).map_err(Error::from)?,
            padding: config.padding,
            width,
        }))
    }
}

#[async_trait]
impl Widget for Svg {
    fn draw(&self, context: Context, _rectangle: &Rectangle) -> Result<()> {
        self.surface
            .with_surface(|surface: &ImageSurface| -> std::result::Result<(), Error> {
                context.set_source_surface(surface, 0.0, 0.0).unwrap();
                context.paint().unwrap();

                // we need to clear all references to the handle
                drop(context);
                Ok(())
            })
            .map_err(|e| e.into())
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
    Cairo(#[from] cairo::Error),
    BorrowCairo(#[from] cairo::BorrowError),
}
