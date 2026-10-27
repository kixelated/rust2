//! The runtime's own behavior, beyond what translated fixtures exercise.

use std::path::PathBuf;
use std::process::Command;

/// `rt.spawn` yields to a macrotask once a batch spends its poll budget.
#[test]
fn spawn_budget() {
	let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("spawn.ts");
	let output = Command::new("node")
		.arg(script)
		.output()
		.expect("running node, which the dev shell provides");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

/// The runtime's B-tree runs `btree.ts`'s script of random operations as std's does.
#[test]
fn btree_script() {
	use std::collections::BTreeMap;

	let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("btree.ts");
	let output = Command::new("node")
		.arg(script)
		.output()
		.expect("running node, which the dev shell provides");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	let got = String::from_utf8(output.stdout).unwrap();

	// xorshift32, as `btree.ts` runs it.
	let mut state: u32 = 0x2545f491;
	let mut rand = move || {
		state ^= state << 13;
		state ^= state >> 17;
		state ^= state << 5;
		state
	};
	fn walk<'a>(
		rand: &mut impl FnMut() -> u32,
		it: &mut impl DoubleEndedIterator<Item = (&'a u32, &'a u32)>,
	) -> String {
		let mut seen = Vec::new();
		for _ in 0..40 {
			let item = if rand() & 1 == 1 { it.next() } else { it.next_back() };
			let Some((k, _)) = item else { break };
			seen.push(k.to_string());
		}
		seen.join(",")
	}
	let show = |v: Option<u32>| v.map_or("-".to_string(), |v| v.to_string());
	let entry = |e: Option<(&u32, &u32)>| e.map_or("-".to_string(), |(k, v)| format!("{k} {v}"));

	let mut want = Vec::new();
	let mut m: BTreeMap<u32, u32> = BTreeMap::new();
	let steps = 120_000;
	for step in 0..steps {
		let keys = 65_536;
		let phase = 3 * step / steps;
		let op = rand() % 10;
		let k = rand() % keys;
		if op < 5 {
			if phase == 0 || (phase == 1 && k % 2 == 0) {
				want.push(format!("insert {k} {}", show(m.insert(k, step))));
			} else {
				want.push(format!("remove {k} {}", show(m.remove(&k))));
			}
		} else if op == 5 {
			want.push(format!("get {k} {}", show(m.get(&k).copied())));
		} else if op == 6 {
			let j = rand() % keys;
			let (lo, hi) = if k < j { (k, j) } else { (j, k) };
			let inclusive = rand() & 1;
			let seen = match inclusive {
				1 => walk(&mut rand, &mut m.range(lo..=hi)),
				_ => walk(&mut rand, &mut m.range(lo..hi)),
			};
			want.push(format!("range {lo} {hi} {inclusive} {seen}"));
		} else if op == 7 {
			let last = rand() & 1 == 1;
			let popped = if last { m.pop_last() } else { m.pop_first() };
			want.push(format!(
				"pop {last} {}",
				popped.map_or("-".to_string(), |(k, v)| format!("{k} {v}"))
			));
		} else if op == 8 {
			want.push(format!("remove {k} {}", show(m.remove(&k))));
		} else if step % 97 == 0 {
			let mut it = m.iter();
			let seen = walk(&mut rand, &mut it);
			want.push(format!("iter {} {} {seen}", m.len(), it.len()));
		} else if step % 1009 == 0 {
			let copy = m.clone();
			m.retain(|key, _| key % 7 != k % 7);
			want.push(format!(
				"retain {} {} {}",
				m.len(),
				entry(m.first_key_value()),
				entry(m.last_key_value())
			));
			if rand() & 1 == 1 {
				m = copy;
			}
			want.push(format!("kept {}", m.len()));
		}
	}
	let keys: Vec<String> = m.keys().map(u32::to_string).collect();
	want.push(format!("end {} {}", m.len(), keys.join(",")));
	let mut drained = Vec::new();
	while !m.is_empty() {
		let popped = if rand() & 1 == 1 { m.pop_last() } else { m.pop_first() };
		drained.push(popped.unwrap().0.to_string());
	}
	want.push(format!("drain {}", drained.join(",")));
	for (got, want) in got.lines().zip(&want) {
		assert_eq!(got, want);
	}
	assert_eq!(got.lines().count(), want.len());
}

/// A value that logs itself when it drops.
struct Logged<'a, T: Copy>(T, &'a std::cell::RefCell<Vec<T>>);

impl<T: Copy> Drop for Logged<'_, T> {
	fn drop(&mut self) {
		self.1.borrow_mut().push(self.0);
	}
}

/// A `retain` whose predicate panics leaves a B-tree, a deque, and a `Vec` as std's do,
/// and a `dedup_by` whose test panics leaves a `Vec`, each dropping what std's drops,
/// which `retain.ts` prints for every keep mask of up to 6 items and each item to panic on.
#[test]
fn retain_unwinds() {
	use std::collections::{BTreeMap, VecDeque};
	use std::panic::{AssertUnwindSafe, catch_unwind};

	let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("tests")
		.join("retain.ts");
	let output = Command::new("node")
		.arg(script)
		.output()
		.expect("running node, which the dev shell provides");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	let got = String::from_utf8(output.stdout).unwrap();

	let hook = std::panic::take_hook();
	std::panic::set_hook(Box::new(|_| {}));
	let mut want = String::new();
	for n in 0..=6u32 {
		for mask in 0..1u32 << n {
			for p in 0..=n {
				let keep = |i: u32| {
					assert!(i != p);
					(mask >> i) & 1 == 1
				};
				let mut tree: BTreeMap<u32, u32> = (0..n).map(|i| (i, i * 10)).collect();
				let _ = catch_unwind(AssertUnwindSafe(|| tree.retain(|&k, _| keep(k))));
				let mut deque = VecDeque::new();
				for i in (0..n / 2).rev() {
					deque.push_front(i);
				}
				deque.extend(n / 2..n);
				let _ = catch_unwind(AssertUnwindSafe(|| deque.retain(|&i| keep(i))));
				let dropped = std::cell::RefCell::new(Vec::new());
				let mut vec: Vec<Logged<u32>> = (0..n).map(|i| Logged(i, &dropped)).collect();
				let _ = catch_unwind(AssertUnwindSafe(|| vec.retain(|i| keep(i.0))));
				let vec: Vec<u32> = vec.iter().map(|i| i.0).collect();
				let dropped = std::mem::take(&mut *dropped.borrow_mut());
				// A run ends at each item the mask clears, and `same` panics comparing `p`.
				let deduped = std::cell::RefCell::new(Vec::new());
				let mut run = 0;
				let mut runs: Vec<Logged<(u32, u32)>> = (0..n)
					.map(|i| {
						if (mask >> i) & 1 == 0 {
							run += 1;
						}
						Logged((i, run), &deduped)
					})
					.collect();
				let _ = catch_unwind(AssertUnwindSafe(|| {
					runs.dedup_by(|a, b| {
						assert!(a.0.0 != p);
						a.0.1 == b.0.1
					})
				}));
				let ran: Vec<u32> = runs.iter().map(|r| r.0.0).collect();
				let deduped: Vec<u32> = std::mem::take(&mut *deduped.borrow_mut()).into_iter().map(|r| r.0).collect();
				let entries: Vec<String> = tree.iter().map(|(k, v)| format!("{k},{v}")).collect();
				let items: Vec<String> = deque.iter().map(u32::to_string).collect();
				let join = |v: &[u32]| v.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
				want += &format!(
					"{n} {mask} {p} btree {} deque {} vec {} dropped {} dedup {} dropped {}\n",
					entries.join(";"),
					items.join(","),
					join(&vec),
					join(&dropped),
					join(&ran),
					join(&deduped),
				);
			}
		}
	}
	std::panic::set_hook(hook);
	for (got, want) in got.lines().zip(want.lines()) {
		assert_eq!(got, want);
	}
	assert_eq!(got.lines().count(), want.lines().count());
}
