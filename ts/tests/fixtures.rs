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

/// Run the driver on a fixture, returning its output directory and what it printed.
fn translate(name: &str) -> (PathBuf, Output) {
	let out = scratch().join(name).join("out");
	let output = Command::new(env!("CARGO_BIN_EXE_rust2-ts"))
		.arg("--config")
		.arg(manifest_dir().join("fixtures").join(name).join("ts.toml"))
		.arg("--out")
		.arg(&out)
		.env("CARGO_TARGET_DIR", scratch().join("build"))
		.output()
		.expect("running rust2-ts");
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
				if rel != Path::new("rt.ts") {
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
	assert_eq!(names(&got), names(&want), "{name}: the generated files differ; rerun with UPDATE_EXPECT=1");
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
	let mut files: Vec<PathBuf> = sources(out).into_iter().map(|(rel, _)| out.join(rel)).collect();
	files.push(out.join("rt.ts"));
	let output = Command::new("tsc")
		.args([
			"--noEmit",
			"--strict",
			"--noUnusedLocals",
			"--noUnusedParameters",
			"--target",
			"es2022",
			"--module",
			"esnext",
			"--moduleResolution",
			"bundler",
			"--allowImportingTsExtensions",
			"--verbatimModuleSyntax",
			"--erasableSyntaxOnly",
		])
		.args(&files)
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
		.args(["build", "--locked", "--package", package, "--target", TARGET, "--target-dir"])
		.arg(&target_dir)
		.arg("--manifest-path")
		.arg(manifest_dir().join("Cargo.toml"))
		.status()
		.expect("running cargo");
	assert!(status.success(), "building {package} for {TARGET} failed");
	let wasm = target_dir.join(TARGET).join("debug").join(format!("{}.wasm", package.replace('-', "_")));
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
	assert!(output.status.success(), "{package} differs from native Rust:\n{stdout}{stderr}");
	println!("{stdout}");
}

#[test]
fn codec() {
	let out = translated("codec");
	snapshot("codec", &out);
	typecheck(&out);
	differential("codec-fixture", &out, 20_000);
}
