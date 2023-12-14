use crate::widgets::{Rectangle, Result, Size, Widget};
use async_trait::async_trait;
use cairo::Context;
use std::fmt::Display;

///Adds empty space between widgets
#[derive(Debug)]
pub struct Spacer {
    size: u32,
}

impl Spacer {
    ///* `size` width of the space widget in pixel
    pub async fn new(size: u32) -> Box<Self> {
        Box::new(Self { size })
    }
}

#[async_trait]
impl Widget for Spacer {
    fn draw(&self, _context: &Context, _rectangle: &Rectangle) -> Result<()> {
        Ok(())
    }

    fn size(&self, _context: &Context) -> Result<Size> {
        Ok(Size::Static(self.size))
    }

    fn padding(&self) -> u32 {
        0
    }
}

impl Display for Spacer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Spacer").fmt(f)
    }
}
