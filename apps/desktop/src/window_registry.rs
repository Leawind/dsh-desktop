use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::webui::Window;

#[derive(Clone, Default)]
pub struct WindowRegistry(Arc<Mutex<HashMap<String, Window>>>);

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

    pub fn is_empty(&self) -> bool {
        self.0.lock().expect("window registry poisoned").is_empty()
    }
}
