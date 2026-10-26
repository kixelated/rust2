//! The command line: run cargo on the configured package with this binary as rustc.

use std::process::ExitCode;

use rust2_driver::cargo::{Args, Check, Output};

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
	let Some(args) = Args::parse(args, USAGE)? else {
		return Ok(ExitCode::SUCCESS);
	};
	let config = Config::load(&args.config)?;
	let output = match &args.output {
		Output::Dir(out) => (config::env::OUT, out.as_os_str()),
		Output::Census(census) => (config::env::CENSUS, census.as_os_str()),
	};
	let env = [(config::env::CONFIG, args.config.as_os_str()), output];
	Check {
		tool: "rust2-ts",
		manifest: &args.manifest,
		package: &config.package,
		target: Some(TARGET),
		args: &args.cargo,
		env: &env,
	}
	.run()
}
