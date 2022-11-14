use crate::statusbar::RightLeft;
use crossbeam_channel::{SendError, Sender};

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

    pub fn send(&self) -> Result<(), SendError<WidgetID>> {
        self.sender.send(self.id)
    }
}
