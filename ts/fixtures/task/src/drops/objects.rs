//! What a `Box<dyn Trait>` drops: the value it was made of, by its own type's drop, a
//! struct's, an enum's glue, or nothing for a number, where the box goes out of scope,
//! is replaced, or is consumed by `self: Box<Self>`. An `Arc<dyn Trait>`
//! drops it with the last `Arc` of it, of the trait object or of the type it was made of.
//! A `dyn Any` drops the same, and one downcast drops as the type it was made of. A
//! `dyn Fn*` drops what its closure owns, and a call through `FnOnce` consumes it.

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

/// Boxed and shared closures as `dyn Fn*`: each drops what it owns where the box or the
/// last `Arc` goes, or, called through `FnOnce`, where its body or the call ends.
pub(super) fn fns(log: &Log, id: u32) {
	let guard = |n: u32| Guard::new(id + n, log.clone());
	// An `FnOnce` closure's body drops what it owns; uncalled, the box drops it.
	let (a, b) = (guard(1), guard(2));
	let moved: Box<dyn FnOnce() -> u32> = Box::new(move || {
		let id = a.id();
		drop(a);
		id
	});
	let uncalled: Box<dyn FnOnce() -> u32> = Box::new(move || b.id());
	log.note(format!("moved {}", moved()));
	drop(uncalled);
	// An `Fn` closure called once through `FnOnce`, boxed as one or upcast to one, drops
	// what it owns after the call.
	let c = guard(3);
	let as_once: Box<dyn FnOnce() -> u32> = Box::new(move || c.id());
	log.note(format!("once {}", as_once()));
	let d = guard(4);
	let up: Box<dyn Fn() -> u32> = Box::new(move || d.id() + 1);
	let up: Box<dyn FnOnce() -> u32> = up;
	log.note(format!("up {}", up()));
	// A replaced box drops the old closure, and a `Vec` drops its boxes in order.
	let e = guard(5);
	let mut slot: Box<dyn FnMut() -> u32> = Box::new(move || e.id());
	log.note(format!("slot {}", slot()));
	let f = guard(6);
	slot = Box::new(move || f.id());
	log.note(format!("slot {}", slot()));
	let (g, h) = (guard(7), guard(8));
	let list: Vec<Box<dyn Fn() -> u32>> = vec![Box::new(move || g.id()), Box::new(move || h.id())];
	let borrowed: &dyn Fn() -> u32 = &*list[1];
	log.note(format!("list {}", borrowed() + list[0]()));
	drop(list);
	// Shared: the last `Arc` drops it.
	let i = guard(9);
	let shared: Arc<dyn Fn() -> u32> = Arc::new(move || i.id());
	let other = shared.clone();
	drop(shared);
	log.note(format!("shared {}", other()));
	drop(other);
	log.note(format!("slot {}", slot()));
	// A box of a reference drops only the reference, and a box of a box is the closure it
	// holds, which a call through `FnOnce` consumes.
	let j = guard(10);
	let lent = move || j.id();
	let boxed: Box<dyn Fn() -> u32 + '_> = Box::new(&lent);
	log.note(format!("lent {}", boxed()));
	drop(boxed);
	log.note(format!("lent {}", lent()));
	let k = guard(11);
	let inner: Box<dyn Fn() -> u32> = Box::new(move || k.id());
	let outer: Box<dyn FnOnce() -> u32> = Box::new(inner);
	log.note(format!("outer {}", outer()));
	drop(lent);
	// An upcast of an `Arc` shares its count: the last clone of either drops it.
	let m = guard(12);
	let shared: Arc<dyn Fn() -> u32> = Arc::new(move || m.id());
	let up: Arc<dyn FnOnce() -> u32> = shared.clone();
	let again = up.clone();
	drop(shared);
	drop(up);
	log.note(format!("up {}", Arc::strong_count(&again)));
	drop(again);
	log.note("gone".into());
	// A box that a generic `FnOnce` call consumes drops what it holds as the call ends.
	let (n, o) = (guard(13), guard(14));
	let boxed: Box<dyn Fn() -> u32> = Box::new(move || n.id());
	log.note(format!("run {}", run(boxed)));
	log.note(format!("run {}", run(Box::new(move || o.id()))));
	log.note("ran".into());
	// So does one std's `Option::map` consumes, and an iterator's adaptor drops its own.
	let (p, q) = (guard(15), guard(16));
	let add: Box<dyn Fn(u32) -> u32> = Box::new(move |x| x + q.id());
	log.note(format!("map {:?}", Some(1).map(add)));
	let s = guard(18);
	let scale: Box<dyn FnMut(&u32) -> u32> = Box::new(move |x| x * s.id());
	let total: u32 = [1, 2].iter().map(scale).sum();
	log.note(format!("sum {total}"));
	let r = guard(17);
	let init: Box<dyn Fn() -> u32> = Box::new(move || r.id());
	let unused: Box<dyn Fn() -> u32> = Box::new(move || p.id());
	let cell: std::cell::OnceCell<u32> = std::cell::OnceCell::default();
	let got = *cell.get_or_init(init);
	log.note(format!("cell {got}"));
	log.note(format!("cell {}", cell.get_or_init(unused)));
	let (u, v) = (guard(19), guard(20));
	let fill: Box<dyn Fn() -> u32> = Box::new(move || u.id());
	let skip: Box<dyn Fn() -> u32> = Box::new(move || v.id());
	let mut lazy: Option<u32> = None;
	log.note(format!("lazy {}", lazy.get_or_insert_with(fill)));
	log.note(format!("lazy {}", lazy.get_or_insert_with(skip)));
	// An adaptor drops the function it holds, but not one it borrows.
	let w = guard(21);
	let f = move |g: Guard| g.id() + w.id();
	let borrowed: &dyn Fn(Guard) -> u32 = &f;
	log.note(format!(
		"borrowed {}",
		vec![guard(22), guard(23)].into_iter().map(borrowed).sum::<u32>()
	));
	log.note(format!("by ref {}", vec![guard(24)].into_iter().map(&f).sum::<u32>()));
	log.note(format!("kept {}", f(guard(25))));
}

/// Call `f` once, which consumes it.
fn run<F: FnOnce() -> u32>(f: F) -> u32 {
	f()
}
