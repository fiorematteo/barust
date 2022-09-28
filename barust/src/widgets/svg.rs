use super::{Result, Widget, WidgetConfig};
use cairo::{Context, Rectangle};
use librsvg::{CairoRenderer, Loader, SvgHandle};
use std::{fmt::Debug, path::Path};

/// Displays an svg image
pub struct Svg {
    handle: SvgHandle,
    padding: f64,
    width: f64,
    on_click: Option<fn(&mut Self)>,
}

impl Debug for Svg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format!(
            "SvgIcon( handle: SvgHandle, padding: {}, width: {}, )",
            self.padding, self.width
        )
        .fmt(f)
    }
}

impl Svg {
    ///* `path_to_svg` : path to the svg file
    ///* `width` width of the icon in pixel
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(
        path_to_svg: &str,
        width: f64,
        config: &WidgetConfig,
        on_click: Option<fn(&mut Self)>,
    ) -> Result<Box<Self>> {
        let handle = Loader::new()
            .read_path(Path::new(path_to_svg))
            .map_err(Error::from)?;
        Ok(Box::new(Self {
            handle,
            padding: config.padding,
            width,
            on_click,
        }))
    }
}

impl Widget for Svg {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        CairoRenderer::new(&self.handle)
            .render_document(
                context,
                &Rectangle {
                    x: 0.0,
                    y: 0.0,
                    ..*rectangle
                },
            )
            .map_err(Error::from)?;
        Ok(())
    }

    fn size(&self, _context: &Context) -> Result<f64> {
        Ok(self.width + 2.0 * self.padding)
    }

    fn padding(&self) -> f64 {
        self.padding
    }
    fn on_click(&mut self) {
        if let Some(cb) = &self.on_click {
            cb(self);
        }
    }
}

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
pub enum Error {
    LoadingSvg(librsvg::LoadingError),
    RenderingSvg(librsvg::RenderingError),
}
