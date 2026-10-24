//! A deterministic FIFO executor that logs every poll, wake, and waker clone and drop:
//! the native side of the executor `tests/task.ts` runs the translation on, which logs
//! the same events. Not translated: it is the boundary, built on `dyn Future` and a
//! hand-made waker.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use crate::log::Log;

/// What every waker of one task points at.
struct Wake {
	task: usize,
	log: Log,
	queue: Arc<Mutex<VecDeque<usize>>>,
	/// Whether the task is in the queue, so a second wake before its poll adds nothing.
	queued: Mutex<bool>,
}

impl Wake {
	fn schedule(&self) {
		let mut queued = self.queued.lock().unwrap();
		if !*queued {
			*queued = true;
			self.queue.lock().unwrap().push_back(self.task);
		}
	}
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

/// # Safety
/// `data` is an `Arc<Wake>` that `waker` made.
unsafe fn clone(data: *const ()) -> RawWaker {
	// SAFETY: the waker being cloned holds a count.
	unsafe { Arc::increment_strong_count(data.cast::<Wake>()) };
	// SAFETY: as above, it is live.
	let wake = unsafe { &*data.cast::<Wake>() };
	wake.log.note(format!("t{} waker clone", wake.task));
	RawWaker::new(data, &VTABLE)
}

unsafe fn wake(data: *const ()) {
	// SAFETY: the waker being consumed holds a count, which this takes.
	let wake = unsafe { Arc::from_raw(data.cast::<Wake>()) };
	wake.log.note(format!("t{} wake", wake.task));
	wake.schedule();
}

unsafe fn wake_by_ref(data: *const ()) {
	// SAFETY: the waker holds a count.
	let wake = unsafe { &*data.cast::<Wake>() };
	wake.log.note(format!("t{} wake", wake.task));
	wake.schedule();
}

unsafe fn drop(data: *const ()) {
	// SAFETY: the waker being dropped holds a count, which this releases.
	let wake = unsafe { Arc::from_raw(data.cast::<Wake>()) };
	wake.log.note(format!("t{} waker drop", wake.task));
}

fn waker(wake: Arc<Wake>) -> Waker {
	let data = Arc::into_raw(wake).cast::<()>();
	// SAFETY: the vtable's functions treat `data` as the `Arc<Wake>` it is.
	unsafe { Waker::from_raw(RawWaker::new(data, &VTABLE)) }
}

struct Task {
	future: Pin<Box<dyn Future<Output = u64>>>,
	waker: Waker,
	wake: Arc<Wake>,
	done: bool,
}

/// Runs tasks in the order they were woken.
pub struct Executor {
	log: Log,
	queue: Arc<Mutex<VecDeque<usize>>>,
	/// Every task spawned, by id; `None` once dropped.
	tasks: Vec<Option<Task>>,
}

impl Executor {
	pub fn new(log: Log) -> Self {
		Self {
			log,
			queue: Arc::new(Mutex::new(VecDeque::new())),
			tasks: Vec::new(),
		}
	}

	/// How many tasks were spawned, dropped ones included.
	pub fn len(&self) -> usize {
		self.tasks.len()
	}

	/// Spawn a task and queue its first poll.
	pub fn spawn(&mut self, future: impl Future<Output = u64> + 'static) {
		let task = self.tasks.len();
		self.log.note(format!("t{task} spawn"));
		let wake = Arc::new(Wake {
			task,
			log: self.log.clone(),
			queue: self.queue.clone(),
			queued: Mutex::new(false),
		});
		wake.schedule();
		self.tasks.push(Some(Task {
			future: Box::pin(future),
			waker: waker(wake.clone()),
			wake,
			done: false,
		}));
	}

	/// Poll queued tasks until none is left.
	pub fn run(&mut self) {
		while self.step() {}
	}

	/// Poll the next queued task, if any. A dropped or finished one is skipped.
	pub fn step(&mut self) -> bool {
		let Some(task) = self.queue.lock().unwrap().pop_front() else {
			return false;
		};
		if self.tasks[task].as_ref().is_some_and(|t| !t.done) {
			self.poll(task);
		}
		true
	}

	/// Poll a task whether or not it is queued or finished: polling a finished one panics.
	pub fn poll(&mut self, task: usize) {
		let Some(t) = self.tasks[task].as_mut() else {
			return;
		};
		*t.wake.queued.lock().unwrap() = false;
		self.log.note(format!("t{task} poll"));
		let mut cx = Context::from_waker(&t.waker);
		match t.future.as_mut().poll(&mut cx) {
			Poll::Ready(value) => {
				t.done = true;
				self.log.note(format!("t{task} ready {value}"));
			}
			Poll::Pending => self.log.note(format!("t{task} pending")),
		}
	}

	/// Drop a task's future, then its waker.
	pub fn drop(&mut self, task: usize) {
		let Some(t) = self.tasks[task].take() else {
			return;
		};
		self.log.note(format!("t{task} drop"));
		std::mem::drop(t.future);
		std::mem::drop(t.waker);
	}
}
