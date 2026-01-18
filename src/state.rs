use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

#[derive(Debug)]
pub struct AppState {
    pub total_lines: AtomicU64,
    pub total_errors: AtomicU64,
    pub start_time: Instant,
    pub last_error: Mutex<Option<String>>,
    // New: For Sparklines
    // REMOVED: error_history and last_history_update.
    // The UI thread will track history by sampling total_errors.
    // This removes the Mutex from the hot path.
    pub webhook_url: Option<String>,
    pub last_webhook_sent: Mutex<Option<Instant>>,
}

impl AppState {
    pub fn new(webhook_url: Option<String>) -> Self {
        Self {
            total_lines: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            start_time: Instant::now(),
            last_error: Mutex::new(None),
            webhook_url,
            last_webhook_sent: Mutex::new(None),
        }
    }

    pub fn should_send_webhook(&self) -> bool {
        let mut last_sent = self.last_webhook_sent.lock().unwrap();
        match *last_sent {
            Some(instant) => {
                if instant.elapsed() > std::time::Duration::from_secs(10) {
                    *last_sent = Some(Instant::now());
                    true
                } else {
                    false
                }
            }
            None => {
                *last_sent = Some(Instant::now());
                true
            }
        }
    }

    pub fn increment_lines(&self) {
        self.total_lines.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self, message: String) {
        self.total_errors.fetch_add(1, Ordering::Relaxed);
        let mut last = self.last_error.lock().unwrap();
        *last = Some(message);
    }
}
