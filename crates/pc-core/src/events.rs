//! Event sink abstraction so the engine can push progress to the UI without depending on Tauri.

use std::sync::{Arc, Mutex};

use serde::Serialize;

pub trait EventSink: Send + Sync {
    fn emit_json(&self, event: &str, payload: serde_json::Value);
}

pub type Sink = Arc<dyn EventSink>;

pub fn emit<T: Serialize>(sink: &Sink, event: &str, payload: &T) {
    match serde_json::to_value(payload) {
        Ok(v) => sink.emit_json(event, v),
        Err(e) => tracing::warn!("failed to serialise event {event}: {e}"),
    }
}

/// Discards events (CLI tools, tests that don't care).
pub struct NullSink;
impl EventSink for NullSink {
    fn emit_json(&self, _: &str, _: serde_json::Value) {}
}

pub fn null_sink() -> Sink {
    Arc::new(NullSink)
}

/// Records events for assertions in tests.
#[derive(Default)]
pub struct RecordingSink {
    pub events: Mutex<Vec<(String, serde_json::Value)>>,
}
impl EventSink for RecordingSink {
    fn emit_json(&self, event: &str, payload: serde_json::Value) {
        self.events.lock().unwrap().push((event.to_string(), payload));
    }
}
impl RecordingSink {
    pub fn names(&self) -> Vec<String> {
        self.events.lock().unwrap().iter().map(|(n, _)| n.clone()).collect()
    }
}

/// Small notification payload for toasts.
#[derive(Debug, Clone, Serialize)]
pub struct Notice {
    pub level: &'static str,
    pub title: String,
    pub message: String,
}

pub fn notify(sink: &Sink, level: &'static str, title: impl Into<String>, message: impl Into<String>) {
    emit(sink, "notice", &Notice { level, title: title.into(), message: message.into() });
}
