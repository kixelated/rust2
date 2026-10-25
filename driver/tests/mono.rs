//! rust2-lint's mono budget on the fixture workspace in `fixtures/mono`, where `user`
//! instantiates each of `exec`'s generics with 20 closure types.

use std::path::{Path, PathBuf};
use std::process::Output;

fn fixture() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures").join("mono")
}

/// An empty target directory for the test `name`: cargo can't tell a rebuilt rust2-lint
/// from the one that built what it keeps, so a kept build would skip the lint.
fn target(name: &str) -> PathBuf {
	let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("mono").join(name);
	match std::fs::remove_dir_all(&dir) {
		Err(err) if err.kind() != std::io::ErrorKind::NotFound => panic!("removing {}: {err}", dir.display()),
		_ => dir,
	}
}

/// Run rust2-lint in the fixture workspace with `args`, into the target directory `target`.
fn lint(target: &Path, args: &[&str]) -> Output {
	std::process::Command::new(env!("CARGO_BIN_EXE_rust2-lint"))
		.args(args)
		.current_dir(fixture())
		.env("CARGO_TARGET_DIR", target)
		.output()
		.expect("running rust2-lint")
}

fn stderr(output: &Output) -> String {
	String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The line `needle` starts in `file` of the fixture, from 1.
fn line_of(file: &str, needle: &str) -> usize {
	let text = std::fs::read_to_string(fixture().join(file)).unwrap();
	text.lines().position(|line| line.starts_with(needle)).expect(needle) + 1
}

#[test]
fn fat_generic_fails_at_its_span_with_its_callers() {
	let build = ["--", "build", "--package", "user", "--features", "fat"];
	let target = target("fat");
	let passed = lint(&target, &[&["--mono-budget", "1000000"][..], &build].concat());
	assert!(passed.status.success(), "{}", stderr(&passed));

	// A new budget rebuilds the crate, which was fresh under the old one.
	let failed = lint(&target, &[&["--mono-budget", "1000"][..], &build].concat());
	let err = stderr(&failed);
	assert!(!failed.status.success(), "{err}");
	// Its async body is its cost: the function alone only makes the future.
	assert!(err.contains("error: `exec::fat` costs "), "{err}");
	assert!(err.contains(" in 21 copies of "), "{err}");
	let at = format!(
		"--> exec/src/lib.rs:{}:1",
		line_of("exec/src/lib.rs", "pub async fn fat")
	);
	assert!(err.contains(&at), "{err}");
	// The heaviest caller first.
	let fat = err.find("note: `fat` instantiates 20").expect(&err);
	let once = err.find("note: `once` instantiates 1").expect(&err);
	assert!(fat < once, "{err}");
	assert_eq!(err.matches("error: `").count(), 1, "only `fat` is over: {err}");
}

#[test]
fn forwarder_and_own_budget_pass() {
	// A forwarder passes any budget; `budgeted` sets its own.
	let output = lint(
		&target("pass"),
		&[
			"--mono-budget",
			"1",
			"--",
			"build",
			"--package",
			"user",
			"--features",
			"thin,budgeted",
		],
	);
	assert!(output.status.success(), "{}", stderr(&output));
}

#[test]
fn misplaced_attributes_fail() {
	let output = lint(&target("misplaced"), &["--", "build", "--package", "misplaced"]);
	let err = stderr(&output);
	assert!(!output.status.success(), "{err}");
	let file = "misplaced/src/lib.rs";
	let errors: Vec<&str> = err.split("error: ").filter(|error| error.contains(file)).collect();
	let expected = [
		("#[cfg_attr(rust2, rust2::mono_budget(limit = 10))]", "takes `limit"),
		(
			"#[cfg_attr(rust2, rust2::mono_budget(limit = 10, reason = \"  \"))]",
			"takes `limit",
		),
		(
			"#[cfg_attr(rust2, rust2::mono_budget(limit = 10, reason = \"not g",
			"goes on a generic function",
		),
		(
			"\t#[cfg_attr(rust2, rust2::mono_budget(limit = 10, reason = \"a st",
			"goes on a generic function",
		),
		(
			"#[cfg_attr(rust2, rust2::mono_bugdet",
			"unknown attribute `rust2::mono_bugdet`",
		),
	];
	for (needle, message) in expected {
		let at = format!("{file}:{}:", line_of(file, needle));
		assert!(
			errors
				.iter()
				.any(|error| error.contains(message) && error.contains(&at)),
			"no `{message}` at {at}: {err}"
		);
	}
	assert_eq!(errors.len(), expected.len(), "{err}");
}

#[test]
fn check_fails() {
	let output = lint(&target("check"), &["--", "check", "--package", "user"]);
	let err = stderr(&output);
	assert!(!output.status.success(), "{err}");
	assert!(err.contains("`cargo check` never collects"), "{err}");
}
