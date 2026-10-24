//! Each fixture under `fixtures/` is translated, compared with its `expected/` snapshot,
//! type-checked with `tsc`, and, if it has a harness, run against its native build.
//!
//! `UPDATE_EXPECT=1 cargo test` rewrites the snapshots.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const TARGET: &str = "wasm32-unknown-unknown";

fn manifest_dir() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scratch() -> PathBuf {
	PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
}

fn cargo() -> Command {
	Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}

/// Run the driver on a fixture with `args`, in its own build directory per `build`.
fn driver(name: &str, args: &[&std::ffi::OsStr], build: &str) -> Output {
	Command::new(env!("CARGO_BIN_EXE_rust2-ts"))
		.arg("--config")
		.arg(manifest_dir().join("fixtures").join(name).join("ts.toml"))
		.args(args)
		.env("CARGO_TARGET_DIR", scratch().join(build))
		.output()
		.expect("running rust2-ts")
}

/// Run the driver on a fixture, returning its output directory and what it printed.
fn translate(name: &str) -> (PathBuf, Output) {
	let out = scratch().join(name).join("out");
	let output = driver(name, &["--out".as_ref(), out.as_ref()], "build");
	(out, output)
}

fn translated(name: &str) -> PathBuf {
	let (out, output) = translate(name);
	assert!(
		output.status.success(),
		"translating {name} failed:\n{}",
		String::from_utf8_lossy(&output.stderr)
	);
	out
}

/// The generated TypeScript, by path relative to `dir`, without the runtime.
fn sources(dir: &Path) -> Vec<(PathBuf, String)> {
	let mut files = Vec::new();
	let mut stack = vec![dir.to_path_buf()];
	while let Some(next) = stack.pop() {
		let Ok(entries) = std::fs::read_dir(&next) else {
			continue;
		};
		for entry in entries {
			let path = entry.unwrap().path();
			if path.is_dir() {
				stack.push(path);
			} else if path.extension().is_some_and(|e| e == "ts") {
				let rel = path.strip_prefix(dir).unwrap().to_path_buf();
				if rel != Path::new("rt.ts") && !rel.starts_with("rt") {
					files.push((rel, std::fs::read_to_string(&path).unwrap()));
				}
			}
		}
	}
	files.sort();
	files
}

/// Compare the output with the fixture's `expected/` directory.
fn snapshot(name: &str, out: &Path) {
	let expected = manifest_dir().join("fixtures").join(name).join("expected");
	let got = sources(out);
	if std::env::var_os("UPDATE_EXPECT").is_some() {
		let _ = std::fs::remove_dir_all(&expected);
		for (rel, text) in &got {
			let path = expected.join(rel);
			std::fs::create_dir_all(path.parent().unwrap()).unwrap();
			std::fs::write(path, text).unwrap();
		}
		return;
	}
	let want = sources(&expected);
	let names = |files: &[(PathBuf, String)]| files.iter().map(|(p, _)| p.clone()).collect::<Vec<_>>();
	assert_eq!(
		names(&got),
		names(&want),
		"{name}: the generated files differ; rerun with UPDATE_EXPECT=1"
	);
	for ((rel, got), (_, want)) in got.iter().zip(&want) {
		assert!(
			got == want,
			"{name}: {} differs from its snapshot; rerun with UPDATE_EXPECT=1\n{}",
			rel.display(),
			diff(want, got)
		);
	}
}

/// The first lines where two texts differ.
fn diff(want: &str, got: &str) -> String {
	let (want, got): (Vec<&str>, Vec<&str>) = (want.lines().collect(), got.lines().collect());
	let start = want.iter().zip(&got).take_while(|(a, b)| a == b).count();
	let mut out = String::new();
	for line in want.iter().skip(start).take(8) {
		out.push_str(&format!("- {line}\n"));
	}
	for line in got.iter().skip(start).take(8) {
		out.push_str(&format!("+ {line}\n"));
	}
	out
}

/// Type-check the output as strict, erasable TypeScript, which Node runs as is.
fn typecheck(out: &Path) {
	// The runtime's settings: strict, and erasable so Node runs it as is.
	let runtime = manifest_dir().join("runtime").join("tsconfig.json");
	let config = format!(
		"{{ \"extends\": {:?}, \"files\": [], \"include\": [\"**/*.ts\"] }}\n",
		runtime.display().to_string()
	);
	std::fs::write(out.join("tsconfig.json"), config).unwrap();
	let output = Command::new("tsc")
		.arg("-p")
		.arg(out)
		.output()
		.expect("running tsc, which the dev shell provides");
	assert!(
		output.status.success(),
		"tsc rejected the output:\n{}",
		String::from_utf8_lossy(&output.stdout)
	);
}

/// Run the harness natively, as wasm32, and translated, on the same inputs.
fn differential(package: &str, out: &Path, cases: u32) {
	let target_dir = scratch().join("wasm");
	let status = cargo()
		.args([
			"build",
			"--locked",
			"--package",
			package,
			"--target",
			TARGET,
			"--target-dir",
		])
		.arg(&target_dir)
		.arg("--manifest-path")
		.arg(manifest_dir().join("Cargo.toml"))
		.status()
		.expect("running cargo");
	assert!(status.success(), "building {package} for {TARGET} failed");
	let wasm = target_dir
		.join(TARGET)
		.join("debug")
		.join(format!("{}.wasm", package.replace('-', "_")));
	std::fs::write(out.join("package.json"), "{ \"type\": \"module\" }\n").unwrap();
	let output = Command::new("node")
		.arg(manifest_dir().join("tests").join("differential.ts"))
		.arg(&wasm)
		.arg(out.join("harness.ts"))
		.arg(cases.to_string())
		.output()
		.expect("running node, which the dev shell provides");
	let stdout = String::from_utf8_lossy(&output.stdout);
	let stderr = String::from_utf8_lossy(&output.stderr);
	assert!(
		output.status.success(),
		"{package} differs from native Rust:\n{stdout}{stderr}"
	);
	println!("{stdout}");
}

#[test]
fn codec() {
	let out = translated("codec");
	snapshot("codec", &out);
	typecheck(&out);
	differential("codec-fixture", &out, 20_000);
}

#[test]
fn lang() {
	let out = translated("lang");
	snapshot("lang", &out);
	typecheck(&out);
	differential("lang-fixture", &out, 50_000);
}

/// Futures and drops: the harness is hand-written TS around the translation, whose FIFO
/// executor logs every poll, wake, and drop as the fixture's native one does.
#[test]
fn task() {
	let out = scratch().join("task").join("out");
	let glue = out.join("harness.ts");
	let _ = std::fs::remove_file(&glue);
	let out = translated("task");
	snapshot("task", &out);
	let harness = manifest_dir().join("tests").join("task.ts").display().to_string();
	let text = format!(
		r#"import * as rt from "./rt.ts";
import {{ Log }} from "./log.ts";
import {{ Channel, Guard, drain, sum }} from "./model.ts";
import {{ harness }} from {harness:?};

export {{ cases }} from {harness:?};
export const run = harness({{ Pending: rt.Pending, Log, Channel, Guard, sum, drain }});
"#
	);
	std::fs::write(&glue, text).unwrap();
	typecheck(&out);
	differential("task-fixture", &out, 20_000);
}

/// Each construct outside the subset fails with its span, and nothing is written.
#[test]
fn reject() {
	let (out, output) = translate("reject");
	assert!(!output.status.success(), "translating reject succeeded");
	assert!(!out.join("lib.ts").exists(), "a failed translation wrote output");
	let stderr = String::from_utf8_lossy(&output.stderr);
	// Each error and where it points, without rustc's rendering of the source.
	let mut got = String::new();
	let mut lines = stderr.lines().peekable();
	while let Some(line) = lines.next() {
		if let Some(error) = line.strip_prefix("error: rust2-ts: ") {
			got.push_str(error);
			if let Some(at) = lines.peek().and_then(|l| l.trim_start().strip_prefix("--> ")) {
				got.push_str(&format!(" at {at}"));
			}
			got.push('\n');
		}
	}
	let path = manifest_dir().join("fixtures").join("reject").join("expected.txt");
	if std::env::var_os("UPDATE_EXPECT").is_some() {
		std::fs::write(&path, &got).unwrap();
		return;
	}
	let want = std::fs::read_to_string(&path).unwrap_or_default();
	assert!(
		got == want,
		"the errors differ; rerun with UPDATE_EXPECT=1\n{}",
		diff(&want, &got)
	);
}

/// A census ranks every refusal in the crate, reached from each of its items, and
/// succeeds where translation fails.
#[test]
fn census() {
	let census = scratch().join("reject").join("census.md");
	let output = driver("reject", &["--census".as_ref(), census.as_ref()], "census");
	assert!(
		output.status.success(),
		"the census of reject failed:\n{}",
		String::from_utf8_lossy(&output.stderr)
	);
	let got = std::fs::read_to_string(&census).unwrap();
	let path = manifest_dir().join("fixtures").join("reject").join("census.md");
	if std::env::var_os("UPDATE_EXPECT").is_some() {
		std::fs::write(&path, &got).unwrap();
		return;
	}
	let want = std::fs::read_to_string(&path).unwrap_or_default();
	assert!(
		got == want,
		"the census differs; rerun with UPDATE_EXPECT=1\n{}",
		diff(&want, &got)
	);
}
