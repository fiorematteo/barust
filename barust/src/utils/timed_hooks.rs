use super::hook_sender::HookSender;
use crossbeam_channel::{bounded, Sender, SendError};
use log::{debug, error};
use std::{
    collections::HashMap,
    thread,
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct TimedHooks {
    thread: Sender<(Duration, HookSender)>,
}

impl Default for TimedHooks {
    fn default() -> Self {
        let (thread, rx) = bounded::<(Duration, HookSender)>(10);
        //let mut senders: Vec<(Instant, Duration, HookSender)> = vec![];
        let mut senders: HashMap<Duration, (Instant, Vec<HookSender>)> = HashMap::new();
        thread::spawn(move || loop {
            while let Ok(id) = rx.try_recv() {
                if let Some((_, a)) = senders.get_mut(&id.0) {
                    a.push(id.1);
                } else {
                    senders.insert(id.0, (Instant::now(), vec![id.1]));
                }
            }
            for (duration, (time, senders)) in &mut senders {
                if time.elapsed() > *duration {
                    *time = Instant::now();
                    for s in senders {
                        if s.send().is_err() {
                            error!("breaking thread loop")
                        }
                    }
                }
            }

            let smallest_time = senders
                .iter()
                .map(|(d, (t, _))| (d.saturating_sub(t.elapsed())))
                .min()
                .unwrap_or_else(|| Duration::from_secs(1));
            thread::sleep(smallest_time);
            debug!("waking from sleep");
        });
        Self { thread }
    }
}

impl TimedHooks {
    pub fn subscribe(
        &mut self,
        duration: Duration,
        sender: HookSender,
    ) -> Result<(), SendError<(Duration, HookSender)>> {
        self.thread.send((duration, sender))?;
        Ok(())
    }
}
