use super::{Result, Widget};
use cairo::{Context, Rectangle};

///Adds empty space between widgets
#[derive(Debug)]
pub struct Spacer {
    size: f64,
}

impl Spacer {
    ///* `size` width of the space widget in pixel
    pub fn new(size: f64) -> Box<Self> {
        Box::new(Self { size })
    }
}

impl Widget for Spacer {
    fn draw(&self, _context: &Context, _rectangle: &Rectangle) -> Result<()> {
        Ok(())
    }

    fn size(&self, _context: &Context) -> Result<f64> {
        Ok(self.size)
    }

    fn padding(&self) -> f64 {
        0.0
    }
}
