//! The command line: run cargo on the configured package with this binary as rustc.

use std::process::ExitCode;

use rust2_driver::cargo::{Args, Check, Output};

use crate::config::{self, Config};

const USAGE: &str = "usage: rust2-ffi --config <ffi.toml> (--out <dir> | --census <file>) [--manifest-path <Cargo.toml>] [-- <cargo args>]

Exports the pub API of the library of the package that ffi.toml names: a crate in <dir>
that builds the C ABI, with its C header in <dir>/include. The package's Cargo.toml is
beside ffi.toml unless --manifest-path says where. Cargo args, such as --features, are
passed to `cargo check`.

--census writes a Markdown report of what exports and of every construct the export
refuses, instead of exporting.";

pub fn run(args: &[String]) -> ExitCode {
	match try_run(args) {
		Ok(code) => code,
		Err(err) => {
			eprintln!("rust2-ffi: {err}");
			ExitCode::FAILURE
		}
	}
}

fn try_run(args: &[String]) -> Result<ExitCode, String> {
	let Some(args) = Args::parse(args, USAGE, &[])? else {
		return Ok(ExitCode::SUCCESS);
	};
	let config = Config::load(&args.config)?;
	let output = match &args.output {
		Output::Dir(out) => (config::env::OUT, out.as_os_str()),
		Output::Census(census) => (config::env::CENSUS, census.as_os_str()),
	};
	Check {
		tool: "rust2-ffi",
		manifest: &args.manifest,
		package: &config.package,
		// The host's: a C ABI is the platform's.
		target: None,
		args: &args.cargo,
		env: &[(config::env::CONFIG, args.config.as_os_str()), output],
	}
	.run()
}
