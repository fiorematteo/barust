use crate::{
    utils::{HookSender, Rectangle, StatusBarInfo, TimedHooks},
    widgets::{Size, Text, Widget, WidgetConfig, WidgetError},
};
use cairo::Context;
use log::error;
use std::{
    fmt,
    ops::{Deref, DerefMut},
};

#[derive(Debug)]
pub struct ReplaceableWidget(Box<dyn Widget>);

impl Deref for ReplaceableWidget {
    type Target = dyn Widget;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl DerefMut for ReplaceableWidget {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}

impl fmt::Display for ReplaceableWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl ReplaceableWidget {
    pub fn new(wd: Box<dyn Widget>) -> Self {
        Self(wd)
    }

    pub async fn draw_or_replace(&mut self, context: Context, rectangle: &Rectangle) {
        if let Err(e) = self.0.draw(context, rectangle) {
            self.replace(e).await;
            // we need to recompute the size before we draw again
        }
    }

    pub async fn size_or_replace(&mut self, context: &Context) -> Size {
        match self.0.size(context) {
            Ok(s) => s,
            Err(e) => {
                self.replace(e).await;
                self.0.size(context).unwrap()
            }
        }
    }

    pub async fn setup_or_replace(&mut self, info: &StatusBarInfo) {
        match self.0.setup(info).await {
            Ok(s) => s,
            Err(e) => {
                self.replace(e).await;
                self.0.setup(info).await.unwrap();
            }
        }
    }
    pub async fn update_or_replace(&mut self) {
        if let Err(e) = self.0.update().await {
            self.replace(e).await;
            self.0.update().await.unwrap();
        }
    }

    pub async fn hook_or_replace(&mut self, sender: HookSender, pool: &mut TimedHooks) {
        if let Err(e) = self.0.hook(sender.clone(), pool).await {
            self.replace(e).await;
            self.0.hook(sender, pool).await.unwrap();
        }
    }

    async fn replace(&mut self, e: WidgetError) {
        error!("{e}");
        error!("Replacing `{}` with default", self.0);
        self.0 = Text::new("Widget Crashed 🙃", &WidgetConfig::default()).await;
    }
}
