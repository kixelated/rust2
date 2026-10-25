//! The command line: run cargo on the configured package with this binary as rustc.

use std::path::Path;
use std::process::ExitCode;

use crate::config::{self, Config};

/// Translation type-checks for this target: its 32-bit `usize` is what a JS number
/// holds exactly, and its `cfg`s are the browser's.
const TARGET: &str = "wasm32-unknown-unknown";

const USAGE: &str = "usage: rust2-ts --config <ts.toml> (--out <dir> | --census <file>) [--manifest-path <Cargo.toml>] [-- <cargo args>]

Translates the library of the package that ts.toml names to TypeScript in <dir>. The
package's Cargo.toml is beside ts.toml unless --manifest-path says where. Cargo args,
such as --features, are passed to `cargo check`.

--census writes a Markdown report of every construct translation refuses in the library,
reached from each of its items as if each were a root, instead of translating it.";

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
	let mut census = None;
	let mut manifest = None;
	let mut cargo_args: &[String] = &[];
	let mut rest = args;
	while let [arg, tail @ ..] = rest {
		rest = tail;
		match arg.as_str() {
			"--config" | "--out" | "--census" | "--manifest-path" => {
				let [value, tail @ ..] = rest else {
					return Err(format!("{arg} needs a value\n\n{USAGE}"));
				};
				rest = tail;
				let slot = match arg.as_str() {
					"--config" => &mut config,
					"--out" => &mut out,
					"--census" => &mut census,
					_ => &mut manifest,
				};
				*slot = Some(rust2_driver::cargo::absolute(Path::new(value))?);
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
	let (output_env, output) = match (out, census) {
		(Some(out), None) => (config::env::OUT, out),
		(None, Some(census)) => (config::env::CENSUS, census),
		_ => return Err(format!("one of --out and --census is required\n\n{USAGE}")),
	};
	let config = Config::load(&config_path)?;
	let manifest = match manifest {
		Some(manifest) => manifest,
		None => config_path
			.parent()
			.expect("an absolute file path has a parent")
			.join("Cargo.toml"),
	};
	rust2_driver::cargo::Check {
		tool: "rust2-ts",
		manifest: &manifest,
		package: &config.package,
		target: Some(TARGET),
		args: cargo_args,
		env: &[
			(config::env::CONFIG, config_path.as_os_str()),
			(output_env, output.as_os_str()),
		],
	}
	.run()
}
