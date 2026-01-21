use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{sleep, Instant};

use crate::state::GlobalState;

pub struct CacheManager {
    state: GlobalState,
    receiver: UnboundedReceiver<()>,
    debounce_duration: Duration,
}

impl CacheManager {
    pub fn new(state: GlobalState, receiver: UnboundedReceiver<()>) -> Self {
        Self {
            state,
            receiver,
            debounce_duration: Duration::from_secs(5),
        }
    }

    pub async fn start(mut self) {
        eprintln!("🚀 CacheManager started");

        let mut last_signal = None;

        loop {
            tokio::select! {
                Some(_) = self.receiver.recv() => {
                    last_signal = Some(Instant::now());
                }
                _ = sleep(Duration::from_secs(1)), if last_signal.is_some() => {
                    if let Some(instant) = last_signal {
                        if instant.elapsed() >= self.debounce_duration {
                            self.perform_save().await;
                            last_signal = None;
                        }
                    }
                }
            }
        }
    }

    async fn perform_save(&self) {
        let vault_opt = self.state.vault.read().await;
        if let Some(vault) = vault_opt.as_ref() {
            let root = vault.workspace.root().to_path_buf();
            let cache_path = root.join(".dendrite").join("cache.bin");

            match vault.save_cache(&cache_path) {
                Ok(_) => {
                    // We don't want to spam the log, but maybe a debug trace
                    // eprintln!("💾 Cache saved to {:?}", cache_path);
                }
                Err(e) => {
                    eprintln!("❌ Failed to save cache: {}", e);
                }
            }
        }
    }
}
