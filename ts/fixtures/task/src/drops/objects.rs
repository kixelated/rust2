//! What a `Box<dyn Trait>` drops: the value it was made of, by its own type's drop, a
//! struct's, an enum's glue, or nothing for a number, where the box goes out of scope,
//! is replaced, or is consumed by `self: Box<Self>`. An `Arc<dyn Trait>`
//! drops it with the last `Arc` of it, of the trait object or of the type it was made of.
//! A `dyn Any` drops the same, and one downcast drops as the type it was made of.

use std::any::Any;
use std::future::Future;
use std::sync::Arc;

use super::{Held, Wrap};
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

impl Part for Wrap<u32> {
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

/// Boxed parts of each kind.
pub(super) fn boxed(log: &Log, id: u32) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let first: Box<dyn Part> = Box::new(guard(1));
	let mut parts: Vec<Box<dyn Part>> = Vec::new();
	parts.push(Box::new(Held::Two(guard(2), guard(3))));
	parts.push(Box::new(7u32));
	parts.push(Box::new(Wrap {
		id: id + 4,
		log: log.clone(),
		inner: 0,
	}));
	parts.push(Box::new(Held::One(guard(5))));
	parts.swap(2, 3);
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
	log.note(format!("took {}", first.take()));
	let last = parts.pop().map(Part::take);
	log.note(format!("last {last:?}"));
	let kept: Option<Box<dyn Part>> = Some(Box::new(guard(7)));
	drop(parts);
	log.note(format!("kept {} {}", kept.as_ref().map_or(0, |p| p.id()), slot.id()));
}

/// Shared parts of each kind, each dropped by the last `Arc` of it, a trait object's or
/// its own.
pub(super) fn shared(log: &Log, id: u32) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let held = Arc::new(Held::Two(guard(1), guard(2)));
	let first: Arc<dyn Part> = held.clone();
	let second: Arc<dyn Part> = held.clone();
	log.note(format!("held {} {}", first.id(), Arc::strong_count(&held)));
	drop(held);
	drop(first);
	log.note(format!("count {}", Arc::strong_count(&second)));
	let kept = Arc::new(guard(3));
	let wrap = Arc::new(Wrap {
		id: id + 4,
		log: log.clone(),
		inner: 0,
	});
	let mut parts: Vec<Arc<dyn Part>> = Vec::new();
	parts.push(wrap.clone());
	parts.push(kept.clone());
	parts.push(second.clone());
	parts.push(Arc::new(7u32));
	let weak = Arc::downgrade(&parts[1]);
	drop(second);
	log.note(format!(
		"ids {}",
		parts.iter().map(|p| p.id()).fold(0u32, u32::wrapping_add)
	));
	// The wrap's last `Arc` is the trait object the `Vec` holds; the guard's is its own.
	drop(wrap);
	drop(parts);
	log.note(format!("weak {}", weak.upgrade().map_or(0, |p| p.id())));
	drop(kept);
	log.note(format!("gone {}", weak.upgrade().is_none()));
	// A future, which drops what it captured with its last `Arc`.
	let captured = guard(6);
	let future: Arc<dyn Future<Output = u32>> = Arc::new(async move { captured.id() });
	let other = future.clone();
	drop(future);
	log.note(format!("future {}", Arc::strong_count(&other)));
	drop(other);
}

/// Boxed and shared values as `dyn Any`: each drops what it was made of, as is or downcast,
/// and an `Arc` held only for its drop drops it with its last clone.
pub(super) fn anys(log: &Log, id: u32) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	let mut boxes: Vec<Box<dyn Any>> = Vec::new();
	boxes.push(Box::new(guard(1)));
	boxes.push(Box::new(Held::Two(guard(2), guard(3))));
	boxes.push(Box::new(7u32));
	boxes.push(Box::new(guard(4)));
	let last = boxes.pop().map(|b| b.downcast::<Guard>());
	log.note(format!(
		"last {:?}",
		last.as_ref().map(|g| g.as_ref().map(|g| g.id()).is_ok())
	));
	drop(last);
	// A miss gives the box back, which still drops what it holds.
	let held = boxes.remove(1).downcast::<Guard>();
	log.note(format!("held {}", held.is_err()));
	let mut slot: Box<dyn Any> = Box::new(guard(5));
	slot = Box::new(Held::Empty);
	log.note(format!("slot {}", slot.is::<Held>()));
	drop(held);
	drop(boxes);
	// A guard held for its drop alone, as moq holds its permits.
	let hold: Arc<dyn Any + Send + Sync> = Arc::new((guard(6), guard(7)));
	let clones = [hold.clone(), hold.clone()];
	drop(hold);
	log.note(format!("clones {}", Arc::strong_count(&clones[0])));
	let [a, b] = clones;
	let typed = a.downcast::<(Guard, Guard)>().ok();
	log.note(format!(
		"typed {:?}",
		typed.as_ref().map(|t| (t.0.id(), Arc::strong_count(t)))
	));
	drop(typed);
	log.note("dropped".into());
	drop(b);
	log.note(format!("slot {}", slot.downcast_ref::<Held>().is_some()));
}
