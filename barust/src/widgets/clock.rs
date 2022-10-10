use super::{OnClickCallback, Result, Text, Widget, WidgetConfig};
use crate::corex::EmptyCallback;
use cairo::{Context, Rectangle};
use chrono::Local;
use crossbeam_channel::Sender;
use log::debug;
use std::{
    fmt::{Debug, Display},
    thread,
    time::Duration,
};

/// Displays a datetime
pub struct Clock {
    format: String,
    inner: Text,
    on_click: OnClickCallback,
}

impl Debug for Clock {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Clock(format: {}, padding: {})",
            self.format,
            self.inner.padding(),
        )
    }
}

impl Clock {
    ///* `format` describes how to display the time following [chrono format rules](chrono::format::strftime)
    ///* `config` a [WidgetConfig]
    ///* `on_click` callback to run on click
    pub fn new(
        format: &str,
        config: &WidgetConfig,
        on_click: Option<&'static EmptyCallback>,
    ) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            inner: *Text::new(&Self::current_time_str(format), config, None),
            on_click: on_click.map(|c| c.into()),
        })
    }

    #[inline(always)]
    fn current_time_str(format: &str) -> String {
        Local::now().format(format).to_string()
    }
}

impl Widget for Clock {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating clock");
        self.inner.set_text(Self::current_time_str(&self.format));
        Ok(())
    }

    fn hook(&mut self, sender: Sender<()>) -> Result<()> {
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(1));
            if sender.send(()).is_err() {
                break;
            }
        });
        Ok(())
    }

    fn size(&self, context: &Context) -> Result<f64> {
        self.inner.size(context)
    }

    fn padding(&self) -> f64 {
        self.inner.padding()
    }

    fn on_click(&self) {
        if let Some(cb) = &self.on_click {
            cb.call(());
        }
    }
}

impl Display for Clock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from("Clock"))
    }
}
