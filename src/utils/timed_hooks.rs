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
        let duration = Duration::from_secs(1) / self.senders.len() as u32;
        spawn(async move {
            for s in self.senders.into_iter().cycle() {
                if s.send().await.is_err() {
                    error!("breaking thread loop")
                }

                sleep(duration).await;
                debug!("waking from sleep");
            }
        });
    }
}
