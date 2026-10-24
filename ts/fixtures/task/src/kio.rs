//! moq's kio, trimmed to what its futures hinge on, at the moq revision `just ts moq`
//! pins (`rs/kio/src/waiter.rs`, `pending.rs`, `producer.rs`, and `consumer.rs`).
//!
//! A `WaiterList` holds each waiter's waker weakly, so a waiter that drops, as its
//! `Park` replaces it or its future is cancelled, frees its slots with no deregister.
//! The channel closes when its last producer drops. kio's atomics, tag records, and
//! inline storage are left out: they change no event the test logs.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Poll, Waker};

/// A task's waker, shared weakly with every list it registers on.
pub struct Waiter {
	waker: Waker,
	shared: Arc<Waker>,
}

impl Waiter {
	pub fn new(waker: Waker) -> Self {
		let shared = Arc::new(waker.clone());
		Self { waker, shared }
	}

	/// Register with `list`, a no-op while the list still holds this waiter.
	pub fn register(&self, list: &mut WaiterList) {
		list.register(self);
	}
}

/// Wakers waiting for one event, held weakly.
pub struct WaiterList {
	entries: Vec<Weak<Waker>>,
	/// Where the next registration looks for a dead slot to reuse.
	cursor: usize,
}

impl WaiterList {
	pub fn new() -> Self {
		Self {
			entries: Vec::new(),
			cursor: 0,
		}
	}

	pub fn register(&mut self, waiter: &Waiter) {
		let weak = Arc::downgrade(&waiter.shared);
		for entry in &self.entries {
			if Weak::ptr_eq(entry, &weak) {
				return;
			}
		}
		// Probe two slots at the cursor for one whose waiter dropped.
		let mut probes = self.entries.len().min(2);
		while probes > 0 {
			if self.entries[self.cursor].strong_count() == 0 {
				self.entries[self.cursor] = weak;
				return;
			}
			self.cursor = (self.cursor + 1) % self.entries.len();
			probes -= 1;
		}
		if self.entries.len() >= 4 {
			// Sweep every dead slot before growing.
			let mut live = Vec::new();
			for entry in std::mem::take(&mut self.entries) {
				if entry.strong_count() > 0 {
					live.push(entry);
				}
			}
			self.entries = live;
			self.cursor = 0;
		}
		self.entries.push(weak);
	}

	/// Drain every entry into a new list, leaving this one empty.
	pub fn take(&mut self) -> Self {
		self.cursor = 0;
		Self {
			entries: std::mem::take(&mut self.entries),
			cursor: 0,
		}
	}

	/// Wake every live waiter, draining the list.
	pub fn wake(&mut self) {
		self.cursor = 0;
		for entry in std::mem::take(&mut self.entries) {
			if let Some(waker) = entry.upgrade() {
				waker.wake_by_ref();
			}
		}
	}
}

/// Holds a parked waiter between polls, so its registrations outlive the poll.
#[derive(Default)]
pub struct Park(Option<Waiter>);

impl Park {
	/// The waiter for this poll: the one held if it wakes the same task, else a new one,
	/// dropping the old and so its registrations.
	pub fn hold(&mut self, cx: &Context<'_>) -> &Waiter {
		let reuse = match &self.0 {
			Some(waiter) => cx.waker().will_wake(&waiter.waker),
			None => false,
		};
		if !reuse {
			self.0 = Some(Waiter::new(cx.waker().clone()));
		}
		self.0.as_ref().unwrap()
	}
}

/// A future from a poll function that receives a [`Waiter`].
pub fn wait<F, R>(poll: F) -> Pending<F>
where
	F: FnMut(&Waiter) -> Poll<R> + Unpin,
{
	Pending {
		poll,
		park: Park::default(),
	}
}

/// What [`wait`] returns.
pub struct Pending<F> {
	poll: F,
	park: Park,
}

impl<F, R> Future for Pending<F>
where
	F: FnMut(&Waiter) -> Poll<R> + Unpin,
{
	type Output = R;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<R> {
		let this = &mut *self;
		let waiter = this.park.hold(cx);
		(this.poll)(waiter)
	}
}

struct State {
	queue: Vec<u64>,
	closed: bool,
	producers: usize,
	waiters: WaiterList,
}

/// The sending side of a channel, which closes when the last one drops.
pub struct Producer {
	state: Arc<Mutex<State>>,
}

impl Producer {
	pub fn new() -> Self {
		let state = State {
			queue: Vec::new(),
			closed: false,
			producers: 1,
			waiters: WaiterList::new(),
		};
		Self {
			state: Arc::new(Mutex::new(state)),
		}
	}

	/// Queue `value` and wake every consumer, after releasing the lock.
	pub fn send(&self, value: u64) {
		let mut waiters = {
			let mut state = self.state.lock().unwrap();
			state.queue.push(value);
			state.waiters.take()
		};
		waiters.wake();
	}

	pub fn consume(&self) -> Consumer {
		Consumer {
			state: self.state.clone(),
			next: 0,
		}
	}
}

impl Clone for Producer {
	fn clone(&self) -> Self {
		self.state.lock().unwrap().producers += 1;
		Self {
			state: self.state.clone(),
		}
	}
}

impl Drop for Producer {
	fn drop(&mut self) {
		let mut waiters = {
			let mut state = self.state.lock().unwrap();
			state.producers -= 1;
			if state.producers > 0 {
				return;
			}
			state.closed = true;
			state.waiters.take()
		};
		waiters.wake();
	}
}

/// The receiving side of a channel: every value sent, in order.
#[derive(Clone)]
pub struct Consumer {
	state: Arc<Mutex<State>>,
	next: usize,
}

impl Consumer {
	/// The next value, or `None` once the channel closed and every value was read.
	pub fn poll_recv(&mut self, waiter: &Waiter) -> Poll<Option<u64>> {
		let mut state = self.state.lock().unwrap();
		if self.next < state.queue.len() {
			let value = state.queue[self.next];
			self.next += 1;
			return Poll::Ready(Some(value));
		}
		if state.closed {
			return Poll::Ready(None);
		}
		waiter.register(&mut state.waiters);
		Poll::Pending
	}

	/// kio's one-line wrapper over a `poll_*` method.
	pub async fn recv(&mut self) -> Option<u64> {
		wait(|waiter| self.poll_recv(waiter)).await
	}
}
