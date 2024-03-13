use crate::{
    utils::{Color, HookSender, TimedHooks},
    widgets::{Rectangle, Result, Size, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::{Context, ImageSurface, ImageSurfaceDataOwned};
use std::{
    fmt::{Debug, Display},
    fs::File,
    sync::Mutex,
};

pub struct Png {
    handle: Mutex<Option<ImageSurfaceDataOwned>>,
    padding: u32,
    fg_color: Color,
    width: u32,
}

impl Debug for Png {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "padding: {:?}, fg_color: {:?}, width: {:?}",
            self.padding, self.fg_color, self.width
        )
    }
}

impl Png {
    pub fn new(path: &str, width: u32, config: &WidgetConfig) -> Result<Box<Self>> {
        let mut file = File::open(path).map_err(Error::from)?;
        let handle = ImageSurface::create_from_png(&mut file)
            .map_err(Error::from)?
            .take_data()
            .map_err(Error::from)?;
        Ok(Box::new(Self {
            handle: Mutex::new(Some(handle)),
            padding: config.padding,
            fg_color: config.fg_color,
            width,
        }))
    }
}

#[async_trait]
impl Widget for Png {
    fn draw(&self, context: Context, rectangle: &Rectangle) -> Result<()> {
        let handle = self
            .handle
            .lock()
            .expect("Mutex is poisoned")
            .take()
            .expect("Handle is missing")
            .into_inner();

        let png_width = handle.width();
        let png_height = handle.height();
        context.scale(
            rectangle.width as f64 / png_width as f64,
            rectangle.height as f64 / png_height as f64,
        );
        context.set_source_surface(&handle, 0.0, 0.0).unwrap();
        context.paint().unwrap();

        // we need to clear all references to the handle
        drop(context);

        let owned_data = handle.take_data().map_err(Error::from)?;
        self.handle
            .lock()
            .expect("Mutex is poisoned")
            .replace(owned_data);
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

impl Display for Png {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt("Png", f)
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    Io(#[from] std::io::Error),
    Cairo(#[from] cairo::Error),
    IoCairo(#[from] cairo::IoError),
    BorrowCairo(#[from] cairo::BorrowError),
}
