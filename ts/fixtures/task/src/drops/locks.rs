//! What a mutex's guard does as it drops: unlocks, or, as a panic unwinds, poisons the
//! mutex, unless that panic had started when it locked. Each guard is held a different
//! way: in a local, in a struct of the crate's, in a lock's `Result`, and by a closure
//! std drops as the panic leaves it. A probe declared before the guard drops after it,
//! as the panic unwinds, and logs what it finds: whether a lock is poisoned, and the value
//! a poisoned lock still hands out. A scenario panics where `at / 8` is odd.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::log::Log;
use crate::model::Guard;

/// As it drops, logs whether its mutex is poisoned, then locks it whatever the answer.
struct Probe<'a>(&'a Mutex<Guard>, Guard);

impl Drop for Probe<'_> {
	fn drop(&mut self) {
		let poisoned = self.0.lock().is_err();
		let held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
		self.1.note(&format!("poisoned {poisoned}, holds g{}", held.id()));
	}
}

/// A guard in a struct of the crate's, as kio's `LockGuard` holds one.
struct Locked<'a> {
	inner: MutexGuard<'a, Guard>,
	tag: Guard,
}

impl Drop for Locked<'_> {
	fn drop(&mut self) {
		self.tag.note(&format!("unlocks g{}", self.inner.id()));
	}
}

/// As it drops, locks a mutex of its own, which a guard taken as a panic unwinds does
/// not poison, and logs it.
struct Fresh(Guard);

impl Drop for Fresh {
	fn drop(&mut self) {
		let mutex = Mutex::new(self.0.id());
		drop(mutex.lock());
		self.0.note(&format!("fresh poisoned {}", mutex.lock().is_err()));
	}
}

/// Run scenario `at`.
pub fn run(log: &Log, id: u32, at: u8) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let fail = at / 8 % 2 == 1;
	match at % 8 {
		// In a local, after a lock that unlocked as its guard dropped.
		0 => {
			let mutex = Mutex::new(guard(1));
			drop(mutex.lock().unwrap());
			let _probe = Probe(&mutex, guard(2));
			let held = mutex.lock().unwrap();
			held.note("held");
			assert!(!fail, "g{} held", held.id());
		}
		// In a struct of the crate's, whose glue drops it after its `Drop` impl.
		1 => {
			let mutex = Mutex::new(guard(1));
			let _probe = Probe(&mutex, guard(2));
			let locked = Locked {
				inner: mutex.lock().unwrap_or_else(PoisonError::into_inner),
				tag: guard(3),
			};
			assert!(!fail, "g{} locked", locked.inner.id());
		}
		// In the lock's `Result`, never unwrapped.
		2 => {
			let mutex = Mutex::new(guard(1));
			let _probe = Probe(&mutex, guard(2));
			let locked = mutex.lock();
			assert!(!fail, "locked {}", locked.is_ok());
		}
		// Moved into a closure, which `Option::map` drops as its call panics.
		3 => {
			let mutex = Mutex::new(guard(1));
			let _probe = Probe(&mutex, guard(2));
			let held = mutex.lock().unwrap();
			let seen = Some(fail).map(move |fail| {
				assert!(!fail, "g{} moved", held.id());
				held.id()
			});
			log.note(format!("seen {}", seen.map_or(0, |n| n - id)));
		}
		// Moved into a closure, which std's `for_each` drops as its call panics.
		4 => {
			let mutex = Mutex::new(guard(1));
			let _probe = Probe(&mutex, guard(2));
			let held = mutex.lock().unwrap();
			[fail, false].into_iter().for_each(move |fail| {
				assert!(!fail, "g{} iterated", held.id());
			});
		}
		// Taken as the panic unwinds, by a value dropping after the guard: no poison.
		5 => {
			let _fresh = Fresh(guard(1));
			let mutex = Mutex::new(guard(2));
			let _probe = Probe(&mutex, guard(3));
			let held = mutex.lock().unwrap();
			assert!(!fail, "g{} held", held.id());
		}
		// Owned by a value that reads it by `get_mut`, without locking, as it drops.
		6 => {
			let owned = Owned(Mutex::new(guard(1)), guard(2));
			let held = owned.0.lock().unwrap();
			assert!(!fail, "g{} owned", held.id());
			drop(held);
			owned.1.note("owned");
		}
		// Shared by an `Arc`, which drops the value with the last handle, after a probe
		// that shows the `PoisonError` a lock gives.
		_ => {
			let mutex = Arc::new(Mutex::new(guard(1)));
			let _shown = Shown(mutex.clone(), guard(2));
			let held = mutex.lock().unwrap();
			assert!(!fail, "g{} shown", held.id());
		}
	}
}

/// A mutex of its own, which `get_mut` reads as it drops.
struct Owned(Mutex<Guard>, Guard);

impl Drop for Owned {
	fn drop(&mut self) {
		let poisoned = self.0.get_mut().is_err();
		let held = self.0.get_mut().unwrap_or_else(PoisonError::into_inner);
		self.1
			.note(&format!("get_mut poisoned {poisoned}, holds g{}", held.id()));
	}
}

/// As it drops, logs what a lock gives: its guard, or the `PoisonError` holding it.
struct Shown(Arc<Mutex<Guard>>, Guard);

impl Drop for Shown {
	fn drop(&mut self) {
		match self.0.lock() {
			Ok(held) => self.1.note(&format!("ok g{}", held.id())),
			Err(err) => self.1.note(&format!("{err} {err:?} g{}", err.get_ref().id())),
		}
	}
}
