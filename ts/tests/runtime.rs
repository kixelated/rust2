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
