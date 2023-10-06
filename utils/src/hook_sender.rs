use async_channel::{SendError, Sender};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RightLeft {
    Right,
    Left,
}

pub type WidgetID = (RightLeft, usize);

#[derive(Debug)]
pub struct HookSender {
    sender: Sender<WidgetID>,
    id: WidgetID,
}

impl HookSender {
    pub fn new(sender: Sender<WidgetID>, id: WidgetID) -> Self {
        Self { sender, id }
    }

    pub async fn send(&self) -> Result<(), SendError<WidgetID>> {
        self.sender.send(self.id).await
    }

    pub fn send_blocking(&self) -> Result<(), SendError<WidgetID>> {
        self.sender.send_blocking(self.id)
    }
}
