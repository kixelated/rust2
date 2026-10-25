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

/// A `retain` whose predicate panics leaves a B-tree and a deque as std's do, which
/// `retain.ts` prints for every keep mask of up to 6 items and each item to panic on.
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
				let entries: Vec<String> = tree.iter().map(|(k, v)| format!("{k},{v}")).collect();
				let items: Vec<String> = deque.iter().map(u32::to_string).collect();
				want += &format!("{n} {mask} {p} btree {} deque {}\n", entries.join(";"), items.join(","));
			}
		}
	}
	std::panic::set_hook(hook);
	for (got, want) in got.lines().zip(want.lines()) {
		assert_eq!(got, want);
	}
	assert_eq!(got.lines().count(), want.lines().count());
}
