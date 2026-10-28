//! What one-time cells drop: the value they hold, as they drop; a value `set` refuses, by
//! whoever it goes back to; an initializing closure, as a call consumes it, or uncalled
//! when the cell is full; and what a `OnceCell` initialized from within its own
//! initialization made, before it panics. A panic in the initializing closure leaves the
//! cell empty, which a value dropping as it unwinds sees and fills.

use std::cell::OnceCell;
use std::sync::{Arc, OnceLock};

use super::Held;
use crate::log::Log;
use crate::model::Guard;

/// As it drops, notes whether `cell` is empty, then fills it.
struct CellProbe<'a>(&'a OnceCell<Guard>, Guard);

impl Drop for CellProbe<'_> {
	fn drop(&mut self) {
		self.1.note(&format!("probe {}", self.0.get().is_some()));
		let id = self.1.id() + 1;
		let log = self.1.log().clone();
		let filled = self.0.get_or_init(|| Guard::new(id, log));
		self.1.note(&format!("filled {}", filled.id()));
	}
}

/// [`CellProbe`] of a `OnceLock`, which must also be free to initialize again.
struct LockProbe<'a>(&'a OnceLock<Guard>, Guard);

impl Drop for LockProbe<'_> {
	fn drop(&mut self) {
		self.1.note(&format!("probe {}", self.0.get().is_some()));
		let id = self.1.id() + 1;
		let log = self.1.log().clone();
		let filled = self.0.get_or_init(|| Guard::new(id, log));
		self.1.note(&format!("filled {}", filled.id()));
	}
}

/// A struct holding a cell, as moq's caches do.
struct Cached {
	sweep: OnceLock<Held>,
	guard: Guard,
}

/// Run scenario `at`: what each cell drops, where Rust drops it, and in Rust's order.
pub fn run(log: &Log, id: u32, at: u8) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	match at % 8 {
		0 => {
			let cell = OnceCell::new();
			log.note(format!("set {}", cell.set(guard(1)).is_ok()));
			let refused = cell.set(guard(2));
			log.note(format!("refused {}", refused.is_err()));
			drop(refused);
			log.note(format!("held {}", cell.get().map_or(0, |g| g.id() - id)));
			drop(cell);
			let lock = OnceLock::from(Held::Two(guard(3), guard(4)));
			// A refused value no one keeps drops as the statement ends.
			_ = lock.set(Held::One(guard(5)));
			log.note(format!("kept {}", lock.get().is_some()));
		}
		1 => {
			let cell = OnceLock::new();
			let owned = guard(11);
			let made = cell.get_or_init(move || {
				owned.note("init");
				guard(12)
			});
			log.note(format!("made {}", made.id() - id));
			let unused = guard(13);
			let kept = cell.get_or_init(move || {
				unused.note("init");
				guard(14)
			});
			log.note(format!("kept {}", kept.id() - id));
			let other: OnceCell<Held> = OnceCell::default();
			let owned = guard(15);
			other.get_or_init(move || Held::Two(owned, guard(16)));
			drop(other);
		}
		2 => {
			let mut cell = OnceCell::from(guard(21));
			let taken = cell.take();
			log.note(format!("taken {}", taken.is_some()));
			drop(taken);
			log.note(format!("empty {}", cell.take().is_none()));
			_ = cell.set(guard(22));
			let lock = OnceLock::from(guard(23));
			drop(lock.into_inner());
			log.note(format!("inner {}", cell.into_inner().map_or(0, |g| g.id() - id)));
		}
		3 => {
			let cell = OnceCell::new();
			let inner = &cell;
			let owned = guard(31);
			cell.get_or_init(move || {
				_ = inner.set(guard(32));
				owned.note("init");
				guard(33)
			});
		}
		4 => {
			let cell = OnceCell::new();
			let _probe = CellProbe(&cell, guard(40));
			let owned = guard(42);
			cell.get_or_init(move || -> Guard {
				owned.note("init");
				panic!("d{id} init");
			});
		}
		5 => {
			let lock = OnceLock::new();
			let _probe = LockProbe(&lock, guard(50));
			let owned = guard(52);
			lock.get_or_init(move || -> Guard {
				owned.note("init");
				panic!("d{id} init");
			});
		}
		6 => {
			let shared = Arc::new(Cached {
				sweep: OnceLock::new(),
				guard: guard(61),
			});
			let other = shared.clone();
			shared.sweep.get_or_init(|| Held::One(guard(62)));
			drop(shared);
			log.note(format!("swept {}", other.sweep.get().is_some()));
			drop(other);
			let cells = list![OnceCell::from(guard(63)), OnceCell::new(), OnceCell::from(guard(64))];
			log.note(format!("cells {}", cells.len()));
			drop(cells);
			let maybe = Some(OnceLock::from(guard(65)));
			drop(maybe);
		}
		_ => {
			// A cell that holds `None`, and one of `()`, which JS can't tell from empty.
			let cell: OnceCell<Option<Guard>> = OnceCell::new();
			log.note(format!("none {}", cell.set(None).is_ok()));
			log.note(format!("full {}", cell.set(Some(guard(71))).is_err()));
			let unit = OnceLock::new();
			log.note(format!("unit {}", unit.set(()).is_ok()));
			log.note(format!("again {}", unit.set(()).is_err()));
			log.note(format!("got {}", cell.get().is_some()));
		}
	}
	log.note(format!("d{id} cells done"));
}
