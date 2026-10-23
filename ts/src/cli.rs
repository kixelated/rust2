//! The command line: run cargo on the configured package with this binary as rustc.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::config::{self, Config};

/// Translation type-checks for this target: its 32-bit `usize` is what a JS number
/// holds exactly, and its `cfg`s are the browser's.
const TARGET: &str = "wasm32-unknown-unknown";

const USAGE: &str = "usage: rust2-ts --config <ts.toml> --out <dir> [-- <cargo args>]

Translates the library of the package that ts.toml names (its Cargo.toml is beside it)
to TypeScript in <dir>. Cargo args, such as --features, are passed to `cargo check`.";

pub fn run(args: &[String]) -> ExitCode {
	match try_run(args) {
		Ok(code) => code,
		Err(err) => {
			eprintln!("rust2-ts: {err}");
			ExitCode::FAILURE
		}
	}
}

fn try_run(args: &[String]) -> Result<ExitCode, String> {
	let mut config = None;
	let mut out = None;
	let mut cargo_args: &[String] = &[];
	let mut rest = args;
	while let [arg, tail @ ..] = rest {
		rest = tail;
		match arg.as_str() {
			"--config" | "--out" => {
				let [value, tail @ ..] = rest else {
					return Err(format!("{arg} needs a value\n\n{USAGE}"));
				};
				rest = tail;
				let slot = if arg == "--config" { &mut config } else { &mut out };
				*slot = Some(absolute(Path::new(value))?);
			}
			"--" => {
				cargo_args = rest;
				break;
			}
			"-h" | "--help" => {
				println!("{USAGE}");
				return Ok(ExitCode::SUCCESS);
			}
			_ => return Err(format!("unexpected argument {arg}\n\n{USAGE}")),
		}
	}
	let config_path = config.ok_or_else(|| format!("--config is required\n\n{USAGE}"))?;
	let out = out.ok_or_else(|| format!("--out is required\n\n{USAGE}"))?;
	let config = Config::load(&config_path)?;
	let manifest = config_path
		.parent()
		.expect("an absolute file path has a parent")
		.join("Cargo.toml");
	let exe = std::env::current_exe().map_err(|err| format!("finding this executable: {err}"))?;
	let target_dir = match std::env::var_os("CARGO_TARGET_DIR") {
		Some(dir) => PathBuf::from(dir),
		// Apart from the workspace's own builds, which a different rustc would invalidate.
		None => workspace_root(&manifest)?.join("target").join("rust2-ts"),
	};

	let package_dir = manifest.parent().expect("a manifest path has a parent").to_path_buf();
	let cargo = |subcommand: &str| {
		let mut cmd = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
		// Cargo reads its config from where it runs, so run where the package is.
		cmd.current_dir(&package_dir);
		cmd.arg(subcommand)
			.arg("--manifest-path")
			.arg(&manifest)
			.arg("--package")
			.arg(&config.package)
			.arg("--target")
			.arg(TARGET)
			.arg("--target-dir")
			.arg(&target_dir);
		cmd
	};

	// Cargo can't tell the TypeScript is stale, so the package always recompiles.
	run_cargo(cargo("clean").arg("--quiet"))?;
	let status = cargo("check")
		.arg("--lib")
		.args(cargo_args)
		.env("RUSTC_WRAPPER", &exe)
		.env(config::env::CONFIG, &config_path)
		.env(config::env::OUT, &out)
		.status()
		.map_err(|err| format!("running cargo: {err}"))?;
	Ok(match status.success() {
		true => ExitCode::SUCCESS,
		false => ExitCode::FAILURE,
	})
}

fn run_cargo(cmd: &mut Command) -> Result<(), String> {
	let status = cmd.status().map_err(|err| format!("running cargo: {err}"))?;
	match status.success() {
		true => Ok(()),
		false => Err(format!("{cmd:?} failed")),
	}
}

fn absolute(path: &Path) -> Result<PathBuf, String> {
	std::path::absolute(path).map_err(|err| format!("resolving {}: {err}", path.display()))
}

/// The directory of the workspace `manifest` belongs to.
fn workspace_root(manifest: &Path) -> Result<PathBuf, String> {
	let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
		.args([
			"locate-project",
			"--workspace",
			"--message-format",
			"plain",
			"--manifest-path",
		])
		.arg(manifest)
		.output()
		.map_err(|err| format!("running cargo: {err}"))?;
	if !output.status.success() {
		return Err(String::from_utf8_lossy(&output.stderr).into_owned());
	}
	let root = String::from_utf8(output.stdout).map_err(|_| "cargo printed a non-UTF-8 path".to_string())?;
	Ok(Path::new(root.trim())
		.parent()
		.expect("a manifest path has a parent")
		.to_path_buf())
}
