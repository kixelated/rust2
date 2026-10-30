//! The command line: run cargo on the configured package with this binary as rustc.

use std::path::Path;
use std::process::ExitCode;

use rust2_driver::cargo::{Args, Check, Output};

use crate::config::{self, Config};

/// Translation type-checks for this target: its 32-bit `usize` is what a JS number
/// holds exactly, and its `cfg`s are the browser's.
pub const TARGET: &str = "wasm32-unknown-unknown";

const USAGE: &str = "usage: rust2-ts --config <ts.toml> (--out <dir> | --census <file>) [--manifest-path <Cargo.toml>] [-- <cargo args>]

Translates the library of the package that ts.toml names to TypeScript in <dir>. The
package's Cargo.toml is beside ts.toml unless --manifest-path says where. Cargo args,
such as --features, are passed to `cargo check`, which checks with the dev profile.

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
	let mut cargo = profile_args()?;
	// The profile `PROFILE` sets, which cargo refuses to combine with one the cargo args
	// pick.
	cargo.extend(["--profile".into(), "dev".into()]);
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

/// The cargo config of the profile every crate compiles with, one `--config` per line.
const PROFILE: &str = include_str!("profile.toml");

/// The std crates `-Zbuild-std` checks, which flake.nix's sysroot holds too.
const STD: &str = "-Zbuild-std=core,alloc,std,panic_abort";

/// Cargo args that check std from source, unless [`sysroot`] has it checked already, and
/// set [`PROFILE`].
fn profile_args() -> Result<Vec<String>, String> {
	let mut args = Vec::new();
	if sysroot()?.is_none() {
		args.push(STD.to_string());
	}
	for line in PROFILE
		.lines()
		.filter(|line| !line.is_empty() && !line.starts_with('#'))
	{
		args.extend(["--config".to_string(), line.to_string()]);
	}
	Ok(args)
}

/// The sysroot of std checked with [`PROFILE`] for [`TARGET`], which rustc takes for the
/// target's crates instead of a `-Zbuild-std` in every target directory, if set.
pub fn sysroot() -> Result<Option<String>, String> {
	let dir = match std::env::var(config::env::SYSROOT) {
		Ok(dir) if !dir.is_empty() => dir,
		Err(std::env::VarError::NotUnicode(_)) => return Err(format!("{} is not UTF-8", config::env::SYSROOT)),
		_ => return Ok(None),
	};
	let lib = Path::new(&dir).join("lib/rustlib").join(TARGET).join("lib");
	if !lib.is_dir() {
		return Err(format!(
			"{} is {dir}, which holds no std for {TARGET} in {}",
			config::env::SYSROOT,
			lib.display()
		));
	}
	Ok(Some(dir))
}
