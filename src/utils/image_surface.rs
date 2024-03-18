use cairo::{BorrowError, ImageSurface, ImageSurfaceDataOwned};
use std::sync::Mutex;

pub struct OwnedImageSurface {
    surface: Mutex<Option<ImageSurfaceDataOwned>>,
}

impl OwnedImageSurface {
    pub fn new(surface: ImageSurface) -> Result<Self, BorrowError> {
        Ok(Self {
            surface: Mutex::new(Some(surface.take_data()?)),
        })
    }

    pub fn with_surface<
        F: FnOnce(&ImageSurface) -> Result<(), E>,
        E: std::error::Error + From<cairo::BorrowError>,
    >(
        &self,
        lambda: F,
    ) -> Result<(), E> {
        let mut guard = self.surface.lock().expect("Mutex is poisoned");
        let surface = guard.take().unwrap().into_inner();
        lambda(&surface)?;
        guard.replace(surface.take_data()?);
        Ok(())
    }
}
