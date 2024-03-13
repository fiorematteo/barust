use crate::{
    utils::{set_source_rgba, Color},
    widgets::{Rectangle, Result, Size, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::Context;
use pango::{FontDescription, Layout};
use pangocairo::functions::{create_context, show_layout};
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
}

impl Text {
    ///* `text` text to display
    ///* `config` a [WidgetConfig]
    pub async fn new(text: impl ToString, config: &WidgetConfig) -> Box<Self> {
        Box::new(Self {
            text: text.to_string(),
            padding: config.padding,
            fg_color: config.fg_color,
            font: config.font.clone(),
            font_size: config.font_size,
            flex: config.flex,
        })
    }

    pub fn set_text(&mut self, text: impl ToString) {
        self.text = text.to_string();
    }

    fn get_layout(&self, context: &Context) -> Result<Layout> {
        let pango_context = create_context(context);
        let layout = Layout::new(&pango_context);
        let mut font = FontDescription::from_string(&self.font);
        font.set_absolute_size(self.font_size * f64::from(pango::SCALE));
        layout.set_font_description(Some(&font));
        Ok(layout)
    }
}

#[async_trait]
impl Widget for Text {
    fn draw(&self, context: Context, rectangle: &Rectangle) -> Result<()> {
        set_source_rgba(&context, self.fg_color);
        let layout = self.get_layout(&context)?;
        context.move_to(
            0.,
            f64::from((rectangle.height - layout.pixel_size().1 as u32) / 2),
        );
        layout.set_text(&self.text);
        show_layout(&context, &layout);
        Ok(())
    }

    fn size(&self, context: &Context) -> Result<Size> {
        if self.flex {
            return Ok(Size::Flex);
        }
        let layout = self.get_layout(context)?;
        layout.set_text(&self.text);
        let size = layout.pixel_size().0 as u32;
        Ok(Size::Static(size))
    }

    fn padding(&self) -> u32 {
        if self.text.is_empty() {
            0
        } else {
            self.padding
        }
    }
}

impl Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Text").fmt(f)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {}
