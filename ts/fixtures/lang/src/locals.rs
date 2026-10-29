//! Thread-locals on one thread, each one value every use shares: of `Cell`s, `RefCell`s,
//! and a struct of both, as kio's `Polling` is, initialized by a `const` block or lazily
//! by an expression; read and written through `with`, given a closure or a function,
//! `try_with`, and the `Cell` and `RefCell` helpers, as kio's `coop` does; nested; and
//! declared in a function, as kio's `serial` is. Each case leaves
//! every thread-local as it found it, so a fresh instance and a reused one agree.

use std::cell::{Cell, RefCell};
use std::sync::{Arc, Mutex};

/// Logs its id when it drops.
struct Noisy {
	id: u8,
	log: Arc<Mutex<Vec<u8>>>,
}

impl Drop for Noisy {
	fn drop(&mut self) {
		self.log.lock().unwrap().push(self.id);
	}
}

/// What a batch holds, as kio's `Polling` does.
struct Polling {
	current: Cell<Option<(u64, u64)>>,
	held: RefCell<Vec<u8>>,
}

thread_local! {
	static COUNT: Cell<u32> = const { Cell::new(7) };
	static LOG: RefCell<Vec<u8>> = RefCell::new(vec![1, 2, 3]);
	static POLLING: Polling = const {
		Polling {
			current: Cell::new(None),
			held: RefCell::new(Vec::new()),
		}
	};
	static HELD: RefCell<Option<Noisy>> = const { RefCell::new(None) };
}

/// Each byte an operation on the thread-locals; each puts what it read and what dropped.
pub fn locals(data: &[u8], out: &mut Vec<u8>) {
	let log = Arc::new(Mutex::new(Vec::new()));
	for &op in data {
		match op % 9 {
			0 => {
				COUNT.with(|c| c.set(c.get().wrapping_add(u32::from(op))));
				out.push(COUNT.with(Cell::get) as u8);
			}
			1 => LOG.with(|log| log.borrow_mut().push(op)),
			2 => out.push(LOG.try_with(|log| log.borrow().len()).map_or(0xfd, |n| n as u8)),
			// One read within another.
			3 => out.push(COUNT.with(|c| LOG.with(|log| c.get() as u8 ^ log.borrow()[0]))),
			4 => {
				let outer = POLLING.with(|p| p.current.replace(Some((u64::from(op), 1))));
				let len = POLLING.with(|p| {
					p.held.borrow_mut().push(op);
					p.held.borrow().len()
				});
				out.push(outer.map_or(0xfd, |(a, _)| a as u8));
				out.push(len as u8);
			}
			// Replaces what it held, which drops.
			5 => HELD.with(|held| {
				*held.borrow_mut() = Some(Noisy {
					id: op,
					log: log.clone(),
				})
			}),
			6 => {
				COUNT.set(COUNT.get().wrapping_mul(3));
				out.push(COUNT.replace(u32::from(op)) as u8);
				out.push(COUNT.take() as u8);
				COUNT.set(u32::from(op) + 1);
			}
			// The old value each helper replaces drops, or is put.
			7 => {
				out.push(LOG.with_borrow(|log| log.len() as u8));
				out.extend(LOG.replace(vec![op]));
				out.extend(LOG.take());
				LOG.with(|log| log.borrow_mut().extend([1, 2, 3]));
				let old = HELD.replace(Some(Noisy {
					id: op,
					log: log.clone(),
				}));
				out.push(old.as_ref().map_or(0xfd, |n| n.id));
				drop(old);
				out.push(HELD.take().map_or(0xfd, |n| n.id));
			}
			_ => out.push(serial(false)),
		}
		out.extend(log.lock().unwrap().iter());
		log.lock().unwrap().clear();
	}
	COUNT.with(|c| c.set(7));
	LOG.with(|log| log.borrow_mut().truncate(3));
	POLLING.with(|p| {
		p.current.set(None);
		p.held.borrow_mut().clear();
	});
	HELD.with(|held| held.borrow_mut().take());
	serial(true);
	out.extend(log.lock().unwrap().iter());
}

/// A serial from a thread-local in a function, as kio's `serial` is; starts over if `reset`.
fn serial(reset: bool) -> u8 {
	thread_local! {
		static NEXT: Cell<(u8, u8)> = const { Cell::new((0, 4)) };
	}
	NEXT.with(|next| {
		let (n, end) = match reset {
			true => (0, 4),
			false => next.get(),
		};
		next.set((n.wrapping_add(u8::from(!reset)), end));
		n
	})
}
