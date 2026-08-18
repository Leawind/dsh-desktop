use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::webui::Window;

#[derive(Clone, Default)]
pub struct WindowRegistry(Arc<Mutex<HashMap<String, Window>>>);

#[derive(Clone, Copy)]
pub struct WindowActivity {
    pub last_seen: u64,
    pub connected: bool,
}

#[derive(Clone, Default)]
pub struct WindowActivityRegistry(Arc<Mutex<HashMap<String, WindowActivity>>>);

impl WindowActivityRegistry {
    pub fn insert(&self, label: String, now: u64) {
        self.0
            .lock()
            .expect("window activity registry poisoned")
            .insert(
                label,
                WindowActivity {
                    last_seen: now,
                    connected: false,
                },
            );
    }

    pub fn record_seen(&self, label: &str, now: u64) {
        if let Some(activity) = self
            .0
            .lock()
            .expect("window activity registry poisoned")
            .get_mut(label)
        {
            activity.last_seen = now;
            activity.connected = true;
        }
    }

    pub fn remove(&self, label: &str) {
        self.0
            .lock()
            .expect("window activity registry poisoned")
            .remove(label);
    }

    pub fn stale_labels(
        &self,
        now: u64,
        disconnect_timeout_millis: u64,
        initial_connection_timeout_millis: u64,
    ) -> Vec<String> {
        self.0
            .lock()
            .expect("window activity registry poisoned")
            .iter()
            .filter_map(|(label, activity)| {
                let timeout_millis = if activity.connected {
                    disconnect_timeout_millis
                } else {
                    initial_connection_timeout_millis
                };
                (now.saturating_sub(activity.last_seen) >= timeout_millis).then_some(label.clone())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_keeps_a_window_registered() {
        let activity = WindowActivityRegistry::default();
        activity.insert("window".to_owned(), 1);
        activity.record_seen("window", 10);
        assert!(activity.stale_labels(12, 3, 30).is_empty());
        assert_eq!(activity.stale_labels(13, 3, 30), vec!["window"]);
    }

    #[test]
    fn heartbeat_expires_at_the_configured_timeout() {
        let activity = WindowActivityRegistry::default();
        activity.insert("window".to_owned(), 0);

        activity.record_seen("window", 0);

        assert!(activity.stale_labels(999, 1_000, 10_000).is_empty());
        assert_eq!(activity.stale_labels(1_000, 1_000, 10_000), vec!["window"]);
    }

    #[test]
    fn new_window_has_a_connection_grace_period() {
        let activity = WindowActivityRegistry::default();
        activity.insert("window".to_owned(), 0);

        assert!(activity.stale_labels(9_999, 1_000, 10_000).is_empty());
        assert_eq!(activity.stale_labels(10_000, 1_000, 10_000), vec!["window"]);
    }
}

impl WindowRegistry {
    pub fn insert(&self, label: String, window: Window) {
        self.0
            .lock()
            .expect("window registry poisoned")
            .insert(label, window);
    }

    pub fn get(&self, label: &str) -> Option<Window> {
        self.0
            .lock()
            .expect("window registry poisoned")
            .get(label)
            .copied()
    }

    pub fn remove(&self, label: &str) -> Option<Window> {
        self.0
            .lock()
            .expect("window registry poisoned")
            .remove(label)
    }
}
