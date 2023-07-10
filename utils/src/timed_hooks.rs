use super::hook_sender::HookSender;
use crossbeam_channel::{unbounded, Receiver, SendError, Sender};
use log::{debug, error};
use std::time::Duration;
use tokio::{task::spawn, time::sleep};

#[derive(Debug)]
pub struct TimedHooks {
    thread: Sender<HookSender>,
}

impl Default for TimedHooks {
    fn default() -> Self {
        let (thread, rx) = unbounded::<HookSender>();
        let senders: Vec<HookSender> = Vec::new();
        spawn(looping(senders, rx));
        Self { thread }
    }
}

async fn looping(mut senders: Vec<HookSender>, rx: Receiver<HookSender>) {
    loop {
        while let Ok(id) = rx.try_recv() {
            senders.push(id);
        }
        for s in &senders {
            if s.send().is_err() {
                error!("breaking thread loop")
            }
        }
        sleep(Duration::from_secs(1)).await;
        debug!("waking from sleep");
    }
}

impl TimedHooks {
    pub fn subscribe(&mut self, sender: HookSender) -> Result<(), SendError<HookSender>> {
        self.thread.send(sender)?;
        Ok(())
    }
}
