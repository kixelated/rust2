//! The trace both sides of the differential test write, one event per line.

use std::sync::{Arc, Mutex};

/// A shared, append-only list of events.
#[derive(Clone, Default)]
pub struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
	pub fn new() -> Self {
		Self(Arc::new(Mutex::new(Vec::new())))
	}

	/// Append one event.
	pub fn note(&self, event: String) {
		self.0.lock().unwrap().push(event);
	}

	/// Every event so far, one per line.
	pub fn text(&self) -> Vec<u8> {
		let events = self.0.lock().unwrap();
		let mut out = Vec::new();
		for event in events.iter() {
			out.extend_from_slice(event.as_bytes());
			out.push(b'\n');
		}
		out
	}
}
