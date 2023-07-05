use super::hook_sender::HookSender;
use crossbeam_channel::{bounded, SendError, Sender};
use log::{debug, error};
use std::{thread, time::Duration};

#[derive(Debug)]
pub struct TimedHooks {
    thread: Sender<HookSender>,
}

impl Default for TimedHooks {
    fn default() -> Self {
        let (thread, rx) = bounded::<HookSender>(10);
        let mut senders: Vec<HookSender> = Vec::new();
        thread::spawn(move || loop {
            while let Ok(id) = rx.try_recv() {
                senders.push(id);
            }
            for s in &senders {
                if s.send().is_err() {
                    error!("breaking thread loop")
                }
            }
            thread::sleep(Duration::from_secs(1));
            debug!("waking from sleep");
        });
        Self { thread }
    }
}

impl TimedHooks {
    pub fn subscribe(&mut self, sender: HookSender) -> Result<(), SendError<HookSender>> {
        self.thread.send(sender)?;
        Ok(())
    }
}
