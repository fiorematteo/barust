use crate::{Result, Size, Widget, WidgetConfig};
use async_trait::async_trait;
use std::{cell::RefCell, fs::File};
use utils::{Rectangle, StatusBarInfo};

#[derive(Debug)]
pub struct Png {
    image: RefCell<cairo::ImageSurface>,
    width: Option<u32>,
    padding: u32,
}

impl Png {
    pub async fn new(
        path: impl ToString,
        width: Option<u32>,
        config: &WidgetConfig,
    ) -> Result<Box<Self>> {
        let mut file = File::open(path.to_string()).map_err(Error::from)?;
        let image = cairo::ImageSurface::create_from_png(&mut file).map_err(Error::from)?;
        Ok(Box::new(Self {
            image: RefCell::new(image),
            width,
            padding: config.padding,
        }))
    }
}

unsafe impl Send for Png {}

#[async_trait]
impl Widget for Png {
    fn setup(&mut self, info: &StatusBarInfo) -> Result<()> {
        if self.width.is_none() {
            self.width = Some(info.height as _);
        }
        Ok(())
    }

    fn draw(&self, context: &cairo::Context, rectangle: &Rectangle) -> Result<()> {
        let image = self.image.borrow_mut();
        let y_scale = rectangle.height as f64 / image.height() as f64;
        let x_scale = self.width.ok_or(Error::MissedSetup)? as f64 / image.width() as f64;
        let scale = y_scale.min(x_scale);
        context.scale(scale, scale);
        context
            .set_source_surface(&image, 0.0, 0.0)
            .map_err(Error::from)?;
        context.paint().map_err(Error::from)?;
        Ok(())
    }

    fn size(&self, _context: &cairo::Context) -> Result<Size> {
        Ok(Size::Static(self.width.ok_or(Error::MissedSetup)?))
    }

    fn padding(&self) -> u32 {
        self.padding
    }
}

impl std::fmt::Display for Png {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Png")
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    Cairo(#[from] cairo::Error),
    CairoIO(#[from] cairo::IoError),
    Io(#[from] std::io::Error),
    #[error("Unreachable")]
    MissedSetup,
}
