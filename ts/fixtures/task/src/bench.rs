//! The handoff `just ts bench` times on every route: a value sent to a task parked on the
//! channel, which wakes it, polls it through `depth` async functions, and parks it again.
//! Not translated: `bench/task.ts` runs the same loop on the translation with its own
//! executor.
//!
//! The input is the depth, then the count of values as a big-endian `u32`; the output is
//! the task's sum, as a big-endian `u64`.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

use crate::model::{self, Channel};

/// Run one handoff workload.
pub fn run(input: &[u8]) -> Vec<u8> {
	let depth = u32::from(input[0]);
	let count = u32::from_be_bytes([input[1], input[2], input[3], input[4]]);
	let channel = Channel::open();
	let mut task = Task::new(model::drain(channel.consumer(), depth));
	task.run();
	for value in 0..count {
		channel.send(u64::from(value));
		task.run();
	}
	drop(channel);
	let total = task.run().expect("the task finishes once its channel closes");
	total.to_be_bytes().to_vec()
}

/// One task, polled whenever its waker was woken: an executor that only ever runs one.
struct Task<F> {
	future: Pin<Box<F>>,
	woken: Arc<Woken>,
	waker: Waker,
}

struct Woken(Mutex<bool>);

impl Wake for Woken {
	fn wake(self: Arc<Self>) {
		self.wake_by_ref();
	}

	fn wake_by_ref(self: &Arc<Self>) {
		*self.0.lock().unwrap() = true;
	}
}

impl<F: Future> Task<F> {
	fn new(future: F) -> Self {
		let woken = Arc::new(Woken(Mutex::new(true)));
		Self {
			future: Box::pin(future),
			waker: Waker::from(woken.clone()),
			woken,
		}
	}

	/// Poll while woken, returning the output once it is ready.
	fn run(&mut self) -> Option<F::Output> {
		while std::mem::take(&mut *self.woken.0.lock().unwrap()) {
			let mut cx = Context::from_waker(&self.waker);
			if let Poll::Ready(output) = self.future.as_mut().poll(&mut cx) {
				return Some(output);
			}
		}
		None
	}
}
