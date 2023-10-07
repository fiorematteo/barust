use super::hook_sender::HookSender;
use log::{debug, error};
use std::time::Duration;
use tokio::{task::spawn, time::sleep};

#[derive(Debug, Default)]
pub struct TimedHooks {
    senders: Vec<HookSender>,
}

impl TimedHooks {
    pub fn subscribe(&mut self, sender: HookSender) {
        self.senders.push(sender);
    }

    pub async fn start(self) {
        spawn(async move {
            loop {
                for s in &self.senders {
                    if s.send().await.is_err() {
                        error!("breaking thread loop")
                    }
                }
                sleep(Duration::from_secs(1)).await;
                debug!("waking from sleep");
            }
        });
    }
}
