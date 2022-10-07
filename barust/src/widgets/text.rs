use super::{OnClickCallback, Result, Widget, WidgetConfig};
use crate::corex::{set_source_rgba, Color, OptionCallback, RawCallback};
use cairo::{Context, Rectangle};
use pango::{FontDescription, Layout};
use pangocairo::{create_context, show_layout};
use std::fmt::Display;

/// Displays custom text
#[derive(Debug)]
pub struct Text {
    text: String,
    padding: f64,
    fg_color: Color,
    font: String,
    font_size: f64,
    on_click: OnClickCallback,
}

impl Text {
    ///* `text` text to display
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(
        text: &str,
        config: &WidgetConfig,
        on_click: Option<&'static RawCallback<(), ()>>,
    ) -> Box<Self> {
        Box::new(Self {
            text: text.to_string(),
            padding: config.padding,
            fg_color: config.fg_color,
            font: config.font.into(),
            font_size: config.font_size,
            on_click: on_click.into(),
        })
    }

    pub fn set_text(&mut self, text: String) -> &Self {
        self.text = text;
        self
    }

    fn get_layout(&self, context: &Context) -> Result<Layout> {
        let pango_context = create_context(context).ok_or(Error::PangoError)?;
        let layout = Layout::new(&pango_context);
        let mut font = FontDescription::from_string(&self.font);
        font.set_absolute_size(self.font_size * pango::SCALE as f64);
        layout.set_font_description(Some(&font));
        Ok(layout)
    }
}

impl Widget for Text {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        set_source_rgba(context, self.fg_color);
        let layout = self.get_layout(context)?;
        context.move_to(
            self.padding,
            (rectangle.height - layout.pixel_size().1 as f64) / 2.0,
        );
        layout.set_text(&self.text);
        show_layout(context, &layout);
        Ok(())
    }

    fn size(&self, context: &Context) -> Result<f64> {
        let layout = self.get_layout(context)?;
        layout.set_text(&self.text);
        Ok(2.0 * self.padding + layout.pixel_size().0 as f64)
    }

    fn padding(&self) -> f64 {
        self.padding
    }

    fn on_click(&self) {
        if let OptionCallback::Some(cb) = &self.on_click {
            cb.call(());
        }
    }
}

impl Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Text").fmt(f)
    }
}

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
pub enum Error {
    PangoError,
}
