use async_channel::{SendError, Sender};

pub type WidgetIndex = usize;

#[derive(Debug, Clone)]
pub struct HookSender {
    sender: Sender<WidgetIndex>,
    id: WidgetIndex,
}

impl HookSender {
    pub fn new(sender: Sender<WidgetIndex>, id: WidgetIndex) -> Self {
        Self { sender, id }
    }

    pub async fn send(&self) -> Result<(), SendError<WidgetIndex>> {
        self.sender.send(self.id).await
    }

    pub fn send_blocking(&self) -> Result<(), SendError<WidgetIndex>> {
        self.sender.send_blocking(self.id)
    }
}
