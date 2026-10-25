//! The harness on the host, where a panic unwinds: a step that panics is logged, and the
//! script goes on, so the log shows what unwinding dropped. `tests/unwind.ts` runs the
//! translation the same way and compares the logs.
//!
//! usage: unwind < scripts
//!
//! Each input line is a script in hex; each output line is its log in hex.

use std::io::{BufRead, Write};

fn main() {
	// The harness logs each panic; the default hook would print it too.
	std::panic::set_hook(Box::new(|_| {}));
	let mut out = std::io::stdout().lock();
	for line in std::io::stdin().lock().lines() {
		let line = line.expect("reading stdin");
		let input: Vec<u8> = (0..line.len())
			.step_by(2)
			.map(|i| u8::from_str_radix(&line[i..i + 2], 16).expect("hex"))
			.collect();
		let log = task_fixture::harness::run(&input);
		let hex: String = log.iter().map(|b| format!("{b:02x}")).collect();
		writeln!(out, "{hex}").expect("writing stdout");
	}
}
