use super::{OnClickCallback, Result, Size, Widget, WidgetConfig};
use crate::{utils::OnClickRaw, widget_default};
use std::{
    fs::File,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub struct Png {
    image: Arc<Mutex<cairo::ImageSurface>>,
    width: Option<u32>,
    padding: u32,
    on_click: OnClickCallback,
}

impl Png {
    pub fn new(
        path: impl ToString,
        width: Option<u32>,
        config: &WidgetConfig,
        on_click: Option<&'static OnClickRaw>,
    ) -> Result<Box<Self>> {
        let mut file = File::open(path.to_string()).map_err(Error::from)?;
        let image = cairo::ImageSurface::create_from_png(&mut file).map_err(Error::from)?;
        Ok(Box::new(Self {
            image: Arc::new(Mutex::new(image)),
            width,
            padding: config.padding,
            on_click: OnClickCallback::new(on_click),
        }))
    }
}

unsafe impl Send for Png {}

impl Widget for Png {
    fn setup(&mut self, info: &crate::statusbar::StatusBarInfo) -> Result<()> {
        if self.width.is_none() {
            self.width = Some(info.height as _);
        }
        Ok(())
    }

    fn draw(&self, context: &cairo::Context, rectangle: &crate::utils::Rectangle) -> Result<()> {
        let image = self.image.lock().map_err(|_| Error::Mutex)?;
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

    widget_default!(on_click);
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
    #[error("Mutex Error")]
    Mutex,
}
