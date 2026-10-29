//! Raw pointers: a circular list of boxed nodes as `hashlink`'s `LinkedHashMap` keeps one,
//! linked through a union and holding a `MaybeUninit` entry, with a free list of nodes to
//! reuse; and pointers to numbers, to fields, and to whole values, read, written,
//! swapped, and compared.

use std::mem::MaybeUninit;
use std::ptr::{self, NonNull};
use std::sync::Arc;

struct Links<V> {
	next: NonNull<Node<V>>,
	prev: NonNull<Node<V>>,
}

impl<V> Clone for Links<V> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<V> Copy for Links<V> {}

struct Free<V> {
	next: Option<NonNull<Node<V>>>,
}

impl<V> Clone for Free<V> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<V> Copy for Free<V> {}

/// A node's links while it is in the list, or while it is free.
union Link<V> {
	value: Links<V>,
	free: Free<V>,
}

struct Node<V> {
	entry: MaybeUninit<(u8, V)>,
	link: Link<V>,
}

/// Entries in the order they were pushed, but for those moved to the front.
struct List<V> {
	/// The guard: `next` is the first entry and `prev` the last, or itself if none.
	guard: Option<NonNull<Node<V>>>,
	free: Option<NonNull<Node<V>>>,
	len: usize,
}

impl<V> List<V> {
	fn new() -> Self {
		List {
			guard: None,
			free: None,
			len: 0,
		}
	}

	fn guard(&mut self) -> NonNull<Node<V>> {
		if let Some(guard) = self.guard {
			return guard;
		}
		let mut guard = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(Node::new()))) };
		unsafe {
			guard.as_mut().link.value = Links {
				next: guard,
				prev: guard,
			}
		};
		self.guard = Some(guard);
		guard
	}

	fn alloc(&mut self) -> NonNull<Node<V>> {
		match self.free {
			Some(free) => {
				self.free = unsafe { free.as_ref().link.free.next };
				free
			}
			None => NonNull::from(Box::leak(Box::new(Node::new()))),
		}
	}

	fn push_back(&mut self, key: u8, value: V) {
		let guard = self.guard();
		let mut node = self.alloc();
		unsafe {
			node.as_mut().entry.as_mut_ptr().write((key, value));
			attach_before(node, guard);
		}
		self.len += 1;
	}

	fn find(&self, key: u8) -> Option<NonNull<Node<V>>> {
		let guard = self.guard?;
		let mut at = unsafe { guard.as_ref().link.value.next };
		while at != guard {
			unsafe {
				if (*at.as_ptr()).entry.assume_init_ref().0 == key {
					return Some(at);
				}
				at = at.as_ref().link.value.next;
			}
		}
		None
	}

	fn remove(&mut self, key: u8) -> Option<(u8, V)> {
		let mut node = self.find(key)?;
		unsafe {
			detach(node);
			let entry = node.as_mut().entry.as_ptr().read();
			node.as_mut().link.free = Free { next: self.free };
			self.free = Some(node);
			self.len -= 1;
			Some(entry)
		}
	}

	fn to_front(&mut self, key: u8) -> bool {
		let Some(node) = self.find(key) else {
			return false;
		};
		unsafe {
			let Some(guard) = self.guard else { return false };
			detach(node);
			attach_before(node, guard.as_ref().link.value.next);
		}
		true
	}

	fn get_mut(&mut self, key: u8) -> Option<&mut V> {
		let node = self.find(key)?;
		unsafe { Some(&mut (*(*node.as_ptr()).entry.as_mut_ptr()).1) }
	}
}

impl List<String> {
	fn each(&self, out: &mut Vec<u8>) {
		let Some(guard) = self.guard else {
			return;
		};
		let mut at = unsafe { guard.as_ref().link.value.next };
		while at != guard {
			let (key, value) = unsafe { at.as_ref().entry.assume_init_ref() };
			out.push(*key);
			out.extend_from_slice(value.as_bytes());
			at = unsafe { at.as_ref().link.value.next };
		}
	}
}

impl<V> Node<V> {
	fn new() -> Self {
		Node {
			entry: MaybeUninit::uninit(),
			link: Link {
				value: Links {
					next: NonNull::dangling(),
					prev: NonNull::dangling(),
				},
			},
		}
	}
}

/// Put `node` in the list just before `at`.
unsafe fn attach_before<V>(mut node: NonNull<Node<V>>, mut at: NonNull<Node<V>>) {
	unsafe {
		node.as_mut().link.value = Links {
			prev: at.as_ref().link.value.prev,
			next: at,
		};
		at.as_mut().link.value.prev = node;
		(*node.as_ref().link.value.prev.as_ptr()).link.value.next = node;
	}
}

unsafe fn detach<V>(mut node: NonNull<Node<V>>) {
	unsafe {
		node.as_mut().link.value.prev.as_mut().link.value.next = node.as_ref().link.value.next;
		node.as_mut().link.value.next.as_mut().link.value.prev = node.as_ref().link.value.prev;
	}
}

impl<V> Drop for List<V> {
	fn drop(&mut self) {
		if let Some(guard) = self.guard {
			unsafe {
				let mut at = guard.as_ref().link.value.next;
				while at != guard {
					let next = at.as_ref().link.value.next;
					let mut node = Box::from_raw(at.as_ptr());
					node.entry.assume_init_drop();
					at = next;
				}
				drop(Box::from_raw(guard.as_ptr()));
			}
		}
		while let Some(free) = self.free {
			unsafe {
				self.free = free.as_ref().link.free.next;
				drop(Box::from_raw(free.as_ptr()));
			}
		}
	}
}

/// Steps on a list of the operation and key each pair of bytes picks, putting what each
/// finds and then the list in order.
pub fn lists(data: &[u8], out: &mut Vec<u8>) {
	let mut list = List::<String>::new();
	let mut data = data;
	while let [op, key, rest @ ..] = data {
		data = rest;
		let (op, key) = (*op, *key % 8);
		match op % 4 {
			0 => list.push_back(key, String::from(["", "v", "vv"][usize::from(op % 3)])),
			1 => match list.remove(key) {
				Some((k, v)) => {
					out.push(k);
					out.extend_from_slice(v.as_bytes());
				}
				None => out.push(0xff),
			},
			2 => out.push(list.to_front(key) as u8),
			_ => match list.get_mut(key) {
				Some(value) => value.push('w'),
				None => out.push(0xfe),
			},
		}
	}
	out.push(list.len as u8);
	list.each(out);
}

/// Steps on a list of shared counts, putting how many hold the one each push clones
/// while the list lives, and after it drops.
pub fn shared(data: &[u8], out: &mut Vec<u8>) {
	let held = Arc::new(0u8);
	{
		let mut list = List::<Arc<u8>>::new();
		let mut data = data;
		while let [op, key, rest @ ..] = data {
			data = rest;
			let (op, key) = (*op, *key % 4);
			match op % 3 {
				0 => list.push_back(key, Arc::clone(&held)),
				1 => out.push(list.remove(key).map_or(0xff, |(_, v)| *v)),
				_ => out.push(list.to_front(key) as u8),
			}
			out.push(Arc::strong_count(&held) as u8);
		}
	}
	out.push(Arc::strong_count(&held) as u8);
}

#[derive(Clone, Copy)]
struct Pair {
	a: u32,
	b: u32,
}

/// Pointers to numbers, to a field, and to a whole struct: read, written, swapped,
/// replaced, compared, and null.
pub fn places(data: &[u8], out: &mut Vec<u8>) {
	let mut x = 0u32;
	let mut y = 1u32;
	let mut pair = Pair { a: 2, b: 3 };
	let px = &raw mut x;
	let py = &raw mut y;
	let pa = &raw mut pair.a;
	let pp = &raw mut pair;
	let mut held: *const u32 = ptr::null();
	for &byte in data {
		let v = u32::from(byte);
		unsafe {
			match byte % 9 {
				0 => *px += v,
				1 => py.write(v),
				2 => ptr::swap(px, py),
				3 => out.push(ptr::replace(pa, v) as u8),
				4 => (*pp).b = v,
				5 => {
					let old = pp.read();
					pp.write(Pair { a: old.b, b: old.a });
				}
				6 => held = if byte & 1 == 0 { px } else { pa },
				7 => out.push(held.is_null() as u8 | ((pp == &raw mut pair) as u8) << 1),
				_ => out.push(if held.is_null() { 0xff } else { *held as u8 }),
			}
		}
	}
	out.extend_from_slice(&[x as u8, y as u8, pair.a as u8, pair.b as u8]);
}

/// Unchecked indexing, a boxed number behind a pointer, a `MaybeUninit` number, and a
/// value dropped in place.
pub fn unchecked(data: &[u8], out: &mut Vec<u8>) {
	let mut counts = [0u32; 4];
	let boxed = Box::into_raw(Box::new(0u32));
	let mut slot = MaybeUninit::<u32>::uninit();
	slot.write(7);
	let held = Arc::new(0u8);
	let mut dropped = MaybeUninit::new(Arc::clone(&held));
	for &byte in data {
		let at = usize::from(byte % 4);
		unsafe {
			*counts.get_unchecked_mut(at) += 1;
			*boxed += u32::from(byte);
			*slot.as_mut_ptr() ^= u32::from(byte);
			out.push(*counts.get_unchecked(at) as u8);
		}
	}
	unsafe {
		out.push(*Box::from_raw(boxed) as u8);
		out.push(slot.assume_init_read() as u8);
		out.push(Arc::strong_count(&held) as u8);
		ptr::drop_in_place(dropped.as_mut_ptr());
	}
	out.push(Arc::strong_count(&held) as u8);
}
