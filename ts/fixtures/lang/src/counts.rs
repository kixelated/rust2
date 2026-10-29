//! Strong and weak counts of `Arc`s and `Rc`s and their `Weak`s, through every way one is
//! made and dropped: cloned, downgraded, upgraded, made dangling, made holding handles to
//! itself, and held in a `Vec`, an `Option`, a struct, and a closure, of a value and of a
//! trait object.

use std::rc::{self, Rc};
use std::sync::{self, Arc, Mutex};

/// Logs its id when it drops, so the last strong drop shows where it happens.
struct Noisy {
	id: u8,
	log: Arc<Mutex<Vec<u8>>>,
}

impl Drop for Noisy {
	fn drop(&mut self) {
		self.log.lock().unwrap().push(self.id);
	}
}

/// Puts what dropped since the last flush.
fn flush(log: &Mutex<Vec<u8>>, out: &mut Vec<u8>) {
	let mut log = log.lock().unwrap();
	out.extend_from_slice(&log);
	log.clear();
}

/// What a trait object of a `Noisy` reads.
trait Id {
	fn id(&self) -> u8;
}

impl Id for Noisy {
	fn id(&self) -> u8 {
		self.id
	}
}

macro_rules! counts {
	($name:ident, $holder:ident, $cyclic:ident, $node:ident, $Ptr:ident, $weak:ident) => {
		/// Holds a handle that doesn't keep its value.
		struct $holder<T> {
			weak: $weak::Weak<T>,
		}

		/// Holds handles to itself, which `new_cyclic` gives it before it exists, and what
		/// the first read then.
		struct $node {
			this: $weak::Weak<$node>,
			more: Vec<$weak::Weak<$node>>,
			noisy: Noisy,
			seen: [u8; 3],
		}

		impl $node {
			fn new(this: &$weak::Weak<$node>, noisy: Noisy, more: u8) -> Self {
				Self {
					this: this.clone(),
					more: (0..more).map(|_| this.clone()).collect(),
					noisy,
					seen: [
						this.upgrade().is_none() as u8,
						$weak::Weak::strong_count(this) as u8,
						$weak::Weak::weak_count(this) as u8,
					],
				}
			}
		}

		/// Each byte a value made by `new_cyclic`, from a closure that moves what it owns,
		/// reads it, reads it as an `Fn`, or owns nothing that drops, holding up to 3 more
		/// handles to itself; then kept, dropped, or dropped with a handle outside. Each puts what it read and what
		/// dropped.
		pub fn $cyclic(data: &[u8], out: &mut Vec<u8>) {
			let log = Arc::new(Mutex::new(Vec::new()));
			let mut kept: Vec<$Ptr<$node>> = Vec::new();
			let mut outside: Vec<$weak::Weak<$node>> = Vec::new();
			for &op in data {
				let noisy = Noisy {
					id: op,
					log: log.clone(),
				};
				let more = (op >> 2) % 4;
				let node = match op % 4 {
					0 => $Ptr::new_cyclic(move |this| $node::new(this, noisy, more)),
					// Its capture drops once it is called.
					1 => $Ptr::new_cyclic(move |this| {
						let copy = Noisy {
							id: noisy.id ^ 0x80,
							log: noisy.log.clone(),
						};
						$node::new(this, copy, more)
					}),
					// An `Fn`, made before the call, so `new_cyclic` drops it after.
					2 => {
						let make = move |this: &$weak::Weak<$node>| {
							let copy = Noisy {
								id: noisy.id ^ 0x40,
								log: noisy.log.clone(),
							};
							$node::new(this, copy, more)
						};
						$Ptr::new_cyclic(make)
					}
					_ => $Ptr::new_cyclic(|this| {
						let fresh = Noisy {
							id: 0xee,
							log: log.clone(),
						};
						$node::new(this, fresh, more)
					}),
				};
				out.extend(node.seen);
				out.push($Ptr::strong_count(&node) as u8);
				out.push($Ptr::weak_count(&node) as u8);
				out.push(node.more.len() as u8);
				out.push(node.this.upgrade().map_or(0xfd, |n| n.noisy.id));
				out.push($weak::Weak::ptr_eq(&node.this, &$Ptr::downgrade(&node)) as u8);
				out.push($Ptr::weak_count(&node) as u8);
				match op >> 6 {
					0 => kept.push(node),
					1 => {
						outside.push($Ptr::downgrade(&node));
						drop(node);
					}
					_ => drop(node),
				}
				flush(&log, out);
			}
			drop(kept);
			out.extend(outside.iter().map(|w| $weak::Weak::strong_count(w) as u8));
			flush(&log, out);
		}

		/// Each byte an operation on pools of strong and weak handles, the rest of it the
		/// handle it picks; each puts what it read.
		pub fn $name(data: &[u8], out: &mut Vec<u8>) {
			let log = Arc::new(Mutex::new(Vec::new()));
			let mut strong: Vec<$Ptr<Noisy>> = Vec::new();
			let mut weak: Vec<$weak::Weak<Noisy>> = Vec::new();
			for &op in data {
				let pick = usize::from(op >> 4);
				match op % 10 {
					0 => strong.push($Ptr::new(Noisy {
						id: op,
						log: log.clone(),
					})),
					1 if !strong.is_empty() => {
						let again = strong[pick % strong.len()].clone();
						strong.push(again);
					}
					2 if !strong.is_empty() => drop(strong.swap_remove(pick % strong.len())),
					3 if !strong.is_empty() => weak.push($Ptr::downgrade(&strong[pick % strong.len()])),
					4 if !weak.is_empty() => {
						let again = weak[pick % weak.len()].clone();
						weak.push(again);
					}
					5 if !weak.is_empty() => drop(weak.swap_remove(pick % weak.len())),
					6 if !weak.is_empty() => match weak[pick % weak.len()].upgrade() {
						Some(up) => {
							out.push(up.id);
							strong.push(up);
						}
						None => out.push(0xfe),
					},
					7 => weak.push($weak::Weak::new()),
					// A weak handle in an `Option`, a struct, and a closure, each dropped
					// before the next byte.
					8 if !strong.is_empty() => {
						let picked = &strong[pick % strong.len()];
						let held = Some($Ptr::downgrade(picked));
						let kept = $holder {
							weak: $Ptr::downgrade(picked),
						};
						let captured = $Ptr::downgrade(picked);
						let read = move || captured.upgrade().map_or(0xfd, |s| s.id);
						out.push($Ptr::weak_count(picked) as u8);
						out.push(read());
						drop(held);
						out.push($Ptr::weak_count(picked) as u8);
						drop(kept);
						drop(read);
						out.push($Ptr::weak_count(picked) as u8);
					}
					// Every count, of each strong handle and each weak one.
					_ => {
						for s in &strong {
							out.push($Ptr::strong_count(s) as u8);
							out.push($Ptr::weak_count(s) as u8);
						}
						for w in &weak {
							out.push($weak::Weak::strong_count(w) as u8);
							out.push($weak::Weak::weak_count(w) as u8);
						}
					}
				}
			}
			// Trait objects of the last strong handle share its counts.
			if let Some(last) = strong.last() {
				let object: $Ptr<dyn Id> = last.clone();
				let weak_object = $Ptr::downgrade(&object);
				out.push($Ptr::weak_count(last) as u8);
				out.push($weak::Weak::weak_count(&weak_object) as u8);
				out.push(weak_object.upgrade().map_or(0xfd, |o| o.id()));
				drop(object);
				out.push($weak::Weak::strong_count(&weak_object) as u8);
			}
			drop(strong);
			out.extend(weak.iter().map(|w| w.upgrade().is_none() as u8));
			out.extend(weak.iter().map(|w| $weak::Weak::weak_count(w) as u8));
			drop(weak);
			out.extend_from_slice(&log.lock().unwrap());
		}
	};
}

counts!(arcs, ArcHolder, cyclic_arcs, ArcNode, Arc, sync);
counts!(rcs, RcHolder, cyclic_rcs, RcNode, Rc, rc);
