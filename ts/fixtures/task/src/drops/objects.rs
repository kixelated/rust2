//! What a `Box<dyn Trait>` drops: the value it was made of, by its own type's drop, a
//! struct's, an enum's glue, or nothing for a number, where the box goes out of scope,
//! is replaced, or is consumed by `self: Box<Self>`, a panic included. An `Arc<dyn Trait>`
//! drops it with the last `Arc` of it, of the trait object or of the type it was made of.

use std::future::Future;
use std::sync::Arc;

use super::{Bomb, Held};
use crate::log::Log;
use crate::model::Guard;

trait Part {
	fn id(&self) -> u32;
	/// Consumes the box, dropping it on return.
	fn take(self: Box<Self>) -> u32 {
		self.id().wrapping_add(1)
	}
}

impl Part for Guard {
	fn id(&self) -> u32 {
		Guard::id(self)
	}
}

impl Part for Held {
	fn id(&self) -> u32 {
		match self {
			Held::One(g) => g.id(),
			Held::Two(a, b) => a.id() + b.id(),
			Held::Empty => 0,
		}
	}
}

impl Part for Bomb {
	fn id(&self) -> u32 {
		self.id
	}
	fn take(self: Box<Self>) -> u32 {
		let id = self.id;
		drop(self);
		id.wrapping_mul(2)
	}
}

impl Part for u32 {
	fn id(&self) -> u32 {
		*self
	}
}

/// Boxed parts of each kind, panicking at point `at`, if any, or in a `Drop` at 5.
pub(super) fn boxed(log: &Log, id: u32, at: u8) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let check = |point: u8| {
		if point == at {
			panic!("d{id} at {point}");
		}
	};
	let first: Box<dyn Part> = Box::new(guard(1));
	let mut parts: Vec<Box<dyn Part>> = Vec::new();
	parts.push(Box::new(Held::Two(guard(2), guard(3))));
	parts.push(Box::new(7u32));
	parts.push(Box::new(Bomb {
		id: id + 4,
		log: log.clone(),
		blows: at == 5,
	}));
	parts.push(Box::new(Held::One(guard(5))));
	parts.swap(2, 3);
	check(1);
	log.note(format!(
		"ids {}",
		parts.iter().map(|p| p.id()).fold(0u32, u32::wrapping_add)
	));
	// The old value drops as the new one takes its place.
	let mut slot: Box<dyn Part> = Box::new(guard(6));
	log.note(format!("slot {}", slot.id()));
	slot = Box::new(Held::Empty);
	log.note(format!("slot {}", slot.id()));
	slot = Box::new(7u32);
	check(2);
	log.note(format!("took {}", first.take()));
	let last = parts.pop().map(Part::take);
	log.note(format!("last {last:?}"));
	let kept: Option<Box<dyn Part>> = Some(Box::new(guard(7)));
	check(3);
	drop(parts);
	check(4);
	log.note(format!("kept {} {}", kept.as_ref().map_or(0, |p| p.id()), slot.id()));
}

/// Shared parts of each kind, panicking at point `at`, if any, or in a `Drop` at 5, which
/// the last `Arc` of it runs, a trait object's, while others still drop.
pub(super) fn shared(log: &Log, id: u32, at: u8) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let check = |point: u8| {
		if point == at {
			panic!("d{id} at {point}");
		}
	};
	let held = Arc::new(Held::Two(guard(1), guard(2)));
	let first: Arc<dyn Part> = held.clone();
	let second: Arc<dyn Part> = held.clone();
	log.note(format!("held {} {}", first.id(), Arc::strong_count(&held)));
	drop(held);
	check(1);
	drop(first);
	log.note(format!("count {}", Arc::strong_count(&second)));
	let kept = Arc::new(guard(3));
	let bomb = Arc::new(Bomb {
		id: id + 4,
		log: log.clone(),
		blows: at == 5,
	});
	let mut parts: Vec<Arc<dyn Part>> = Vec::new();
	parts.push(bomb.clone());
	parts.push(kept.clone());
	parts.push(second.clone());
	parts.push(Arc::new(7u32));
	let weak = Arc::downgrade(&parts[1]);
	check(2);
	drop(second);
	log.note(format!(
		"ids {}",
		parts.iter().map(|p| p.id()).fold(0u32, u32::wrapping_add)
	));
	// The bomb's last `Arc` is the trait object the `Vec` holds; the guard's is its own.
	drop(bomb);
	drop(parts);
	log.note(format!("weak {}", weak.upgrade().map_or(0, |p| p.id())));
	drop(kept);
	log.note(format!("gone {}", weak.upgrade().is_none()));
	// A future, which drops what it captured with its last `Arc`.
	let captured = guard(6);
	let future: Arc<dyn Future<Output = u32>> = Arc::new(async move { captured.id() });
	let other = future.clone();
	check(3);
	drop(future);
	log.note(format!("future {}", Arc::strong_count(&other)));
	check(4);
	drop(other);
}
