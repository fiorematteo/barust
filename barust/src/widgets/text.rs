use super::{OnClickCallback, Rectangle, Result, Size, Widget, WidgetConfig};
use crate::{
    utils::{set_source_rgba, Color, OnClickRaw},
    widget_default,
};
use cairo::Context;
use pango::{FontDescription, Layout};
use pangocairo::{create_context, show_layout};
use std::fmt::Display;

/// Displays custom text
#[derive(Debug)]
pub struct Text {
    text: String,
    padding: u32,
    fg_color: Color,
    font: String,
    font_size: f64,
    flex: bool,
    on_click: OnClickCallback,
}

impl Text {
    ///* `text` text to display
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(
        text: impl ToString,
        config: &WidgetConfig,
        on_click: Option<&'static OnClickRaw>,
    ) -> Box<Self> {
        Box::new(Self {
            text: text.to_string(),
            padding: config.padding,
            fg_color: config.fg_color,
            font: config.font.into(),
            font_size: config.font_size,
            flex: config.flex,
            on_click: OnClickCallback::new(on_click),
        })
    }

    pub fn set_text(&mut self, text: impl ToString) -> &Self {
        self.text = text.to_string();
        self
    }

    fn get_layout(&self, context: &Context) -> Result<Layout> {
        let pango_context = create_context(context).ok_or(Error::PangoError)?;
        let layout = Layout::new(&pango_context);
        let mut font = FontDescription::from_string(&self.font);
        font.set_absolute_size(self.font_size * f64::from(pango::SCALE));
        layout.set_font_description(Some(&font));
        Ok(layout)
    }
}

impl Widget for Text {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        set_source_rgba(context, self.fg_color);
        let layout = self.get_layout(context)?;
        context.move_to(
            f64::from(self.padding),
            f64::from((rectangle.height - layout.pixel_size().1 as u32) / 2),
        );
        layout.set_text(&self.text);
        show_layout(context, &layout);
        Ok(())
    }

    fn size(&self, context: &Context) -> Result<Size> {
        if self.flex {
            return Ok(Size::Flex);
        }
        let layout = self.get_layout(context)?;
        layout.set_text(&self.text);
        let size = 2 * self.padding() + layout.pixel_size().0 as u32;
        Ok(Size::Static(size))
    }

    fn padding(&self) -> u32 {
        if self.text.is_empty() {
            0
        } else {
            self.padding
        }
    }

    widget_default!(on_click);
}

impl Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Text").fmt(f)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Pango error")]
    PangoError,
}
