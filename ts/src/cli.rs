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
	// `cargo clean` reads the profiles too.
	let env = [
		(config::env::CONFIG, args.config.as_os_str()),
		output,
		("CARGO_UNSTABLE_PROFILE_RUSTFLAGS", "true".as_ref()),
	];
	let mut cargo = std_from_source();
	cargo.extend(args.cargo);
	Check {
		tool: "rust2-ts",
		manifest: &args.manifest,
		package: &config.package,
		target: Some(TARGET),
		args: &cargo,
		env: &env,
	}
	.run()
}

/// Cargo args that build std from source, unwinding and with every function's MIR, so
/// the bodies of std's combinators lower as the crate's do, cleanups included: the
/// target's prebuilt std aborts, so its MIR has none. Set through the profile, which
/// cargo's fingerprint sees. `panic_abort` keeps aborting, since a `cdylib` dependency
/// links it.
fn std_from_source() -> Vec<String> {
	let mut args = vec!["-Zbuild-std=core,alloc,std,panic_unwind,panic_abort".to_string()];
	for krate in ["core", "alloc", "std"] {
		args.push("--config".into());
		args.push(format!(
			"profile.dev.package.{krate}.rustflags=[\"-Cpanic=unwind\", \"-Zalways-encode-mir\"]"
		));
	}
	args
}

